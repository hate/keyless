//! Output sink enumeration and selection service.
//!
//! Provides a trait-based abstraction for listing available output sinks (paste, clipboard, file)
//! and selecting a sink. The implementation reads from the current configuration and updates
//! it when a sink is selected.

use std::path::PathBuf;

use keyless_core::config::{Config, OutputMode, storage};

/// Service trait for output sink management.
///
/// Provides methods to list available sinks and select a sink. Sink selection updates
/// the persisted configuration.
pub trait SinksService: Send + Sync {
    /// List all available output sink IDs.
    ///
    /// Returns a list of sink IDs: "paste", "clipboard", "file".
    fn list(&self) -> Vec<String>;

    /// Select an output sink by ID and persist the selection.
    ///
    /// Updates the configuration with the selected sink. For "file" sink, uses the
    /// existing file path from config or creates a default path.
    ///
    /// # Errors
    ///
    /// Returns an error string if the sink ID is unknown or config save fails.
    fn select(&self, id: &str) -> Result<SinkSelectionResult, String>;
}

/// Result of selecting an output sink.
#[derive(Debug, Clone)]
pub struct SinkSelectionResult {
    /// Selected sink ID ("paste", "clipboard", or "file").
    pub sink_id: String,
    /// File path, if sink is "file".
    pub file_path: Option<PathBuf>,
}

/// Default implementation of `SinksService` using configuration storage.
pub struct SinksServiceImpl;

impl SinksServiceImpl {
    /// Create a new `SinksServiceImpl` instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SinksServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl SinksService for SinksServiceImpl {
    fn list(&self) -> Vec<String> {
        // Return the three supported output sinks: paste (simulates typing), clipboard (copies to clipboard),
        // and file (writes to a text file).
        vec!["paste".into(), "clipboard".into(), "file".into()]
    }

    fn select(&self, id: &str) -> Result<SinkSelectionResult, String> {
        // Load the current configuration (or defaults if none exists).
        let mut cfg: Config = storage::load_config().unwrap_or_default();
        let file_path;

        // Update the output mode based on the selected sink ID.
        match id {
            "paste" => {
                // Paste sink: simulates keyboard typing to insert text into the active application.
                cfg.output_mode = OutputMode::Paste;
                file_path = None; // No file path needed for paste sink.
            }
            "clipboard" => {
                // Clipboard sink: copies transcribed text to the system clipboard.
                cfg.output_mode = OutputMode::Clipboard;
                file_path = None; // No file path needed for clipboard sink.
            }
            "file" => {
                // File sink: writes transcribed text to a file.
                // If the config already has a file path, reuse it. Otherwise, create a default path.
                let target_path: PathBuf = match &cfg.output_mode {
                    OutputMode::File(p) => {
                        // Reuse existing file path from config.
                        p.clone()
                    }
                    _ => {
                        // No existing file path: create a default one.
                        // Try to use HOME directory, fall back to temp directory if HOME isn't set.
                        let base = std::env::var("HOME")
                            .map(PathBuf::from)
                            .unwrap_or_else(|_| std::env::temp_dir());
                        let candidate = base.join("keyless.txt");
                        // Ensure the parent directory exists (create it if needed).
                        // Ignore errors here as the file write will fail if we can't create the directory anyway.
                        if let Some(parent) = candidate.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        candidate
                    }
                };
                file_path = Some(target_path.clone());
                cfg.output_mode = OutputMode::File(target_path);
            }
            other => return Err(format!("unknown sink: {other}")),
        }

        // Persist the updated configuration to disk.
        storage::save_config(&cfg).map_err(|e| format!("save config: {e}"))?;

        // Return the selection result with the sink ID and optional file path.
        Ok(SinkSelectionResult {
            sink_id: id.to_string(),
            file_path,
        })
    }
}
