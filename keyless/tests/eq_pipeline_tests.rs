//! Unit tests for EQ pipeline DSP.

use keyless_audio::eq_pipeline::{EqConfig, EqState, compute_bars};

fn dummy_cfg() -> EqConfig {
    EqConfig {
        sample_rate_hz: 48_000.0,
        nfft: 1024,
        bands: 16,
        start_hz: 120.0,
        noise_reduction: 0.46,
        window_db: 50.0,
        gamma: 1.3,
        attack: 0.5,
        decay: 0.1,
    }
}

#[test]
fn silence_produces_low_bars() {
    let cfg = dummy_cfg();
    let mut state = EqState::new(cfg.bands, cfg.nfft);
    let frame = vec![0.0f32; cfg.nfft];
    let bars = compute_bars(&frame, &cfg, &mut state);
    assert_eq!(bars.len(), cfg.bands);
    assert!(bars.iter().all(|&b| b == 0));
}

#[test]
fn tone_increases_some_bars() {
    let cfg = dummy_cfg();
    let mut state = EqState::new(cfg.bands, cfg.nfft);
    let freq = 1000.0f32; // 1kHz tone
    let mut frame = vec![0.0f32; cfg.nfft];
    for (i, sample) in frame.iter_mut().enumerate() {
        *sample = (2.0 * std::f32::consts::PI * freq * (i as f32) / cfg.sample_rate_hz).sin() * 0.5;
    }
    let bars = compute_bars(&frame, &cfg, &mut state);
    assert_eq!(bars.len(), cfg.bands);
    assert!(bars.iter().any(|&b| b > 0));
}

#[test]
fn smoothing_has_memory() {
    let cfg = dummy_cfg();
    let mut state = EqState::new(cfg.bands, cfg.nfft);
    // baseline silence
    let frame0 = vec![0.0f32; cfg.nfft];
    let _ = compute_bars(&frame0, &cfg, &mut state);
    // strong signal
    let mut frame1 = vec![0.0f32; cfg.nfft];
    for x in frame1.iter_mut() {
        *x = 0.5;
    }
    let bars1 = compute_bars(&frame1, &cfg, &mut state);
    // back to silence (decay path)
    let frame2 = vec![0.0f32; cfg.nfft];
    // apply multiple decay steps to avoid rounding masking small changes
    let mut bars2 = compute_bars(&frame2, &cfg, &mut state);
    for _ in 0..4 {
        bars2 = compute_bars(&frame2, &cfg, &mut state);
    }
    // Bars after decay should not exceed previous peak in any band
    assert!(bars2.iter().zip(bars1.iter()).all(|(a, b)| a <= b));
}
