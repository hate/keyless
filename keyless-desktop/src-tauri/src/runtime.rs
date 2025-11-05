//! runtime thread for the desktop app.
//!
//! Manages the audio pipeline on a dedicated background thread to avoid
//! Send constraints (cpal Stream isn't Send). Commands are sent via channels.
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock, mpsc};

use keyless_core::config::{Config, storage};
use keyless_core::error::{KeylessError, KeylessResult};
use keyless_runtime::pipeline::{PipelineChannels, PipelineHandles};
use tauri::{AppHandle, Emitter};

/// Commands sent to the runtime thread.
enum RuntimeCommand {
    /// Start the pipeline (audio + whisper).
    Start,
    /// Stop the pipeline.
    Stop,
}

/// Global handle to the runtime thread.
static RUNTIME_HANDLE: OnceLock<RuntimeHandle> = OnceLock::new();

/// Handle to communicate with the runtime thread.
struct RuntimeHandle {
    /// Channel to send commands to the runtime thread.
    tx: mpsc::Sender<RuntimeCommand>,
}

/// Initialize the runtime thread at app startup.
pub fn init(app: AppHandle) -> KeylessResult<()> {
    let (tx, rx) = mpsc::channel::<RuntimeCommand>();
    let app_clone = app.clone();

    std::thread::spawn(move || {
        runtime_loop(app_clone, rx);
    });

    RUNTIME_HANDLE
        .set(RuntimeHandle { tx })
        .map_err(|_| KeylessError::System("runtime already initialized".into()))?;

    Ok(())
}

/// Start the pipeline.
pub fn start(app: &AppHandle) -> KeylessResult<()> {
    let handle = RUNTIME_HANDLE
        .get()
        .ok_or_else(|| KeylessError::System("runtime not initialized".into()))?;
    handle
        .tx
        .send(RuntimeCommand::Start)
        .map_err(|_| KeylessError::System("runtime thread disconnected".into()))?;
    let _ = app.emit("status_changed", "listening");
    Ok(())
}

/// Stop the pipeline.
pub fn stop(app: &AppHandle, _cancel: bool) -> KeylessResult<()> {
    let handle = RUNTIME_HANDLE
        .get()
        .ok_or_else(|| KeylessError::System("runtime not initialized".into()))?;
    handle
        .tx
        .send(RuntimeCommand::Stop)
        .map_err(|_| KeylessError::System("runtime thread disconnected".into()))?;
    let _ = app.emit("status_changed", "idle");
    Ok(())
}

/// Main runtime loop running on a dedicated thread.
fn runtime_loop(app: AppHandle, rx: mpsc::Receiver<RuntimeCommand>) {
    let mut pipeline: Option<PipelineHandles> = None;

    for cmd in rx {
        match cmd {
            RuntimeCommand::Start => {
                if pipeline.is_some() {
                    let _ = app.emit("log_message", "pipeline already running");
                    continue;
                }

                match start_pipeline_internal(&app) {
                    Ok(handles) => {
                        pipeline = Some(handles);
                        let _ = app.emit("log_message", "pipeline started");
                    }
                    Err(e) => {
                        let _ = app.emit("log_message", format!("start error: {}", e));
                    }
                }
            }
            RuntimeCommand::Stop => {
                if let Some(handles) = pipeline.take() {
                    let _ = handles.audio.stop();
                    handles.whisper.stop();
                    let _ = app.emit("log_message", "pipeline stopped");
                }
            }
        }
    }
}

/// Start the pipeline and spawn event bridge (called from runtime thread).
fn start_pipeline_internal(app: &AppHandle) -> KeylessResult<PipelineHandles> {
    let cfg: Config = storage::load_config().unwrap_or_default();
    let hold_flag = Arc::new(AtomicBool::new(true));

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
