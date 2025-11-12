//! PTT (push-to-talk) preset handling and listener lifecycle helpers.
use super::events::emit_log;
use super::state::{PTT, runtime_state};
use keyless_core::config::{Config, storage};
use keyless_runtime::ptt;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Update PTT preset from the current config (control+option vs control+shift).
/// Live updates the atomic read by the rdev backend without respawning.
pub fn update_ptt_selection() {
    // Load current configuration to determine which hotkey preset to use.
    let cfg: Config = storage::load_config().unwrap_or_default();
    // Determine preset mode: 1 = control+shift, 0 = control+option (default).
    let mode = if cfg.hotkey.to_lowercase().contains("shift") {
        1
    } else {
        0
    };
    // Update the preset mode atomically (the listener thread reads this value).
    if let Some(ptt_cell) = PTT.get()
        && let Ok(ptt) = ptt_cell.lock()
    {
        let prev = ptt.preset_mode.load(Ordering::Relaxed);
        // Only update and log if the mode actually changed.
        if prev != mode {
            ptt.preset_mode.store(mode, Ordering::Relaxed);
            let label = if mode == 1 {
                "control+shift"
            } else {
                "control+option"
            };
            emit_log(format!("PTT preset changed to {}", label));
        }
    }
}

/// Query whether the runtime is currently listening (PTT held).
pub fn is_listening() -> bool {
    // Read the listening flag from runtime state (set by the dispatch thread when PTT is held).
    runtime_state().listening.load(Ordering::Relaxed)
}

/// Start the rdev listener if allowed and not already started.
pub fn ensure_ptt_listener_started() {
    // Don't start the listener until the pipeline is active (avoids race conditions).
    if !runtime_state().pipeline_active.load(Ordering::Relaxed) {
        emit_log("PTT listener pending: pipeline still starting".to_string());
        return;
    }

    // Get the PTT controller and check if listener is already started.
    if let Some(ptt_cell) = PTT.get() {
        let preset_mode;
        let enabled;
        let stop_flag;
        let tx_hold;
        let tx_evt;
        {
            // Lock the PTT controller and extract the values needed to spawn the listener.
            if let Ok(ptt) = ptt_cell.lock() {
                // If listener already exists, don't spawn another one.
                if ptt.listener.is_some() {
                    return;
                }
                // Clone Arc references needed for the listener thread.
                preset_mode = Arc::clone(&ptt.preset_mode);
                enabled = Arc::clone(&ptt.enabled);
                stop_flag = Arc::clone(&ptt.stop_flag);
                tx_hold = ptt.tx_hold.clone();
                tx_evt = ptt.tx_evt.clone();
            } else {
                // Lock failed (poisoned), can't proceed.
                return;
            }
        }

        // Spawn the rdev listener thread (monitors global hotkey events).
        // The listener reads preset_mode to determine which hotkey combo to listen for.
        let handle = match ptt::spawn_listener(preset_mode, enabled, stop_flag, tx_hold, tx_evt) {
            Ok(h) => h, // Success: got the thread handle.
            Err(e) => {
                // Failed to spawn listener (likely missing Accessibility permission).
                let msg = format!("PTT listener failed: {}", e);
                eprintln!("[runtime] {}", msg);
                emit_log(msg);
                // Return a dummy handle to avoid panics (listener won't work, but app continues).
                std::thread::spawn(|| {})
            }
        };

        // Store the listener thread handle in the PTT controller.
        // Double-check that listener is still None (another thread might have started it).
        if let Some(ptt_cell) = PTT.get()
            && let Ok(mut ptt) = ptt_cell.lock()
            && ptt.listener.is_none()
        {
            ptt.listener = Some(handle);
            emit_log("PTT listener started".to_string());
        }
    }
}
