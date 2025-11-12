//! Startup wiring for PTT controller and runtime thread bootstrap.
use super::events::{emit_event, emit_log, with_events};
use super::state::{PTT, PttController, RUNTIME_TX, RuntimeCommand, runtime_state};
use crate::overlay;
use keyless_core::config::{Config, storage};
use keyless_core::error::KeylessResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Current epoch time in milliseconds.
fn now_millis() -> u64 {
    // Get current system time and convert to milliseconds since UNIX epoch.
    // Returns 0 if time calculation fails (shouldn't happen in practice).
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Initialize the runtime thread and PTT listener at app startup.
/// Only starts PTT if Accessibility permission is granted.
/// This function is idempotent - multiple calls are safe and will only initialize once.
pub fn init() -> KeylessResult<()> {
    let state = runtime_state();

    // Fast path: check if already initialized to avoid redundant work.
    if state.initialized.load(Ordering::Relaxed) {
        eprintln!("[runtime::init] Runtime already initialized, skipping");
        return Ok(());
    }

    // Prevent concurrent initialization attempts using compare-and-swap.
    // If another thread is already initializing, return early (idempotent behavior).
    if state
        .initializing
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        eprintln!("[runtime::init] Initialization already in progress, skipping");
        return Ok(());
    }

    // Load configuration (or use defaults if no config exists).
    let cfg: Config = storage::load_config().unwrap_or_default();

    // Create PTT (push-to-talk) primitives:
    // - hold_flag: atomic flag set when PTT hotkey is held (used by pipeline to gate audio).
    let hold_flag = Arc::new(AtomicBool::new(false));
    // - preset_mode: determines which hotkey combo to listen for (0 = control+option, 1 = control+shift).
    let preset_mode = Arc::new(std::sync::atomic::AtomicU8::new(
        if cfg.hotkey.to_lowercase().contains("shift") {
            1 // Use control+shift preset.
        } else {
            0 // Use control+option preset (default).
        },
    ));
    // - ptt_stop: flag to signal the listener thread to stop.
    let ptt_stop = Arc::new(AtomicBool::new(false));
    // - ptt_enabled: tracks whether PTT listener is currently enabled (only enabled after pipeline starts).
    let ptt_enabled = Arc::new(AtomicBool::new(false));
    // Initialize runtime state flags.
    state.ptt_enabled.store(false, Ordering::Relaxed);
    state.paste_guard_until_ms.store(0, Ordering::Relaxed);

    // Create channels for PTT communication (similar to TUI's Channels.control):
    // - tx_hold/rx_hold: channel for PTT hold state (true = pressed, false = released).
    let (tx_hold, rx_hold) = mpsc::sync_channel::<bool>(8);
    // - tx_evt/rx_evt: channel for heartbeat events (proves listener is alive).
    let (tx_evt, rx_evt) = mpsc::sync_channel::<()>(8);

    // Spawn a dispatch thread that bridges PTT events to UI and pipeline.
    // This thread runs continuously and processes hold state changes and heartbeats.
    let hold_for_dispatch = Arc::clone(&hold_flag);
    let dispatch = std::thread::spawn(move || {
        let state = runtime_state();
        let mut last_heartbeat: Option<Instant> = None; // Track last heartbeat time.
        let mut last_state = false; // Track last PTT hold state to detect edges.
        loop {
            let mut progressed = false; // Track if we processed any events this iteration.
            
            // Process heartbeat events (proves the PTT listener thread is alive).
            // Heartbeats are sent periodically by the listener when it's running.
            if rx_evt.try_recv().is_ok() {
                last_heartbeat = Some(Instant::now());
                // Update global heartbeat timestamp (used for permission warnings).
                state
                    .last_heartbeat_ms
                    .store(now_millis(), Ordering::Relaxed);
                progressed = true;
            }
            
            // Process PTT hold state changes (edge detection).
            // The listener thread sends hold state changes through this channel.
            if let Ok(held) = rx_hold.try_recv() {
                progressed = true;
                // Only process if state actually changed (edge detection).
                if held != last_state {
                    last_state = held;
                    // Update the hold flag that the pipeline reads (gates audio recording).
                    hold_for_dispatch.store(held, Ordering::Relaxed);
                    // Update global listening state (used by UI).
                    state.listening.store(held, Ordering::Relaxed);
                    // Emit status change event to frontend.
                    let status = if held { "listening" } else { "idle" }.to_string();
                    eprintln!("[ptt] {}", if held { "pressed" } else { "released" });
                    emit_log(if held { "PTT pressed" } else { "PTT released" });
                    emit_event("status_changed", status.clone());
                    emit_log(format!("status_changed: {}", status));
                    // Show/hide overlay based on hold state.
                    with_events(|events| {
                        let app_handle = events.app_handle();
                        if held {
                            // Show recording indicator overlay when PTT is held.
                            overlay::show_overlay(&app_handle);
                        }
                    });
                }
            }

            // Periodic heartbeat check: if PTT is enabled but no heartbeats received in 5 seconds,
            // log a warning (likely means Accessibility permission is missing).
            if let Some(ts) = last_heartbeat
                && ts.elapsed() > Duration::from_secs(5)
            {
                emit_log("PTT inactive: no events in 5s (check Accessibility)".to_string());
                // Reset heartbeat timestamp to avoid spamming logs (will warn again in 5s if still inactive).
                last_heartbeat = None;
            }

            // If no events processed this iteration, sleep briefly to avoid busy-waiting.
            if !progressed {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    });

    // Publish the PTT controller to the global state (accessible to other modules).
    let preset_mode_clone = Arc::clone(&preset_mode);
    let _ = PTT.set(Mutex::new(PttController {
        hold_flag: Arc::clone(&hold_flag),
        preset_mode: preset_mode_clone,
        stop_flag: ptt_stop,
        enabled: ptt_enabled,
        tx_hold,
        tx_evt,
        listener: None, // Listener thread will be spawned later (after pipeline starts).
        _dispatch: dispatch,
    }));

    // Log PTT configuration.
    emit_log("PTT listener will start after pipeline".to_string());
    let preset = if preset_mode.load(Ordering::Relaxed) == 1 {
        "control+shift"
    } else {
        "control+option"
    };
    emit_log(format!("PTT preset: {}", preset));

    // Spawn the runtime thread (handles pipeline lifecycle and command processing).
    // Create a command channel for sending commands to the runtime loop.
    let (tx_cmd, rx_cmd) = mpsc::channel::<RuntimeCommand>();
    // Store the sender globally (accessible via request_restart()).
    let _ = RUNTIME_TX.set(tx_cmd);
    // Get a clone of the hold_flag to pass to the runtime loop.
    let hold_flag_clone = if let Some(ptt_cell) = PTT.get() {
        match ptt_cell.lock() {
            Ok(ptt) => Arc::clone(&ptt.hold_flag),
            Err(_) => Arc::clone(&hold_flag), // Fallback if lock fails.
        }
    } else {
        Arc::clone(&hold_flag) // Fallback if PTT not yet set (shouldn't happen).
    };
    // Spawn the runtime loop thread (runs forever, manages pipeline lifecycle).
    std::thread::spawn(move || {
        super::runtime_loop(hold_flag_clone, rx_cmd);
    });

    // Mark initialization complete and release the initializing lock.
    // This allows future calls to init() to proceed (though they'll return early due to initialized check).
    state.initialized.store(true, Ordering::Release);
    state.initializing.store(false, Ordering::Release);
    Ok(())
}
