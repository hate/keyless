//! keyless-whisper
//!
//! Real‑time speech‑to‑text transcription using Whisper via Candle. Provides a clean,
//! modular Whisper transcription engine using Hugging Face's Candle ML framework.
//!
//! Architecture: `Audio Thread → push_audio() → Queue → Worker Thread (resample/accumulate) → Inference Thread (Candle) → Event Queue → try_next_event()`
//!
//! ## Key Features
//!
//! - **Quantized model support**: 3-4x faster inference with GGUF models
//! - **Quality improvements**: No-speech detection, temperature fallback, quality metrics
//! - **Performance optimized**: Zero-copy model access, single encoder pass
//! - **Language auto-detection**: Automatic language identification when not specified
//! - **Segment-aware**: Live preview while speaking; single Final on PTT release
//! - **Windowed inference**: ≤30s windows for long utterances (Whisper's native context)
//! - **Non-blocking**: Audio/resample never blocks; inference on dedicated thread
//! - **High-quality resampling**: Arbitrary-rate via rubato (16k, 32k, 44.1k, 48k all supported)
//!
//! ## Submodules
//!
//! - `model`: Model downloading/loading, quantized support (GGUF + safetensors)
//! - `preprocessing`: PCM audio preprocessing and mel spectrogram computation
//! - `decode`: Token generation, temperature fallback, no-speech detection, quality metrics
//! - `inference`: Encoder‑decoder pipeline with windowing and language auto-detection
//!
//! ## Public API
//!
//! - `Whisper`: Main transcriber implementation
//! - `RealtimeTranscriber`: Trait for real-time transcription
//! - `WhisperConfig`: Configuration (model path, language, sample rate)
//!
//! ## Quick Example
//!
//! ```rust,no_run
//! use keyless_whisper::{Whisper, WhisperConfig, RealtimeTranscriber};
//! use std::path::PathBuf;
//!
//! let cfg = WhisperConfig {
//!     model_path: PathBuf::from("openai/whisper-base"),  // Multilingual recommended
//!     language: Some("en".to_string()),                   // Manual faster than auto-detect
//!     source_sample_hz: 48_000,
//! };
//! let mut transcriber = Whisper::new(cfg)?;
//! // transcriber.push_audio(&frame)?;
//! // if let Some(event) = transcriber.try_next_event() { /* handle */ }
//! # Ok::<(), keyless_core::error::KeylessError>(())
//! ```

#![deny(warnings)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(
    doc,
    deny(
        missing_docs,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_html_tags,
        rustdoc::bare_urls
    )
)]

/// Configuration types for Whisper transcriber.
mod config;
/// Token generation and text decoding.
mod decode;
/// Device selection and caching.
mod device;
/// Whisper inference implementation.
pub mod inference;
/// Whisper model loading and management.
pub mod model;
/// Audio preprocessing and mel spectrogram computation.
mod preprocessing;
/// RealtimeTranscriber trait definition.
mod transcriber;
/// Whisper transcriber implementation.
mod whisper;

pub use config::{PhaseState, WhisperConfig, WhisperLoadPhase};
pub use device::preload_device;
pub use transcriber::RealtimeTranscriber;
pub use whisper::Whisper;
