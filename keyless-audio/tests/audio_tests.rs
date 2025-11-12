//! Tests for keyless-audio crate.

use keyless_audio::{AudioConfig, AudioInput, CpalAudioInput};

#[test]
fn audio_config_creation() {
    let config = AudioConfig {
        sample_hz: 16000,
        frame_samples: 1600,
    };

    assert_eq!(config.sample_hz, 16000);
    assert_eq!(config.frame_samples, 1600);
}

#[test]
fn audio_config_clone() {
    let config = AudioConfig {
        sample_hz: 48000,
        frame_samples: 4800,
    };

    let cloned = config.clone();
    assert_eq!(config.sample_hz, cloned.sample_hz);
    assert_eq!(config.frame_samples, cloned.frame_samples);
}

#[test]
fn cpal_audio_input_new() {
    // Creating a new input should succeed without hardware
    let _input = CpalAudioInput::new();

    // Constructor succeeds; we can't check internal state (private fields)
    // but we can verify stop() returns the expected error when not started
}

#[test]
fn cpal_audio_input_getters_before_start() {
    let input = CpalAudioInput::new();
    assert!(input.chosen_sample_rate_hz().is_none());
    assert!(input.chosen_frame_samples().is_none());
}

#[test]
fn cpal_audio_input_stop_when_not_started() {
    let mut input = CpalAudioInput::new();

    // Stopping when not started should return an error
    let result = input.stop();
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(format!("{}", e).contains("not started"));
    }
}

#[test]
fn audio_config_various_sample_rates() {
    let configs = vec![
        (16000, 1600), // 100ms at 16k
        (44100, 4410), // 100ms at 44.1k
        (48000, 4800), // 100ms at 48k
        (22050, 2205), // 100ms at 22.05k
    ];

    for (hz, samples) in configs {
        let config = AudioConfig {
            sample_hz: hz,
            frame_samples: samples,
        };
        assert_eq!(config.sample_hz, hz);
        assert_eq!(config.frame_samples, samples);

        // Verify frame duration is ~100ms
        let duration_ms = (samples as f64 / hz as f64) * 1000.0;
        assert!(
            (duration_ms - 100.0).abs() < 0.1,
            "Expected ~100ms, got {}ms for {}Hz",
            duration_ms,
            hz
        );
    }
}
