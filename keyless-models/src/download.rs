//! Model download with progress reporting and cancellation support.
//!
//! Downloads use Range headers to resume from `.partial` files if paused.
//! Progress events are emitted via channel for UI integration.

use crate::{hf, meta, net};
use keyless_core::error::{KeylessError, KeylessResult};
use std::fs::{self, File};
use std::io::Write;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use tracing::{error, info};

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

/// Ensure the required Whisper model files are present in the local cache.
///
/// Downloads are implemented with `reqwest` (no `hf-hub`). We fetch small
/// files (`config.json`, `tokenizer.json`) with retry/backoff, then stream
/// `model.safetensors` with Range resume into a `.partial` file and atomically
/// rename on completion. Progress and stage events are emitted for UI integration.
///
/// An optional token (`HF_TOKEN` or `HUGGINGFACE_HUB_TOKEN`) is used if set.
///
/// # Arguments
/// * `model_id` - Hugging Face model identifier (e.g., "openai/whisper-base")
/// * `tx` - Channel to send progress events
/// * `cancel` - Atomic flag to check for cancellation requests
pub fn ensure_model_cached(
    model_id: String,
    tx: SyncSender<DownloadEvent>,
    cancel: Arc<AtomicBool>,
) {
    // Try-send for non-blocking start event (may drop if channel full; OK for best-effort).
    let _ = tx.try_send(DownloadEvent::Started {
        model: model_id.clone(),
    });
    info!(model = %model_id, "starting model download");
    // Clone channel senders for different scopes (closure captures require owned values).
    let tx_clone = tx.clone();
    let tx_for_loop = tx.clone();
    let result: KeylessResult<()> = (|| {
        // Blocking reqwest client (with timeouts); this function is not async.
        let client =
            net::build_blocking_client().map_err(|e| KeylessError::Other(e.to_string()))?;
        // Optional auth header (HF token from env); works without token for public models.
        let auth = net::auth_header();

        // Ensure repo cache dir exists (e.g., ~/.cache/keyless/openai--whisper-base/).
        let repo_dir = hf::keyless_cache_repo_dir(&model_id);
        fs::create_dir_all(&repo_dir).map_err(KeylessError::from)?;

        // Plan line (if size known from cache): show total expected size to user.
        if let Some(total) = meta::plan_total_bytes(&model_id) {
            let _ = tx_clone.try_send(DownloadEvent::Stage {
                text: format!("plan: {}", keyless_core::utils::human_size(total)),
            });
        }

        // Small files (with retry/backoff): fetch into memory, then write atomically.
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
            let url = net::hf_resolve_url(&model_id, name);
            let _ = tx_for_loop.try_send(DownloadEvent::Stage {
                text: format!("downloading {}", name),
            });
            // Retry with exponential backoff (handles transient network errors).
            let (attempts, initial, max_ms) = meta::backoff_config();
            let (bytes, content_len_hdr) =
                net::blocking_get_with_backoff(&client, &url, &auth, attempts, initial, max_ms)?;
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

        // Large file streaming with progress: stream chunks to avoid memory issues.
        if cancel.load(Ordering::Relaxed) {
            return Err(KeylessError::Other("cancelled".into()));
        }
        let weights = repo_dir.join("model.safetensors");
        if !weights.exists() {
            let url = net::hf_resolve_url(&model_id, "model.safetensors");
            let mut req = client.get(&url);
            // Add auth header if available (for private models or rate limit increases).
            if let Some((h, v)) = &auth {
                req = req.header(h, v.clone());
            }
            // Resume support: check for existing partial file to resume download.
            let tmp = repo_dir.join("model.safetensors.partial");
            let mut downloaded: u64 = 0;
            // Read partial file size to determine resume offset.
            if tmp.exists()
                && let Ok(m) = std::fs::metadata(&tmp)
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
            let mut file: Box<dyn std::io::Write>;
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
                let _ = std::fs::remove_file(&tmp);
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
                meta::update_saved_size(&model_id, total);
            }
            // Track time for speed/ETA calculations.
            let start = std::time::Instant::now();
            let mut last_emit = std::time::Instant::now();
            // 64KB buffer: balances memory usage vs I/O syscall overhead.
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                // Check cancellation before reading (allows prompt abort on slow networks).
                if cancel.load(Ordering::Relaxed) {
                    return Err(KeylessError::Other("cancelled".into()));
                }
                use std::io::Read;
                // Read chunk from response stream (may return < buf.len() at end).
                let n = resp
                    .read(&mut buf)
                    .map_err(|e| KeylessError::Other(format!("read chunk: {}", e)))?;
                // EOF: n == 0 indicates stream end (normal completion).
                if n == 0 {
                    break;
                }
                // Check cancellation after read (allows abort during write).
                if cancel.load(Ordering::Relaxed) {
                    return Err(KeylessError::Other("cancelled".into()));
                }
                // Write chunk to file (only write n bytes, not full buffer).
                file.write_all(&buf[..n]).map_err(KeylessError::from)?;
                downloaded += n as u64;
                // Throttle progress events to at most every 250ms (prevents channel saturation).
                if last_emit.elapsed().as_millis() >= 250 {
                    last_emit = std::time::Instant::now();
                    if total > 0 && downloaded <= total {
                        // Calculate speed (MB/s) and ETA (seconds remaining).
                        // max(0.001) prevents division by zero on very fast downloads.
                        let secs = start.elapsed().as_secs_f64().max(0.001);
                        let mbps = (downloaded as f64 / 1_000_000.0) / secs;
                        // saturating_sub prevents underflow if downloaded > total (shouldn't happen).
                        let left = (total.saturating_sub(downloaded)) as f64;
                        let bps = (downloaded as f64) / secs;
                        // ETA = remaining_bytes / bytes_per_second; max(0.0) prevents negative ETA.
                        let eta = if bps > 0.0 {
                            (left / bps).max(0.0)
                        } else {
                            0.0
                        };
                        let _ = tx_clone.try_send(DownloadEvent::Progress {
                            bytes: downloaded,
                            total: Some(total),
                            mbps,
                            eta_s: eta,
                        });
                    } else {
                        // Total unknown or downloaded exceeds expected (shouldn't happen).
                        let _ = tx_clone.try_send(DownloadEvent::Progress {
                            bytes: downloaded,
                            total: None,
                            mbps: 0.0,
                            eta_s: 0.0,
                        });
                    }
                }
            }
            // Close file handle before rename (ensures all buffers flushed to disk).
            drop(file);
            // Atomic rename: partial → final (prevents partial files from being used).
            fs::rename(&tmp, &weights).map_err(KeylessError::from)?;
            // Verify final size if we know total (catches truncation/corruption).
            if total > 0 {
                let final_len = std::fs::metadata(&weights)
                    .map_err(KeylessError::from)?
                    .len();
                if final_len != total {
                    // Clean up corrupted file (better to have no file than wrong size).
                    // Controller will surface error to user and allow retry.
                    let _ = std::fs::remove_file(&weights);
                    return Err(KeylessError::Other(format!(
                        "downloaded size mismatch ({} != {}), please retry",
                        final_len, total
                    )));
                }
                // Emit a final 100% progress to fully fill the gauge (UI completeness).
                let _ = tx_clone.try_send(DownloadEvent::Progress {
                    bytes: total,
                    total: Some(total),
                    mbps: 0.0,
                    eta_s: 0.0,
                });
            }
        }
        Ok(())
    })();

    // Log result for debugging (goes to session.log; not shown in TUI).
    match &result {
        Ok(()) => info!(model = %model_id, "model download completed successfully"),
        Err(e) => error!(model = %model_id, error = %e, "model download failed"),
    }

    // Use blocking send for completion event (must be delivered; wait if channel full).
    // try_send would drop the event if channel saturated, hiding errors from user.
    let _ = tx.send(DownloadEvent::Done(result));
}
