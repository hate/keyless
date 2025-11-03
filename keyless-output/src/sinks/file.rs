//! File output sink.
//!
//! Appends transcribed text to a file, one line per transcription.
//! Useful for logging, archiving, or feeding to other tools.

use std::fs::OpenOptions;
use std::io::Write as _;

use keyless_core::error::KeylessResult;
use keyless_core::output::OutputSink;
use tracing::{error, info};

/// File output sink.
///
/// Appends each transcribed text segment to a file on disk, one line per segment.
/// This is useful for:
/// - Logging transcriptions for later review
/// - Archiving dictation sessions
/// - Feeding transcriptions to other tools (e.g., text analysis pipelines)
///
/// The file is created if it doesn't exist; parent directories must already exist
/// (validated during Config::validate()).
pub struct FileSink {
    /// Path to the output file. Each call to `send_text` appends a line.
    pub path: std::path::PathBuf,
}

impl OutputSink for FileSink {
    fn send_text(&self, text: &str) -> KeylessResult<()> {
        // Avoid work if there's nothing to write
        if text.is_empty() {
            return Ok(());
        }
        // Open in append mode, creating the file if it doesn't exist
        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(e) => {
                error!(path = %self.path.display(), error = %e, "failed to open output file");
                return Err(keyless_core::error::KeylessError::Output(format!(
                    "failed to open file {}: {}",
                    self.path.display(),
                    e
                )));
            }
        };

        // Write the text followed by a newline without allocating an intermediate String
        if let Err(e) = file.write_all(text.as_bytes()) {
            error!(path = %self.path.display(), error = %e, "failed to write to file");
            return Err(keyless_core::error::KeylessError::Output(format!(
                "failed to write to file {}: {}",
                self.path.display(),
                e
            )));
        }
        match file.write_all(b"\n") {
            Ok(()) => {
                // bytes: text length + 1 for newline
                info!(path = %self.path.display(), bytes = text.len() + 1, "appended transcription to file");
                Ok(())
            }
            Err(e) => {
                error!(path = %self.path.display(), error = %e, "failed to write to file");
                Err(keyless_core::error::KeylessError::Output(format!(
                    "failed to write to file {}: {}",
                    self.path.display(),
                    e
                )))
            }
        }
    }
}
