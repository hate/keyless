//! Audio configuration for input streams.
//!
//! This module defines `AudioConfig`, which controls the sample rate and frame
//! size for microphone capture.

#[derive(Clone, Debug)]
/// Audio stream configuration (sample rate and frame size).
pub struct AudioConfig {
    /// Target sample rate in Hz (e.g., 16000, 44100, 48000).
    ///
    /// Special values:
    /// - `0` = automatic: use the device default input sample rate.
    ///   Implementations may cap excessively high defaults (e.g., choose the highest
    ///   supported rate ≤ 48_000 Hz) for predictable CPU use.
    pub sample_hz: u32,

    /// Number of samples per frame (e.g., 1600 for 100ms at 16kHz).
    ///
    /// Special values:
    /// - `0` = automatic: derive approximately 100 ms frames from the chosen sample rate
    ///   (i.e., `frame_samples = sample_rate / 10`, integer division).
    ///
    /// User callback is invoked once per frame with exactly this many samples.
    pub frame_samples: usize,
}
