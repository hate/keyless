//! Paste output sink.
//!
//! Simulates keystrokes to type transcribed text into the active window.
//! Uses the `enigo` crate for cross-platform keystroke simulation.

use std::thread;
use std::time::Duration;

use enigo::{Enigo, Keyboard, Settings as EnigoSettings};
use keyless_core::error::KeylessResult;
use keyless_core::output::OutputSink;
use tracing::{error, info};

/// Paste output sink using `enigo`.
///
/// Simulates keystrokes to type the transcribed text into the currently active window.
/// This is the most natural UX—text appears directly where the user is typing.
///
/// Requirements:
/// - **macOS**: Accessibility permissions (System Preferences → Security & Privacy → Accessibility)
/// - **Linux**: X11/Wayland input permissions (typically granted automatically)
/// - **Windows**: No special permissions required
///
/// ## Implementation Strategy: Per-Call Enigo Creation
///
/// This sink creates a new `Enigo` instance for each `send_text()` call rather than
/// storing one in the struct. This is a pragmatic workaround for cross-platform thread safety.
///
/// **Why we do this**:
/// - On macOS, `Enigo` wraps `CGEventSource`, which contains raw pointers (`*mut ()`)
/// - Raw pointers aren't `Send`, so `Enigo` can't be stored in a `Mutex` for thread-safe access
/// - Our architecture needs `OutputSink` to be `Send + Sync` for Tauri IPC and concurrent usage
///
/// **Tradeoffs**:
/// - **Pro**: Simple, safe, works identically on all platforms
/// - **Pro**: Enigo creation is cheap (<1ms); keystroke typing dominates latency anyway
/// - **Pro**: We only paste ~every 2s (on Final events), so overhead is negligible
/// - **Con**: Small per-call allocation overhead vs. reusing a stored instance
///
/// **Alternatives considered**:
/// - Store in `Mutex`: Doesn't compile on macOS due to `Send` requirements
/// - Platform-specific code: Complex to maintain, unnecessary for our use case
/// - Thread-local storage: Overkill for rare calls; adds complexity
///
/// For our usage pattern (infrequent, non-hot-path), this is the recommended approach.
pub struct PasteSink;

impl PasteSink {
    /// Initialize the paste sink.
    ///
    /// This is a no-op constructor since we create Enigo instances on-demand.
    /// We validate that enigo can be created to fail fast if permissions are missing.
    ///
    /// # Errors
    /// Returns `KeylessError::Output` if enigo initialization fails.
    pub fn new() -> KeylessResult<Self> {
        // Validate that we can create an Enigo instance
        match Enigo::new(&EnigoSettings::default()) {
            Ok(_) => Ok(Self),
            Err(e) => Err(keyless_core::error::KeylessError::Output(format!(
                "failed to initialize keystroke simulator: {}",
                e
            ))),
        }
    }
}

impl OutputSink for PasteSink {
    fn send_text(&self, text: &str) -> KeylessResult<()> {
        // No-op for empty strings (avoids delay and enigo setup)
        if text.is_empty() {
            return Ok(());
        }
        // Brief delay to ensure the active window is ready to receive input.
        // This prevents text from being lost if the window isn't fully focused yet.
        thread::sleep(Duration::from_millis(50));

        // Create a new Enigo instance for this paste operation.
        // This works around Send/Sync limitations on platforms like macOS.
        let mut enigo = match Enigo::new(&EnigoSettings::default()) {
            Ok(e) => e,
            Err(e) => {
                error!(error = %e, "failed to create keystroke simulator");
                return Err(keyless_core::error::KeylessError::Output(format!(
                    "failed to create keystroke simulator: {}",
                    e
                )));
            }
        };

        // Type the text character-by-character using enigo's text() method.
        // This handles special characters, Unicode, and platform differences automatically.
        match enigo.text(text) {
            Ok(()) => {
                info!(chars = text.chars().count(), "pasted text via keystrokes");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "keystroke simulation failed");
                Err(keyless_core::error::KeylessError::Output(format!(
                    "keystroke simulation failed: {}",
                    e
                )))
            }
        }
    }
}
