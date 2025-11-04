//! Progress events for model download.

use keyless_core::error::KeylessResult;

/// Progress events for model download.
#[derive(Debug)]
pub enum DownloadEvent {
    /// Download started for the given model.
    Started {
        /// Model identifier (e.g., "openai/whisper-tiny").
        model: String,
    },
    /// Stage update (e.g., "downloading config.json").
    Stage {
        /// Human-readable stage description.
        text: String,
    },
    /// Progress update with bytes downloaded, total size, speed, and ETA.
    Progress {
        /// Bytes downloaded so far.
        bytes: u64,
        /// Total bytes if known (Some), else None.
        total: Option<u64>,
        /// Throughput in megabytes per second.
        mbps: f64,
        /// Estimated seconds remaining.
        eta_s: f64,
    },
    /// Download completed (Ok) or failed (Err).
    Done(KeylessResult<()>),
}
