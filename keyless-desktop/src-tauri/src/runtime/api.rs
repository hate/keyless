//! Public API for controlling and querying the runtime thread.
//!
//! These functions provide a safe interface for the main thread to interact with the runtime
//! thread without direct access to its internal state. All operations are thread-safe.

use super::state::{RUNTIME_TX, RuntimeCommand, runtime_state};
use std::sync::atomic::Ordering;

/// Request the runtime to restart the pipeline.
///
/// This is typically called after configuration changes that require a pipeline restart
/// (e.g., device change, model change). The restart happens asynchronously on the runtime
/// thread. If the runtime thread hasn't been initialized, this is a no-op.
pub fn request_restart() {
    // Get the command channel sender (if runtime thread is initialized).
    if let Some(tx) = RUNTIME_TX.get() {
        // Send restart command to the runtime loop (non-blocking).
        // The runtime loop will process this command and restart the pipeline.
        let _ = tx.send(RuntimeCommand::RestartPipeline);
    }
    // If RUNTIME_TX is not set, the runtime thread hasn't been initialized yet.
    // This is safe to ignore (no-op).
}

/// Query whether the audio pipeline is currently active.
///
/// Returns `true` if the pipeline is running and processing audio, `false` otherwise.
/// This is useful for UI state management (e.g., showing "listening" vs "idle" status).
pub fn is_pipeline_active() -> bool {
    // Read the pipeline_active flag from the runtime state.
    // This is updated by the runtime loop when the pipeline starts/stops.
    runtime_state().pipeline_active.load(Ordering::Relaxed)
}

/// Whether the runtime thread has been initialized.
///
/// Returns `true` if `init()` has been called and the runtime thread is running, `false`
/// otherwise. This can be used to check if runtime operations are available.
pub fn is_initialized() -> bool {
    // Read the initialized flag from the runtime state.
    // This is set to true after init() successfully spawns the runtime thread.
    runtime_state().initialized.load(Ordering::Relaxed)
}
