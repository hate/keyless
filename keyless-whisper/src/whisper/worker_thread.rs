use std::sync::mpsc;
use std::time::{Duration, Instant};

use rubato::Resampler;
use tracing::error;

use crate::config::WhisperConfig;

use super::types::{InferReq, WhisperCmd};

/// Worker thread function: handles audio resampling, accumulation, and partial previews.
///
/// This thread:
/// - Receives audio frames from the audio thread
/// - Resamples from source rate to 16 kHz if needed
/// - Accumulates audio in a buffer
/// - Emits partial previews periodically
/// - Handles end_segment() commands
pub(crate) fn run_worker_thread(
    cfg: WhisperConfig,
    rx: mpsc::Receiver<Vec<f32>>,
    shutdown_rx: mpsc::Receiver<()>,
    cmd_rx: mpsc::Receiver<WhisperCmd>,
    inf_tx_worker: mpsc::SyncSender<InferReq>,
) {
    // Set up resampling if the microphone doesn't run at 16 kHz
    //
    // Why resample: Whisper expects exactly 16,000 samples per second (16 kHz). If the mic
    // is 48 kHz, we have too many samples. If it's 44.1 kHz, we have the wrong count.
    //
    // How rubato works: Uses FFT-based sinc interpolation with anti-aliasing. This prevents
    // high-frequency content from "folding back" (aliasing) when we downsample.
    //
    // Think of it like: resizing an image. You can't just delete pixels—you need to blend
    // nearby values smoothly. Same idea for audio.
    let needs_resample = cfg.source_sample_hz != 16000;
    let mut resampler: Option<rubato::FftFixedInOut<f32>> = if needs_resample {
        tracing::info!(
            source_hz = cfg.source_sample_hz,
            target_hz = 16_000,
            "initializing rubato resampler"
        );
        match rubato::FftFixedInOut::<f32>::new(
            cfg.source_sample_hz as usize,
            16_000,
            1024, // chunk size (balanced for latency/quality)
            1,    // mono
        ) {
            Ok(r) => Some(r),
            Err(e) => {
                error!(error = ?e, "rubato init failed; falling back to passthrough");
                None
            }
        }
    } else {
        tracing::info!("no resampling needed (source is 16 kHz)");
        None
    };

    // Processing loop state.
    // input_accum: source-rate (e.g., 48 kHz) accumulator used to feed rubato in fixed-size chunks.
    // Rubato requires exact chunk sizes (1024 samples); accumulate until we have enough.
    let mut input_accum: Vec<f32> = Vec::new();
    // buffer: 16 kHz domain accumulation used by inference (resampled audio).
    // Grows during speaking segment; cleared on end_segment().
    let mut buffer: Vec<f32> = Vec::new();
    // Preview cadence: emit every ~0.1s of new audio (1600 samples at 16 kHz).
    // Partial stride: minimum new samples before emitting next preview.
    let partial_stride = 16_000 / 10; // 0.1s partials
    let mut last_partial_emit_at: usize = 0;
    // Time gate: prevent partial spam (minimum 120ms between previews).
    let mut last_partial_time: Instant = Instant::now();

    // Main loop: receive audio, resample, accumulate, infer.
    loop {
        // Check shutdown signal (non-blocking; allows graceful exit).
        if shutdown_rx.try_recv().is_ok() {
            break;
        }
        // Check commands (non-blocking; allows end_segment() to interrupt audio processing).
        if let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                WhisperCmd::EndSegment => {
                    // Clone buffer for final inference (worker continues, inference runs async).
                    let pcm = buffer.clone();
                    // Send final request (blocking; ensures request is queued).
                    if let Err(e) = inf_tx_worker.send(InferReq::Final(pcm)) {
                        error!(error = %e, "failed to send final inference request");
                    }
                    // Clear buffer for next segment (start fresh after PTT release).
                    buffer.clear();
                    last_partial_emit_at = 0;
                    last_partial_time = Instant::now();
                }
            }
        }
        // Receive audio frame with timeout (allows periodic shutdown/cmd checks).
        // Timeout: 1s (balance between responsiveness and CPU usage).
        let frame = match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(f) => f,
            // Timeout: loop back to check shutdown/cmd (keeps loop responsive).
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            // Disconnected: audio thread stopped, exit worker.
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if let Some(r) = resampler.as_mut() {
            // Accumulate source-rate samples, feed rubato with the next requested block size.
            // Rubato needs exact chunk sizes (1024 samples at source rate); accumulate until enough.
            input_accum.extend_from_slice(&frame);
            loop {
                // Query rubato: how many input frames needed for next output chunk?
                let need = r.input_frames_next();
                // If we don't have enough, break and wait for next frame.
                if input_accum.len() < need {
                    break;
                }
                // Take exactly the number of frames rubato expects this call.
                // drain(0..need) removes from front (FIFO order preserves audio continuity).
                let chunk: Vec<f32> = input_accum.drain(0..need).collect();
                match r.process(&[&chunk], None) {
                    Ok(out) => {
                        let out0 = &out[0];
                        // One-time confirmation log for sizes/ratio (debugging resampling).
                        static ONCE: std::sync::Once = std::sync::Once::new();
                        ONCE.call_once(|| {
                            tracing::info!(
                                in_len = need,
                                out_len = out0.len(),
                                "rubato processed chunk"
                            );
                        });
                        // Append resampled output to 16 kHz buffer (ready for inference).
                        buffer.extend_from_slice(out0);
                    }
                    Err(e) => {
                        error!(error = ?e, "rubato process failed; skipping chunk");
                        // On failure, break to avoid tight error loop; keep remaining samples for next round.
                        // Remaining samples in input_accum will be processed on next frame.
                        break;
                    }
                }
            }
        } else {
            // Already 16 kHz: append frame directly into 16 kHz buffer (no resampling needed).
            buffer.extend_from_slice(&frame);
        }

        // Check if enough new samples accumulated since last partial (sample-based gate).
        // saturating_sub prevents underflow if buffer was cleared.
        let enough_new_samples =
            buffer.len().saturating_sub(last_partial_emit_at) >= partial_stride;
        // Time gate: minimum spacing between previews (120ms prevents spam).
        // Dual gate (samples + time) ensures reasonable preview cadence.
        let enough_time = last_partial_time.elapsed() >= Duration::from_millis(120);

        // Emit partial previews for live feedback while the user is still speaking.
        // Strategy: Every ~0.2s, transcribe just the last ~1s of audio and show a preview.
        // Why short windows: Faster inference = lower perceived latency. We don't need perfect
        // accuracy for previews (they're just "last few words" hints).
        // Why not the full buffer: Transcribing 10+ seconds every 0.2s would be too slow.
        // Instead, we only look at recent context, then do a full transcription on PTT release.
        if enough_new_samples && enough_time && !buffer.is_empty() {
            // Last 1 second of audio (enough context for preview, fast inference).
            const PARTIAL_WINDOW_S: usize = 1;

            // Grab the last N seconds (partial window).
            // saturating_sub ensures we don't go negative if buffer < 1s.
            let max_samples = crate::inference::WHISPER_SAMPLE_RATE * PARTIAL_WINDOW_S;
            let start = buffer.len().saturating_sub(max_samples);
            // Clone slice for inference thread (worker continues, inference runs async).
            let pcm = buffer[start..].to_vec();

            // Send to inference thread (non-blocking; skip if inference is busy).
            // try_send fails if inference is still processing previous request (size=1 channel).
            // Silently skip (preview is best-effort; don't block audio processing).
            if let Err(_e) = inf_tx_worker.try_send(InferReq::Partial(pcm)) {}
            // Update tracking: mark this position as last emit point.
            last_partial_emit_at = buffer.len();
            last_partial_time = Instant::now();
        }
    }
}
