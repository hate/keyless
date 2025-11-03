//! Size emission workers for TUI integration.

use std::sync::mpsc::SyncSender;

/// Emit sizes for given model ids by inspecting the local cache.
pub fn emit_cached_sizes(models: Vec<String>, tx: SyncSender<(String, Option<u64>)>) {
    for id in models {
        let size = crate::hf::get_local_model_size(&id);
        let _ = tx.try_send((id, size));
    }
}

/// Fetch sizes immediately via HEAD, persist to cache, and emit results.
pub fn fetch_sizes_now(models: Vec<String>, tx: SyncSender<(String, Option<u64>)>) {
    let auth = crate::net::auth_header();
    let (attempts, initial, max_ms) = crate::meta::backoff_config();
    let client = match crate::net::build_blocking_client() {
        Ok(c) => c,
        Err(_) => {
            for id in models {
                let _ = tx.try_send((id, None));
            }
            return;
        }
    };
    let (mut sizes_map, _) = crate::meta::load_sizes_cache();
    for id in models {
        let url = crate::net::hf_resolve_url(&id, "model.safetensors");
        let ok = crate::net::head_len_with_backoff_with_client(
            &client, &url, &auth, attempts, initial, max_ms,
        );
        if let Some(n) = ok {
            sizes_map.insert(id.clone(), n);
        }
        let _ = tx.try_send((id.clone(), ok));
    }
    crate::meta::save_sizes_cache(&sizes_map);
}
