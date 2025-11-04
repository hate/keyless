//! Mel filter bank generation for Whisper models.
//!
//! Mel filters convert linear frequency bins (from FFT) into "mel scale" bins,
//! which mimic how humans perceive pitch. The mel scale is logarithmic below ~1000 Hz and
//! nearly linear above, matching our ear's sensitivity.
//!
//! Why Whisper uses mel: Speech models work better with mel spectrograms because they focus
//! on perceptually-important frequency ranges (where speech formants and phonemes live).

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

    // FFT bins: N_FFT/2 + 1 (Nyquist theorem: can only represent frequencies up to sample_rate/2).
    // Example: N_FFT=400 → 201 bins (0 Hz to 8000 Hz at 16 kHz sample rate).
    let num_fft_bins = m::N_FFT / 2 + 1;
    let num_mel_bins = config.num_mel_bins;

    // Calculate mel frequencies for bin edges (need num_mel_bins + 2 frequencies).
    // Why +2: Each mel bin needs 3 frequencies (left, center, right) for triangular filter.
    // Bin i uses mel_freqs[i] (left), mel_freqs[i+1] (center), mel_freqs[i+2] (right).
    let mut mel_freqs = Vec::with_capacity(num_mel_bins + 2);
    // Convert Hz boundaries to mel scale (0 Hz → mel_min, Nyquist → mel_max).
    let mel_min = hz_to_mel(0.0);
    // Nyquist frequency: sample_rate/2 (highest representable frequency, 8000 Hz at 16 kHz).
    let mel_max = hz_to_mel(m::SAMPLE_RATE as f32 / 2.0);

    // Generate evenly-spaced mel frequencies (num_mel_bins + 1 intervals → num_mel_bins + 2 edges).
    // Range: 0..=num_mel_bins+1 (inclusive) produces num_mel_bins+2 frequencies.
    for i in 0..=num_mel_bins + 1 {
        // Linear interpolation in mel space (even spacing in perceptual scale).
        let mel = mel_min + (mel_max - mel_min) * i as f32 / (num_mel_bins + 1) as f32;
        // Convert back to Hz for triangular filter construction.
        mel_freqs.push(mel_to_hz(mel));
    }

    // Convert FFT bin indices to frequencies (Hz).
    // Formula: freq = bin_index * sample_rate / N_FFT.
    // Example: bin 0 = 0 Hz, bin 100 = 100 * 16000 / 400 = 4000 Hz.
    let fft_freqs: Vec<f32> = (0..num_fft_bins)
        .map(|i| i as f32 * m::SAMPLE_RATE as f32 / m::N_FFT as f32)
        .collect();

    // Build triangular mel filter bank.
    // What's a triangular filter: Each mel bin gets a triangle-shaped filter centered at its
    // frequency. The filter ramps up from zero (left edge), peaks at the center frequency,
    // then ramps back down to zero (right edge).
    // Why triangular: Simple, overlaps nicely with neighbors, and mimics cochlear filter shapes.
    // How it's stored: Flat array of size (num_mel_bins * num_fft_bins). For each mel bin,
    // we store the weight for every FFT bin.
    let mut filters = vec![0.0f32; num_fft_bins * num_mel_bins];

    // Build one triangular filter per mel bin.
    for i in 0..num_mel_bins {
        // Extract three frequencies: left edge, center (peak), right edge.
        // Indices: i (left), i+1 (center), i+2 (right) → need num_mel_bins+2 total frequencies.
        let left = mel_freqs[i];
        let center = mel_freqs[i + 1];
        let right = mel_freqs[i + 2];

        // Compute weight for each FFT bin (how much this FFT bin contributes to mel bin i).
        for (j, &freq) in fft_freqs.iter().enumerate() {
            // Compute triangle weight: 0 outside [left, right], linear ramp inside.
            // Boundary checks: freq < left or freq > right → weight = 0 (outside triangle).
            let weight = if freq < left || freq > right {
                0.0 // Outside this mel bin's range (no contribution).
            } else if freq <= center {
                // Rising edge: linear interpolation from 0 (at left) to 1 (at center).
                // Formula: (freq - left) / (center - left) → [0, 1] range.
                (freq - left) / (center - left)
            } else {
                // Falling edge: linear interpolation from 1 (at center) to 0 (at right).
                // Formula: (right - freq) / (right - center) → [1, 0] range.
                (right - freq) / (right - center)
            };
            // Store weight in flattened array: row i (mel bin), column j (FFT bin).
            // Layout: [mel0_fft0, mel0_fft1, ..., mel0_fftN, mel1_fft0, ...].
            filters[i * num_fft_bins + j] = weight;
        }
    }

    filters
}

/// Convert frequency in Hz to mel scale.
/// Formula: mel = 2595 * log10(1 + hz / 700).
/// Constants: 2595 (scaling factor), 700 Hz (breakpoint where mel ≈ Hz).
/// This is the standard Stevens-Volkmann mel scale (perceptual frequency scale).
fn hz_to_mel(hz: f32) -> f32 {
    // Logarithmic compression: low frequencies expand, high frequencies compress.
    // Below ~700 Hz: mel scale is roughly linear (mel ≈ Hz).
    // Above ~700 Hz: mel scale is logarithmic (compresses high frequencies).
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert mel scale to frequency in Hz.
/// Inverse of hz_to_mel: hz = 700 * (10^(mel/2595) - 1).
fn mel_to_hz(mel: f32) -> f32 {
    // Reverse the logarithmic compression: expand high mel values back to Hz.
    // Formula derived by solving mel = 2595 * log10(1 + hz/700) for hz.
    700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
}
