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

/// Type alias for the audio callback closure.
pub type AudioCallback = Box<dyn FnMut(&[f32]) + Send>;

/// Grouped transmit channels for the audio callback.
pub struct CallbackTx {
    /// RMS audio level → TUI gauge (0..100).
    pub tx_level: mpsc::SyncSender<u16>,
    /// Log lines → TUI status panel.
    pub tx_log: mpsc::SyncSender<String>,
    /// Spectrum bars (EQ) → TUI visualizer.
    pub tx_spec: mpsc::SyncSender<Vec<u16>>,
    /// Live preview text → TUI preview area.
    pub tx_preview: mpsc::SyncSender<String>,
    /// VAD open/closed → TUI indicator.
    pub tx_vad: mpsc::SyncSender<bool>,
}

/// Parameters required to construct the audio callback.
pub struct AudioCallbackParams {
    /// EQ configuration for spectrum computation.
    pub eq_cfg: EqConfig,
    /// Voice Activity Detection thresholds.
    pub vad: VadThresholds,
    /// Whisper worker/transcriber handle.
    pub transcriber: workers::whisper::WhisperHandle,
    /// Output destination for final transcriptions.
    pub sink: Arc<dyn OutputSink>,
    /// Sound effects player for PTT/VAD cues.
    pub sfx: Arc<SfxPlayer>,
    /// Shared PTT hold flag (hotkey state).
    pub hold_flag: Arc<AtomicBool>,
    /// Grouped outbound channels to the TUI.
    pub tx: CallbackTx,
}

/// Create the audio frame callback with PTT state machine.
pub fn create_audio_callback(params: AudioCallbackParams) -> AudioCallback {
    let AudioCallbackParams {
        eq_cfg,
        vad,
        transcriber,
        sink,
        sfx,
        hold_flag,
        tx,
    } = params;
    // Clone inner transcriber handle (needed for closure capture; Arc is cheap).
    let trans_clone = transcriber.inner_clone();

    // Track PTT and VAD state transitions (state machine variables).
    let mut eq_state = EqState::new(eq_cfg.bands, eq_cfg.nfft);
    let mut gate_open = false;
    let mut held_prev = false;
    // VAD gate: hysteresis-based voice activity detection (prevents flickering).
    let mut vad = VadGate::new(
        vad.start_db,
        vad.stop_db,
        vad.min_duration_ms,
        vad.max_silence_ms,
        100.0, // Sample rate in Hz (used for timing calculations).
    );

    // Cache zero spectrum vector for idle/closed VAD visuals (avoid per-frame allocation).
    let zero_spectrum = vec![0u16; eq_cfg.bands];
    let mut last_spec_was_zero = true;

    Box::new(move |frame: &[f32]| {
        // Compute RMS (Root Mean Square) audio level for UI feedback.
        // RMS = sqrt(sum(samples²) / count); normalized to [0, 100] for gauge display.
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
        let level = (rms * 100.0).clamp(0.0, 100.0) as u16;
        let _ = tx.tx_level.try_send(level);
        // Convert to dB for VAD (20 * log10(rms)); max(1e-8) prevents log(0) = -inf.
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
            // Hotkey is released (PTT not held).
            if held_prev {
                // JUST released (transition): finalize the segment (flush transcriber, emit Final).
                handle_ptt_released(&trans_clone, &sink, &tx.tx_log, &tx.tx_preview, &sfx);
                sfx.play_release();
                held_prev = false;
            }
            // CRITICAL: Always drain events even when not held, to catch asynchronous Final events
            // from the inference thread (they arrive later, after hotkey is released).
            process_transcription_events(&trans_clone, &sink, &tx.tx_log, &tx.tx_preview, &sfx);

            // Show flat/empty visualizer when idle (only send if state changed to avoid spam).
            if !last_spec_was_zero {
                let _ = tx.tx_spec.try_send(zero_spectrum.clone());
                last_spec_was_zero = true;
            }
            return;
        }

        // Hotkey is held (PTT active).
        if !held_prev {
            // JUST pressed (transition): start a new dictation session.
            handle_ptt_pressed(&tx.tx_log, &tx.tx_preview);
            sfx.play_press();
            // Reset VAD timers (don't carry over from previous session; fresh start).
            vad.reset();
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
            let _ = tx.tx_log.try_send(format!(
                "VAD {} at {:.1} dB",
                if gate_open { "OPEN" } else { "CLOSED" },
                level_db
            ));
            let _ = tx.tx_vad.try_send(gate_open);
        }

        // Update EQ visualizer and transcription based on VAD state.
        if open_now {
            // Compute bars only when VAD is open (reduces CPU when gate is closed; skip FFT).
            let out = compute_bars(frame, &eq_cfg, &mut eq_state);
            let _ = tx.tx_spec.try_send(out);
            last_spec_was_zero = false;

            // Push audio to transcriber (non-blocking lock; ignore if poisoned).
            // push_audio may fail if transcriber buffer is full (backpressure).
            if let Ok(mut t) = trans_clone.lock()
                && let Err(e) = t.push_audio(frame)
            {
                error!(error = %e, "failed to push audio to transcriber (backpressure)");
                let _ = tx.tx_log.try_send(format!("push_audio error: {}", e));
            }
            // Drain Partial events (show live preview while speaking).
            process_transcription_events(&trans_clone, &sink, &tx.tx_log, &tx.tx_preview, &sfx);
        } else {
            // Gate closed: show empty spectrum only if it changed (skip compute to save CPU).
            if !last_spec_was_zero {
                let _ = tx.tx_spec.try_send(zero_spectrum.clone());
                last_spec_was_zero = true;
            }
        }
    })
}
