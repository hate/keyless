//! Whisper transcriber worker.

use std::sync::{Arc, Mutex};

use keyless_core::error::{KeylessError, KeylessResult};
use keyless_whisper::{PhaseState, Whisper, WhisperConfig, WhisperLoadPhase};

/// Handle to the Whisper transcriber worker (Send via Arc/Mutex).
#[derive(Clone)]
pub struct WhisperHandle {
    /// Inner Whisper instance wrapped for thread-safe access.
    inner: Arc<Mutex<Whisper>>,
}

impl WhisperHandle {
    /// Get a clone of the inner `Arc<Mutex<Whisper>>` for cross-thread use.
    pub fn inner_clone(&self) -> Arc<Mutex<Whisper>> {
        self.inner.clone()
    }
    /// Stop the worker by consuming the handle (drop signals shutdown).
    pub fn stop(self) {
        // Dropping the Arc will signal the worker to stop (via its Drop/shutdown)
    }
}

/// Start the Whisper transcriber without progress callbacks.
pub fn start_whisper(cfg: WhisperConfig) -> KeylessResult<WhisperHandle> {
    let t = Whisper::new(cfg).map_err(|e| KeylessError::Whisper(e.to_string()))?;
    Ok(WhisperHandle {
        inner: Arc::new(Mutex::new(t)),
    })
}

/// Start the Whisper transcriber and forward load progress to the callback.
pub fn start_whisper_with_progress<F>(
    cfg: WhisperConfig,
    progress: Option<F>,
) -> KeylessResult<WhisperHandle>
where
    F: FnMut(WhisperLoadPhase, PhaseState),
{
    let t = Whisper::new_with_progress(cfg, progress)
        .map_err(|e| KeylessError::Whisper(e.to_string()))?;
    Ok(WhisperHandle {
        inner: Arc::new(Mutex::new(t)),
    })
}
