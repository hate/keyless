//! Config persistence (load/save) used by both the CLI and Tauri.
//!
//! Location:
//! - macOS: ~/Library/Application Support/keyless/config.json
//! - Linux: ~/.config/keyless/config.json
//! - Windows: %APPDATA%/keyless/config.json

use std::fs;
use std::io;
use std::path::PathBuf;

use dirs_next::config_dir;

use super::Config;

/// Current on-disk config schema version.
pub const CURRENT_VERSION: u32 = 1;

/// Migrate config to the current version.
fn migrate_config(mut config: Config) -> Config {
    // Placeholder for future migrations. For now, just ensure version is current.
    // Update version field if config is from an older schema (enables future migration logic).
    if config.version < CURRENT_VERSION {
        config.version = CURRENT_VERSION;
    }
    config
}

/// Compute the cross-platform path for the config file.
pub fn config_path() -> PathBuf {
    // Get platform-specific config directory; fallback to current directory if unavailable.
    // Fallback to "." is defensive (rare edge case where config_dir() fails).
    let base = config_dir().unwrap_or_else(|| PathBuf::from("."));
    // Join app name and filename to form full path (e.g., ~/.config/keyless/config.json).
    let p = base.join("keyless").join("config.json");
    tracing::trace!(path=%p.display(), "config: computed config path");
    p
}

/// Load config from disk (returns None if missing or invalid).
pub fn load_config() -> Option<Config> {
    let path = config_path();
    // Read entire file into memory; small file (<10KB) so one-shot read is fine.
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            // File missing is expected on first run; log as debug, not error.
            tracing::debug!(path=%path.display(), error=%e, "config: no config file yet");
            return None;
        }
    };
    // Parse JSON; handle parse errors gracefully (malformed config = None, not panic).
    match serde_json::from_slice::<Config>(&data) {
        Ok(config) => {
            // Apply migrations if needed (updates version field for older configs).
            let config = migrate_config(config);
            tracing::debug!(path=%path.display(), "config: loaded successfully");
            Some(config)
        }
        Err(e) => {
            // Log parse error as warning (user may have corrupted config manually).
            tracing::warn!(path=%path.display(), error=%e, "config: failed to parse");
            None
        }
    }
}

/// Save config to disk (atomic-ish: create parent, write file).
pub fn save_config(config: &Config) -> io::Result<()> {
    let path = config_path();
    // Create parent directory if missing (e.g., ~/.config/keyless/).
    // Propagate errors to surface permission issues early (better than silent failure).
    if let Some(dir) = path.parent() {
        // Propagate directory creation errors to surface permission issues early
        fs::create_dir_all(dir)?;
    }
    // Serialize to pretty-printed JSON (indented, human-readable).
    // Convert serde error to io::Error for consistent error handling.
    let data = serde_json::to_vec_pretty(config)
        .map_err(|e| io::Error::other(format!("serialize config: {}", e)))?;
    // Atomic-ish write: write to temp file then rename (prevents partial writes).
    // Temp file: config.json.tmp (same directory, atomic rename on most platforms).
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &data)?;
    match fs::rename(&tmp, &path) {
        Ok(()) => {}
        Err(_) => {
            // Retry once after a short delay (helps on platforms with transient locks).
            // Some filesystems/antivirus temporarily lock files during writes.
            std::thread::sleep(std::time::Duration::from_millis(10));
            if fs::rename(&tmp, &path).is_err() {
                // Fallback: copy then remove temp file (non-atomic but ensures write succeeds).
                // Prefer copy over leaving temp file (avoids stale temp files on disk).
                fs::copy(&tmp, &path)?;
                let _ = fs::remove_file(&tmp);
            }
        }
    }
    tracing::debug!(path=%path.display(), bytes=data.len(), "config: saved successfully");
    Ok(())
}
