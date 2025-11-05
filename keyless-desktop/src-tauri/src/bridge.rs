//! ipc command surface for the desktop app.
//!
//! These commands are scaffolding; they will be wired to the existing
//! runtime and config crates next. For now they return minimal placeholders
//! so the UI can be integrated without blocking.
// Intentionally do not depend on KeylessError for IPC return types.
use keyless_core::config::{self, Config, OutputMode, storage};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::{AppHandle, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

/// High-level status exposed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum Status {
    /// Application is idle.
    #[serde(rename = "idle")]
    Idle,
    /// Listening; `speaking` indicates VAD speaking state.
    #[serde(rename = "listening")]
    Listening {
        /// Whether voice activity detection currently considers the user as speaking.
        speaking: bool,
    },
}

/// Return current status.
type IpcResult<T> = Result<T, String>;

/// Return the current high-level status.
#[tauri::command]
pub fn get_status() -> IpcResult<Status> {
    // PTT controls listening state; status is updated via events, not polling
    Ok(Status::Idle)
}

/// List available output sinks.
#[tauri::command]
pub fn list_sinks() -> IpcResult<Vec<String>> {
    Ok(vec!["paste".into(), "clipboard".into(), "file".into()])
}

/// List available input devices.
#[tauri::command]
pub fn list_input_devices() -> IpcResult<Vec<String>> {
    keyless_audio::input::list_input_device_names()
        .map_err(|e| format!("failed to list devices: {}", e))
}

/// Select output sink by id.
#[tauri::command]
pub fn select_sink(id: String) -> IpcResult<()> {
    let mut cfg: Config = storage::load_config().unwrap_or_default();
    match id.as_str() {
        "paste" => cfg.output_mode = OutputMode::Paste,
        "clipboard" => cfg.output_mode = OutputMode::Clipboard,
        "file" => {
            if let OutputMode::File(_) = cfg.output_mode {
                // keep existing file path
            } else {
                return Err("file sink requires a path (set in settings)".into());
            }
        }
        _ => return Err(format!("unknown sink: {}", id)),
    }
    storage::save_config(&cfg).map_err(|e| format!("save config: {}", e))
}

/// Get current configuration as JSON.
#[tauri::command]
pub fn get_config() -> IpcResult<serde_json::Value> {
    let cfg: Config = storage::load_config().unwrap_or_default();
    serde_json::to_value(&cfg).map_err(|e| format!("serialize: {}", e))
}

/// Apply a partial configuration update.
#[tauri::command]
pub fn update_config(_patch: serde_json::Value) -> IpcResult<()> {
    let mut cfg: Config = storage::load_config().unwrap_or_default();
    // minimal patcher: handle outputMode and hotkey, and file path like { outputMode:"file", filePath:"/path" }
    if let Some(om) = _patch.get("outputMode").and_then(|v| v.as_str()) {
        match config::OutputMode::from_str(om) {
            Ok(OutputMode::Paste) => cfg.output_mode = OutputMode::Paste,
            Ok(OutputMode::Clipboard) => cfg.output_mode = OutputMode::Clipboard,
            Ok(OutputMode::File(_)) => {
                if let Some(p) = _patch.get("filePath").and_then(|v| v.as_str()) {
                    cfg.output_mode = OutputMode::File(std::path::PathBuf::from(p));
                } else if !matches!(cfg.output_mode, OutputMode::File(_)) {
                    return Err("file sink requires filePath".into());
                }
            }
            Err(e) => return Err(format!("outputMode: {}", e)),
        }
    }
    if let Some(h) = _patch.get("hotkey").and_then(|v| v.as_str()) {
        cfg.hotkey = h.to_string();
        // Update PTT listener preset_mode when hotkey changes
        let _ = crate::runtime::update_preset_mode(h);
    }
    // Device name (optional)
    if let Some(dev_val) = _patch.get("deviceName") {
        if dev_val.is_null() {
            cfg.device_name = None;
        } else if let Some(s) = dev_val.as_str() {
            cfg.device_name = Some(s.to_string());
        }
    }
    // Optional language (None means auto)
    if let Some(lang_val) = _patch.get("language") {
        if lang_val.is_null() {
            cfg.language = None;
        } else if let Some(s) = lang_val.as_str() {
            cfg.language = Some(s.to_string());
        }
    }
    // Optional model (by id/path string)
    if let Some(model_id) = _patch.get("modelPath").and_then(|v| v.as_str()) {
        cfg.model_path = std::path::PathBuf::from(model_id);
    }
    // VAD thresholds
    if let Some(vad) = _patch.get("vad").and_then(|v| v.as_object()) {
        if let Some(v) = vad.get("startDb").and_then(|v| v.as_f64()) {
            cfg.vad.start_db = v as f32;
        }
        if let Some(v) = vad.get("stopDb").and_then(|v| v.as_f64()) {
            cfg.vad.stop_db = v as f32;
        }
        if let Some(v) = vad.get("minDurationMs").and_then(|v| v.as_u64()) {
            cfg.vad.min_duration_ms = (v as u16).min(u16::MAX);
        }
        if let Some(v) = vad.get("maxSilenceMs").and_then(|v| v.as_u64()) {
            cfg.vad.max_silence_ms = (v as u16).min(u16::MAX);
        }
    }
    // EQ tuning
    if let Some(eq) = _patch.get("eq").and_then(|v| v.as_object()) {
        if let Some(v) = eq.get("bands").and_then(|v| v.as_u64()) {
            cfg.eq.bands = (v as u16).max(16).min(128);
        }
        if let Some(v) = eq.get("noiseReduction").and_then(|v| v.as_f64()) {
            cfg.eq.noise_reduction = v as f32;
        }
        if let Some(v) = eq.get("windowDb").and_then(|v| v.as_f64()) {
            cfg.eq.window_db = v as f32;
        }
        if let Some(v) = eq.get("gamma").and_then(|v| v.as_f64()) {
            cfg.eq.gamma = v as f32;
        }
        if let Some(v) = eq.get("attack").and_then(|v| v.as_f64()) {
            cfg.eq.attack = v as f32;
        }
        if let Some(v) = eq.get("decay").and_then(|v| v.as_f64()) {
            cfg.eq.decay = v as f32;
        }
    }
    storage::save_config(&cfg).map_err(|e| format!("save config: {}", e))
}

/// Get the current push-to-talk hotkey label.
#[tauri::command]
pub fn get_hotkey() -> IpcResult<String> {
    let cfg: Config = storage::load_config().unwrap_or_default();
    Ok(cfg.hotkey)
}

/// Set a new push-to-talk hotkey label.
#[tauri::command]
pub fn set_hotkey(_label: String) -> IpcResult<()> {
    let mut cfg: Config = storage::load_config().unwrap_or_default();
    cfg.hotkey = _label.clone();
    storage::save_config(&cfg).map_err(|e| format!("save config: {}", e))?;

    // Update PTT listener preset_mode when hotkey changes
    crate::runtime::update_preset_mode(&_label).map_err(|e| e.to_string())?;

    Ok(())
}

/// Show the pill HUD at the bottom center.
#[tauri::command]
pub fn show_pill(app: AppHandle) -> IpcResult<()> {
    if let Some(win) = app.get_webview_window("pill") {
        let _ = win.move_window(Position::BottomCenter);
        let _ = win.show();
        let _ = win.set_focus();
        Ok(())
    } else {
        Err("missing pill window".into())
    }
}

/// Hide the pill HUD.
#[tauri::command]
pub fn hide_pill(app: AppHandle) -> IpcResult<()> {
    if let Some(win) = app.get_webview_window("pill") {
        let _ = win.hide();
        Ok(())
    } else {
        Err("missing pill window".into())
    }
}
