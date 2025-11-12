//! Runtime control IPC commands (status, devices, initialize).
use super::DesktopState;
use crate::popover::PopoverBlurGuard;
use tauri::{AppHandle, Manager, State};

/// Simple runtime status for the UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "state")]
pub enum Status {
    #[serde(rename = "idle")]
    /// The runtime is idle (PTT not held).
    Idle,
    #[serde(rename = "listening")]
    /// The runtime is listening (PTT held).
    Listening {
        /// Current VAD state while listening.
        speaking: bool,
    },
}

/// Get current runtime status (idle or listening).
#[tauri::command]
pub fn get_status(ctx: DesktopState<'_>) -> Result<Status, String> {
    // Check if PTT (push-to-talk) hotkey is currently held down.
    if ctx.ptt.is_listening() {
        // PTT is held: runtime is listening (recording audio).
        // Note: speaking state is not yet implemented, always false.
        Ok(Status::Listening { speaking: false })
    } else {
        // PTT is not held: runtime is idle (not recording).
        Ok(Status::Idle)
    }
}

/// List available input device names.
#[tauri::command]
pub fn list_input_devices() -> Result<Vec<String>, String> {
    // Query the system for available audio input devices (microphones).
    // Returns a list of device names that can be selected in settings.
    keyless_audio::input::list_input_device_names()
        .map_err(|e| format!("failed to list devices: {e}"))
}

/// Initialize the runtime and pipeline if not already active.
#[tauri::command]
pub fn initialize_runtime(ctx: DesktopState<'_>) -> Result<(), String> {
    // Initialize the runtime thread and start the audio pipeline.
    // This is typically called when the app starts or after permissions are granted.
    ctx.pipeline.init()
}

/// Report whether the runtime is active.
#[tauri::command]
pub fn runtime_is_ready(ctx: DesktopState<'_>) -> Result<bool, String> {
    // Check if the audio pipeline is currently running and ready to process audio.
    // Returns true if pipeline is active, false if not initialized or stopped.
    Ok(ctx.pipeline.is_active())
}

/// Toggle whether the popover should ignore blur events (used when opening modal dialogs).
#[tauri::command]
pub fn suppress_popover_blur(guard: State<'_, PopoverBlurGuard>, value: bool) {
    // When opening native dialogs (file picker, etc.), we need to prevent the popover
    // from hiding when it loses focus. This guard allows temporarily suppressing the
    // auto-hide behavior.
    guard.set(value);
}

/// Toggle the popover visibility (useful when opening native dialogs).
#[tauri::command]
pub fn set_popover_visibility(app: AppHandle, visible: bool) -> Result<(), String> {
    // Get the popover window (the main settings window).
    if let Some(win) = app.get_webview_window("popover") {
        if visible {
            // Show the window and bring it to focus.
            win.show()
                .and_then(|_| win.set_focus())
                .map_err(|e| format!("show popover window: {e}"))
        } else {
            // Hide the window.
            win.hide().map_err(|e| format!("hide popover window: {e}"))
        }
    } else {
        // Window doesn't exist yet (shouldn't happen, but handle gracefully).
        Ok(())
    }
}

/// Get the arrow direction based on popover position relative to tray.
/// Returns "up" if popover is below tray, "down" if above tray.
#[tauri::command]
pub fn get_arrow_direction(app: AppHandle) -> Result<String, String> {
    // Get the popover window.
    let Some(win) = app.get_webview_window("popover") else {
        // Default to "up" if window doesn't exist (shouldn't happen, but safe fallback).
        return Ok("up".to_string());
    };

    // Get the window's current position.
    // outer_position() returns PhysicalPosition, convert to logical for comparison
    let window_pos_physical = win
        .outer_position()
        .map_err(|e| format!("get window position: {e}"))?;

    // Get the primary monitor to determine screen dimensions.
    let monitor = app
        .primary_monitor()
        .map_err(|e| format!("get primary monitor: {e}"))?
        .ok_or_else(|| "no primary monitor found".to_string())?;

    let scale = monitor.scale_factor();
    let work_area = monitor.work_area();

    // Convert to logical pixels for comparison.
    let work_y = work_area.position.y as f64 / scale;
    let work_height = work_area.size.height as f64 / scale;
    let screen_center_y = work_y + work_height / 2.0;

    // Get window Y position in logical pixels (convert from physical).
    let window_y = window_pos_physical.y as f64 / scale;

    // Determine arrow direction:
    // - If window is in top half of screen (near tray), arrow should point DOWN
    // - If window is in bottom half of screen (below tray), arrow should point UP
    // We use a small threshold (10% of screen height) to account for tray icon height.
    let threshold = work_height * 0.1;
    let direction = if window_y < screen_center_y - threshold {
        "down" // Popover is above tray (near top of screen)
    } else {
        "up" // Popover is below tray (near bottom of screen)
    };

    Ok(direction.to_string())
}
