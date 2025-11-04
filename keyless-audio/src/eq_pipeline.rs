//! EQ spectrum pipeline (pure DSP):
//! - Hann window → 1024pt Real FFT
//! - Log-spaced bands from a start frequency (default ~120 Hz)
//! - Auto-sensitivity via max normalization
//! - Configurable noise reduction filtering and attack/decay smoothing
//!
//! Performance: FFT planner and all buffers are cached in EqState for zero per-frame allocations.

use realfft::{RealFftPlanner, RealToComplex, num_complex::Complex32};
use std::sync::Arc;

#[derive(Clone, Copy)]
/// EQ visualization configuration.
///
/// Controls FFT size, band layout and shaping parameters.
pub struct EqConfig {
    /// Input sample rate in Hz (e.g., 48_000).
    pub sample_rate_hz: f32,
    /// FFT size (power of two, e.g., 1024).
    pub nfft: usize,
    /// Number of log-spaced bands to compute.
    pub bands: usize,
    /// Start frequency of the lowest band (e.g., 120 Hz).
    pub start_hz: f32,
    /// Noise reduction threshold (0=noisy, 1=smooth; threshold before scaling).
    pub noise_reduction: f32,
    /// Window scaling in dB applied after noise reduction.
    pub window_db: f32,
    /// Gamma curve applied after scaling (perceptual shaping).
    pub gamma: f32,
    /// Attack smoothing factor (0..1).
    pub attack: f32,
    /// Decay smoothing factor (0..1).
    pub decay: f32,
}

/// Cached state/buffers for the EQ computation.
pub struct EqState {
    /// Smoothed bar values in 0..100 range.
    pub smooth_bars: Vec<f32>,
    // Cached FFT resources (initialized once, reused every frame)
    /// FFT plan used for forward transforms.
    fft_plan: Arc<dyn RealToComplex<f32>>,
    /// Windowed input buffer (time-domain).
    input_buf: Vec<f32>,
    /// Output spectrum buffer (complex frequency-domain bins).
    spectrum_buf: Vec<Complex32>,
    /// Scratch buffer used by the FFT implementation.
    scratch_buf: Vec<Complex32>,
    /// Magnitudes buffer reused per frame.
    mags_buf: Vec<f32>,
    /// Per-band magnitude buffer (log-spaced).
    band_mags_buf: Vec<f32>,
    /// Unsmoothened bar values (used for attack/decay smoothing).
    bars_buf: Vec<f32>,
}

impl EqState {
    /// Create a new state with cached FFT/buffers for the given layout.
    pub fn new(bands: usize, nfft: usize) -> Self {
        // Initialize FFT planner and plan once (expensive operation, cached for reuse).
        let mut planner = RealFftPlanner::<f32>::new();
        let fft_plan = planner.plan_fft_forward(nfft);
        // Real FFT produces N/2+1 complex bins (second half is mirror image).
        let spectrum_size = fft_plan.complex_len();

        let scratch_size = fft_plan.get_scratch_len();

        Self {
            smooth_bars: vec![0.0; bands],
            fft_plan,
            input_buf: vec![0.0; nfft],
            spectrum_buf: vec![Complex32::new(0.0, 0.0); spectrum_size],
            scratch_buf: vec![Complex32::new(0.0, 0.0); scratch_size],
            // Use with_capacity (not fixed size) since mags_buf length = spectrum_size (varies by FFT).
            mags_buf: Vec::with_capacity(spectrum_size),
            band_mags_buf: vec![0.0; bands],
            bars_buf: vec![0.0; bands],
        }
    }
}

/// Compute bar heights (0..=100) for the most recent audio `frame`.
/// Uses auto-sensitivity (max normalization) for consistent visualization.
/// All buffers are reused from state for zero allocations.
pub fn compute_bars(frame: &[f32], cfg: &EqConfig, state: &mut EqState) -> Vec<u16> {
    // Early return if frame too short: can't fill FFT window, reuse previous smoothed values.
    // This handles startup/edge cases where buffer hasn't accumulated enough samples yet.
    if frame.len() < cfg.nfft {
        // Not enough samples; keep the current smooth values
        return state
            .smooth_bars
            .iter()
            .map(|v| v.clamp(0.0, 100.0) as u16)
            .collect();
    }

    // Step 1: Apply Hann window to reduce "spectral leakage" (blur in frequency domain)
    //
    // What's happening: We're about to convert time-domain audio into frequency bins via FFT.
    // The problem: FFT assumes the signal repeats infinitely. If the chunk doesn't loop smoothly,
    // we get false high-frequency noise at the edges.
    //
    // Fix: Multiply each sample by a "window" that fades to zero at both ends (bell curve shape).
    // Hann window formula: w[i] = 0.5 - 0.5 * cos(2π * i / N)
    // This gives smooth transitions and cleaner frequency bins.
    // Take last nfft samples (most recent audio); ensures we analyze latest data.
    let start = frame.len() - cfg.nfft;
    for (i, s) in frame[start..].iter().enumerate() {
        // Hann window: symmetric bell curve (peaks at center, fades to 0 at edges).
        // Formula: 0.5 - 0.5*cos(2π*i/N) produces values in [0, 1] range.
        let w = 0.5 - 0.5 * ((2.0 * std::f32::consts::PI * i as f32) / (cfg.nfft as f32)).cos();
        state.input_buf[i] = *s * w;
    }

    // Step 2: Run FFT to convert time-domain audio into frequency bins
    //
    // What's happening: Fast Fourier Transform (FFT) takes time-domain samples (what we recorded)
    // and tells us "how much of each frequency is present."
    //
    // Output: Complex numbers (real + imaginary). Each complex number represents one frequency bin.
    // We only care about the first half because the second half is a mirror (real FFT property).
    // Input buffer is modified in-place; spectrum_buf receives frequency-domain output.
    let _ = state.fft_plan.process_with_scratch(
        &mut state.input_buf,
        &mut state.spectrum_buf,
        &mut state.scratch_buf,
    );

    // Step 3: Convert complex numbers to magnitudes (loudness per frequency)
    //
    // What's happening: Each FFT bin is a complex number (re + im). The "magnitude" tells us
    // how loud that frequency is, computed as: sqrt(re² + im²).
    //
    // Analogy: Think of (re, im) as a 2D point. The magnitude is its distance from origin.
    // This gives us a single "loudness" value per frequency bin.
    // Real FFT produces N/2+1 bins (mirror property); we only need first half for visualization.
    let half = state.spectrum_buf.len();
    // Clear and reuse mags_buf (with_capacity avoids realloc, clear resets length to 0).
    state.mags_buf.clear();
    for c in &state.spectrum_buf[0..half] {
        // Magnitude = sqrt(re² + im²); represents amplitude/loudness of this frequency bin.
        state.mags_buf.push((c.re * c.re + c.im * c.im).sqrt());
    }

    // Step 4: Group FFT bins into log-spaced bands (like piano keys)
    //
    // Why logarithmic: Human hearing is logarithmic—each octave (doubling of frequency) sounds
    // like an equal "step" to our ears. A linear spacing would waste resolution on high frequencies
    // where we're less sensitive.
    //
    // What we do: Create bands that span wider frequency ranges as we go higher (like piano keys).
    // For each band, we find the max magnitude across all FFT bins in that range.
    //
    // Math: f(i) = start_hz * (nyquist / 50)^(i / (bands - 1))
    // This spreads bands from ~120 Hz (low bass) to Nyquist (half the sample rate) on a log scale.
    let nyquist = cfg.sample_rate_hz / 2.0;
    // Reset band magnitudes to zero before accumulation (reuse buffer).
    state.band_mags_buf.fill(0.0);
    for (i, band_mag) in state.band_mags_buf.iter_mut().enumerate() {
        // Compute frequency range for this band (exponential growth).
        // Formula spreads bands logarithmically from start_hz to nyquist.
        // Division by (bands - 1) ensures last band ends at nyquist (i = bands - 1).
        let f0 = cfg.start_hz * (nyquist / 50.0).powf(i as f32 / (cfg.bands - 1) as f32);
        let f1 = cfg.start_hz * (nyquist / 50.0).powf((i + 1) as f32 / (cfg.bands - 1) as f32);

        // Map frequency range to FFT bin indices (linear mapping: freq → bin).
        // Clamp to valid bin range [0, half-1] to prevent OOB access.
        // half-1 because bins are 0-indexed; last valid bin is at index half-1.
        let bin0 = ((f0 / nyquist) * (half as f32 - 1.0)).clamp(0.0, half as f32 - 1.0) as usize;
        let bin1 = ((f1 / nyquist) * (half as f32 - 1.0)).clamp(0.0, half as f32 - 1.0) as usize;

        // Find the loudest frequency in this band (peak detection, not average).
        // Max preserves sharp peaks better than averaging (important for visualization).
        let mut max_v = 0.0f32;
        // Inclusive range bin0..=bin1; max(bin0) handles edge case where bin1 < bin0 (shouldn't happen).
        for b in bin0..=bin1.max(bin0) {
            // Bounds check: get() returns None if index OOB (defensive).
            if let Some(&m) = state.mags_buf.get(b)
                && m > max_v
            {
                max_v = m;
            }
        }
        *band_mag = max_v;
    }

    // Step 5: Auto-sensitivity via max normalization
    //
    // Problem: Audio loudness varies wildly (quiet room vs. loud speech). If we used fixed scaling,
    // the visualizer would be tiny during quiet moments and clip during loud ones.
    //
    // Fix: Find the loudest band right now, and normalize all bands relative to that max.
    // This ensures bars fill the screen consistently regardless of absolute volume.
    // Fold finds maximum magnitude across all bands (O(n) scan).
    let max_mag = state.band_mags_buf.iter().copied().fold(0.0f32, f32::max);

    // Step 6: Shape the bars for clean visualization
    for (i, bar) in state.bars_buf.iter_mut().enumerate() {
        let mag = state.band_mags_buf[i];

        // Normalize against the loudest band (0.0 = silence, 1.0 = loudest right now).
        // Check max_mag > 1e-12 to avoid division by zero (silence case).
        let normalized = if max_mag > 1e-12 { mag / max_mag } else { 0.0 };

        // Apply noise reduction: filter out low-level background noise
        //
        // How it works: If a band is below the threshold, zero it out. Otherwise, rescale
        // the range (threshold..1.0) back to (0..1.0) so the visualizer stays smooth.
        //
        // Example: threshold=0.3, band=0.5 → (0.5 - 0.3) / (1.0 - 0.3) = 0.29 (rescaled)
        // Rescaling preserves relative differences above threshold while suppressing noise below.
        let filtered = if normalized < cfg.noise_reduction {
            0.0
        } else {
            // Linear rescale: maps [threshold, 1.0] → [0.0, 1.0].
            (normalized - cfg.noise_reduction) / (1.0 - cfg.noise_reduction)
        };

        // Apply perceptual shaping (window_db and gamma curve)
        //
        // window_db: Compresses dynamic range (makes quiet sounds more visible)
        // gamma: Power curve for perceptual brightness (like monitor gamma correction)
        // Clamp filtered*window_db to [0, window_db] then normalize back to [0, 1].
        // This effectively applies window_db as a scaling factor.
        let scaled = (filtered * cfg.window_db).min(cfg.window_db) / cfg.window_db;
        // Gamma curve: pow(scaled, gamma) darkens (gamma>1) or brightens (gamma<1) the visualization.
        let curved = scaled.powf(cfg.gamma);
        // Convert to 0..100 range for final bar height; clamp prevents overflow.
        *bar = (curved * 100.0).clamp(0.0, 100.0);
    }

    // Step 7: Smooth bars over time to prevent jittery animation
    //
    // Problem: Raw bar values jump around frame-to-frame, making the visualizer look choppy.
    //
    // Fix: Exponential smoothing. Each frame, move the current bar a fraction of the way toward
    // the target value. This creates fluid animation.
    //
    // Attack vs. Decay: Bars rise quickly (high attack) but fall slowly (low decay).
    // This mimics how we perceive sound: sudden onsets are sharp, but we "remember" loud sounds.
    //
    // Formula: new_value = old_value + α * (target - old_value)
    // α = attack (if rising) or decay (if falling); typical values 0.35 attack, 0.12 decay
    for i in 0..cfg.bands {
        // Bounds check: get_mut() returns None if index OOB (defensive, shouldn't happen).
        if let Some(s) = state.smooth_bars.get_mut(i) {
            // Get target bar value; fallback to 0.0 if index OOB (defensive).
            let target = state.bars_buf.get(i).copied().unwrap_or(0.0);
            // Choose smoothing factor: attack for rising, decay for falling (asymmetric response).
            let alpha = if target > *s { cfg.attack } else { cfg.decay };
            // Exponential smoothing: move fraction α of the way toward target each frame.
            *s = *s + alpha * (target - *s);
        }
    }

    // Convert smoothed bars to u16 (0..100 range) for final output.
    // Clamp to [0, 100] to handle any floating-point precision issues or smoothing overshoot.
    state
        .smooth_bars
        .iter()
        .map(|v| v.clamp(0.0, 100.0) as u16)
        .collect()
}
