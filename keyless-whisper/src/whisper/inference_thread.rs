use std::sync::mpsc;

use candle_core::Device;
use tracing::error;

use keyless_core::events::TranscriptionEvent;

use crate::model::WhisperModel;

use super::types::InferReq;

/// Inference thread function: handles Partial and Final inference requests.
///
/// This thread:
/// - Receives inference requests (Partial or Final)
/// - Runs Whisper inference using the model
/// - Sends transcription events back via the event channel
pub(crate) fn run_inference_thread(
    mut model_components: WhisperModel,
    device: Device,
    inf_rx: mpsc::Receiver<InferReq>,
    tx_evt: mpsc::SyncSender<TranscriptionEvent>,
) {
    loop {
        // Blocking receive: inference thread waits for requests (serialized by size=1 channel).
        match inf_rx.recv() {
            Ok(InferReq::Partial(pcm)) => {
                // Run windowed inference (last 1s for preview; fast, low latency).
                let text = match crate::inference::run_inference_window(
                    &mut model_components,
                    &pcm,
                    &device,
                ) {
                    Ok(t) => t,
                    Err(e) => {
                        error!(error = %e, "inference error on partial");
                        // Return empty string on error (preview is best-effort).
                        String::new()
                    }
                };
                // Send Partial event (best-effort; ignore send failure).
                let _ = tx_evt.send(TranscriptionEvent::Partial(text));
            }
            Ok(InferReq::Final(mut pcm)) => {
                // Add 120ms silence tail (catches trailing speech after PTT release).
                // Some users release PTT slightly before finishing words; tail captures this.
                let tail = (0.12 * 16_000.0) as usize;
                let new_len = pcm.len() + tail;
                pcm.resize(new_len, 0.0f32);
                // Enforce minimum 3s length (Whisper works better with sufficient context).
                // Very short segments (< 3s) may produce lower quality transcriptions.
                let min_final = 16_000 * 3;
                if pcm.len() < min_final {
                    pcm.resize(min_final, 0.0f32);
                }
                // Run full inference (all windows, complete transcription).
                let text = match crate::inference::run_inference_full(
                    &mut model_components,
                    &pcm,
                    &device,
                ) {
                    Ok(t) => t,
                    Err(e) => {
                        error!(error = %e, "inference error on end-segment final");
                        // Return empty string on error (caller should handle gracefully).
                        String::new()
                    }
                };
                // Send Final event (best-effort; ignore send failure).
                let _ = tx_evt.send(TranscriptionEvent::Final(text));
            }
            // Channel disconnected: exit inference thread (clean shutdown).
            Err(_) => break,
        }
    }
}
