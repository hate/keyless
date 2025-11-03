//! Typed sizes cache with TTL and backoff helpers.

use crate::net;
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
/// On-disk structure for the sizes cache.
struct SizesCache {
    /// Last update timestamp in milliseconds.
    updated: u64,
    /// Map of model id → bytes for `model.safetensors`.
    sizes: HashMap<String, u64>,
}

/// Load the sizes cache, returning the map and the last updated timestamp.
pub fn load_sizes_cache() -> (HashMap<String, u64>, u64) {
    let path = sizes_cache_path();
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(cache) = serde_json::from_slice::<SizesCache>(&bytes)
    {
        return (cache.sizes, cache.updated);
    }
    (HashMap::new(), 0)
}

/// Persist the sizes cache to disk (pretty JSON).
pub fn save_sizes_cache(sizes: &HashMap<String, u64>) {
    let path = sizes_cache_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let cache = SizesCache {
        updated: keyless_core::utils::now_millis(),
        sizes: sizes.clone(),
    };
    let _ = std::fs::write(path, serde_json::to_vec_pretty(&cache).unwrap_or_default());
}

/// Update (or insert) the saved size for a model in the cache.
pub fn update_saved_size(model_id: &str, bytes: u64) {
    let (mut map, _) = load_sizes_cache();
    map.insert(model_id.to_string(), bytes);
    save_sizes_cache(&map);
}

/// Path to the sizes cache file, with `KEYLESS_CACHE_DIR` override.
pub fn sizes_cache_path() -> std::path::PathBuf {
    if let Ok(root) = std::env::var("KEYLESS_CACHE_DIR") {
        return std::path::PathBuf::from(root)
            .join("meta")
            .join("models.json");
    }
    if let Some(base) = dirs_next::cache_dir() {
        return base.join("keyless").join("meta").join("models.json");
    }
    std::path::PathBuf::from(".cache")
        .join("keyless")
        .join("meta")
        .join("models.json")
}

/// TTL for sizes cache in milliseconds. Env override: `KEYLESS_SIZES_TTL_MS`.
pub fn sizes_ttl_ms() -> u64 {
    std::env::var("KEYLESS_SIZES_TTL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(6 * 60 * 60 * 1000)
}

/// Backoff configuration (attempts, initial_ms, max_ms) with env overrides:
/// `KEYLESS_HTTP_ATTEMPTS`, `KEYLESS_BACKOFF_INITIAL_MS`, `KEYLESS_BACKOFF_MAX_MS`.
pub fn backoff_config() -> (usize, u64, u64) {
    let attempts = std::env::var("KEYLESS_HTTP_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let initial = std::env::var("KEYLESS_BACKOFF_INITIAL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400);
    let max_ms = std::env::var("KEYLESS_BACKOFF_MAX_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5_000);
    (attempts, initial, max_ms)
}

/// Compute a lightweight plan for a model: total bytes for weights, if determinable.
/// Prefers cached value; falls back to a HEAD request on `model.safetensors`.
pub fn plan_total_bytes(model_id: &str) -> Option<u64> {
    let (map, _ts) = load_sizes_cache();
    if let Some(n) = map.get(model_id) {
        return Some(*n);
    }
    let url = net::hf_resolve_url(model_id, "model.safetensors");
    let auth = net::auth_header();
    let (attempts, initial, max_ms) = backoff_config();
    let client = match net::build_blocking_client() {
        Ok(c) => c,
        Err(_) => return None,
    };
    net::head_len_with_backoff_with_client(&client, &url, &auth, attempts, initial, max_ms)
}
