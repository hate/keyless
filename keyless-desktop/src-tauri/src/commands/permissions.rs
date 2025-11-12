//! Permissions IPC commands (check and open system settings).
use super::DesktopState;

/// Check microphone and accessibility permissions; returns JSON summary.
#[tauri::command]
pub fn check_permissions(ctx: DesktopState<'_>) -> Result<serde_json::Value, String> {
    // Delegate to the permissions service to check system permissions.
    // Returns a JSON object with `microphone`, `accessibility`, and `needsOnboarding` fields.
    // Results are cached for 750ms to avoid excessive system calls.
    ctx.permissions.check()
}

/// Open the OS settings pane for the given permission (platform dependent).
#[tauri::command]
pub fn open_system_settings(ctx: DesktopState<'_>, pane: String) -> Result<(), String> {
    // Delegate to the permissions service to open the system settings pane.
    // On macOS, opens System Settings > Privacy & Security > [pane].
    // Supported panes: "Microphone" or "Accessibility".
    ctx.permissions.open_system_settings(&pane)
}
