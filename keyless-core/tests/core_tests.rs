//! Tests for keyless-core crate.

use keyless_core::config::{Config, OutputMode, VadThresholds};
use std::env;
use std::path::PathBuf;

#[test]
fn default_config() {
    let config = Config::default();
    assert_eq!(config.language, Some("en".to_string()));
    assert_eq!(config.hotkey, "control+option");
    assert_eq!(config.output_mode, OutputMode::Paste);
    assert!(config.validate().is_ok());
}

#[test]
fn vad_thresholds_default() {
    let vad = VadThresholds::default();
    assert_eq!(vad.start_db, -45.0);
    assert_eq!(vad.stop_db, -50.0);
    assert_eq!(vad.min_duration_ms, 200);
    assert_eq!(vad.max_silence_ms, 800);
}

#[test]
fn config_validation_valid_language() {
    let config = Config {
        language: Some("en".to_string()),
        ..Default::default()
    };
    assert!(config.validate().is_ok());

    let config = Config {
        language: Some("es".to_string()),
        ..Default::default()
    };
    assert!(config.validate().is_ok());

    let config = Config {
        language: Some("zh-CN".to_string()),
        ..Default::default()
    };
    assert!(config.validate().is_ok());
}

#[test]
fn config_validation_invalid_language() {
    let config = Config {
        language: Some("x".to_string()), // too short
        ..Default::default()
    };
    assert!(config.validate().is_err());

    let config = Config {
        language: Some("toolongcode".to_string()), // too long
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn config_validation_file_output_parent_missing() {
    let mut config = Config::default();
    let nonexistent_dir = env::temp_dir()
        .join("keyless-test-nonexistent-dir-12345")
        .join("output.txt");
    config.output_mode = OutputMode::File(nonexistent_dir);

    // Should fail because parent directory doesn't exist
    assert!(config.validate().is_err());
}

#[test]
fn config_validation_file_output_parent_exists() {
    let mut config = Config::default();
    let temp_file = env::temp_dir().join("keyless-test-output.txt");
    config.output_mode = OutputMode::File(temp_file);

    // Should succeed because temp_dir exists
    assert!(config.validate().is_ok());
}

#[test]
fn output_mode_serde() -> Result<(), Box<dyn std::error::Error>> {
    // Test that OutputMode serializes/deserializes correctly
    let modes = vec![
        OutputMode::Paste,
        OutputMode::Clipboard,
        OutputMode::File(PathBuf::from("/tmp/test.txt")),
    ];

    for mode in modes {
        let json = serde_json::to_string(&mode)?;
        let deserialized: OutputMode = serde_json::from_str(&json)?;
        assert_eq!(mode, deserialized);
    }
    Ok(())
}

#[test]
fn config_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let json = serde_json::to_string(&config)?;
    let deserialized: Config = serde_json::from_str(&json)?;
    assert_eq!(config, deserialized);
    Ok(())
}
