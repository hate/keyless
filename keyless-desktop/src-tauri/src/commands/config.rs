//! Config-related IPC commands.
//!
//! Provides Tauri command handlers for configuration management: loading, updating, and
//! bootstrapping settings data for the frontend. All commands delegate to `ConfigService`
//! for business logic.

use super::DesktopState;
use crate::services::models::ModelInfoWithSize;
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// Get the current persisted configuration as JSON.
///
/// Returns the full configuration object serialized to JSON with camelCase keys for
/// frontend consumption. If no config exists, returns defaults.
///
/// # Errors
///
/// Returns an error string if JSON serialization fails.
#[tauri::command]
pub fn get_config(ctx: DesktopState<'_>) -> Result<serde_json::Value, String> {
    // Load the current configuration (or defaults if no config file exists).
    let cfg = ctx.config.load();
    // Serialize to JSON with camelCase keys for frontend consumption.
    serde_json::to_value(&cfg).map_err(|e| format!("serialize: {e}"))
}

/// Apply a partial configuration update (patch) and persist.
///
/// Takes a JSON patch object with camelCase keys and applies it to the current config.
/// Automatically triggers side effects based on what changed:
/// - Hotkey changes: Updates PTT preset
/// - Device changes: Requests pipeline restart
/// - Sink changes: Updates tray menu and emits UI event
/// - Model changes: Requests pipeline restart and emits UI event
///
/// # Errors
///
/// Returns an error string if the patch is invalid or save fails.
#[tauri::command]
pub fn update_config(ctx: DesktopState<'_>, patch: serde_json::Value) -> Result<(), String> {
    // Load current config and apply the patch.
    let mut cfg = ctx.config.load();
    // Update returns an outcome indicating which fields changed (for triggering side effects).
    let outcome = ctx.config.update(&mut cfg, &patch)?;
    // Persist the updated config to disk.
    ctx.config.save(&cfg)?;

    // Trigger side effects based on what changed:

    // Hotkey changed: update the PTT (push-to-talk) listener with the new hotkey.
    if outcome.hotkey_changed {
        eprintln!("[config] hotkey changed to {}", cfg.hotkey);
        // Update the PTT preset so the hotkey listener uses the new hotkey immediately.
        ctx.ptt.update_preset_from_config();
        // Notify the frontend that the hotkey changed.
        ctx.events
            .emit("log_message", format!("hotkey changed to {}", cfg.hotkey));
    }

    // Device changed: restart the audio pipeline to use the new input device.
    if outcome.device_changed {
        // Request a pipeline restart (happens asynchronously on the runtime thread).
        ctx.pipeline.request_restart();
    }

    // Sink changed: update tray menu and notify frontend.
    if outcome.sink_changed {
        // Update the tray icon menu to show the correct sink checkmark.
        crate::tray::update_sink_checkmarks_from_config();
        // Determine the sink ID and create a user-friendly message.
        let (sink_id, message) = match &cfg.output_mode {
            keyless_core::config::OutputMode::Paste => {
                ("paste".to_string(), String::from("sink changed to paste"))
            }
            keyless_core::config::OutputMode::Clipboard => (
                "clipboard".to_string(),
                String::from("sink changed to clipboard"),
            ),
            keyless_core::config::OutputMode::File(path) => (
                "file".to_string(),
                format!("sink changed to file ({})", path.display()),
            ),
        };
        eprintln!("[config] {message}");
        // Emit log message for UI display.
        ctx.events.emit("log_message", &message);
        // Emit output mode change event so frontend can update UI state.
        ctx.events.emit("output_mode_changed", &sink_id);
    }

    // Model changed: restart pipeline and notify frontend.
    if outcome.model_changed {
        let model_id = cfg.model_path.to_string_lossy().to_string();
        eprintln!("[config] model changed to {}", model_id);
        // Restart pipeline to load the new model.
        ctx.pipeline.request_restart();
        // Notify frontend that model selection changed.
        ctx.events.emit("model_selected", &model_id);
        // Update tray menu to show the correct model checkmark.
        crate::tray::update_model_checkmarks_from_config();
    }

    // Log the patch that was applied (for debugging).
    if !patch.is_null() && !patch.as_object().map(|o| o.is_empty()).unwrap_or(true) {
        eprintln!("[config] update patch applied: {patch}");
    }

    Ok(())
}

/// Get the normalized hotkey label (lowercased) and persist normalization.
///
/// Returns the current hotkey string, normalized to lowercase. Also persists the normalized
/// form to ensure consistency.
///
/// # Errors
///
/// Returns an error string if config save fails.
#[tauri::command]
pub fn get_hotkey(ctx: DesktopState<'_>) -> Result<String, String> {
    // Load current config and normalize the hotkey (trim whitespace, lowercase).
    let mut cfg = ctx.config.load();
    let label = cfg.hotkey.trim().to_lowercase();
    // Persist the normalized form to ensure consistency (prevents case mismatches).
    cfg.hotkey = label.clone();
    ctx.config.save(&cfg)?;
    Ok(label)
}

/// Set the global hotkey and update the PTT preset immediately.
///
/// Updates the hotkey configuration and immediately applies it to the PTT listener.
/// This is a convenience wrapper around `update_config` that ensures the PTT preset
/// is updated synchronously.
///
/// # Errors
///
/// Returns an error string if config save fails.
#[tauri::command]
pub fn set_hotkey(ctx: DesktopState<'_>, label: String) -> Result<(), String> {
    // Load config and update the hotkey.
    let mut cfg = ctx.config.load();
    cfg.hotkey = label.clone();
    // Persist the change.
    ctx.config.save(&cfg)?;
    // Immediately update the PTT listener with the new hotkey (don't wait for next restart).
    ctx.ptt.update_preset_from_config();
    Ok(())
}

/// Convenience command to set the model ID.
///
/// Delegates to `update_config` with a `modelPath` patch. Automatically triggers pipeline
/// restart if the model changed.
///
/// # Errors
///
/// Returns an error string if the update fails.
#[tauri::command]
pub fn set_model(ctx: DesktopState<'_>, model_id: String) -> Result<(), String> {
    // Convenience wrapper around update_config for setting the model.
    // This automatically triggers pipeline restart if the model changed.
    update_config(ctx, json!({ "modelPath": model_id }))
}

/// Convenience command to set the language.
///
/// Delegates to `update_config` with a `language` patch. Pass `None` for auto-detection.
///
/// # Errors
///
/// Returns an error string if the update fails.
#[tauri::command]
pub fn set_language(ctx: DesktopState<'_>, language: Option<String>) -> Result<(), String> {
    // Convenience wrapper around update_config for setting the language.
    // Pass None to enable auto-detection, or Some("en") for a specific language.
    update_config(
        ctx,
        json!({
            "language": language
        }),
    )
}

/// Reset the configuration to defaults and restart impacted subsystems.
///
/// Restores all configuration values to their defaults, saves the config, and triggers
/// side effects: PTT preset update, pipeline restart, tray menu update, and UI event emission.
///
/// # Errors
///
/// Returns an error string if config save fails.
#[tauri::command]
pub fn reset_config(ctx: DesktopState<'_>) -> Result<(), String> {
    // Create a default configuration (all values reset to defaults).
    let default_cfg = keyless_core::config::Config::default();
    // Persist the default config to disk.
    ctx.config.save(&default_cfg)?;

    // Update runtime subsystems impacted by config changes:
    // - Update PTT preset with default hotkey.
    ctx.ptt.update_preset_from_config();
    // - Restart pipeline to apply default device/model settings.
    ctx.pipeline.request_restart();

    // Update tray menu to show the correct sink checkmark (default is paste).
    crate::tray::update_sink_checkmarks_from_config();

    // Determine the default sink ID and emit output mode change event to frontend.
    let (sink_id, _message) = match &default_cfg.output_mode {
        keyless_core::config::OutputMode::Paste => {
            ("paste".to_string(), String::from("sink changed to paste"))
        }
        keyless_core::config::OutputMode::Clipboard => (
            "clipboard".to_string(),
            String::from("sink changed to clipboard"),
        ),
        keyless_core::config::OutputMode::File(_path) => {
            ("file".to_string(), String::from("sink changed to file"))
        }
    };
    ctx.events.emit("output_mode_changed", &sink_id);
    Ok(())
}

/// Bootstrap payload for Settings view initialization.
///
/// Contains all data needed to populate the Settings UI: current config, hotkey, autostart
/// status, available devices, installed models, and the full model catalog with sizes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBootstrap {
    /// Current configuration as JSON.
    pub config: serde_json::Value,
    /// Normalized hotkey label (lowercased).
    pub hotkey: String,
    /// Whether autostart is enabled.
    pub autostart: bool,
    /// Available audio input device names.
    pub input_devices: Vec<String>,
    /// Installed model IDs (sorted).
    pub installed_models: Vec<String>,
    /// All known models with size metadata (sorted by ID).
    pub models: Vec<ModelInfoWithSize>,
}

/// Gather bootstrap data for Settings view initialization.
///
/// Collects all data needed to populate the Settings UI in a single call: configuration,
/// hotkey, autostart status, input devices, installed models, and the full model catalog
/// with download sizes. This reduces frontend initialization overhead.
///
/// # Errors
///
/// Returns an error string if any data gathering operation fails (config load, autostart
/// check, device listing, model listing, or size cache loading).
#[tauri::command]
pub fn load_settings_bootstrap(
    app: AppHandle,
    ctx: DesktopState<'_>,
) -> Result<SettingsBootstrap, String> {
    // Load current configuration and serialize to JSON for frontend consumption.
    let cfg = ctx.config.load();
    let config_json = serde_json::to_value(&cfg).map_err(|e| format!("serialize config: {e}"))?;

    // Normalize hotkey to lowercase for consistent display.
    let hotkey = cfg.hotkey.trim().to_lowercase();

    // Check if autostart is enabled (app launches on system startup).
    let autostart_enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|e| format!("autostart status: {e}"))?;

    // List all available audio input devices (microphones).
    let devices = keyless_audio::input::list_input_device_names()
        .map_err(|e| format!("list input devices: {e}"))?;

    // Get list of installed models and sort for consistent display.
    let mut installed = ctx
        .models
        .list_installed_models()
        .map_err(|e| format!("list installed models: {e}"))?;
    installed.sort();
    // Create a HashSet for fast lookup when checking if a model is installed.
    let installed_set: HashSet<String> = installed.iter().cloned().collect();

    // Load the model size cache (contains download sizes for known models).
    // This cache is pre-populated and allows showing sizes even when offline.
    let (sizes_map, _) = keyless_models::meta::load_sizes_cache();

    // Pre-allocate vector with capacity for all models (sizes_map + installed models).
    let mut models: Vec<ModelInfoWithSize> =
        Vec::with_capacity(sizes_map.len() + installed_set.len());

    // Add all models from the size cache (these are models we know about, with sizes).
    for (id, size) in sizes_map.iter() {
        models.push(ModelInfoWithSize {
            id: id.clone(),
            downloaded: installed_set.contains(id), // Check if this model is installed.
            downloading: false,                     // Bootstrap doesn't track download state.
            size_bytes: Some(*size),                // Use cached size.
        });
    }

    // Add any installed models that aren't in the size cache (edge case: locally installed
    // models that aren't in the remote catalog, or models added before size cache was populated).
    for id in installed.iter() {
        if !sizes_map.contains_key(id) {
            models.push(ModelInfoWithSize {
                id: id.clone(),
                downloaded: true, // We know it's installed.
                downloading: false,
                size_bytes: None, // Size unknown (not in cache).
            });
        }
    }

    // Sort models by ID for consistent display in the UI.
    models.sort_by(|a, b| a.id.cmp(&b.id));

    // Return all bootstrap data in a single response to minimize frontend initialization overhead.
    Ok(SettingsBootstrap {
        config: config_json,
        hotkey,
        autostart: autostart_enabled,
        input_devices: devices,
        installed_models: installed,
        models,
    })
}
