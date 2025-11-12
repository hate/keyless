//! Configuration service for loading, saving, and patching application config.
//!
//! Provides a trait-based abstraction over `keyless_core::config::storage` functions, allowing
//! for testability and clean separation of concerns. The implementation delegates to the core
//! storage module for actual persistence.

use keyless_core::config::{Config, OutputMode, storage};
use std::path::PathBuf;

/// Service trait for configuration management.
///
/// All methods operate on the persisted config file. `load()` returns defaults if no config
/// exists. `update()` applies a JSON patch and returns an `UpdateOutcome` indicating which
/// fields changed (for triggering side effects like pipeline restarts).
pub trait ConfigService: Send + Sync {
    /// Load the current persisted configuration, or return defaults if none exists.
    fn load(&self) -> Config;

    /// Save a configuration to persistent storage.
    ///
    /// # Errors
    ///
    /// Returns an error string if serialization or file I/O fails.
    fn save(&self, cfg: &Config) -> Result<(), String>;

    /// Apply a JSON patch to the configuration and return which fields changed.
    ///
    /// The patch is a `serde_json::Value` object containing camelCase keys matching the
    /// frontend's config structure. Supported keys: `outputMode`, `filePath`, `hotkey`,
    /// `deviceName`, `language`, `modelPath`, `vad`, `eq`.
    ///
    /// # Errors
    ///
    /// Returns an error string if the patch is invalid (e.g., unknown sink, filePath without
    /// file sink, invalid value types).
    fn update(&self, cfg: &mut Config, patch: &serde_json::Value) -> Result<UpdateOutcome, String>;
}

/// Outcome of a config patch indicating which fields changed.
///
/// Callers use this to trigger side effects like PTT preset updates, pipeline restarts,
/// or UI event emissions. All fields default to `false`.
#[derive(Debug, Default, Clone)]
pub struct UpdateOutcome {
    /// Whether the hotkey changed (requires PTT preset update).
    pub hotkey_changed: bool,
    /// Whether the input device changed (requires pipeline restart).
    pub device_changed: bool,
    /// Whether the output sink changed (requires tray menu update and UI event).
    pub sink_changed: bool,
    /// Whether the model changed (requires pipeline restart).
    pub model_changed: bool,
}

/// Default implementation of `ConfigService` using `keyless_core::config::storage`.
pub struct ConfigServiceImpl;

impl ConfigServiceImpl {
    /// Create a new `ConfigServiceImpl` instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfigServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigService for ConfigServiceImpl {
    fn load(&self) -> Config {
        // Load config from persistent storage, or return defaults if no config file exists.
        // This is safe to unwrap_or_default because we always want a valid config, even if it's defaults.
        storage::load_config().unwrap_or_default()
    }

    fn save(&self, cfg: &Config) -> Result<(), String> {
        // Save the configuration to persistent storage (typically a JSON file).
        // Convert any storage errors to a user-friendly string.
        storage::save_config(cfg).map_err(|e| format!("save config: {e}"))
    }

    fn update(&self, cfg: &mut Config, patch: &serde_json::Value) -> Result<UpdateOutcome, String> {
        // Track which fields changed so callers can trigger appropriate side effects
        // (e.g., restart pipeline when device changes, update PTT preset when hotkey changes).
        let mut outcome = UpdateOutcome::default();

        // Extract filePath from patch if present (used when switching to file sink).
        // This is extracted separately because it can be provided independently or with outputMode.
        let pending_file_path = patch
            .get("filePath")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        // Handle outputMode changes (paste, clipboard, or file sink).
        if let Some(om) = patch.get("outputMode").and_then(|v| v.as_str()) {
            match om.to_ascii_lowercase().as_str() {
                "paste" => {
                    // Switch to paste sink (simulates keyboard typing).
                    cfg.output_mode = OutputMode::Paste;
                    outcome.sink_changed = true; // Mark sink as changed for side effects.
                }
                "clipboard" => {
                    // Switch to clipboard sink (copies to system clipboard).
                    cfg.output_mode = OutputMode::Clipboard;
                    outcome.sink_changed = true;
                }
                "file" => {
                    // Switch to file sink (writes to a text file).
                    // Determine the target file path: use pending_file_path from patch if provided,
                    // otherwise reuse existing path from config, or error if neither exists.
                    let target_path = pending_file_path
                        .clone()
                        .or_else(|| match &cfg.output_mode {
                            OutputMode::File(p) => Some(p.clone()), // Reuse existing file path.
                            _ => None,
                        })
                        .ok_or_else(|| String::from("file sink requires filePath"))?;
                    // Ensure the parent directory exists before saving the config.
                    // Ignore errors here; file writes will fail later if directory creation fails.
                    if let Some(dir) = target_path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    cfg.output_mode = OutputMode::File(target_path);
                    outcome.sink_changed = true;
                }
                other => return Err(format!("outputMode: unknown sink: {other}")),
            }
        } else if let Some(pb) = pending_file_path {
            // Handle filePath update without outputMode change (only valid if current sink is file).
            if matches!(cfg.output_mode, OutputMode::File(_)) {
                // Current sink is file, so updating the path is valid.
                if let Some(dir) = pb.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                cfg.output_mode = OutputMode::File(pb);
                outcome.sink_changed = true;
            } else {
                // Error: can't set filePath when current sink is not file.
                return Err("filePath provided but current sink is not file".into());
            }
        }

        // Handle hotkey changes.
        if let Some(h) = patch.get("hotkey").and_then(|v| v.as_str()) {
            let normalized = h.to_string();
            // Only mark as changed if the value actually differs (avoid unnecessary side effects).
            if cfg.hotkey != normalized {
                cfg.hotkey = normalized;
                outcome.hotkey_changed = true; // PTT preset needs to be updated.
            }
        }

        // Handle device name changes (can be set to a string or null to clear).
        if let Some(dev_val) = patch.get("deviceName") {
            if dev_val.is_null() {
                // Clearing device selection (use system default).
                if cfg.device_name.is_some() {
                    outcome.device_changed = true; // Only mark changed if there was a device set.
                }
                cfg.device_name = None;
            } else if let Some(s) = dev_val.as_str() {
                // Setting a specific device name.
                if cfg.device_name.as_deref() != Some(s) {
                    outcome.device_changed = true; // Pipeline needs restart for device change.
                }
                cfg.device_name = Some(s.to_string());
            }
        }

        // Handle language changes (can be set to a string or null for auto-detection).
        if let Some(lang_val) = patch.get("language") {
            if lang_val.is_null() {
                // Clear language (enable auto-detection).
                cfg.language = None;
            } else if let Some(s) = lang_val.as_str() {
                // Set specific language code (e.g., "en", "es", "fr").
                cfg.language = Some(s.to_string());
            }
        }

        // Handle model path changes.
        if let Some(model_id) = patch.get("modelPath").and_then(|v| v.as_str()) {
            let new_path = PathBuf::from(model_id);
            // Only mark as changed if the path actually differs.
            if cfg.model_path != new_path {
                outcome.model_changed = true; // Pipeline needs restart for model change.
                cfg.model_path = new_path;
            }
        }

        // Handle VAD (Voice Activity Detection) threshold changes.
        // VAD settings control when speech is detected (start/stop thresholds, min duration, max silence).
        if let Some(vad) = patch.get("vad").and_then(|v| v.as_object()) {
            // Update start threshold (dB level to start detecting speech).
            if let Some(v) = vad.get("startDb").and_then(|v| v.as_f64()) {
                cfg.vad.start_db = v as f32;
            }
            // Update stop threshold (dB level to stop detecting speech).
            if let Some(v) = vad.get("stopDb").and_then(|v| v.as_f64()) {
                cfg.vad.stop_db = v as f32;
            }
            // Update minimum speech duration (ms) - speech must last this long to be considered valid.
            if let Some(v) = vad.get("minDurationMs").and_then(|v| v.as_u64()) {
                cfg.vad.min_duration_ms = v as u16;
            }
            // Update maximum silence duration (ms) - silence longer than this stops transcription.
            if let Some(v) = vad.get("maxSilenceMs").and_then(|v| v.as_u64()) {
                cfg.vad.max_silence_ms = v as u16;
            }
        }

        // Handle EQ (Equalizer) tuning changes.
        // EQ settings control audio processing (frequency bands, noise reduction, etc.).
        if let Some(eq) = patch.get("eq").and_then(|v| v.as_object()) {
            // Update number of frequency bands (clamped to valid range 16-128).
            if let Some(v) = eq.get("bands").and_then(|v| v.as_u64()) {
                cfg.eq.bands = (v as u16).clamp(16, 128);
            }
            // Update noise reduction factor (0.0 = noisy, 1.0 = smooth).
            if let Some(v) = eq.get("noiseReduction").and_then(|v| v.as_f64()) {
                cfg.eq.noise_reduction = v as f32;
            }
            // Update EQ window size in dB (controls dynamic range).
            if let Some(v) = eq.get("windowDb").and_then(|v| v.as_f64()) {
                cfg.eq.window_db = v as f32;
            }
            // Update gamma (controls frequency response curve).
            if let Some(v) = eq.get("gamma").and_then(|v| v.as_f64()) {
                cfg.eq.gamma = v as f32;
            }
            // Update attack time (how quickly EQ responds to changes).
            if let Some(v) = eq.get("attack").and_then(|v| v.as_f64()) {
                cfg.eq.attack = v as f32;
            }
            // Update decay time (how quickly EQ returns to baseline).
            if let Some(v) = eq.get("decay").and_then(|v| v.as_f64()) {
                cfg.eq.decay = v as f32;
            }
        }

        // Return the outcome indicating which fields changed (for triggering side effects).
        Ok(outcome)
    }
}
