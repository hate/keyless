//! Modal overlay states (error, download, expert mode).

use keyless_core::error::KeylessError;

use crate::tui::config::expert::EqParameter;

/// Modal overlay states for the TUI.
pub struct Overlays {
    pub error_message: Option<KeylessError>,
    pub download: Option<DownloadState>,
    pub expert: Option<ExpertOverlay>,
}

impl Overlays {
    pub fn new() -> Self {
        Self {
            error_message: None,
            download: None,
            expert: None,
        }
    }
}

impl Default for Overlays {
    fn default() -> Self {
        Self::new()
    }
}

/// Active download state.
pub struct DownloadState {
    pub model: String,
    pub message: String,
}

impl DownloadState {
    pub fn new(model: String, message: String) -> Self {
        Self { model, message }
    }
}

/// Expert mode UI overlay (selection cursor and confirmations only).
/// The actual EQ tuning lives in AppState and is always accessible.
pub struct ExpertOverlay {
    pub selected: EqParameter,
    pub confirm_reset: bool,
    pub confirm_purge: bool,
}

impl ExpertOverlay {
    pub fn new() -> Self {
        Self {
            selected: EqParameter::Bands,
            confirm_reset: false,
            confirm_purge: false,
        }
    }
}

impl Default for ExpertOverlay {
    fn default() -> Self {
        Self::new()
    }
}
