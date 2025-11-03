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
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(BeepKind::Press);
        }
    }

    /// Plays a beep when PTT is released.
    pub fn play_release(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(BeepKind::Release);
        }
    }

    /// Plays a double beep when final transcription is delivered.
    pub fn play_final(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(BeepKind::Final);
        }
    }

    /// Internal constructor that attempts to initialize the CPAL output stream.
    fn try_new_internal() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no output device available".to_string())?;
        let chosen = device
            .default_output_config()
            .map_err(|e| format!("failed default output config: {}", e))?;
        if chosen.sample_format() != cpal::SampleFormat::F32 {
            error!("sfx requires f32 output; falling back to no-op");
            return Ok(Self { tx: None });
        }

        let sample_rate = chosen.sample_rate().0 as f32;
        let channels = chosen.channels().max(1) as usize;
        let (tx, rx) = mpsc::sync_channel::<BeepKind>(8);
        let state = Arc::new(Mutex::new(CallbackState::new(sample_rate)));
        let state_cb = Arc::clone(&state);
        let err_fn = |err| error!(error = %err, "cpal output stream error");
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
        if let Err(e) = stream.play() {
            return Err(format!("failed to start output stream: {}", e));
        }
        info!(
            sample_rate_hz = sample_rate,
            channels = channels,
            "sfx output stream started"
        );
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
        if let Ok(mut st) = state.lock() {
            if let Ok(kind) = rx.try_recv() {
                st.start_beep(kind);
                while rx.try_recv().is_ok() {}
            }
            let frames = output.len() / channels;
            for f in 0..frames {
                let s = st.next_sample();
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
