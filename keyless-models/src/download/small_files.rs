//! Small file download utilities.

use super::events::DownloadEvent;
use crate::net;
use keyless_core::error::{KeylessError, KeylessResult};
use reqwest::header::{HeaderName, HeaderValue};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

/// Download small model files (config.json, tokenizer.json) with retry/backoff.
///
/// Files are fetched into memory, then written atomically. Already downloaded
/// files are skipped (idempotent operation).
pub fn download_small_files(
    model_id: &str,
    repo_dir: &Path,
    client: &reqwest::blocking::Client,
    auth: &Option<(HeaderName, HeaderValue)>,
    tx: &SyncSender<DownloadEvent>,
    cancel: &Arc<AtomicBool>,
) -> KeylessResult<()> {
    for name in ["config.json", "tokenizer.json"] {
        // Check cancellation before each file (allows prompt abort).
        if cancel.load(Ordering::Relaxed) {
            return Err(KeylessError::Other("cancelled".into()));
        }
        let dst = repo_dir.join(name);
        // Skip if already downloaded (idempotent operation).
        if dst.exists() {
            continue;
        }
        let url = net::hf_resolve_url(model_id, name);
        let _ = tx.try_send(DownloadEvent::Stage {
            text: format!("downloading {}", name),
        });
        // Retry with exponential backoff (handles transient network errors).
        let (attempts, initial, max_ms) = crate::meta::backoff_config();
        let (bytes, content_len_hdr) =
            net::blocking_get_with_backoff(client, &url, auth, attempts, initial, max_ms)?;
        // Write file atomically (create overwrites if exists; we checked above, but defensive).
        let mut f = File::create(&dst).map_err(KeylessError::from)?;
        f.write_all(&bytes).map_err(KeylessError::from)?;
        // Verify downloaded size matches Content-Length header (catches truncation/corruption).
        if let Some(cl) = content_len_hdr
            && cl != bytes.len() as u64
        {
            return Err(KeylessError::Other(format!(
                "{} size mismatch ({} != {})",
                name,
                bytes.len(),
                cl
            )));
        }
    }
    Ok(())
}
