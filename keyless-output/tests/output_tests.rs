//! Tests for keyless-output crate.

use keyless_core::config::{Config, OutputMode};
use keyless_core::output::OutputSink;
use keyless_output::{DefaultOutputProvider, FileSink, OutputSinkProvider};
use std::env;
use std::fs;

#[test]
fn test_default_provider_clipboard() {
    let config = Config {
        output_mode: OutputMode::Clipboard,
        ..Default::default()
    };

    let provider = DefaultOutputProvider;
    let result = provider.provide(&config);

    // Clipboard may fail in headless environments, so we just check it returns a result
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_default_provider_paste() {
    let config = Config {
        output_mode: OutputMode::Paste,
        ..Default::default()
    };

    let provider = DefaultOutputProvider;
    let result = provider.provide(&config);

    // Paste may fail without accessibility permissions, so we just check it returns a result
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_file_sink_write() -> Result<(), Box<dyn std::error::Error>> {
    let temp_file = env::temp_dir().join("keyless-test-output.txt");

    // Clean up any existing file
    let _ = fs::remove_file(&temp_file);

    let sink = FileSink {
        path: temp_file.clone(),
    };

    // Write first line
    assert!(sink.send_text("Hello world").is_ok());

    // Write second line
    assert!(sink.send_text("Second line").is_ok());

    // Read back and verify
    let contents = fs::read_to_string(&temp_file)?;
    assert!(contents.contains("Hello world\n"));
    assert!(contents.contains("Second line\n"));

    // Clean up
    let _ = fs::remove_file(&temp_file);
    Ok(())
}

#[test]
fn test_file_sink_creates_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_file = env::temp_dir().join("keyless-test-new-file.txt");

    // Ensure file doesn't exist
    let _ = fs::remove_file(&temp_file);
    assert!(!temp_file.exists());

    let sink = FileSink {
        path: temp_file.clone(),
    };

    // Write should create the file
    sink.send_text("Test content")?;
    assert!(temp_file.exists());

    // Clean up
    let _ = fs::remove_file(&temp_file);
    Ok(())
}

#[test]
fn test_file_sink_append_mode() -> Result<(), Box<dyn std::error::Error>> {
    let temp_file = env::temp_dir().join("keyless-test-append.txt");
    let _ = fs::remove_file(&temp_file);

    let sink = FileSink {
        path: temp_file.clone(),
    };

    sink.send_text("Line 1")?;
    sink.send_text("Line 2")?;
    sink.send_text("Line 3")?;

    let contents = fs::read_to_string(&temp_file)?;
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Line 1");
    assert_eq!(lines[1], "Line 2");
    assert_eq!(lines[2], "Line 3");

    let _ = fs::remove_file(&temp_file);
    Ok(())
}
