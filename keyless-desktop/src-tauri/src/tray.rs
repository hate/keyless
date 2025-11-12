//! tray icon and tray-related behaviors.
//!
//! Provides helpers to initialize the tray and to toggle/show windows
//! anchored near the menu bar area.
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Wry;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Listener, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

use keyless_core::config::{self, storage};
use keyless_core::error::{KeylessError, KeylessResult};

/// Models submenu and its checkable items map.
type ModelsSubmenuAndItems = (Submenu<Wry>, HashMap<String, CheckMenuItem<Wry>>);

/// Static handle to the paste sink menu item for external updates.
static SINK_PASTE_ITEM: OnceLock<Mutex<CheckMenuItem<Wry>>> = OnceLock::new();
/// Static handle to the clipboard sink menu item for external updates.
static SINK_CLIPBOARD_ITEM: OnceLock<Mutex<CheckMenuItem<Wry>>> = OnceLock::new();
/// Static handle to the file sink menu item for external updates.
static SINK_FILE_ITEM: OnceLock<Mutex<CheckMenuItem<Wry>>> = OnceLock::new();

/// Track model check items to toggle on selection.
static MODEL_ITEMS: OnceLock<Mutex<HashMap<String, CheckMenuItem<Wry>>>> = OnceLock::new();
/// Global tray icon handle.
static TRAY_ICON: OnceLock<Mutex<TrayIcon<Wry>>> = OnceLock::new();

/// Activate the macOS app once (required for tray icon to work properly).
#[cfg(target_os = "macos")]
fn activate_app_once() {
    use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyAccessory};
    use cocoa::base::YES;

    // Track if we've already activated (idempotent).
    static ACTIVATED: AtomicBool = AtomicBool::new(false);
    if ACTIVATED.load(Ordering::Acquire) {
        return;
    }

    // Use unsafe FFI to access macOS NSApplication APIs.
    // This is required to set the app as an accessory (menubar-only) app.
    unsafe {
        let app = NSApp();
        if !app.is_null() {
            // Activate the app (brings it to foreground, required for tray to work).
            app.activateIgnoringOtherApps_(YES);
            // Set activation policy to Accessory (no Dock icon, menubar-only).
            app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
        }
    }

    // Mark as activated to prevent redundant calls.
    ACTIVATED.store(true, Ordering::Release);
}

/// No-op on non-macOS platforms.
#[cfg(not(target_os = "macos"))]
fn activate_app_once() {}

/// Components of the tray menu for storage and updates.
struct TrayMenuParts {
    /// Root menu.
    menu: Menu<Wry>,
    /// Paste sink check menu item.
    sink_paste: CheckMenuItem<Wry>,
    /// Clipboard sink check menu item.
    sink_clipboard: CheckMenuItem<Wry>,
    /// File sink check menu item.
    sink_file: CheckMenuItem<Wry>,
    /// Model check menu items keyed by model ID.
    model_items: HashMap<String, CheckMenuItem<Wry>>,
}

/// Store sink menu item handles in static storage for external updates.
/// These handles allow other modules to update checkmarks when config changes.
fn store_sink_handles(parts: &TrayMenuParts) {
    // Store paste sink menu item handle.
    let paste_slot = SINK_PASTE_ITEM.get_or_init(|| Mutex::new(parts.sink_paste.clone()));
    if let Ok(mut guard) = paste_slot.lock() {
        *guard = parts.sink_paste.clone();
    }
    // Store clipboard sink menu item handle.
    let clipboard_slot =
        SINK_CLIPBOARD_ITEM.get_or_init(|| Mutex::new(parts.sink_clipboard.clone()));
    if let Ok(mut guard) = clipboard_slot.lock() {
        *guard = parts.sink_clipboard.clone();
    }
    // Store file sink menu item handle.
    let file_slot = SINK_FILE_ITEM.get_or_init(|| Mutex::new(parts.sink_file.clone()));
    if let Ok(mut guard) = file_slot.lock() {
        *guard = parts.sink_file.clone();
    }
}

/// Store model menu item handles in static storage for external updates.
/// These handles allow other modules to update checkmarks when model selection changes.
fn store_model_handles(map: HashMap<String, CheckMenuItem<Wry>>) {
    // Store the map of model ID -> menu item handle.
    if let Ok(mut store) = MODEL_ITEMS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        *store = map;
    }
}

/// Build the models submenu with checkable items for installed models.
fn build_models_submenu(
    handle: &AppHandle,
    current_model: &str,
) -> Result<ModelsSubmenuAndItems, KeylessError> {
    // Helper to convert model ID to a safe menu item ID.
    // Model IDs contain slashes (e.g., "openai/whisper-base"), which aren't valid in menu IDs.
    // We replace "/" with "__" and prefix with "model_use::" for uniqueness.
    let to_id = |m: &str| format!("model_use::{}", m.replace('/', "__"));

    // Get the desktop context to access model service.
    let ctx = handle.state::<crate::services::DesktopContext>();
    let mut model_items: Vec<CheckMenuItem<Wry>> = Vec::new();
    let mut models_downloaded: Vec<String> = Vec::new();
    // Query downloaded models (local only; avoid network hit on menu open).
    // This is fast and doesn't require network access.
    if let Ok(models) = ctx.models.list_installed_models() {
        models_downloaded = models;
    }
    // Create a checkable menu item for each installed model.
    for id in &models_downloaded {
        // Check if this model is currently selected.
        let is_current = *id == current_model;
        // Create checkable menu item (checkmark shown if is_current is true).
        let item = CheckMenuItem::with_id(handle, to_id(id), id, true, is_current, None::<&str>)
            .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
        model_items.push(item);
    }
    // Create "manage" menu item (opens Models view in popover).
    let manage = MenuItem::with_id(handle, "models_manage", "⌞⌝  manage", true, None::<&str>)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;

    // Build a map of menu item IDs to menu items (for later updates).
    let mut map: HashMap<String, CheckMenuItem<Wry>> = HashMap::new();
    for it in &model_items {
        map.insert(it.id().as_ref().to_string(), it.clone());
    }

    // Build the submenu structure: label, model items (or placeholder), separator, manage item.
    let mut refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = Vec::new();
    // Add "installed" label (non-clickable).
    let available_label = MenuItem::with_id(
        handle,
        "models_label_available",
        "installed",
        false,
        None::<&str>,
    )
    .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    refs.push(&available_label as &dyn tauri::menu::IsMenuItem<Wry>);
    // Add placeholder if no models installed, otherwise add model items.
    let mut placeholder_item: Option<MenuItem<Wry>> = None;
    if model_items.is_empty() {
        // No models installed: show "none available" placeholder.
        placeholder_item = Some(
            MenuItem::with_id(handle, "models_none", "none available", false, None::<&str>)
                .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?,
        );
    } else {
        // Add each model item to the menu.
        for it in &model_items {
            refs.push(it as &dyn tauri::menu::IsMenuItem<Wry>);
        }
    }
    // Add placeholder if it exists.
    if let Some(ref item) = placeholder_item {
        refs.push(item as &dyn tauri::menu::IsMenuItem<Wry>);
    }
    // Add separator before "manage" item.
    let sep = PredefinedMenuItem::separator(handle)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    refs.push(&sep as &dyn tauri::menu::IsMenuItem<Wry>);
    // Add "manage" item.
    refs.push(&manage as &dyn tauri::menu::IsMenuItem<Wry>);
    // Create the submenu with all items.
    let submenu = Submenu::with_items(handle, "၊၊||၊  models", true, refs.as_slice())
        .map_err(|e| KeylessError::System(format!("submenu error: {e}")))?;

    Ok((submenu, map))
}

/// Build the complete tray menu structure.
fn build_tray_menu(handle: &AppHandle) -> KeylessResult<TrayMenuParts> {
    // Load current config to determine which items should be checked.
    let cfg = storage::load_config().unwrap_or_default();
    // Determine which sink is currently selected (for checkmarks).
    let current_is_paste = matches!(cfg.output_mode, config::OutputMode::Paste);
    let current_is_clipboard = matches!(cfg.output_mode, config::OutputMode::Clipboard);
    let current_is_file = matches!(cfg.output_mode, config::OutputMode::File(_));

    // Create checkable menu items for each output sink.
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

    // Create output submenu with all sink options.
    let output_submenu = Submenu::with_items(
        handle,
        "↗    output",
        true,
        &[&sink_paste, &sink_clipboard, &sink_file],
    )
    .map_err(|e| KeylessError::System(format!("submenu error: {e}")))?;

    // Build models submenu (includes installed models and "manage" option).
    let current_model = cfg.model_path.to_string_lossy().to_string();
    let (models_submenu, model_items) = build_models_submenu(handle, &current_model)?;

    // Create settings and quit menu items.
    let settings = MenuItem::with_id(handle, "settings", "≡     settings", true, None::<&str>)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let quit = MenuItem::with_id(handle, "quit", "⏻    quit", true, None::<&str>)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;

    // Build the root menu and append all submenus/items in order.
    let menu = Menu::new(handle).map_err(|e| KeylessError::System(format!("menu error: {e}")))?;
    let _ = menu.append(&output_submenu);
    let _ = menu.append(&models_submenu);
    // Add separator between models and settings.
    let sep_models_settings = PredefinedMenuItem::separator(handle)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let _ = menu.append(&sep_models_settings);
    let _ = menu.append(&settings);
    // Add version info (non-clickable).
    let version_text = format!("keyless v{}", env!("CARGO_PKG_VERSION"));
    let version_item =
        MenuItem::with_id(handle, "about_version", &version_text, false, None::<&str>)
            .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let _ = menu.append(&version_item);
    // Add separator between settings and quit.
    let sep_settings_quit = PredefinedMenuItem::separator(handle)
        .map_err(|e| KeylessError::System(format!("menu item error: {e}")))?;
    let _ = menu.append(&sep_settings_quit);
    let _ = menu.append(&quit);

    // Return all menu parts for storage and updates.
    Ok(TrayMenuParts {
        menu,
        sink_paste,
        sink_clipboard,
        sink_file,
        model_items,
    })
}

/// Rebuild and update the tray menu from current config.
/// This is called when models are added/removed to refresh the menu structure.
fn rebuild_tray_menu(handle: &AppHandle) -> KeylessResult<()> {
    // Build a fresh menu structure from current config.
    let parts = build_tray_menu(handle)?;
    // Store menu item handles for later updates (checkmarks).
    store_sink_handles(&parts);
    store_model_handles(parts.model_items.clone());
    // Update the tray icon's menu with the new structure.
    if let Some(tray_mutex) = TRAY_ICON.get()
        && let Ok(tray) = tray_mutex.lock()
    {
        tray.set_menu(Some(parts.menu.clone()))
            .map_err(|e| KeylessError::System(format!("tray menu update error: {e}")))?;
    }
    // Update checkmarks to reflect current config (double-check).
    update_sink_checkmarks_from_config();
    update_model_checkmarks_from_config();
    Ok(())
}

/// Update model checkmarks to reflect the current selected model.
fn sync_model_checkmarks(current_model: &str) {
    // Get the map of model menu items.
    if let Some(map_mutex) = MODEL_ITEMS.get()
        && let Ok(map) = map_mutex.lock()
    {
        // Update checkmark for each model item.
        for (menu_id, item) in map.iter() {
            // Decode menu ID back to model ID (reverse of to_id() function).
            // Remove "model_use::" prefix and replace "__" with "/".
            let decoded = menu_id
                .strip_prefix("model_use::")
                .unwrap_or(menu_id)
                .replace("__", "/");
            // Set checkmark if this model matches the current selection.
            let _ = item.set_checked(decoded == current_model);
        }
    }
}

/// Update tray sink checkmarks to match the current config.
pub fn update_sink_checkmarks_from_config() {
    // Load current config to determine which sink is selected.
    let cfg = storage::load_config().unwrap_or_default();
    // Determine which sink is currently selected.
    let is_paste = matches!(cfg.output_mode, config::OutputMode::Paste);
    let is_clip = matches!(cfg.output_mode, config::OutputMode::Clipboard);
    let is_file = matches!(cfg.output_mode, config::OutputMode::File(_));
    // Update paste sink checkmark.
    if let Some(slot) = SINK_PASTE_ITEM.get()
        && let Ok(item) = slot.lock()
    {
        let _ = item.set_checked(is_paste);
    }
    // Update clipboard sink checkmark.
    if let Some(slot) = SINK_CLIPBOARD_ITEM.get()
        && let Ok(item) = slot.lock()
    {
        let _ = item.set_checked(is_clip);
    }
    // Update file sink checkmark.
    if let Some(slot) = SINK_FILE_ITEM.get()
        && let Ok(item) = slot.lock()
    {
        let _ = item.set_checked(is_file);
    }
}

/// Update model checkmarks from the current config.
pub fn update_model_checkmarks_from_config() {
    // Load current config to get the selected model.
    let cfg = storage::load_config().unwrap_or_default();
    // Sync checkmarks to match the selected model.
    sync_model_checkmarks(&cfg.model_path.to_string_lossy());
}

/// Tracks if we're currently toggling the popover to prevent multiple rapid toggles
static TOGGLING: OnceLock<AtomicBool> = OnceLock::new();
/// Last toggle timestamp in milliseconds since epoch to throttle rapid events
static LAST_TOGGLE_MS: OnceLock<AtomicU64> = OnceLock::new();

/// Initialize the tray icon and click behavior.
///
/// Left-click toggles the `popover` window anchored near the menu bar.
pub fn init(app: &mut App) -> KeylessResult<()> {
    let handle = app.handle();

    // Build the initial tray menu structure.
    let parts = build_tray_menu(handle)?;
    // Store menu item handles for later updates (checkmarks).
    store_sink_handles(&parts);
    store_model_handles(parts.model_items.clone());
    // Update checkmarks to match current config.
    update_sink_checkmarks_from_config();
    update_model_checkmarks_from_config();

    // Initialize toggle throttling state (prevents rapid toggles).
    TOGGLING.get_or_init(|| AtomicBool::new(false));
    LAST_TOGGLE_MS.get_or_init(|| AtomicU64::new(0));

    // Load tray icon from bundled assets.
    let icon_bytes = include_bytes!("../icons/64x64.png");
    // Decode PNG image to RGBA format.
    let img = image::load_from_memory(icon_bytes)
        .map_err(|e| KeylessError::System(format!("decode tray icon: {e}")))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    // Create Tauri image from raw RGBA data.
    let icon = tauri::image::Image::new_owned(rgba.into_raw(), w, h);

    // Build the tray icon with menu and event handlers.
    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&parts.menu)
        .show_menu_on_left_click(false) // Left-click toggles popover, right-click shows menu.
        .on_tray_icon_event(|tray, event| {
            // Handle window positioning (tray_plugin_positioner integration).
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

            // Handle left-click events (toggle popover).
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                // Throttle rapid clicks to prevent double-toggles.
                if let Some(toggling) = TOGGLING.get() {
                    // Check if enough time has passed since last toggle (250ms throttle).
                    if let Some(last) = LAST_TOGGLE_MS.get() {
                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let prev = last.load(Ordering::Relaxed);
                        // If clicked too soon after last toggle, ignore this click.
                        if now_ms.saturating_sub(prev) < 250 {
                            return;
                        }
                        last.store(now_ms, Ordering::Relaxed);
                    }
                    // Try to acquire the toggle lock (prevents concurrent toggles).
                    if toggling
                        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                        .is_ok()
                    {
                        let app_handle = tray.app_handle().clone();

                        // Toggle the popover window.
                        if let Err(e) = toggle_popover(&app_handle) {
                            eprintln!("[tray] Failed to toggle popover: {}", e);
                            // Release lock on error.
                            toggling.store(false, Ordering::Release);
                        } else {
                            // Release lock after a short delay (allows toggle to complete).
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                if let Some(guard) = TOGGLING.get() {
                                    guard.store(false, Ordering::Release);
                                }
                            });
                        }
                    }
                } else if let Err(e) = toggle_popover(tray.app_handle()) {
                    // Fallback if TOGGLING not initialized (shouldn't happen).
                    eprintln!("[tray] Failed to toggle popover: {}", e);
                }
            }
        })
        .on_menu_event(|app, event| {
            // Handle menu item clicks.
            let id = event.id().as_ref();
            match id {
                "sink_paste" => {
                    // Update checkmarks immediately (optimistic UI update).
                    if let Some(slot) = SINK_PASTE_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(true);
                    }
                    if let Some(slot) = SINK_CLIPBOARD_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(false);
                    }
                    if let Some(slot) = SINK_FILE_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(false);
                    }
                    // Actually select the sink (updates config and restarts pipeline).
                    if let Err(e) = crate::commands::sinks::select_sink(
                        app.state::<crate::services::DesktopContext>(),
                        "paste".to_string(),
                    ) {
                        eprintln!("[tray] failed to select sink: {e}");
                    }
                }
                "sink_clipboard" => {
                    // Update checkmarks immediately (optimistic UI update).
                    if let Some(slot) = SINK_PASTE_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(false);
                    }
                    if let Some(slot) = SINK_CLIPBOARD_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(true);
                    }
                    if let Some(slot) = SINK_FILE_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(false);
                    }
                    // Actually select the sink.
                    if let Err(e) = crate::commands::sinks::select_sink(
                        app.state::<crate::services::DesktopContext>(),
                        "clipboard".to_string(),
                    ) {
                        eprintln!("[tray] failed to select sink: {e}");
                    }
                }
                "sink_file" => {
                    // Update checkmarks immediately (optimistic UI update).
                    if let Some(slot) = SINK_PASTE_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(false);
                    }
                    if let Some(slot) = SINK_CLIPBOARD_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(false);
                    }
                    if let Some(slot) = SINK_FILE_ITEM.get()
                        && let Ok(item) = slot.lock()
                    {
                        let _ = item.set_checked(true);
                    }
                    // Actually select the sink.
                    if let Err(e) = crate::commands::sinks::select_sink(
                        app.state::<crate::services::DesktopContext>(),
                        "file".to_string(),
                    ) {
                        eprintln!("[tray] failed to select sink: {e}");
                    }
                }
                "settings" => {
                    // Open Settings view in popover.
                    if let Some(win) = app.app_handle().get_webview_window("popover") {
                        let _ = win.move_window(Position::TrayCenter);
                        // Update arrow direction based on actual position.
                        update_arrow_direction(app.app_handle(), &win);
                        let _ = win.show();
                        let _ = win.set_focus();
                        // Navigate to settings view.
                        let _ = win.emit("navigate", "settings");
                    }
                }
                "models_manage" => {
                    // Open Models view in popover.
                    if let Some(win) = app.app_handle().get_webview_window("popover") {
                        let _ = win.move_window(Position::TrayCenter);
                        // Update arrow direction based on actual position.
                        update_arrow_direction(app.app_handle(), &win);
                        let _ = win.show();
                        let _ = win.set_focus();
                        // Navigate to models view.
                        let _ = win.emit("navigate", "models");
                    }
                }
                "quit" => app.exit(0),
                id_str => {
                    // Handle model selection (menu IDs start with "model_use::").
                    if let Some(encoded) = id_str.strip_prefix("model_use::") {
                        // Decode model ID (replace "__" with "/").
                        let model_id = encoded.replace("__", "/");
                        // Set the model in config (triggers pipeline restart).
                        if let Err(e) = crate::commands::config::set_model(
                            app.state::<crate::services::DesktopContext>(),
                            model_id.clone(),
                        ) {
                            eprintln!("[tray] failed to set model: {e}");
                        } else {
                            // Update checkmarks to reflect the new selection.
                            sync_model_checkmarks(&model_id);
                        }
                    }
                }
            }
        })
        .build(app)
        .map_err(|e| KeylessError::System(format!("tray build error: {e}")))?;

    // Store tray icon handle globally (for menu updates).
    let tray_slot = TRAY_ICON.get_or_init(|| Mutex::new(tray.clone()));
    if let Ok(mut guard) = tray_slot.lock() {
        *guard = tray;
    }

    // Activate macOS app (required for tray to work properly).
    activate_app_once();

    // Hide popover initially (shown on tray click).
    if let Some(win) = handle.get_webview_window("popover") {
        let _ = win.hide();
    }

    // Listen for model catalog events and rebuild menu when models change.
    // This keeps the menu in sync when models are downloaded/removed.
    for name in ["download_complete", "model_removed", "catalog_refreshed"] {
        let app_handle = handle.clone();
        handle.listen(name, move |_| {
            // Rebuild menu to reflect new model list.
            if let Err(e) = rebuild_tray_menu(&app_handle) {
                eprintln!("[tray] failed to rebuild models submenu: {e}");
            }
        });
    }

    Ok(())
}

/// Toggle the visibility of the `popover` window.
///
/// This function is called from the tray icon event handler, which runs on the main thread.
/// Therefore, we can directly access and modify windows without `run_on_main_thread`.
pub fn toggle_popover(app: &AppHandle) -> KeylessResult<()> {
    if let Some(win) = app.get_webview_window("popover") {
        // Check visibility synchronously (we're on main thread).
        let is_visible = win.is_visible().unwrap_or(false);

        if is_visible {
            // Window is visible: hide it.
            let _ = win.hide();
            let _ = app.emit("log_message", "popover hidden");
            return Ok(());
        }
    }
    // Window not found or not visible: show it.
    show_popover(app)
}

/// Update arrow direction based on popover position relative to tray.
/// Emits "arrow_direction" event with "up" or "down".
fn update_arrow_direction(app: &AppHandle, win: &tauri::WebviewWindow) {
    // Determine arrow direction based on actual window position.
    // After positioning, check if popover is above or below tray and emit direction.
    if let Ok(Some(mon)) = app.primary_monitor() {
        let scale = mon.scale_factor();
        let work_area = mon.work_area();
        let work_y = work_area.position.y as f64 / scale;
        let work_height = work_area.size.height as f64 / scale;
        let work_bottom = work_y + work_height;

        if let Ok(window_pos_physical) = win.outer_position() {
            // Convert physical position to logical pixels
            let window_y = window_pos_physical.y as f64 / scale;

            // Get window height to find bottom edge
            if let Ok(window_size_physical) = win.outer_size() {
                // outer_size() returns PhysicalSize<u32>, convert to logical pixels
                let window_height = window_size_physical.height as f64 / scale;
                let window_bottom = window_y + window_height;

                // Determine arrow direction based on where the tray is relative to the popover.
                // The arrow is at the TOP of the popover content, so:
                // - If popover is BELOW tray: arrow should point UP (toward tray above)
                // - If popover is ABOVE tray: arrow should point DOWN (toward tray below)
                //
                // We detect tray position by checking which edge of the screen the popover is closer to:
                // - If popover top is very close to screen top (< 50px): tray is likely at top → arrow UP
                // - If popover bottom is very close to screen bottom (< 50px): tray is likely at bottom → arrow DOWN
                // - Otherwise, use platform-specific defaults
                let edge_threshold = 50.0; // pixels from edge
                let distance_from_top = window_y - work_y;
                let distance_from_bottom = work_bottom - window_bottom;

                let direction = if distance_from_top < edge_threshold {
                    // Popover is very close to top → tray is at top → arrow points UP
                    "up"
                } else if distance_from_bottom < edge_threshold {
                    // Popover is very close to bottom → tray is at bottom → arrow points DOWN
                    "down"
                } else {
                    // Not near either edge - use platform-specific default
                    // But if window is in upper half, assume tray at top
                    let default = crate::platform::default_tray_arrow_direction();
                    if window_y < work_y + work_height / 2.0 {
                        "up"
                    } else {
                        default
                    }
                };

                let _ = app.emit("arrow_direction", direction);
            }
        }
    }
}

/// Show the `popover` window and focus it.
///
/// This function should be called from the main thread (e.g., tray event handler).
/// Window operations are performed synchronously to prevent race conditions.
pub fn show_popover(app: &AppHandle) -> KeylessResult<()> {
    if let Some(win) = app.get_webview_window("popover") {
        // Determine which view to show based on current runtime state.
        // If PTT is held, show "listening" view; otherwise show "idle" view.
        let dest = if crate::runtime::is_listening() {
            "listening"
        } else {
            "idle"
        };
        // Navigate to the appropriate view.
        let _ = app.emit("navigate", dest);
        // Position window near the tray icon (tray_plugin_positioner integration).
        // TrayCenter positions popover below tray (centered). The plugin automatically
        // adjusts if tray is near screen edges.
        let _ = win.move_window(Position::TrayCenter);

        // Update arrow direction based on actual position.
        update_arrow_direction(app, &win);

        // Show the window.
        let _ = win.show();
        let _ = app.emit("log_message", "popover shown");
        // Focus the window (brings it to front).
        let _ = win.set_focus();
    }
    Ok(())
}
