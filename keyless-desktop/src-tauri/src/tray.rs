//! tray icon and tray-related behaviors.
//!
//! Provides helpers to initialize the tray and to toggle/show windows
//! anchored near the menu bar area.
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

use keyless_core::error::{KeylessError, KeylessResult};

/// Initialize the tray icon and click behavior.
///
/// Left-click toggles the `popover` window anchored near the menu bar.
pub fn init(app: &mut App) -> KeylessResult<()> {
    let handle = app.handle();

    // Read current sink from config to set checkmarks
    let cfg = keyless_core::config::storage::load_config().unwrap_or_default();
    let current_is_paste = matches!(cfg.output_mode, keyless_core::config::OutputMode::Paste);
    let current_is_clipboard =
        matches!(cfg.output_mode, keyless_core::config::OutputMode::Clipboard);
    let current_is_file = matches!(cfg.output_mode, keyless_core::config::OutputMode::File(_));

    // Build tray menu with output submenu and checkmarks
    let sink_paste = CheckMenuItem::with_id(
        handle,
        "sink_paste",
        "paste",
        true,
        current_is_paste,
        None::<&str>,
    )
    .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let sink_clipboard = CheckMenuItem::with_id(
        handle,
        "sink_clipboard",
        "clipboard",
        true,
        current_is_clipboard,
        None::<&str>,
    )
    .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let sink_file = CheckMenuItem::with_id(
        handle,
        "sink_file",
        "file",
        true,
        current_is_file,
        None::<&str>,
    )
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
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            // Track tray position for positioner plugin
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

            // Handle clicks: left-click toggles popover, right-click shows menu automatically
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let _ = toggle_popover(tray.app_handle());
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
                    // Show popover and navigate to settings
                    let app_handle = app.app_handle().clone();
                    if let Some(win) = app_handle.get_webview_window("popover") {
                        let _ = win.move_window(Position::TrayCenter);
                        let _ = win.show();
                        let _ = win.set_focus();
                        // Emit after showing so window is ready to receive events
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            let _ = app_handle.emit("navigate", "settings");
                        });
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
    // Reset page to current status before showing to avoid any visible page flash
    let dest = if crate::runtime::is_listening() {
        "listening"
    } else {
        "idle"
    };
    let _ = app.emit("navigate", dest);
    // Anchor to tray icon (positioner tracks tray position via on_tray_event)
    let _ = win.move_window(Position::TrayCenter);
    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}
