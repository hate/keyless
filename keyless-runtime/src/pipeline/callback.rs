//! Audio frame callback with PTT state machine.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use keyless_audio::{EqConfig, EqState, SfxPlayer, VadGate, compute_bars};
use keyless_core::config::VadThresholds;
use keyless_core::output::OutputSink;
use keyless_whisper::RealtimeTranscriber;
use tracing::{debug, error};

use super::events::process_transcription_events;
use crate::ptt::handlers::{handle_ptt_pressed, handle_ptt_released};
use crate::workers;

/// Create the audio frame callback with PTT state machine.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn create_audio_callback(
    eq_cfg: EqConfig,
    vad_config: &VadThresholds,
    transcriber: workers::whisper::WhisperHandle,
    sink: Arc<Box<dyn OutputSink>>,
    sfx: Arc<SfxPlayer>,
    hold_flag: Arc<AtomicBool>,
    tx_level: mpsc::SyncSender<u16>,
    tx_log: mpsc::SyncSender<String>,
    tx_spec: mpsc::SyncSender<Vec<u16>>,
    tx_preview: mpsc::SyncSender<String>,
    tx_vad: mpsc::SyncSender<bool>,
) -> Box<dyn FnMut(&[f32]) + Send> {
    let trans_clone = transcriber.inner_clone();

    // Track PTT and VAD state transitions
    let mut eq_state = EqState::new(eq_cfg.bands, eq_cfg.nfft);
    let mut gate_open = false;
    let mut held_prev = false;
    let mut vad = VadGate::new(
        vad_config.start_db,
        vad_config.stop_db,
        vad_config.min_duration_ms,
        vad_config.max_silence_ms,
        100.0,
    );

    // Cache zero spectrum vector for idle/closed VAD visuals
    let zero_spectrum = vec![0u16; eq_cfg.bands];
    let mut last_spec_was_zero = true;

    Box::new(move |frame: &[f32]| {
        // Compute audio level for UI feedback
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
        let level = (rms * 100.0).clamp(0.0, 100.0) as u16;
        let _ = tx_level.try_send(level);
        let level_db = 20.0 * (rms.max(1e-8)).log10();

        let held_now = hold_flag.load(Ordering::Relaxed);

        // PTT state machine: track hotkey press/release and trigger actions
        //
        // States:
        // - NOT held (released): Don't capture audio, but drain events (for async Finals)
        // - JUST pressed: Clear preview, reset VAD, start fresh session
        // - HELD: Push audio to transcriber, compute EQ, drain Partial events
        // - JUST released: Finalize segment, emit Final event
        //
        // Why this matters: PTT controls when we transcribe. Audio only flows to Whisper
        // while the hotkey is held, and we finalize (emit Final) when released.
        if !held_now {
            // Hotkey is released
            if held_prev {
                // JUST released (transition): finalize the segment
                handle_ptt_released(&trans_clone, &sink, &tx_log, &tx_preview, &sfx);
                sfx.play_release();
                held_prev = false;
            }
            // CRITICAL: Always drain events even when not held, to catch
            // asynchronous Final events from the inference thread (they arrive later)
            process_transcription_events(&trans_clone, &sink, &tx_log, &tx_preview, &sfx);

            // Show flat/empty visualizer when idle
            if !last_spec_was_zero {
                let _ = tx_spec.try_send(zero_spectrum.clone());
                last_spec_was_zero = true;
            }
            return;
        }

        // Hotkey is held
        if !held_prev {
            // JUST pressed (transition): start a new dictation session
            handle_ptt_pressed(&tx_log, &tx_preview);
            sfx.play_press();
            vad.reset(); // Clear VAD timers (don't carry over from previous session)
            gate_open = false;
            held_prev = true;
        }

        // VAD (Voice Activity Detection): decide if this frame contains speech or just noise
        //
        // How it works (hysteresis-based gate):
        // - Start threshold (e.g., -30 dB): If audio stays above this for min_duration, gate opens
        // - Stop threshold (e.g., -35 dB): If audio stays below this for max_silence, gate closes
        //
        // Why hysteresis (two thresholds): Prevents flickering on/off during pauses or breaths.
        // The gap between start and stop creates "sticky" behavior—once open, it stays open through
        // brief dips; once closed, it needs sustained sound to reopen.
        //
        // Result: open_now tells us if we should forward this frame to Whisper or discard it.
        let open_now = vad.update(level_db);
        if open_now != gate_open {
            gate_open = open_now;
            debug!(
                state = if gate_open { "OPEN" } else { "CLOSED" },
                level_db = %level_db,
                "VAD gate state changed"
            );
            let _ = tx_log.try_send(format!(
                "VAD {} at {:.1} dB",
                if gate_open { "OPEN" } else { "CLOSED" },
                level_db
            ));
            let _ = tx_vad.try_send(gate_open);
        }

        // Update EQ visualizer and transcription based on VAD state
        if open_now {
            // Compute bars only when VAD is open; reduces CPU when gate is closed
            let out = compute_bars(frame, &eq_cfg, &mut eq_state);
            let _ = tx_spec.try_send(out);
            last_spec_was_zero = false;

            if let Ok(mut t) = trans_clone.lock()
                && let Err(e) = t.push_audio(frame)
            {
                error!(error = %e, "failed to push audio to transcriber (backpressure)");
                let _ = tx_log.try_send(format!("push_audio error: {}", e));
            }
            process_transcription_events(&trans_clone, &sink, &tx_log, &tx_preview, &sfx);
        } else {
            // Gate closed: show empty spectrum only if it changed, skip compute
            if !last_spec_was_zero {
                let _ = tx_spec.try_send(zero_spectrum.clone());
                last_spec_was_zero = true;
            }
        }
    })
}
