//! Hugging Face cache path helpers.

use keyless_core::error::KeylessResult;

/// Root directory for keyless model cache.
///
/// Downloads are written here in `<org>--<repo>/` subdirectories.
/// Env override: `KEYLESS_CACHE_DIR` (primarily for tests).
pub fn keyless_cache_root() -> std::path::PathBuf {
    // Use platform-specific cache directory (e.g., ~/.cache/keyless/models on Linux).
    // Fallback to .cache in current directory if cache_dir() unavailable (rare edge case).
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
    // splitn(2, '/') limits to 2 parts (prevents splitting on multiple slashes).
    let mut parts = model_id.splitn(2, '/');
    // Fallback to "openai" if no org (malformed model_id like "whisper-tiny").
    let org = parts.next().unwrap_or("openai");
    // Extract repo name from DEFAULT_MODEL_ID for fallback (if model_id has no slash).
    // nth(1) gets second part after split; fallback to "whisper-tiny" if DEFAULT_MODEL_ID malformed.
    let default_repo = keyless_core::config::DEFAULT_MODEL_ID
        .split('/')
        .nth(1)
        .unwrap_or("whisper-tiny");
    // Use default_repo if model_id has no repo part (e.g., just "openai").
    let repo = parts.next().unwrap_or(default_repo);
    // Format as "org--repo" (double dash replaces slash for filesystem compatibility).
    keyless_cache_root().join(format!("{}--{}", org, repo))
}

/// Get the size of a locally cached model's weights file (if present).
pub fn get_local_model_size(model_id: &str) -> Option<u64> {
    let cache_root = keyless_cache_root();
    // splitn(2, '/') limits to 2 parts (prevents splitting on multiple slashes).
    let mut parts = model_id.splitn(2, '/');
    // Empty fallbacks for validation (unlike keyless_cache_repo_dir which uses defaults).
    let org = parts.next().unwrap_or("");
    let repo = parts.next().unwrap_or("");
    // Return None for malformed model_id (empty org or repo).
    if org.is_empty() || repo.is_empty() {
        return None;
    }
    // Construct path: cache_root/org--repo/model.safetensors.
    let weights = cache_root
        .join(format!("{}--{}", org, repo))
        .join("model.safetensors");
    // ok() converts Result to Option; map extracts file size if metadata exists.
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
    // Check existence before deletion (avoids error if file already gone).
    if partial.exists() {
        // ? operator auto-converts io::Error to KeylessError::Io via From impl.
        std::fs::remove_file(&partial)?;
        Ok(true)
    } else {
        // File doesn't exist; return false (not an error).
        Ok(false)
    }
}
