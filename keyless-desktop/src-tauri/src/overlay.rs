use std::env;

use tauri::{
    AppHandle, LogicalPosition, Manager, Position, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

/// Width of the recording overlay indicator in logical pixels.
const OVERLAY_WIDTH: f64 = 80.0;
/// Height of the recording overlay indicator in logical pixels.
const OVERLAY_HEIGHT: f64 = 36.0;

/// Bottom margin for the overlay on macOS.
#[cfg(target_os = "macos")]
const OVERLAY_BOTTOM_OFFSET: f64 = 4.0;
/// Bottom margin for the overlay on Windows and Linux.
#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_BOTTOM_OFFSET: f64 = 12.0;
/// Bottom margin for the overlay on other platforms.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
const OVERLAY_BOTTOM_OFFSET: f64 = 24.0;

/// Width of the toast notification window in logical pixels.
const TOAST_WIDTH: f64 = 400.0;
/// Height of the toast notification window in logical pixels.
const TOAST_HEIGHT: f64 = 120.0;
/// Right margin for the toast notification from screen edge.
const TOAST_RIGHT_OFFSET: f64 = 20.0;
/// Top margin for the toast notification from screen edge.
const TOAST_TOP_OFFSET: f64 = 20.0;

/// Configuration for creating an overlay window.
pub struct OverlayWindowConfig {
    /// Window label (must be unique).
    pub label: &'static str,
    /// HTML file to load (e.g., "overlay.html").
    pub html_file: &'static str,
    /// Initial window position (x, y) in logical pixels.
    pub position: (f64, f64),
    /// Window size (width, height) in logical pixels.
    pub size: (f64, f64),
}

/// Build a standard overlay window (transparent, always-on-top, no decorations).
pub fn build_overlay_window(
    app: &AppHandle,
    config: OverlayWindowConfig,
) -> Result<WebviewWindow, tauri::Error> {
    // Determine the URL to load based on build mode:
    // - Debug: load from dev server (TAURI_DEV_URL env var or default localhost:1420)
    // - Release: load from bundled assets
    let url = if cfg!(debug_assertions) {
        // Development mode: load from dev server for hot reload.
        let dev_url =
            env::var("TAURI_DEV_URL").unwrap_or_else(|_| "http://localhost:1420".to_string());
        let url_string = format!("{}/{}", dev_url, config.html_file);
        // Parse the URL string into a proper URL type.
        let parsed = url_string
            .parse()
            .map_err(|e| tauri::Error::AssetNotFound(format!("Failed to parse dev URL: {}", e)))?;
        WebviewUrl::External(parsed)
    } else {
        // Production mode: load from bundled assets.
        WebviewUrl::App(config.html_file.into())
    };

    // Build the overlay window with specific properties:
    // - Transparent background (allows custom styling)
    // - Always on top (appears above all other windows)
    // - No decorations (no title bar, borders, etc.)
    // - Not resizable/minimizable/closable (overlay behavior)
    // - Not focusable (doesn't steal keyboard focus)
    // - Initially hidden (shown/hidden programmatically)
    WebviewWindowBuilder::new(app, config.label, url)
        .title(config.label)
        .position(config.position.0, config.position.1)
        .resizable(false)
        .inner_size(config.size.0, config.size.1)
        .shadow(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .focused(false)
        .visible(false)
        .build()
}

/// Calculate bottom-center position for the overlay on the primary monitor.
fn bottom_center_on_primary(app: &AppHandle) -> Option<(f64, f64)> {
    // Get the primary monitor (main display).
    match app.primary_monitor() {
        Ok(Some(monitor)) => {
            // Get the monitor's scale factor (for high-DPI displays).
            let scale = monitor.scale_factor();
            // Get the work area (screen area excluding menu bar/dock).
            let work_area = monitor.work_area();

            // Convert work area coordinates to logical pixels (divide by scale factor).
            let work_x = work_area.position.x as f64 / scale;
            let work_y = work_area.position.y as f64 / scale;
            let work_width = work_area.size.width as f64 / scale;
            let work_height = work_area.size.height as f64 / scale;

            // Calculate desired position: centered horizontally, near bottom vertically.
            let desired_x = work_x + (work_width - OVERLAY_WIDTH) / 2.0;
            // Calculate usable height (work area height minus overlay height).
            let usable_height = (work_height - OVERLAY_HEIGHT).max(0.0);
            // Position near bottom with platform-specific offset.
            let desired_y = work_y + (usable_height - OVERLAY_BOTTOM_OFFSET).max(0.0);

            // Define bounds for clamping (ensure overlay stays within work area).
            let min_x = work_x;
            let max_x = work_x + work_width - OVERLAY_WIDTH;
            let min_y = work_y;
            let max_y = work_y + usable_height.max(0.0);

            // Clamp desired position to valid bounds (handle edge cases like very small screens).
            let clamped_x = if max_x < min_x {
                // Edge case: screen too narrow, just use left edge.
                work_x
            } else {
                desired_x.clamp(min_x, max_x)
            };
            let clamped_y = if max_y < min_y {
                // Edge case: screen too short, just use top edge.
                work_y
            } else {
                desired_y.clamp(min_y, max_y)
            };

            Some((clamped_x, clamped_y))
        }
        _ => None, // No monitor found (shouldn't happen, but handle gracefully).
    }
}

/// Create the recording overlay window at app startup (initially hidden).
pub fn create_overlay(app: &AppHandle) {
    // Check if overlay window already exists (idempotent).
    if app.get_webview_window("overlay").is_some() {
        return;
    }

    // Calculate bottom-center position on primary monitor.
    if let Some((x, y)) = bottom_center_on_primary(app) {
        // Build and create the overlay window.
        match build_overlay_window(
            app,
            OverlayWindowConfig {
                label: "overlay",
                html_file: "overlay.html",
                position: (x, y),
                size: (OVERLAY_WIDTH, OVERLAY_HEIGHT),
            },
        ) {
            Ok(window) => {
                // Configure window to not steal focus or intercept mouse events.
                // This ensures the overlay doesn't interfere with user interaction.
                let _ = window.set_focusable(false);
                let _ = window.set_ignore_cursor_events(true);
            }
            Err(err) => {
                // Log error but don't crash (overlay is non-critical).
                eprintln!("[overlay] failed to create overlay window: {err}");
            }
        }
    }
}

/// Show the recording overlay indicator and update its position.
pub fn show_overlay(app: &AppHandle) {
    // Calculate position before moving to main thread (position calculation can happen on any thread).
    let handle = app.clone();
    let position = bottom_center_on_primary(app);
    // Window operations must happen on the main thread (Tauri requirement).
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("overlay") {
            // Update position if we got a valid position (handles monitor changes).
            if let Some((x, y)) = position {
                let _ = window.set_position(Position::Logical(LogicalPosition { x, y }));
            }
            // Ensure overlay stays on top and doesn't interfere with user interaction.
            let _ = window.set_always_on_top(true);
            let _ = window.set_focusable(false);
            let _ = window.set_ignore_cursor_events(true);
            // Show the overlay window.
            let _ = window.show();
        }
    });
}

/// Hide the recording overlay indicator with a delay for exit animation.
pub fn hide_overlay(app: &AppHandle) {
    let handle = app.clone();
    // Delay hiding to allow exit animation to play (150ms).
    // This provides a smooth visual transition when PTT is released.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        // Window operations must happen on the main thread.
        let handle_for_hide = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if let Some(window) = handle_for_hide.get_webview_window("overlay") {
                // Re-enable cursor events before hiding (cleanup).
                let _ = window.set_ignore_cursor_events(false);
                // Hide the overlay window.
                let _ = window.hide();
            }
        });
    });
}

/// Calculate top-right position for the toast notification on the primary monitor.
fn top_right_on_primary(app: &AppHandle) -> Option<(f64, f64)> {
    // Get the primary monitor (main display).
    match app.primary_monitor() {
        Ok(Some(monitor)) => {
            // Get the monitor's scale factor (for high-DPI displays).
            let scale = monitor.scale_factor();
            // Get monitor size in logical pixels.
            let size = monitor.size().to_logical::<f64>(scale);
            // Calculate top-right position: right edge minus toast width and offset.
            let desired_x = size.width - TOAST_WIDTH - TOAST_RIGHT_OFFSET;
            // Position at top with small offset from edge.
            let desired_y = TOAST_TOP_OFFSET;
            Some((desired_x, desired_y))
        }
        _ => None, // No monitor found (shouldn't happen, but handle gracefully).
    }
}

/// Create the toast notification window at app startup (initially hidden).
pub fn create_toast(app: &AppHandle) {
    eprintln!("[toast] create_toast called");
    // Check if toast window already exists (idempotent).
    if app.get_webview_window("toast").is_some() {
        eprintln!("[toast] toast window already exists");
        return;
    }

    // Calculate top-right position on primary monitor.
    if let Some((x, y)) = top_right_on_primary(app) {
        eprintln!("[toast] creating toast at position ({}, {})", x, y);
        // Build and create the toast window.
        match build_overlay_window(
            app,
            OverlayWindowConfig {
                label: "toast",
                html_file: "toast.html",
                position: (x, y),
                size: (TOAST_WIDTH, TOAST_HEIGHT),
            },
        ) {
            Ok(window) => {
                eprintln!("[toast] toast window created successfully");
                // Configure window to not steal focus or intercept mouse events.
                // This ensures the toast doesn't interfere with user interaction.
                let _ = window.set_focusable(false);
                let _ = window.set_ignore_cursor_events(true);
            }
            Err(err) => {
                // Log error but don't crash (toast is non-critical).
                eprintln!("[toast] failed to create toast window: {err}");
            }
        }
    } else {
        eprintln!("[toast] failed to get monitor position");
    }
}

/// Show the toast notification window.
pub fn show_toast(app: &AppHandle) {
    let handle = app.clone();
    // Window operations must happen on the main thread (Tauri requirement).
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("toast") {
            // Ensure toast stays on top and doesn't interfere with user interaction.
            let _ = window.set_always_on_top(true);
            let _ = window.set_focusable(false);
            let _ = window.set_ignore_cursor_events(true);
            // Show the toast window.
            let _ = window.show();
        }
    });
}

/// Hide the toast notification window.
pub fn hide_toast(app: &AppHandle) {
    let handle = app.clone();
    // Window operations must happen on the main thread (Tauri requirement).
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("toast") {
            // Hide the toast window.
            let _ = window.hide();
        }
    });
}
