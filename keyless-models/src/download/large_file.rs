//! Large file streaming download with progress reporting and resume support.

use super::events::DownloadEvent;
use super::progress;
use crate::net;
use keyless_core::error::{KeylessError, KeylessResult};
use reqwest::header::{HeaderName, HeaderValue};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

/// Download the large model.safetensors file with streaming, progress reporting, and resume support.
///
/// Uses HTTP Range headers to resume from `.partial` files if paused. Progress events
/// are emitted at most every 250ms to prevent channel saturation.
pub fn download_large_file(
    model_id: &str,
    repo_dir: &Path,
    client: &reqwest::blocking::Client,
    auth: &Option<(HeaderName, HeaderValue)>,
    tx: &SyncSender<DownloadEvent>,
    cancel: &Arc<AtomicBool>,
) -> KeylessResult<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(KeylessError::Other("cancelled".into()));
    }

    let weights = repo_dir.join("model.safetensors");
    if weights.exists() {
        return Ok(());
    }

    let url = net::hf_resolve_url(model_id, "model.safetensors");
    let mut req = client.get(&url);
    // Add auth header if available (for private models or rate limit increases).
    if let Some((h, v)) = auth {
        req = req.header(h, v.clone());
    }

    // Resume support: check for existing partial file to resume download.
    let tmp = repo_dir.join("model.safetensors.partial");
    let mut downloaded: u64 = 0;
    // Read partial file size to determine resume offset.
    if tmp.exists()
        && let Ok(m) = fs::metadata(&tmp)
    {
        downloaded = m.len();
    }
    // Add Range header if resuming (bytes=<offset>- requests from offset to end).
    if downloaded > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={}-", downloaded));
    }

    let mut resp = req
        .send()
        .map_err(|e| KeylessError::Other(format!("GET {}: {}", url, e)))?;
    let status = resp.status();

    let mut file: Box<dyn Write>;
    if status == reqwest::StatusCode::PARTIAL_CONTENT {
        // Resume OK (206): server honored Range header; append to existing partial file.
        resp = resp
            .error_for_status()
            .map_err(|e| KeylessError::Other(e.to_string()))?;
        file = Box::new(
            std::fs::OpenOptions::new()
                .append(true)
                .open(&tmp)
                .map_err(KeylessError::from)?,
        );
    } else if status == reqwest::StatusCode::OK {
        // Range ignored (200): server doesn't support Range; restart from beginning.
        // Remove partial file to avoid corruption (truncate would also work).
        let _ = fs::remove_file(&tmp);
        downloaded = 0;
        resp = resp
            .error_for_status()
            .map_err(|e| KeylessError::Other(e.to_string()))?;
        // Create new file (truncate overwrites existing; defensive for edge cases).
        file = Box::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)
                .map_err(KeylessError::from)?,
        );
    } else {
        // Unexpected status (e.g., 416 Range Not Satisfiable); fail fast.
        return Err(KeylessError::Other(format!("unexpected status {}", status)));
    }

    // Calculate total size: for 206, add downloaded bytes to Content-Length (partial response).
    // For 200, Content-Length is full file size (no resume offset).
    let total = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        resp.content_length()
            .unwrap_or(0)
            .saturating_add(downloaded)
    } else {
        resp.content_length().unwrap_or(0)
    };
    // Cache total size for future resume attempts (if known).
    if total > 0 {
        crate::meta::update_saved_size(model_id, total);
    }

    // Stream and write chunks with progress reporting.
    progress::stream_with_progress(&mut resp, &mut file, total, downloaded, tx, cancel)?;

    // Close file handle before rename (ensures all buffers flushed to disk).
    drop(file);
    // Atomic rename: partial → final (prevents partial files from being used).
    fs::rename(&tmp, &weights).map_err(KeylessError::from)?;

    // Verify final size if we know total (catches truncation/corruption).
    if total > 0 {
        let final_len = fs::metadata(&weights).map_err(KeylessError::from)?.len();
        if final_len != total {
            // Clean up corrupted file (better to have no file than wrong size).
            // Controller will surface error to user and allow retry.
            let _ = fs::remove_file(&weights);
            return Err(KeylessError::Other(format!(
                "downloaded size mismatch ({} != {}), please retry",
                final_len, total
            )));
        }
        // Emit a final 100% progress to fully fill the gauge (UI completeness).
        let _ = tx.try_send(DownloadEvent::Progress {
            bytes: total,
            total: Some(total),
            mbps: 0.0,
            eta_s: 0.0,
        });
    }

    Ok(())
}
