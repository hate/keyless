//! Hugging Face cache path helpers.

use keyless_core::error::KeylessResult;

/// Root directory for keyless model cache.
///
/// Downloads are written here in `<org>--<repo>/` subdirectories.
/// Env override: `KEYLESS_CACHE_DIR` (primarily for tests).
pub fn keyless_cache_root() -> std::path::PathBuf {
    if let Some(base) = dirs_next::cache_dir() {
        return base.join("keyless").join("models");
    }
    std::path::PathBuf::from(".cache")
        .join("keyless")
        .join("models")
}

/// Get the cache directory for a specific model.
///
/// Converts `owner/repo` to `<cache_root>/owner--repo/`.
pub fn keyless_cache_repo_dir(model_id: &str) -> std::path::PathBuf {
    let mut parts = model_id.splitn(2, '/');
    let org = parts.next().unwrap_or("openai");
    // Extract repo name from DEFAULT_MODEL_ID for fallback
    let default_repo = keyless_core::config::DEFAULT_MODEL_ID
        .split('/')
        .nth(1)
        .unwrap_or("whisper-tiny");
    let repo = parts.next().unwrap_or(default_repo);
    keyless_cache_root().join(format!("{}--{}", org, repo))
}

/// Get the size of a locally cached model's weights file (if present).
pub fn get_local_model_size(model_id: &str) -> Option<u64> {
    let cache_root = keyless_cache_root();
    let mut parts = model_id.splitn(2, '/');
    let org = parts.next().unwrap_or("");
    let repo = parts.next().unwrap_or("");
    if org.is_empty() || repo.is_empty() {
        return None;
    }
    let weights = cache_root
        .join(format!("{}--{}", org, repo))
        .join("model.safetensors");
    std::fs::metadata(weights).ok().map(|m| m.len())
}

/// Delete the partial download file for a model (if it exists).
///
/// This is used when a user cancels a download and wants to free up disk space.
/// Returns `Ok(true)` if the file existed and was deleted, `Ok(false)` if it
/// didn't exist.
///
/// # Errors
/// Returns `KeylessError::Io` if deletion fails.
pub fn delete_partial_file(model_id: &str) -> KeylessResult<bool> {
    let repo_dir = keyless_cache_repo_dir(model_id);
    let partial = repo_dir.join("model.safetensors.partial");
    if partial.exists() {
        std::fs::remove_file(&partial)?; // auto-converts to KeylessError::Io
        Ok(true)
    } else {
        Ok(false)
    }
}
