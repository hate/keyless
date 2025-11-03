//! Output sink provider (factory).
//!
//! Creates the appropriate output sink based on user config.

use keyless_core::config::{Config, OutputMode};
use keyless_core::error::KeylessResult;
use keyless_core::output::OutputSink;

use crate::{ClipboardSink, FileSink, PasteSink};

/// Factory trait for creating output sinks from config.
///
/// Implementations inspect the `Config::output_mode` and return a boxed
/// `OutputSink` trait object, allowing dynamic dispatch.
///
/// This abstraction keeps configuration logic separate from output mechanics.
pub trait OutputSinkProvider {
    /// Create an output sink based on the given config.
    ///
    /// # Arguments
    /// * `config` - User configuration specifying the desired output mode
    ///
    /// # Returns
    /// A boxed trait object implementing `OutputSink`, ready to receive text.
    ///
    /// # Errors
    /// Returns `KeylessError::Output` if the requested mode isn't available or
    /// initialization fails (e.g., clipboard unavailable, file path invalid).
    fn provide(&self, config: &Config) -> KeylessResult<Box<dyn OutputSink>>;
}

/// Default output sink factory.
///
/// This provider supports Clipboard, File, and Paste modes.
pub struct DefaultOutputProvider;

impl OutputSinkProvider for DefaultOutputProvider {
    fn provide(&self, config: &Config) -> KeylessResult<Box<dyn OutputSink>> {
        match &config.output_mode {
            // Clipboard mode: wrap arboard Clipboard in our ClipboardSink
            OutputMode::Clipboard => Ok(Box::new(ClipboardSink::new()?)),

            // File mode: create a sink that appends to the specified path
            OutputMode::File(path) => Ok(Box::new(FileSink { path: path.clone() })),

            // Paste mode: simulate keystrokes into the active window
            OutputMode::Paste => Ok(Box::new(PasteSink::new()?)),
        }
    }
}
