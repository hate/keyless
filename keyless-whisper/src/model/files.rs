use std::path::{Path, PathBuf};

use keyless_core::error::{KeylessError, KeylessResult};

/// Detect if a model directory contains quantized (GGUF) weights.
///
/// Returns true if any .gguf file is found in the directory.
pub(crate) fn detect_quantized(dir: &Path) -> bool {
    // Early return if directory doesn't exist (defensive check).
    if !dir.exists() || !dir.is_dir() {
        return false;
    }

    // Scan directory for .gguf extension (quantized format indicator).
    // Only need to find one .gguf file to determine format.
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            // Check extension (case-sensitive; .gguf is lowercase).
            if let Some(ext) = entry.path().extension()
                && ext == "gguf"
            {
                return true;
            }
        }
    }
    false
}

/// Locate normal (safetensors) model files in a directory.
///
/// Returns paths to config.json, model.safetensors, and tokenizer.json.
pub(crate) fn locate_normal_files(dir: &Path) -> KeylessResult<(PathBuf, PathBuf, PathBuf)> {
    // Standard Hugging Face model layout: config.json, model.safetensors, tokenizer.json.
    let config = dir.join("config.json");
    let weights = dir.join("model.safetensors");
    let tokenizer = dir.join("tokenizer.json");

    // Validate all required files exist (fail fast with clear error messages).
    if !config.exists() {
        return Err(KeylessError::Whisper(format!(
            "config.json not found in {:?}",
            dir
        )));
    }
    if !weights.exists() {
        return Err(KeylessError::Whisper(format!(
            "model.safetensors not found in {:?}",
            dir
        )));
    }
    if !tokenizer.exists() {
        return Err(KeylessError::Whisper(format!(
            "tokenizer.json not found in {:?}",
            dir
        )));
    }

    Ok((config, weights, tokenizer))
}

/// Locate quantized (GGUF) model files in a directory.
///
/// Returns paths to config.json, first .gguf file found, and tokenizer.json.
pub(crate) fn locate_quantized_files(dir: &Path) -> KeylessResult<(PathBuf, PathBuf, PathBuf)> {
    let config = dir.join("config.json");
    let tokenizer = dir.join("tokenizer.json");

    if !config.exists() {
        return Err(KeylessError::Whisper(format!(
            "config.json not found in {:?}",
            dir
        )));
    }
    if !tokenizer.exists() {
        return Err(KeylessError::Whisper(format!(
            "tokenizer.json not found in {:?}",
            dir
        )));
    }

    // Find first .gguf file (quantized models may have multiple .gguf files; use first).
    // Some quantized models split weights across multiple files, but we only need one for detection.
    let weights = std::fs::read_dir(dir)
        .map_err(|e| KeylessError::Whisper(format!("failed to read dir {:?}: {}", dir, e)))?
        // Skip I/O errors (flatten() drops failed entries).
        .filter_map(|e| e.ok())
        // Match .gguf extension (case-sensitive comparison).
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("gguf"))
        .map(|e| e.path())
        .ok_or_else(|| KeylessError::Whisper(format!("no .gguf file found in {:?}", dir)))?;

    Ok((config, weights, tokenizer))
}
