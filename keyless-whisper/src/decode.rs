//! Token generation and text decoding.
//!
//! This module handles the decoder side of Whisper inference:
//! - Autoregressive token generation using the TextDecoder
//! - Token ID → text conversion using the tokenizer
//! - Filtering of special tokens (SOT, EOT, language tags, etc.)
//! - Quality metrics: no-speech detection, avg log probability, compression ratio
//! - Temperature fallback for difficult audio
//!
//! - Feed full token sequence every step (not last-token-only)
//! - Flush KV cache only on first iteration (`i == 0`)
//! - Use `decoder.final_linear()` for vocab projection
//! - Greedy sampling with repetition guards

/// Decoding constants (thresholds, temperatures).
mod constants;
/// Temperature fallback decoding logic.
mod fallback;
/// Helper functions for token decoding.
mod helpers;
/// Language detection from audio features.
mod language;
/// Decoding result type with quality metrics.
mod result;
/// Single-temperature decoding implementation.
mod temperature;

pub use fallback::decode_with_fallback;
pub use language::detect_language_from_features;
