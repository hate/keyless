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
    Ok(Status::Idle)
}

/// Begin listening (push-to-talk press).
#[tauri::command]
pub fn start_listening(app: AppHandle) -> IpcResult<()> {
    crate::runtime::start(&app).map_err(|e| e.to_string())
}

/// Stop listening (push-to-talk release). If `cancel` is true, discard output.
#[tauri::command]
pub fn stop_listening(app: AppHandle, cancel: bool) -> IpcResult<()> {
    crate::runtime::stop(&app, cancel).map_err(|e| e.to_string())
}

/// List available output sinks.
#[tauri::command]
pub fn list_sinks() -> IpcResult<Vec<String>> {
    Ok(vec!["paste".into(), "clipboard".into(), "file".into()])
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
    cfg.hotkey = _label;
    storage::save_config(&cfg).map_err(|e| format!("save config: {}", e))
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
