//! desktop frontend bridge for keyless.
//!
//! This crate hosts the tauri runtime setup and the ipc surface for the
//! keyless desktop app. It does not duplicate business logic; instead it
//! delegates to existing workspace crates and exposes a small set of
//! commands and events to the React UI.
//!
//! # Errors
//! All fallible functions return `KeylessResult` and map failures to
//! `KeylessError` variants. No panics are used in production paths.
//!
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use keyless_core::error::{KeylessError, KeylessResult};
mod bridge;
mod tray;
use tauri::Manager;

#[tauri::command]
/// A placeholder command used by the default scaffold; returns a greeting.
/// This exists only to verify IPC wiring during early development.
fn greet(name: &str) -> String {
    format!("hello, {}", name)
}

/// Builds and runs the Tauri application.
///
/// This function configures plugins and the command handler, then starts
/// the runtime. Errors are mapped into `KeylessError::System`.
fn run_inner() -> KeylessResult<()> {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = tray::show_popover(app.app_handle());
        }))
        .setup(|app| {
            let _ = tray::init(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            bridge::get_status,
            bridge::start_listening,
            bridge::stop_listening,
            bridge::list_sinks,
            bridge::select_sink,
            bridge::get_config,
            bridge::update_config,
            bridge::get_hotkey,
            bridge::set_hotkey,
        ]);

    builder
        .run(tauri::generate_context!())
        .map_err(|e| KeylessError::System(format!("tauri runtime error: {e}")))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = run_inner() {
        // Use stderr for early scaffold; later we'll wire tracing.
        eprintln!("{e}");
    }
}
