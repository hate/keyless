//! ipc command surface for the desktop app.
//!
//! These commands are scaffolding; they will be wired to the existing
//! runtime and config crates next. For now they return minimal placeholders
//! so the UI can be integrated without blocking.
// Intentionally do not depend on KeylessError for IPC return types.
use serde::{Deserialize, Serialize};

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
pub fn start_listening() -> IpcResult<()> {
    Ok(())
}

/// Stop listening (push-to-talk release). If `cancel` is true, discard output.
#[tauri::command]
pub fn stop_listening(_cancel: bool) -> IpcResult<()> {
    Ok(())
}

/// List available output sinks.
#[tauri::command]
pub fn list_sinks() -> IpcResult<Vec<String>> {
    Ok(Vec::new())
}

/// Select output sink by id.
#[tauri::command]
pub fn select_sink(_id: String) -> IpcResult<()> {
    Ok(())
}

/// Get current configuration as JSON.
#[tauri::command]
pub fn get_config() -> IpcResult<serde_json::Value> {
    Ok(serde_json::json!({}))
}

/// Apply a partial configuration update.
#[tauri::command]
pub fn update_config(_patch: serde_json::Value) -> IpcResult<()> {
    Ok(())
}

/// Get the current push-to-talk hotkey label.
#[tauri::command]
pub fn get_hotkey() -> IpcResult<String> {
    Ok("right command".to_string())
}

/// Set a new push-to-talk hotkey label.
#[tauri::command]
pub fn set_hotkey(_label: String) -> IpcResult<()> {
    Ok(())
}
