use keyless_core::error::KeylessError;
use keyless_core::error::KeylessResult;
use keyless_core::events::TranscriptionEvent;

use crate::transcriber::RealtimeTranscriber;

use super::types::{Whisper, WhisperCmd};

impl RealtimeTranscriber for Whisper {
    fn push_audio(&mut self, pcm_f32: &[f32]) -> KeylessResult<()> {
        // Non-blocking send: try_send fails if queue is full (backpressure).
        // Clone frame (Vec<f32>) for worker thread (audio thread doesn't block).
        match self.tx.try_send(pcm_f32.to_vec()) {
            Ok(()) => Ok(()),
            // Queue full: worker thread is overloaded; return error (caller should back off).
            Err(_) => Err(keyless_core::error::KeylessError::Whisper(
                "transcriber queue full".to_string(),
            )),
        }
    }

    fn try_next_event(&mut self) -> Option<TranscriptionEvent> {
        // Non-blocking receive: returns None if no events available.
        // Should be called frequently (every audio frame) to drain events and prevent backpressure.
        self.rx_evt.try_recv().ok()
    }

    fn end_segment(&mut self) -> KeylessResult<()> {
        // Blocking send: ensures command is queued (don't want to lose end_segment()).
        // Unbounded channel won't block unless receiver is dropped (worker shutdown).
        match self.cmd_tx.send(WhisperCmd::EndSegment) {
            Ok(()) => Ok(()),
            // Channel disconnected: worker thread already shut down.
            Err(e) => Err(KeylessError::Whisper(format!(
                "failed to send EndSegment: {}",
                e
            ))),
        }
    }
}

impl Drop for Whisper {
    fn drop(&mut self) {
        // Signal worker to shutdown (non-blocking; best-effort).
        // Worker checks shutdown_rx in main loop and exits gracefully.
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Drop inference sender to stop inference thread (channel disconnect signals exit).
        // inf_tx is cloned; dropping original causes inf_rx.recv() to return Err.
        // Inference thread exits when it sees channel disconnect.
        // Wait for worker to exit cleanly (join ensures no resource leaks).
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
        // Wait for inference worker to exit cleanly (ensures model cleanup).
        if let Some(h) = self.inf_worker.take() {
            let _ = h.join();
        }
    }
}
