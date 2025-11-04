//! Audio capture worker.

use keyless_audio::AudioConfig;
use keyless_audio::input::{AudioInput, CpalAudioInput, FrameCallback, InputDevice};
use keyless_core::error::{KeylessError, KeylessResult};

/// Handle to a running audio capture stream.
pub struct AudioHandle {
    /// Inner CPAL-backed audio input handle.
    inner: CpalAudioInput,
}

impl AudioHandle {
    /// Stop audio capture and release resources.
    pub fn stop(mut self) -> KeylessResult<()> {
        // Consume self to ensure handle is dropped after stop (prevents double-stop).
        // Map CPAL error to KeylessError::Audio for consistent error handling.
        self.inner
            .stop()
            .map_err(|e| KeylessError::Audio(e.to_string()))
    }
}

/// Start audio capture with the selected device and configuration.
///
/// The `callback` is invoked with mono f32 frames at the configured frame size.
/// Returns an `AudioHandle` which can be used to stop capture.
pub fn start_audio(
    selected_device: Option<InputDevice>,
    cfg: AudioConfig,
    callback: FrameCallback,
) -> KeylessResult<AudioHandle> {
    let mut audio = CpalAudioInput::new();
    // Start capture stream (callback invoked from audio thread; non-blocking).
    // Map CPAL error to KeylessError::Audio for consistent error handling.
    audio
        .start(selected_device, cfg, callback)
        .map_err(|e| KeylessError::Audio(e.to_string()))?;
    // Return handle for graceful shutdown (stop() consumes handle).
    Ok(AudioHandle { inner: audio })
}
