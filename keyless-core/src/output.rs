//! Output sink trait for delivering transcribed text.
//!
//! This module defines the interface that all output destinations must implement.
//! Implementations live in the `keyless-output` crate (clipboard, paste, file).
//!
//! The trait is object-safe (Send + Sync) to allow dynamic dispatch via `Box<dyn OutputSink>`.

use crate::error::KeylessResult;

/// Trait for delivering transcribed text to an output destination.
///
/// All output sinks must implement this trait. Implementations handle:
/// - Clipboard: copy text to system clipboard
/// - Paste: simulate keystrokes in the active application
/// - File: append text to a file
///
/// Design notes:
/// - `Send + Sync` because sinks may be used across threads (e.g., Tauri IPC)
/// - Method takes `&self` (not `&mut self`) to support concurrent usage
/// - Returns `KeylessResult<()>` for explicit error handling (no panics)
pub trait OutputSink: Send + Sync {
    /// Send transcribed text to this output destination.
    ///
    /// Implementations should:
    /// - Handle errors gracefully (e.g., clipboard unavailable, file write failure)
    /// - Be non-blocking where practical to avoid blocking UI threads
    /// - Log or report failures without panicking
    ///
    /// # Arguments
    /// * `text` - The transcribed text to deliver (always a Final event, never Partial)
    ///
    /// # Errors
    /// Returns `KeylessError::Output` if delivery fails (e.g., clipboard locked,
    /// file permission denied).
    fn send_text(&self, text: &str) -> KeylessResult<()>;
}
