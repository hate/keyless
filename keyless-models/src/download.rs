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
    let _ = tx.try_send(DownloadEvent::Started {
        model: model_id.clone(),
    });
    info!(model = %model_id, "starting model download");
    let tx_clone = tx.clone();
    let tx_for_loop = tx.clone();
    let result: KeylessResult<()> = (|| {
        // Blocking reqwest client (with timeouts)
        let client =
            net::build_blocking_client().map_err(|e| KeylessError::Other(e.to_string()))?;
        let auth = net::auth_header();

        // Ensure repo cache dir
        let repo_dir = hf::keyless_cache_repo_dir(&model_id);
        fs::create_dir_all(&repo_dir).map_err(KeylessError::from)?;

        // Plan line (if size known)
        if let Some(total) = meta::plan_total_bytes(&model_id) {
            let _ = tx_clone.try_send(DownloadEvent::Stage {
                text: format!("plan: {}", keyless_core::utils::human_size(total)),
            });
        }

        // Small files (with retry/backoff)
        for name in ["config.json", "tokenizer.json"] {
            if cancel.load(Ordering::Relaxed) {
                return Err(KeylessError::Other("cancelled".into()));
            }
            let dst = repo_dir.join(name);
            if dst.exists() {
                continue;
            }
            let url = net::hf_resolve_url(&model_id, name);
            let _ = tx_for_loop.try_send(DownloadEvent::Stage {
                text: format!("downloading {}", name),
            });
            let (attempts, initial, max_ms) = meta::backoff_config();
            let (bytes, content_len_hdr) =
                net::blocking_get_with_backoff(&client, &url, &auth, attempts, initial, max_ms)?;
            let mut f = File::create(&dst).map_err(KeylessError::from)?;
            f.write_all(&bytes).map_err(KeylessError::from)?;
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

        // Large file streaming with progress
        if cancel.load(Ordering::Relaxed) {
            return Err(KeylessError::Other("cancelled".into()));
        }
        let weights = repo_dir.join("model.safetensors");
        if !weights.exists() {
            let url = net::hf_resolve_url(&model_id, "model.safetensors");
            let mut req = client.get(&url);
            if let Some((h, v)) = &auth {
                req = req.header(h, v.clone());
            }
            // resume support
            let tmp = repo_dir.join("model.safetensors.partial");
            let mut downloaded: u64 = 0;
            if tmp.exists()
                && let Ok(m) = std::fs::metadata(&tmp)
            {
                downloaded = m.len();
            }
            if downloaded > 0 {
                req = req.header(reqwest::header::RANGE, format!("bytes={}-", downloaded));
            }
            let mut resp = req
                .send()
                .map_err(|e| KeylessError::Other(format!("GET {}: {}", url, e)))?;
            let status = resp.status();
            let mut file: Box<dyn std::io::Write>;
            if status == reqwest::StatusCode::PARTIAL_CONTENT {
                // Resume OK (206); keep downloaded as-is and append
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
                // Range ignored; restart from 0
                let _ = std::fs::remove_file(&tmp);
                downloaded = 0;
                resp = resp
                    .error_for_status()
                    .map_err(|e| KeylessError::Other(e.to_string()))?;
                file = Box::new(
                    std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&tmp)
                        .map_err(KeylessError::from)?,
                );
            } else {
                return Err(KeylessError::Other(format!("unexpected status {}", status)));
            }

            let total = if status == reqwest::StatusCode::PARTIAL_CONTENT {
                resp.content_length()
                    .unwrap_or(0)
                    .saturating_add(downloaded)
            } else {
                resp.content_length().unwrap_or(0)
            };
            if total > 0 {
                meta::update_saved_size(&model_id, total);
            }
            let start = std::time::Instant::now();
            let mut last_emit = std::time::Instant::now();
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err(KeylessError::Other("cancelled".into()));
                }
                use std::io::Read;
                let n = resp
                    .read(&mut buf)
                    .map_err(|e| KeylessError::Other(format!("read chunk: {}", e)))?;
                if n == 0 {
                    break;
                }
                if cancel.load(Ordering::Relaxed) {
                    return Err(KeylessError::Other("cancelled".into()));
                }
                file.write_all(&buf[..n]).map_err(KeylessError::from)?;
                downloaded += n as u64;
                // Throttle progress events to at most every 250ms
                if last_emit.elapsed().as_millis() >= 250 {
                    last_emit = std::time::Instant::now();
                    if total > 0 && downloaded <= total {
                        let secs = start.elapsed().as_secs_f64().max(0.001);
                        let mbps = (downloaded as f64 / 1_000_000.0) / secs;
                        let left = (total.saturating_sub(downloaded)) as f64;
                        let bps = (downloaded as f64) / secs;
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
                        let _ = tx_clone.try_send(DownloadEvent::Progress {
                            bytes: downloaded,
                            total: None,
                            mbps: 0.0,
                            eta_s: 0.0,
                        });
                    }
                }
            }
            drop(file);
            fs::rename(&tmp, &weights).map_err(KeylessError::from)?;
            // Verify final size if we know total
            if total > 0 {
                let final_len = std::fs::metadata(&weights)
                    .map_err(KeylessError::from)?
                    .len();
                if final_len != total {
                    // clean up and error out; controller will surface error
                    let _ = std::fs::remove_file(&weights);
                    return Err(KeylessError::Other(format!(
                        "downloaded size mismatch ({} != {}), please retry",
                        final_len, total
                    )));
                }
                // Emit a final 100% progress to fully fill the gauge
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

    // Log result for debugging (goes to session.log)
    match &result {
        Ok(()) => info!(model = %model_id, "model download completed successfully"),
        Err(e) => error!(model = %model_id, error = %e, "model download failed"),
    }

    // Ensure the terminal receives completion even when the channel is saturated
    let _ = tx.send(DownloadEvent::Done(result));
}
