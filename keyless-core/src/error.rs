//! Error types for the keyless application.
//!
//! This module defines all errors that can occur across the application layers.
//! Using a single error enum simplifies error propagation and handling.
//!
//! Design rationale:
//! - Categorized by module (Audio, Whisper, Output, Config, etc.) for clarity
//! - String payloads for context (e.g., device names, file paths, error details)
//! - thiserror auto-generates Display and Error trait implementations
//! - No `unwrap` or `expect` anywhere in the codebase; all errors are explicit

use std::{io, path::PathBuf};

use thiserror::Error;

/// Application-wide error type.
///
/// Each variant represents a category of error that can occur in different subsystems.
/// Errors carry context strings to aid debugging without panicking.
#[derive(Debug, Error)]
pub enum KeylessError {
    /// Audio subsystem errors (device unavailable, stream build failure, etc.)
    #[error("audio error: {0}")]
    Audio(String),

    /// Whisper transcription errors (model loading, inference failure, queue overflow)
    #[error("whisper error: {0}")]
    Whisper(String),

    /// Output sink errors (clipboard access, file write, paste simulation failure)
    #[error("output error: {0}")]
    Output(String),

    /// Configuration validation errors (invalid language code, bad paths, etc.)
    #[error("config error: {0}")]
    Config(String),

    /// Standard I/O errors (file not found, permission denied, etc.)
    /// Auto-converted via `#[from]` for ergonomic `?` usage.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// Path validation errors (directory doesn't exist, invalid model path, etc.)
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),

    /// OS-level or system integration errors (permissions, platform APIs, etc.)
    #[error("system error: {0}")]
    System(String),

    /// Global push-to-talk (PTT) listener errors (backend init/listen failures, etc.)
    #[error("ptt error: {0}")]
    Ptt(String),

    /// Permissions-related errors (e.g., macOS Accessibility not enabled)
    #[error("permissions error: {0}")]
    Permissions(String),

    /// Catch-all for unclassified errors; prefer specific variants when possible
    #[error("unknown error: {0}")]
    Other(String),
}

/// Convenience alias for Result<T, KeylessError>.
///
/// Use this throughout the codebase for consistency and readability.
/// Example: `fn do_work() -> KeylessResult<()> { ... }`
pub type KeylessResult<T> = Result<T, KeylessError>;
