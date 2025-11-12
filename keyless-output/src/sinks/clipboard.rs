//! Clipboard output sink.
//!
//! Copies transcribed text to the system clipboard using the `arboard` crate.
//! Users can then paste manually with Cmd+V (macOS) or Ctrl+V (Windows/Linux).

use std::sync::Mutex;

use arboard::Clipboard;
use keyless_core::error::KeylessResult;
use keyless_core::output::OutputSink;
use tracing::{error, info};

/// Clipboard output sink using `arboard`.
///
/// Copies transcribed text to the system clipboard. The user can then paste
/// it anywhere with Cmd+V (macOS) or Ctrl+V (Windows/Linux).
///
/// Thread safety: `Clipboard` isn't Send/Sync, so we wrap it in a Mutex.
/// This allows the sink to be shared across threads (e.g., Tauri IPC).
pub struct ClipboardSink {
    /// Underlying clipboard handle (guarded by a mutex for thread safety).
    clipboard: Mutex<Clipboard>,
}

impl ClipboardSink {
    /// Initialize the clipboard sink.
    ///
    /// This may fail if the clipboard isn't available (e.g., headless environment,
    /// permission denied on some platforms).
    ///
    /// # Errors
    /// Returns `KeylessError::Output` if clipboard access fails.
    pub fn new() -> KeylessResult<Self> {
        // Initialize clipboard; may fail in headless environments or without permissions.
        match Clipboard::new() {
            Ok(cb) => Ok(Self {
                // Wrap in Mutex for thread safety (Clipboard isn't Send/Sync).
                clipboard: Mutex::new(cb),
            }),
            Err(e) => Err(keyless_core::error::KeylessError::Output(format!(
                "failed to access clipboard: {}",
                e
            ))),
        }
    }
}

impl OutputSink for ClipboardSink {
    fn send_text(&self, text: &str) -> KeylessResult<()> {
        // Lock mutex to access clipboard (guards against concurrent access).
        match self.clipboard.lock() {
            Ok(mut cb) => {
                // to_string() clones text (set_text requires owned String).
                match cb.set_text(text.to_string()) {
                    Ok(()) => {
                        info!(len = text.len(), "copied text to clipboard");
                        Ok(())
                    }
                    Err(e) => {
                        // Clipboard operation failed (may be permission or system issue).
                        error!(error = %e, "clipboard set_text failed");
                        Err(keyless_core::error::KeylessError::Output(format!(
                            "clipboard set_text failed: {}",
                            e
                        )))
                    }
                }
            }
            Err(_) => {
                // Mutex poisoned (previous panic left lock in bad state).
                error!("clipboard mutex poisoned");
                Err(keyless_core::error::KeylessError::Output(
                    "clipboard mutex poisoned".to_string(),
                ))
            }
        }
    }

    fn label(&self) -> String {
        "Clipboard".to_string()
    }
}
