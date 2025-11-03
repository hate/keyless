//! Discover locally cached Whisper repositories in the keyless cache.

use std::fs;

/// Return model IDs present in the local cache (installed models).
///
/// Scans the keyless cache root for `<org>--<repo>` directories that contain
/// a final `model.safetensors` (ignores `.partial`).
pub fn discover_local_whisper_repos() -> Vec<String> {
    let root = crate::hf::keyless_cache_root();
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&root) else {
        return out;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        // Expect <org>--<repo>
        if !name.contains("--") {
            continue;
        }
        let mut parts = name.splitn(2, "--");
        let org = parts.next().unwrap_or("");
        let repo = parts.next().unwrap_or("");
        if org.is_empty() || repo.is_empty() {
            continue;
        }
        let repo_path = e.path();
        let weights = repo_path.join("model.safetensors");
        // Mark as installed ONLY when final weights exist (ignore .partial)
        if weights.exists() {
            out.push(format!("{}/{}", org, repo));
        }
    }
    out
}
