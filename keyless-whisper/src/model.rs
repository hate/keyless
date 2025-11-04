//! Whisper model loading and management.
//!
//! This module handles:
//! - Downloading models from Hugging Face Hub (with caching)
//! - Loading models from local directories
//! - Parsing model configs and tokenizers
//! - Setting up Candle VarBuilder for weight loading
//! - Support for both normal (.safetensors) and quantized (.gguf) models
//! - Support for both multilingual and .en (English-only) models
//!
//! Default model: "openai/whisper-tiny" (~75 MB, fast, multilingual)

/// File detection and location helpers.
mod files;
/// Model loading with progress callbacks.
mod loader;
/// Mel filter bank generation.
mod mel_filters;
/// Model type definitions (Model enum, WhisperModel, WhisperTokens).
mod types;

pub use loader::{load_whisper_model, load_whisper_model_with_progress};
pub use types::{Model, WhisperModel, WhisperTokens};
