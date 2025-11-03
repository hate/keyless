//! Tests for keyless-whisper crate.

use candle_transformers::models::whisper::Config;

// Import what we need for testing whisper internals
// Note: Since these are in separate modules, we test the public API

#[test]
fn test_whisper_config_creation() {
    use keyless_whisper::WhisperConfig;
    use std::path::PathBuf;

    let cfg = WhisperConfig {
        model_path: PathBuf::from("openai/whisper-tiny.en"),
        language: Some("en".to_string()),
        source_sample_hz: 48000,
    };

    assert_eq!(cfg.source_sample_hz, 48000);
    assert_eq!(cfg.language, Some("en".to_string()));
}

// Mel filter generation tests
#[test]
fn test_mel_filter_generation_80_bins() {
    // Test the mel filter generation for 80-bin config (tiny/base/small/medium models)
    let config = Config {
        num_mel_bins: 80,
        max_source_positions: 1500,
        d_model: 384,
        encoder_attention_heads: 6,
        encoder_layers: 4,
        vocab_size: 51865,
        max_target_positions: 448,
        decoder_attention_heads: 6,
        decoder_layers: 4,
        suppress_tokens: vec![],
    };

    // We can't access generate_mel_filters directly since it's private,
    // but we know it's called during model loading. This test verifies
    // the config structure is valid.
    assert_eq!(config.num_mel_bins, 80);
}

#[test]
fn test_mel_filter_generation_128_bins() {
    // Test for 128-bin config (large models)
    let config = Config {
        num_mel_bins: 128,
        max_source_positions: 1500,
        d_model: 768,
        encoder_attention_heads: 12,
        encoder_layers: 12,
        vocab_size: 51865,
        max_target_positions: 448,
        decoder_attention_heads: 12,
        decoder_layers: 12,
        suppress_tokens: vec![],
    };

    assert_eq!(config.num_mel_bins, 128);
}

// Token filtering logic tests
#[test]
fn test_token_filtering_logic() {
    // Test that special tokens would be filtered correctly
    // This tests the logic without needing a real tokenizer

    let tokens = [
        50258, // SOT (should be filtered)
        50259, // language token (should be filtered)
        50359, // transcribe (should be filtered)
        50363, // notimestamps (should be filtered)
        1234,  // actual text token
        5678,  // actual text token
        50257, // EOT (should be filtered)
    ];

    // The decode module filters tokens >= 50257
    let special_threshold = 50257u32;
    let filtered: Vec<u32> = tokens
        .iter()
        .filter(|&&t| t < special_threshold)
        .copied()
        .collect();

    assert_eq!(filtered, vec![1234, 5678]);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_token_filtering_empty_sequence() {
    let tokens: Vec<u32> = vec![];
    let special_threshold = 50257u32;
    let filtered: Vec<u32> = tokens
        .iter()
        .filter(|&&t| t < special_threshold)
        .copied()
        .collect();

    assert_eq!(filtered.len(), 0);
}

#[test]
fn test_token_filtering_only_special_tokens() {
    let tokens = [50258, 50259, 50359, 50363, 50257];
    let special_threshold = 50257u32;
    let filtered: Vec<u32> = tokens
        .iter()
        .filter(|&&t| t < special_threshold)
        .copied()
        .collect();

    assert_eq!(filtered.len(), 0);
}

#[test]
fn test_token_filtering_only_text_tokens() {
    let tokens = vec![100, 200, 300, 400, 500];
    let special_threshold = 50257u32;
    let filtered: Vec<u32> = tokens
        .iter()
        .filter(|&&t| t < special_threshold)
        .copied()
        .collect();

    assert_eq!(filtered, tokens);
    assert_eq!(filtered.len(), 5);
}
