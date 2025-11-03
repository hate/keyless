//! keyless-core
//!
//! Shared domain types and utilities for the keyless workspace. Provides configuration,
//! error handling, event types, and output abstractions used by all other crates.
//!
//! ## Key Features
//!
//! - **Unified error type**: `KeylessError` with granular variants (Audio, Whisper, Output, Config, Ptt, Permissions)
//! - **Type-safe config**: `Config` with serde serialization and validation
//! - **Event model**: `TranscriptionEvent::Partial` (preview) and `Final` (complete)
//! - **Output abstraction**: `OutputSink` trait for pluggable backends
//! - **Language metadata**: Curated `LANG_CODES` and helpers (`lang_name`, `is_english_only_model`)
//!
//! ## Submodules
//!
//! - `config`: Application configuration schema and storage helpers
//! - `error`: `KeylessError` enum and `KeylessResult` type alias
//! - `events`: Transcription event types
//! - `options`: Model and language metadata
//! - `output`: `OutputSink` trait
//! - `permissions`: Platform permission helpers
//! - `utils`: Shared utilities (`now_millis`, `human_size`)
//!
//! ## Public API
//!
//! - `Config`, `OutputMode`, `VadThresholds`, `EqTuning`
//! - `KeylessError`, `KeylessResult`
//! - `TranscriptionEvent`
//! - `OutputSink` trait
//! - `LANG_CODES`, `lang_name()`, `is_english_only_model()`
//! - `now_millis()`, `human_size()`
//!
//! ## Quick Example
//!
//! ```no_run
//! use keyless_core::config::{Config, OutputMode};
//! use keyless_core::config::storage;
//!
//! // Load or create default config
//! let mut config = storage::load_config().unwrap_or_default();
//! config.output_mode = OutputMode::Clipboard;
//! config.validate()?;
//!
//! // Save atomically
//! storage::save_config(&config)?;
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

pub mod config;
pub mod error;
pub mod events;
pub mod options;
pub mod output;
pub mod permissions;
pub mod utils;
