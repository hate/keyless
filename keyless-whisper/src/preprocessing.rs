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

/// Generate mel filter banks for the given Whisper config.
///
/// Mel filters: Convert linear frequency bins (from FFT) into "mel scale" bins,
/// which mimic how humans perceive pitch. The mel scale is logarithmic below ~1000 Hz and
/// nearly linear above, matching our ear's sensitivity.
///
/// Why Whisper uses mel: Speech models work better with mel spectrograms because they focus
/// on perceptually-important frequency ranges (where speech formants and phonemes live).
///
/// How it works:
/// 1. Create triangular filters spaced evenly on the mel scale
/// 2. Each filter covers a range of FFT bins, with a peak at its center frequency
/// 3. When we multiply FFT bins by these filters, we get mel bins
///
/// # Arguments
/// * `config` - Whisper model config specifying num_mel_bins
///
/// # Returns
/// A Vec<f32> of mel filter values for use in pcm_to_mel.
pub fn generate_mel_filters(config: &Config) -> Vec<f32> {
    // Generate triangular mel filter bank
    //
    // Mel scale formula: mel = 2595 * log10(1 + f / 700)
    // This compresses high frequencies (we're less sensitive there) and expands low frequencies
    // (where speech formants like vowels are).
    //
    // Whisper uses:
    // - FFT size: 400 → 201 FFT bins (0 to Nyquist)
    // - Sample rate: 16000 Hz
    // - Mel bins: 80 (tiny/base/small) or 128 (large models)

    let num_fft_bins = m::N_FFT / 2 + 1; // 201 bins (0 to N_FFT/2)
    let num_mel_bins = config.num_mel_bins;

    // Calculate mel frequencies for bin edges
    let mut mel_freqs = Vec::with_capacity(num_mel_bins + 2);
    let mel_min = hz_to_mel(0.0);
    let mel_max = hz_to_mel(m::SAMPLE_RATE as f32 / 2.0); // Nyquist frequency

    for i in 0..=num_mel_bins + 1 {
        let mel = mel_min + (mel_max - mel_min) * i as f32 / (num_mel_bins + 1) as f32;
        mel_freqs.push(mel_to_hz(mel));
    }

    // Convert frequencies to FFT bin indices
    let fft_freqs: Vec<f32> = (0..num_fft_bins)
        .map(|i| i as f32 * m::SAMPLE_RATE as f32 / m::N_FFT as f32)
        .collect();

    // Build triangular mel filter bank
    //
    // What's a triangular filter: Each mel bin gets a triangle-shaped filter centered at its
    // frequency. The filter ramps up from zero (left edge), peaks at the center frequency,
    // then ramps back down to zero (right edge).
    //
    // Why triangular: Simple, overlaps nicely with neighbors, and mimics cochlear filter shapes.
    //
    // How it's stored: Flat array of size (num_mel_bins * num_fft_bins). For each mel bin,
    // we store the weight for every FFT bin.
    let mut filters = vec![0.0f32; num_fft_bins * num_mel_bins];

    for i in 0..num_mel_bins {
        let left = mel_freqs[i];
        let center = mel_freqs[i + 1];
        let right = mel_freqs[i + 2];

        for (j, &freq) in fft_freqs.iter().enumerate() {
            // Compute triangle weight: 0 outside [left, right], linear ramp inside
            let weight = if freq < left || freq > right {
                0.0 // Outside this mel bin's range
            } else if freq <= center {
                (freq - left) / (center - left) // Rising edge
            } else {
                (right - freq) / (right - center) // Falling edge
            };
            filters[i * num_fft_bins + j] = weight;
        }
    }

    filters
}

/// Convert frequency in Hz to mel scale.
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert mel scale to frequency in Hz.
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
}

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
    // Use Candle's built-in PCM→mel conversion (handles FFT, windowing, log scaling)
    m::audio::pcm_to_mel(config, pcm, mel_filters)
}
