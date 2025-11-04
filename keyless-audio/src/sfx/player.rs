//! SFX player: public API to trigger procedural beeps for PTT events.
//!
//! Provides `play_press`, `play_release`, `play_final` methods. Internally opens
//! a single always-on CPAL output stream and renders short tones in the realtime
//! callback using `CallbackState`.

use std::sync::{Arc, Mutex, mpsc};

use super::state::{BeepKind, CallbackState};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tracing::{error, info};

/// SFX player for procedural beeps on PTT events.
#[derive(Clone)]
pub struct SfxPlayer {
    /// Channel sender for beep events (None if SFX unavailable).
    pub(crate) tx: Option<mpsc::SyncSender<BeepKind>>, // crate-visible for tests if needed
}

impl SfxPlayer {
    /// Creates a new SFX player, falling back to no-op if audio output is unavailable.
    pub fn new() -> Self {
        // Graceful degradation: if audio output fails, return no-op player (tx = None).
        // This allows the app to continue without SFX rather than failing entirely.
        match Self::try_new_internal() {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "SfxPlayer init failed; running as no-op");
                Self { tx: None }
            }
        }
    }

    /// Plays a beep when PTT is pressed.
    pub fn play_press(&self) {
        // Try-send is non-blocking; drops beep if channel full (defensive, prevents stalling).
        // No-op if SFX unavailable (tx = None from failed initialization).
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(BeepKind::Press);
        }
    }

    /// Plays a beep when PTT is released.
    pub fn play_release(&self) {
        // Try-send is non-blocking; drops beep if channel full (defensive, prevents stalling).
        // No-op if SFX unavailable (tx = None from failed initialization).
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(BeepKind::Release);
        }
    }

    /// Plays a double beep when final transcription is delivered.
    pub fn play_final(&self) {
        // Try-send is non-blocking; drops beep if channel full (defensive, prevents stalling).
        // No-op if SFX unavailable (tx = None from failed initialization).
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(BeepKind::Final);
        }
    }

    /// Internal constructor that attempts to initialize the CPAL output stream.
    fn try_new_internal() -> Result<Self, String> {
        let host = cpal::default_host();
        // Use OS default output device (usually speakers/headphones).
        let device = host
            .default_output_device()
            .ok_or_else(|| "no output device available".to_string())?;
        let chosen = device
            .default_output_config()
            .map_err(|e| format!("failed default output config: {}", e))?;
        // Require F32 format for simplicity (avoid format conversion in callback).
        // Gracefully degrade to no-op if device doesn't support F32.
        if chosen.sample_format() != cpal::SampleFormat::F32 {
            error!("sfx requires f32 output; falling back to no-op");
            return Ok(Self { tx: None });
        }

        let sample_rate = chosen.sample_rate().0 as f32;
        // Max(1) prevents zero channels (defensive; shouldn't happen but handles edge case).
        let channels = chosen.channels().max(1) as usize;
        // Bounded sync channel (capacity 8): allows multiple queued beeps without blocking.
        // Small capacity is fine; beeps are short and callback drains quickly.
        let (tx, rx) = mpsc::sync_channel::<BeepKind>(8);
        // Shared state protected by Mutex (accessed from OS audio callback thread).
        let state = Arc::new(Mutex::new(CallbackState::new(sample_rate)));
        let state_cb = Arc::clone(&state);
        // Error callback runs on OS audio thread; just log, don't block or panic.
        let err_fn = |err| error!(error = %err, "cpal output stream error");
        // Move rx into callback (owned by callback closure, not accessible elsewhere).
        let stream_res = device.build_output_stream(
            &chosen.config(),
            move |data: &mut [f32], _| Self::render_callback_f32(data, channels, &state_cb, &rx),
            err_fn,
            None,
        );

        let stream = match stream_res {
            Ok(s) => s,
            Err(e) => return Err(format!("failed to build output stream: {}", e)),
        };
        // Start the stream (begins OS audio callbacks); must succeed or stream is invalid.
        if let Err(e) = stream.play() {
            return Err(format!("failed to start output stream: {}", e));
        }
        info!(
            sample_rate_hz = sample_rate,
            channels = channels,
            "sfx output stream started"
        );
        // Forget stream to keep it alive for the lifetime of the process.
        // Stream runs in background until process exits (no explicit cleanup needed).
        std::mem::forget(stream);
        Ok(Self { tx: Some(tx) })
    }

    /// CPAL audio callback: renders beeps to the output buffer.
    fn render_callback_f32(
        output: &mut [f32],
        channels: usize,
        state: &Arc<Mutex<CallbackState>>,
        rx: &mpsc::Receiver<BeepKind>,
    ) {
        // Mutex lock must be fast (runs on OS audio thread); no blocking operations inside.
        if let Ok(mut st) = state.lock() {
            // Drain all pending beep events (non-blocking); process most recent if multiple queued.
            // Multiple try_recv() handles burst of events without blocking.
            if let Ok(kind) = rx.try_recv() {
                st.start_beep(kind);
                // Drain remaining events (latest beep wins if multiple queued).
                while rx.try_recv().is_ok() {}
            }
            // Calculate frame count: interleaved samples (L R L R ...) so len / channels.
            let frames = output.len() / channels;
            // Generate one sample per frame, duplicate to all channels (mono beep).
            for f in 0..frames {
                let s = st.next_sample();
                // Write same sample to all channels (mono beep, stereo/multi-channel output).
                for ch in 0..channels {
                    output[f * channels + ch] = s;
                }
            }
        }
    }
}

impl Default for SfxPlayer {
    fn default() -> Self {
        Self::new()
    }
}
