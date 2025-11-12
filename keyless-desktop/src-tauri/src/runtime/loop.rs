//! Runtime main loop driving pipeline lifecycle and command handling.
use super::events::{emit_event, emit_log, with_events};
use super::ptt::ensure_ptt_listener_started;
use super::state::{PTT, RuntimeCommand, runtime_state};
use crate::overlay;
use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc};
use std::thread;

/// Background loop that processes control commands and keeps the pipeline alive.
pub fn runtime_loop(
    hold_flag: Arc<std::sync::atomic::AtomicBool>,
    rx_cmd: mpsc::Receiver<RuntimeCommand>,
) {
    // Get the runtime state for updating pipeline status flags.
    let state = runtime_state();

    // Attempt to start the pipeline immediately on thread startup.
    // The pipeline handles audio input, Whisper transcription, and output sink.
    let mut pipeline = match super::start_pipeline_internal(Arc::clone(&hold_flag)) {
        Ok(handles) => {
            // Pipeline started successfully: update state and enable PTT.
            state.pipeline_active.store(true, Ordering::Relaxed);
            eprintln!("[runtime] pipeline started");
            emit_log("pipeline started".to_string());

            // Start the PTT listener now that the pipeline is active.
            // The listener monitors global hotkey events and sends hold state to the pipeline.
            ensure_ptt_listener_started();

            // Enable PTT in the controller and global state.
            if let Some(ptt_cell) = PTT.get()
                && let Ok(ptt) = ptt_cell.lock()
            {
                ptt.enabled.store(true, Ordering::Relaxed);
            }
            state.ptt_enabled.store(true, Ordering::Relaxed);
            emit_log("PTT enabled".to_string());
            Some(handles) // Store handles for later cleanup/restart.
        }
        Err(e) => {
            // Pipeline failed to start (likely missing permissions or invalid config).
            let msg = format!("pipeline start error: {}", e);
            emit_log(msg);
            // Mark pipeline as inactive and keep PTT disabled (can't use PTT without pipeline).
            state.pipeline_active.store(false, Ordering::Relaxed);
            state.ptt_enabled.store(false, Ordering::Relaxed);
            None // No handles to store (pipeline not running).
        }
    };

    // Main event loop: processes commands and keeps the thread alive.
    // This loop runs forever, handling restart commands and maintaining pipeline lifecycle.
    loop {
        // Process all pending commands without blocking (non-blocking receive).
        // Commands are typically restart requests triggered by config changes.
        while let Ok(cmd) = rx_cmd.try_recv() {
            match cmd {
                RuntimeCommand::RestartPipeline => {
                    emit_log("restart requested".to_string());

                    // Stop the current pipeline components gracefully.
                    // This releases audio resources and stops the Whisper model.
                    if let Some(handles) = pipeline.take() {
                        // Stop audio input stream.
                        let _ = handles.audio.stop();
                        // Stop Whisper transcription.
                        handles.whisper.stop();
                    }
                    // Mark pipeline as inactive.
                    state.pipeline_active.store(false, Ordering::Relaxed);

                    // Disable PTT during restart to avoid race conditions.
                    // We don't want PTT events while the pipeline is restarting.
                    if let Some(ptt_cell) = PTT.get()
                        && let Ok(p) = ptt_cell.lock()
                    {
                        p.enabled.store(false, Ordering::Relaxed);
                        p.hold_flag.store(false, Ordering::Relaxed); // Clear any held state.
                    }
                    state.ptt_enabled.store(false, Ordering::Relaxed);

                    // Notify frontend that status changed to idle.
                    emit_event("status_changed", "idle");
                    // Hide overlays during restart (clean UI state).
                    with_events(|events| {
                        overlay::hide_overlay(&events.app_handle());
                        overlay::hide_toast(&events.app_handle());
                    });
                    emit_log("PTT disabled".to_string());

                    // Restart the pipeline with the latest configuration.
                    // This reloads the config, so device/model/sink changes are applied.
                    pipeline = match super::start_pipeline_internal(Arc::clone(&hold_flag)) {
                        Ok(handles) => {
                            // Pipeline restarted successfully.
                            state.pipeline_active.store(true, Ordering::Relaxed);
                            emit_log("pipeline restarted".to_string());

                            // Ensure PTT listener is started (might not be if it failed before).
                            ensure_ptt_listener_started();

                            // Re-enable PTT now that pipeline is active again.
                            if let Some(ptt_cell) = PTT.get()
                                && let Ok(ptt) = ptt_cell.lock()
                            {
                                ptt.enabled.store(true, Ordering::Relaxed);
                            }
                            state.ptt_enabled.store(true, Ordering::Relaxed);
                            emit_log("PTT enabled".to_string());
                            Some(handles) // Store new handles.
                        }
                        Err(e) => {
                            // Restart failed (config might be invalid, permissions missing, etc.).
                            let msg = format!("pipeline restart error: {}", e);
                            emit_log(msg);
                            // Keep PTT disabled since pipeline isn't running.
                            state.pipeline_active.store(false, Ordering::Relaxed);
                            state.ptt_enabled.store(false, Ordering::Relaxed);
                            None // No handles (pipeline not running).
                        }
                    };
                }
            }
        }

        // Sleep briefly to avoid busy-waiting (100ms is sufficient for command processing).
        thread::sleep(std::time::Duration::from_millis(100));

        // Keep hold_flag alive (prevent it from being dropped).
        // The pipeline needs this Arc to stay alive for the lifetime of the runtime thread.
        let _ = &hold_flag;

        // Keep PTT controller alive by touching a flag (prevents it from being dropped).
        // This ensures the PTT controller and its listener thread stay alive.
        if let Some(ptt_cell) = PTT.get()
            && let Ok(ptt) = ptt_cell.lock()
        {
            let _ = ptt.stop_flag.load(Ordering::Relaxed);
        }
    }
}
