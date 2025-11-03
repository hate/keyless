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
    if config.version < CURRENT_VERSION {
        config.version = CURRENT_VERSION;
    }
    config
}

/// Compute the cross-platform path for the config file.
pub fn config_path() -> PathBuf {
    let base = config_dir().unwrap_or_else(|| PathBuf::from("."));
    let p = base.join("keyless").join("config.json");
    tracing::trace!(path=%p.display(), "config: computed config path");
    p
}

/// Load config from disk (returns None if missing or invalid).
pub fn load_config() -> Option<Config> {
    let path = config_path();
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!(path=%path.display(), error=%e, "config: no config file yet");
            return None;
        }
    };
    match serde_json::from_slice::<Config>(&data) {
        Ok(config) => {
            let config = migrate_config(config);
            tracing::debug!(path=%path.display(), "config: loaded successfully");
            Some(config)
        }
        Err(e) => {
            tracing::warn!(path=%path.display(), error=%e, "config: failed to parse");
            None
        }
    }
}

/// Save config to disk (atomic-ish: create parent, write file).
pub fn save_config(config: &Config) -> io::Result<()> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        // Propagate directory creation errors to surface permission issues early
        fs::create_dir_all(dir)?;
    }
    let data = serde_json::to_vec_pretty(config)
        .map_err(|e| io::Error::other(format!("serialize config: {}", e)))?;
    // Atomic-ish write: write to a temp file then rename
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &data)?;
    match fs::rename(&tmp, &path) {
        Ok(()) => {}
        Err(_) => {
            // Retry once after a short delay (helps on platforms with transient locks)
            std::thread::sleep(std::time::Duration::from_millis(10));
            if fs::rename(&tmp, &path).is_err() {
                // Fallback: copy then remove the temp file
                fs::copy(&tmp, &path)?;
                let _ = fs::remove_file(&tmp);
            }
        }
    }
    tracing::debug!(path=%path.display(), bytes=data.len(), "config: saved successfully");
    Ok(())
}
