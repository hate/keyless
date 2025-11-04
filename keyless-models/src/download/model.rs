//! Model download orchestration.

use crate::{hf, meta};
use keyless_core::error::{KeylessError, KeylessResult};
use std::fs;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, atomic::AtomicBool};
use tracing::{error, info};

use super::events::DownloadEvent;
use super::{large_file, small_files};

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

    let tx_clone = tx.clone();
    let result: KeylessResult<()> = (|| {
        // Blocking reqwest client (with timeouts); this function is not async.
        let client =
            crate::net::build_blocking_client().map_err(|e| KeylessError::Other(e.to_string()))?;
        // Optional auth header (HF token from env); works without token for public models.
        let auth = crate::net::auth_header();

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
        small_files::download_small_files(
            &model_id, &repo_dir, &client, &auth, &tx_clone, &cancel,
        )?;

        // Large file streaming with progress: stream chunks to avoid memory issues.
        large_file::download_large_file(&model_id, &repo_dir, &client, &auth, &tx_clone, &cancel)?;

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
