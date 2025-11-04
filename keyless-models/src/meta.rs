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
    // Chained let-ok pattern: read file AND parse JSON must both succeed.
    // Graceful fallback: return empty map and timestamp 0 if file missing or corrupted.
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(cache) = serde_json::from_slice::<SizesCache>(&bytes)
    {
        return (cache.sizes, cache.updated);
    }
    // Empty cache: no file or parse error (treat as cache miss, not fatal).
    (HashMap::new(), 0)
}

/// Persist the sizes cache to disk (pretty JSON).
pub fn save_sizes_cache(sizes: &HashMap<String, u64>) {
    let path = sizes_cache_path();
    // Ensure parent directory exists (create if missing; ignore errors, best-effort).
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let cache = SizesCache {
        // Use current timestamp for TTL tracking (milliseconds since epoch).
        updated: keyless_core::utils::now_millis(),
        // Clone HashMap (needed for owned value; cache owns the map).
        sizes: sizes.clone(),
    };
    // Serialize to pretty JSON; unwrap_or_default falls back to empty vec on error (best-effort).
    // Ignore write errors (best-effort persistence; won't break downloads if cache fails).
    let _ = std::fs::write(path, serde_json::to_vec_pretty(&cache).unwrap_or_default());
}

/// Update (or insert) the saved size for a model in the cache.
pub fn update_saved_size(model_id: &str, bytes: u64) {
    // Load existing cache (may be empty if file missing).
    let (mut map, _) = load_sizes_cache();
    // Insert or update entry (HashMap::insert overwrites existing key).
    map.insert(model_id.to_string(), bytes);
    // Persist entire cache (reload + modify + save pattern).
    save_sizes_cache(&map);
}

/// Path to the sizes cache file, with `KEYLESS_CACHE_DIR` override.
pub fn sizes_cache_path() -> std::path::PathBuf {
    // Prefer env override (for tests); then platform cache dir; fallback to .cache.
    if let Ok(root) = std::env::var("KEYLESS_CACHE_DIR") {
        return std::path::PathBuf::from(root)
            .join("meta")
            .join("models.json");
    }
    // Use platform-specific cache directory (e.g., ~/.cache/keyless/meta/models.json).
    if let Some(base) = dirs_next::cache_dir() {
        return base.join("keyless").join("meta").join("models.json");
    }
    // Fallback to .cache in current directory (rare edge case).
    std::path::PathBuf::from(".cache")
        .join("keyless")
        .join("meta")
        .join("models.json")
}

/// TTL for sizes cache in milliseconds. Env override: `KEYLESS_SIZES_TTL_MS`.
pub fn sizes_ttl_ms() -> u64 {
    // Parse env var: ok() converts Result to Option, and_then chains parse, unwrap_or provides default.
    // Default: 6 hours (6 * 60 * 60 * 1000 ms = 21,600,000 ms).
    std::env::var("KEYLESS_SIZES_TTL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(6 * 60 * 60 * 1000)
}

/// Backoff configuration (attempts, initial_ms, max_ms) with env overrides:
/// `KEYLESS_HTTP_ATTEMPTS`, `KEYLESS_BACKOFF_INITIAL_MS`, `KEYLESS_BACKOFF_MAX_MS`.
pub fn backoff_config() -> (usize, u64, u64) {
    // Parse each env var with fallback defaults; and_then chains parse (returns None on parse error).
    let attempts = std::env::var("KEYLESS_HTTP_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    // Initial delay: 400ms (exponential backoff starts here).
    let initial = std::env::var("KEYLESS_BACKOFF_INITIAL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400);
    // Max delay: 5 seconds (caps exponential backoff).
    let max_ms = std::env::var("KEYLESS_BACKOFF_MAX_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5_000);
    (attempts, initial, max_ms)
}

/// Compute a lightweight plan for a model: total bytes for weights, if determinable.
/// Prefers cached value; falls back to a HEAD request on `model.safetensors`.
pub fn plan_total_bytes(model_id: &str) -> Option<u64> {
    // Load cache (ignore timestamp; we don't check TTL here, caller may).
    let (map, _ts) = load_sizes_cache();
    // Cache hit: return cached size immediately (fast path, no network).
    if let Some(n) = map.get(model_id) {
        return Some(*n);
    }
    // Cache miss: fall back to HEAD request (determines size without downloading).
    let url = net::hf_resolve_url(model_id, "model.safetensors");
    let auth = net::auth_header();
    let (attempts, initial, max_ms) = backoff_config();
    // Build client; return None on error (can't make request without client).
    let client = match net::build_blocking_client() {
        Ok(c) => c,
        Err(_) => return None,
    };
    // HEAD request with retry/backoff; returns Option<u64> (None if request fails).
    net::head_len_with_backoff_with_client(&client, &url, &auth, attempts, initial, max_ms)
}
