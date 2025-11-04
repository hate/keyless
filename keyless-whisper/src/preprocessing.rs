//! Audio preprocessing for Whisper inference.
//!
//! This module converts raw PCM audio (from `keyless-audio`) into mel spectrograms,
//! which is the input format Whisper's encoder expects.
//!
//! **Distinction from `keyless-audio` crate**:
//! - `keyless-audio`: Captures audio from microphone → produces PCM samples
//! - This module: Transforms PCM → mel spectrograms for ML inference
//!
//! Whisper requirements:
//! - 16 kHz mono audio (handled by resampling in lib.rs)
//! - 80-bin mel spectrogram (tiny/base/small/medium) or 128-bin (large)
//! - Log-mel scale with specific filter banks and FFT windowing

use candle_transformers::models::whisper::{self as m, Config};

/// Convert PCM audio to a mel spectrogram.
///
/// Mel spectrogram: A visual representation of sound where:
/// - X-axis: time (divided into small windows, e.g., 25ms)
/// - Y-axis: frequency (on the mel scale, not linear Hz)
/// - Color/value: how loud each frequency is at each moment
///
/// Why Whisper needs this: Neural networks can't directly understand raw audio samples.
/// A mel spectrogram is like a "picture of sound" that highlights speech-relevant features
/// (formants, pitch changes) while compressing less-important details.
///
/// Steps (handled by Candle):
/// 1. Split PCM into overlapping windows (25ms each, sliding by 10ms)
/// 2. Apply FFT to each window → frequency bins
/// 3. Multiply by mel filters → convert linear frequency to mel scale
/// 4. Take logarithm (log-mel) → compress dynamic range (makes quiet sounds more visible)
///
/// # Arguments
/// * `config` - Whisper model config
/// * `pcm` - Audio samples at 16 kHz mono, f32 in [-1.0, 1.0]
/// * `mel_filters` - Precomputed mel filter bank
///
/// # Returns
/// Flattened mel spectrogram as Vec<f32>.
pub fn pcm_to_mel(config: &Config, pcm: &[f32], mel_filters: &[f32]) -> Vec<f32> {
    // Use Candle's built-in PCM→mel conversion (handles FFT, windowing, log scaling).
    // Delegates to Candle's optimized implementation (avoids reimplementing FFT/windowing).
    // Returns flattened mel spectrogram: [mel0_t0, mel1_t0, ..., melN_t0, mel0_t1, ...].
    m::audio::pcm_to_mel(config, pcm, mel_filters)
}
