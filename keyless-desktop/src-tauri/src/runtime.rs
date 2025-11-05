//! runtime thread for the desktop app.
//!
//! Manages the audio pipeline on a dedicated background thread to avoid
//! Send constraints (cpal Stream isn't Send). PTT listener is spawned at startup
//! and controls audio gating via hold_flag.
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock, mpsc};

use keyless_core::config::{Config, storage};
use keyless_core::error::{KeylessError, KeylessResult};
use keyless_runtime::pipeline::{PipelineChannels, PipelineHandles};
use tauri::{AppHandle, Emitter, Manager};

/// Global handle to the runtime thread.
static RUNTIME_HANDLE: OnceLock<RuntimeHandle> = OnceLock::new();

/// Tracks whether PTT is currently held (listening).
static LISTENING: AtomicBool = AtomicBool::new(false);

/// Handle to communicate with the runtime thread.
struct RuntimeHandle {
    /// Channel to update preset_mode when hotkey changes.
    tx_preset: mpsc::Sender<u8>,
}

/// Initialize the runtime thread and PTT listener at app startup.
pub fn init(app: AppHandle) -> KeylessResult<()> {
    let cfg: Config = storage::load_config().unwrap_or_default();

    // Create hold_flag that PTT listener will update
    let hold_flag = Arc::new(AtomicBool::new(false));

    // Determine preset_mode from hotkey string (0 = ctrl+option, 1 = ctrl+shift)
    let preset_mode = Arc::new(AtomicU8::new(if cfg.hotkey.contains("shift") {
        1
    } else {
        0
    }));

    let ptt_enabled = Arc::new(AtomicBool::new(true)); // Always enabled for desktop
    let ptt_stop = Arc::new(AtomicBool::new(false));

    // Create channels for PTT events
    let (tx_hold, rx_hold) = mpsc::sync_channel::<bool>(8);
    let (tx_ptt_evt, _rx_ptt_evt) = mpsc::sync_channel::<()>(8);

    // Create channel for preset_mode updates
    let (tx_preset, rx_preset) = mpsc::channel::<u8>();

    // Spawn PTT listener thread
    let ptt_handle = keyless_runtime::ptt::spawn_listener(
        Arc::clone(&preset_mode),
        Arc::clone(&ptt_enabled),
        Arc::clone(&ptt_stop),
        tx_hold,
        tx_ptt_evt,
    )
    .map_err(|e| KeylessError::System(format!("PTT listener failed: {}", e)))?;

    // Spawn runtime thread to manage pipeline and drain PTT events
    let app_clone = app.clone();
    let hold_flag_clone = Arc::clone(&hold_flag);
    std::thread::spawn(move || {
        runtime_loop(
            app_clone,
            hold_flag_clone,
            preset_mode,
            rx_hold,
            rx_preset,
            ptt_handle,
        );
    });

    RUNTIME_HANDLE
        .set(RuntimeHandle { tx_preset })
        .map_err(|_| KeylessError::System("runtime already initialized".into()))?;

    Ok(())
}

/// Update the hotkey preset mode (called when user changes hotkey in settings).
pub fn update_preset_mode(hotkey: &str) -> KeylessResult<()> {
    let handle = RUNTIME_HANDLE
        .get()
        .ok_or_else(|| KeylessError::System("runtime not initialized".into()))?;

    let mode = if hotkey.contains("shift") { 1 } else { 0 };
    handle
        .tx_preset
        .send(mode)
        .map_err(|_| KeylessError::System("runtime thread disconnected".into()))?;

    Ok(())
}

/// Query whether the runtime is currently listening (PTT held).
pub fn is_listening() -> bool {
    LISTENING.load(Ordering::Relaxed)
}

/// Main runtime loop running on a dedicated thread.
fn runtime_loop(
    app: AppHandle,
    hold_flag: Arc<AtomicBool>,
    preset_mode: Arc<AtomicU8>,
    rx_hold: mpsc::Receiver<bool>,
    rx_preset: mpsc::Receiver<u8>,
    _ptt_handle: std::thread::JoinHandle<()>,
) {
    // Start pipeline once and keep it running (PTT gates audio via hold_flag)
    let _pipeline = match start_pipeline_internal(&app, Arc::clone(&hold_flag)) {
        Ok(handles) => {
            let _ = app.emit("log_message", "pipeline started");
            Some(handles)
        }
        Err(e) => {
            let _ = app.emit("log_message", format!("pipeline start error: {}", e));
            None
        }
    };

    // Drain PTT hold events and update hold_flag + status
    loop {
        // Check for preset_mode updates (hotkey changed in settings)
        if let Ok(mode) = rx_preset.try_recv() {
            preset_mode.store(mode, Ordering::Relaxed);
        }

        // Drain PTT hold events (true = pressed, false = released)
        if let Ok(held) = rx_hold.try_recv() {
            hold_flag.store(held, Ordering::Relaxed);
            LISTENING.store(held, Ordering::Relaxed);

            let status = if held { "listening" } else { "idle" };
            let _ = app.emit("status_changed", status);

            // Show/hide pill HUD based on PTT state
            if held {
                if let Some(win) = app.get_webview_window("pill") {
                    let _ = win.show();
                }
            } else {
                if let Some(win) = app.get_webview_window("pill") {
                    let _ = win.hide();
                }
            }
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Start the pipeline and spawn event bridge (called from runtime thread).
fn start_pipeline_internal(
    app: &AppHandle,
    hold_flag: Arc<AtomicBool>,
) -> KeylessResult<PipelineHandles> {
    let cfg: Config = storage::load_config().unwrap_or_default();

    let (tx_level, rx_level) = mpsc::sync_channel(8);
    let (tx_log, rx_log) = mpsc::sync_channel(64);
    let (tx_spec, rx_spec) = mpsc::sync_channel(8);
    let (tx_preview, rx_preview) = mpsc::sync_channel(64);
    let (tx_vad, rx_vad) = mpsc::sync_channel(8);

    let channels = PipelineChannels {
        tx_level,
        tx_log,
        tx_spec,
        tx_preview,
        tx_vad,
    };

    let handles = keyless_runtime::pipeline::start_pipeline(
        &cfg,
        cfg.eq.clone(),
        cfg.vad.clone(),
        channels,
        hold_flag,
        None,
    )?;

    // Spawn event bridge to stream to frontend
    let app_clone = app.clone();
    std::thread::spawn(move || {
        bridge_events(app_clone, rx_level, rx_log, rx_spec, rx_preview, rx_vad);
    });

    Ok(handles)
}

/// Bridge pipeline events to Tauri frontend.
fn bridge_events(
    app: AppHandle,
    rx_level: mpsc::Receiver<u16>,
    rx_log: mpsc::Receiver<String>,
    rx_spec: mpsc::Receiver<Vec<u16>>,
    rx_preview: mpsc::Receiver<String>,
    rx_vad: mpsc::Receiver<bool>,
) {
    loop {
        let mut any_alive = false;

        if let Ok(level) = rx_level.try_recv() {
            any_alive = true;
            let _ = app.emit("audio_level", level);
        }
        if let Ok(msg) = rx_log.try_recv() {
            any_alive = true;
            let _ = app.emit("log_message", msg);
        }
        if let Ok(bars) = rx_spec.try_recv() {
            any_alive = true;
            let _ = app.emit("eq_update", bars);
        }
        if let Ok(txt) = rx_preview.try_recv() {
            any_alive = true;
            let _ = app.emit("transcript_partial", txt);
        }
        if let Ok(speaking) = rx_vad.try_recv() {
            any_alive = true;
            let _ = app.emit("vad_speaking", speaking);
        }

        if !any_alive {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
