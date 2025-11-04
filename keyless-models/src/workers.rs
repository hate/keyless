//! Size emission workers for TUI integration.

use std::sync::mpsc::SyncSender;

/// Emit sizes for given model ids by inspecting the local cache.
pub fn emit_cached_sizes(models: Vec<String>, tx: SyncSender<(String, Option<u64>)>) {
    // Iterate over models, check local cache for each (fast path, no network).
    for id in models {
        let size = crate::hf::get_local_model_size(&id);
        // Try-send is non-blocking (drops if channel full; OK for best-effort updates).
        // id moved into tuple (owned value needed for send).
        let _ = tx.try_send((id, size));
    }
}

/// Fetch sizes immediately via HEAD, persist to cache, and emit results.
pub fn fetch_sizes_now(models: Vec<String>, tx: SyncSender<(String, Option<u64>)>) {
    let auth = crate::net::auth_header();
    let (attempts, initial, max_ms) = crate::meta::backoff_config();
    // Build client; if failed, emit None for all models and return (can't fetch without client).
    let client = match crate::net::build_blocking_client() {
        Ok(c) => c,
        Err(_) => {
            // Emit None for all models (signal that sizes unavailable due to client error).
            for id in models {
                let _ = tx.try_send((id, None));
            }
            return;
        }
    };
    // Load existing cache before loop (batch updates, single save at end).
    let (mut sizes_map, _) = crate::meta::load_sizes_cache();
    for id in models {
        let url = crate::net::hf_resolve_url(&id, "model.safetensors");
        // HEAD request with retry/backoff (determines size without downloading).
        let ok = crate::net::head_len_with_backoff_with_client(
            &client, &url, &auth, attempts, initial, max_ms,
        );
        // Update cache if size found (clone id for HashMap key).
        if let Some(n) = ok {
            sizes_map.insert(id.clone(), n);
        }
        // Emit result (clone id for send; ok may be None if request failed).
        let _ = tx.try_send((id.clone(), ok));
    }
    // Save entire cache after loop (single write operation, more efficient than per-model saves).
    crate::meta::save_sizes_cache(&sizes_map);
}
