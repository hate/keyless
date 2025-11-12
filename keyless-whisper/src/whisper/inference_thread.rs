use std::sync::mpsc;

use candle_core::Device;
use tracing::error;

use keyless_core::events::TranscriptionEvent;

use crate::inference::voiced::{VoicedCfg, build_voiced_spans};
use crate::inference::{
    decode_voiced_units,
    params::{FINAL_OVERLAP_SAMPLES, FINAL_UNIT_SAMPLES, FINAL_UNIT_SECONDS, WHISPER_SAMPLE_RATE},
    remove_overlap,
    voiced::units_from_spans,
};
use crate::model::WhisperModel;

use super::preview_cache::PreviewCache;
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
    // Simple per-unit preview cache: (start,end,tail_hash) -> text
    let mut preview_cache = PreviewCache::new(crate::inference::PREVIEW_HASH_TAIL);

    loop {
        // Blocking receive: inference thread waits for requests (serialized by size=1 channel).
        match inf_rx.recv() {
            // Partial variant removed; Previews now use voiced-mask pipeline below.
            Ok(InferReq::Preview(pcm)) => {
                // Unified preview: run the same voiced-mask pipeline as Finals,
                // but reuse cached unit texts and only decode changed tail units.
                let spans = build_voiced_spans(&pcm, WHISPER_SAMPLE_RATE, &VoicedCfg::default());
                let spans = if spans.is_empty() {
                    std::iter::once(0..pcm.len()).collect::<Vec<_>>()
                } else {
                    spans
                };
                let units = units_from_spans(&spans, FINAL_UNIT_SAMPLES, FINAL_OVERLAP_SAMPLES);

                let mut out = String::new();
                let mut reused_units = 0usize;
                let mut decoded_units = 0usize;
                for u in units.iter() {
                    if u.is_empty() || u.start >= pcm.len() {
                        continue;
                    }
                    let end = u.end.min(pcm.len());
                    if end <= u.start {
                        continue;
                    }
                    let (mut text, reused) = match preview_cache.get_or_decode(u, &pcm, || {
                        decode_voiced_units(
                            &mut model_components,
                            &pcm,
                            std::slice::from_ref(u),
                            &device,
                        )
                    }) {
                        Ok(res) => res,
                        Err(e) => {
                            tracing::debug!(error = %e, "preview decode failed for unit");
                            (String::new(), false)
                        }
                    };
                    if reused {
                        reused_units += 1;
                    } else {
                        decoded_units += 1;
                    }
                    if text.is_empty() {
                        continue;
                    }
                    if !out.is_empty() {
                        remove_overlap(&out, &mut text);
                    }
                    if text.is_empty() {
                        continue;
                    }
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(&text);
                }
                tracing::debug!(
                    unit_seconds = FINAL_UNIT_SECONDS,
                    units_total = units.len(),
                    units_reused = reused_units,
                    units_decoded = decoded_units,
                    preview_len = out.len(),
                    "preview units processed"
                );
                let _ = tx_evt.send(TranscriptionEvent::Partial(out));
            }
            Ok(InferReq::Final(mut pcm)) => {
                // Add 120ms silence tail (catches trailing speech after PTT release).
                let tail = (0.12 * WHISPER_SAMPLE_RATE as f32) as usize;
                let new_len = pcm.len() + tail;
                pcm.resize(new_len, 0.0f32);

                let spans = build_voiced_spans(&pcm, WHISPER_SAMPLE_RATE, &VoicedCfg::default());
                let spans = if spans.is_empty() {
                    std::iter::once(0..pcm.len()).collect::<Vec<_>>()
                } else {
                    spans
                };

                // Build smaller units (≤10s) with 0.5s overlap to improve SNR and boundary quality.
                let units = units_from_spans(&spans, FINAL_UNIT_SAMPLES, FINAL_OVERLAP_SAMPLES);

                let text = match decode_voiced_units(&mut model_components, &pcm, &units, &device) {
                    Ok(t) => t,
                    Err(e) => {
                        error!(error = %e, "inference error on end-segment final");
                        String::new()
                    }
                };
                tracing::debug!(
                    unit_seconds = FINAL_UNIT_SECONDS,
                    units_total = units.len(),
                    units_reused = 0,
                    units_decoded = units.len(),
                    preview_len = text.len(),
                    "final units processed"
                );
                // Clear preview cache after Final to avoid stale reuse next session.
                preview_cache.clear();
                // Send Final event (best-effort; ignore send failure).
                let _ = tx_evt.send(TranscriptionEvent::Final(text));
            }
            // Channel disconnected: exit inference thread (clean shutdown).
            Err(_) => break,
        }
    }
}
