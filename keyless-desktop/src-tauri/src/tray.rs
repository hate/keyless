//! tray icon and tray-related behaviors.
//!
//! Provides helpers to initialize the tray and to toggle/show windows
//! anchored near the menu bar area.
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

use keyless_core::error::{KeylessError, KeylessResult};

/// Initialize the tray icon and click behavior.
///
/// Left-click toggles the `popover` window anchored near the menu bar.
pub fn init(app: &mut App) -> KeylessResult<()> {
    let handle = app.handle();
    // Build tray menu with output submenu
    let sink_paste = MenuItem::with_id(handle, "sink_paste", "paste", true, None::<&str>)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let sink_clipboard =
        MenuItem::with_id(handle, "sink_clipboard", "clipboard", true, None::<&str>)
            .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let sink_file = MenuItem::with_id(handle, "sink_file", "file", true, None::<&str>)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;

    let output_submenu = Submenu::with_items(
        handle,
        "output",
        true,
        &[&sink_paste, &sink_clipboard, &sink_file],
    )
    .map_err(|e| KeylessError::System(format!("submenu error: {e}")))?;

    let settings = MenuItem::with_id(handle, "settings", "settings", true, None::<&str>)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let quit = MenuItem::with_id(handle, "quit", "quit", true, None::<&str>)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;

    let menu = Menu::new(handle).map_err(|e| KeylessError::System(format!("menu error: {e}")))?;
    let _ = menu.append(&output_submenu);
    let _ = menu.append(&settings);
    let _ = menu.append(&quit);

    // Load and decode tray icon (64x64 for better clarity on retina displays)
    let icon_bytes = include_bytes!("../icons/64x64.png");
    let img = image::load_from_memory(icon_bytes)
        .map_err(|e| KeylessError::System(format!("decode tray icon: {e}")))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let icon = tauri::image::Image::new_owned(rgba.into_raw(), w, h);

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .on_tray_icon_event(|app, event| {
            if let TrayIconEvent::Click { .. } = event {
                let _ = toggle_popover(app.app_handle());
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "sink_paste" => {
                    let _ = crate::bridge::select_sink("paste".to_string());
                }
                "sink_clipboard" => {
                    let _ = crate::bridge::select_sink("clipboard".to_string());
                }
                "sink_file" => {
                    let _ = crate::bridge::select_sink("file".to_string());
                }
                "settings" => {
                    if let Some(win) = app.get_webview_window("settings") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
                "quit" => app.exit(0),
                _ => {}
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
