//! tray icon and tray-related behaviors.
//!
//! Provides helpers to initialize the tray and to toggle/show windows
//! anchored near the menu bar area.
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

use keyless_core::error::{KeylessError, KeylessResult};

/// Initialize the tray icon and click behavior.
///
/// Left-click toggles the `popover` window anchored near the menu bar.
pub fn init(app: &mut App) -> KeylessResult<()> {
    let handle = app.handle();
    let _tray = TrayIconBuilder::new()
        // use default app icon if available; otherwise the OS provides one
        .on_tray_icon_event(|app, event| {
            if let TrayIconEvent::Click { .. } = event {
                let _ = toggle_popover(app.app_handle());
            }
        })
        .build(app)
        .map_err(|e| KeylessError::System(format!("tray build error: {e}")))?;

    // ensure windows exist but hidden; created via tauri.conf.json
    if let Some(win) = handle.get_webview_window("popover") {
        let _ = win.hide();
    }
    if let Some(win) = handle.get_webview_window("pill") {
        let _ = win.hide();
    }
    Ok(())
}

/// Toggle the visibility of the `popover` window.
pub fn toggle_popover(app: &AppHandle) -> KeylessResult<()> {
    let Some(win) = app.get_webview_window("popover") else {
        return Err(KeylessError::System("missing popover window".into()));
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return Ok(());
    }
    show_popover(app)
}

/// Show the `popover` window and focus it.
pub fn show_popover(app: &AppHandle) -> KeylessResult<()> {
    let Some(win) = app.get_webview_window("popover") else {
        return Err(KeylessError::System("missing popover window".into()));
    };
    // approximate anchoring (top-right); refine later with per-platform anchor
    let _ = win.move_window(Position::TopRight);
    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}
