//! Integration tests for keyless.
//!
//! These tests verify that the different crates work together correctly
//! without requiring real hardware or downloaded models.

use std::env;
use std::fs;
use std::path::PathBuf;

use keyless_audio::AudioConfig;
use keyless_core::config::{Config, OutputMode};
use keyless_core::error::KeylessError;
use keyless_core::output::OutputSink;
use keyless_output::{DefaultOutputProvider, FileSink, OutputSinkProvider};
use keyless_whisper::WhisperConfig;

#[test]
fn test_config_to_file_sink_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    // Test the full config → provider → sink → file write pipeline

    let temp_file = env::temp_dir().join("keyless-integration-test.txt");
    let _ = fs::remove_file(&temp_file);

    // Create config with file output
    let config = Config {
        output_mode: OutputMode::File(temp_file.clone()),
        ..Default::default()
    };

    // Validate config
    assert!(config.validate().is_ok());

    // Use provider to create sink
    let provider = DefaultOutputProvider;
    let sink = provider.provide(&config)?;

    // Send text through sink
    sink.send_text("Integration test message")?;

    // Verify file was created and contains the text
    assert!(temp_file.exists());
    let contents = fs::read_to_string(&temp_file)?;
    assert!(contents.contains("Integration test message"));

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    Ok(())
}

#[test]
fn test_file_sink_direct() -> Result<(), Box<dyn std::error::Error>> {
    // Test FileSink directly without going through the provider

    let temp_file = env::temp_dir().join("keyless-direct-sink-test.txt");
    let _ = fs::remove_file(&temp_file);

    let sink = FileSink {
        path: temp_file.clone(),
    };

    // Send multiple transcriptions
    sink.send_text("First transcription")?;
    sink.send_text("Second transcription")?;

    let contents = fs::read_to_string(&temp_file)?;
    let lines: Vec<&str> = contents.lines().collect();

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "First transcription");
    assert_eq!(lines[1], "Second transcription");

    let _ = fs::remove_file(&temp_file);
    Ok(())
}

#[test]
fn test_audio_config_matches_whisper_config() {
    // Verify that AudioConfig and WhisperConfig can be created with compatible sample rates

    let audio_cfg = AudioConfig {
        sample_hz: 48_000,
        frame_samples: 4800,
    };

    let whisper_cfg = WhisperConfig {
        model_path: PathBuf::from("openai/whisper-tiny.en"),
        language: Some("en".to_string()),
        source_sample_hz: audio_cfg.sample_hz,
    };

    // Configs should have matching sample rates
    assert_eq!(audio_cfg.sample_hz, whisper_cfg.source_sample_hz);
}

#[test]
fn test_error_types_are_send_sync() {
    // Verify that our error type can be sent across threads (required for async)

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<KeylessError>();
    assert_sync::<KeylessError>();
}

#[test]
fn test_output_sink_trait_is_object_safe() {
    // Verify that OutputSink can be used as a trait object (required for dynamic dispatch)

    let temp_file = env::temp_dir().join("keyless-trait-object-test.txt");
    let _ = fs::remove_file(&temp_file);

    let sink = FileSink {
        path: temp_file.clone(),
    };

    // Use as trait object
    let boxed: Box<dyn OutputSink> = Box::new(sink);

    // Should be able to call through trait object
    assert!(boxed.send_text("Test via trait object").is_ok());

    let _ = fs::remove_file(&temp_file);
}
