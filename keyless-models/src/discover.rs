//! Discover locally cached Whisper repositories in the keyless cache.

use std::fs;

/// Return model IDs present in the local cache (installed models).
///
/// Scans the keyless cache root for `<org>--<repo>` directories that contain
/// a final `model.safetensors` (ignores `.partial`).
pub fn discover_local_whisper_repos() -> Vec<String> {
    let root = crate::hf::keyless_cache_root();
    let mut out = Vec::new();
    // Return empty list if cache dir doesn't exist or can't be read (graceful degradation).
    let Ok(entries) = fs::read_dir(&root) else {
        return out;
    };
    // flatten() skips I/O errors on individual entries (best-effort scanning).
    for e in entries.flatten() {
        let name = e.file_name();
        // Convert OsStr to String (lossy; invalid UTF-8 replaced with �).
        let name = name.to_string_lossy();
        // Expect <org>--<repo> format (HF model IDs use `/`, but filesystem uses `--` to avoid conflicts).
        if !name.contains("--") {
            continue;
        }
        // splitn(2, "--") limits to 2 parts (prevents splitting on multiple `--` sequences).
        let mut parts = name.splitn(2, "--");
        // unwrap_or is safe here: we checked contains("--") above, but defensive fallback handles edge cases.
        let org = parts.next().unwrap_or("");
        let repo = parts.next().unwrap_or("");
        // Skip malformed directory names (empty org or repo after split).
        if org.is_empty() || repo.is_empty() {
            continue;
        }
        let repo_path = e.path();
        let weights = repo_path.join("model.safetensors");
        // Mark as installed ONLY when final weights exist (ignore .partial downloads in progress).
        if weights.exists() {
            // Convert back to HF model ID format (`org/repo`).
            out.push(format!("{}/{}", org, repo));
        }
    }
    out
}
