//! keyless-desktop
//!
//! Desktop frontend bridge for keyless using Tauri. This crate hosts the Tauri runtime setup
//! and the IPC surface for the keyless desktop app. It does not duplicate business logic;
//! instead it delegates to existing workspace crates and exposes a small set of commands and
//! events to the React UI.
//!
//! Architecture: `main.rs → lib.rs::run() → Tauri Builder → Commands (IPC) → Services → Workspace Crates`
//!
//! ## Key Features
//!
//! - **IPC Commands**: Tauri command handlers for config, models, permissions, runtime, and sinks
//! - **Service Layer**: Abstraction layer for business logic (config, models, permissions, pipeline, PTT, sinks)
//! - **Runtime Thread**: Dedicated background thread for audio pipeline (avoids Send constraints)
//! - **Event System**: Tauri event emitter for frontend notifications (status, downloads, config changes)
//! - **Overlay Windows**: Recording indicator overlay and toast notification windows
//! - **System Integration**: Tray icon, autostart, single-instance enforcement, system settings integration
//!
//! ## Submodules
//!
//! - `commands`: Tauri IPC command handlers (config, models, permissions, runtime, sinks)
//! - `services`: Service trait definitions and implementations (business logic abstraction)
//! - `runtime`: Background runtime thread for audio pipeline and PTT handling
//! - `overlay`: Overlay window management for recording indicators and toasts
//! - `popover`: Popover window blur suppression state management
//! - `tray`: System tray icon and menu management
//!
//! ## Public API
//!
//! - `run()`: Main entry point that builds and runs the Tauri application
//!
//! ## Errors
//!
//! All fallible functions return `KeylessResult` and map failures to `KeylessError` variants.
//! No panics are used in production paths.
//!
//! Learn more about Tauri commands at <https://tauri.app/develop/calling-rust/>
#![deny(warnings)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(
    doc,
    deny(
        missing_docs,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_html_tags,
        rustdoc::bare_urls
    )
)]

use keyless_core::error::{KeylessError, KeylessResult};
use popover::PopoverBlurGuard;
use std::sync::Arc;
use tauri::{Emitter, Manager};

/// Tauri command surface and runtime bootstrap.
mod commands;
/// Overlay window management for recording indicators.
mod overlay;
/// Platform-specific detection and utilities.
mod platform;
/// Popover suppression state management.
mod popover;
/// Background runtime thread for audio pipeline and PTT handling.
mod runtime;
/// Service layer abstractions for business logic.
pub mod services;
/// System tray icon and menu management.
mod tray;

/// Builds and runs the Tauri application.
///
/// This function configures Tauri plugins (devtools, single-instance, autostart, dialog, etc.),
/// sets up the command handler, initializes services, creates overlay windows, and starts the
/// runtime thread. Errors are mapped into `KeylessError::System`.
///
/// # Errors
///
/// Returns `KeylessError::System` if Tauri initialization fails, tray setup fails, or runtime
/// initialization fails.
///
/// # Panics
///
/// Panics are caught and converted to `KeylessError::System` via `catch_unwind`.
fn run_inner() -> KeylessResult<()> {
    eprintln!("[run_inner] Creating Tauri builder...");
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_devtools::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            eprintln!("[single_instance] Second instance detected - exiting silently");
            // Minimal callback - just log and let the second instance exit
            // Windows might not be ready yet when this callback fires
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            platform::get_autostart_launcher(),
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_positioner::init());

    eprintln!("[run_inner] Setting up app...");
    let builder = builder
        .setup(|app| {
            eprintln!("[setup] Starting setup hook...");
            // Create the event emitter facade for runtime threads to send events to the frontend.
            // This is shared between the main thread and runtime thread.
            let events = Arc::new(services::Events::new(app.handle().clone()));
            // Register the event emitter with the runtime module (so runtime thread can emit events).
            crate::runtime::set_events(Arc::clone(&events));

            // Create the desktop context with all service implementations.
            // Services are wrapped in Arc<dyn Trait> for testability and thread safety.
            let ctx = services::DesktopContext::new(
                Arc::new(services::PermissionsServiceImpl::new()),
                Arc::new(services::ConfigServiceImpl::new()),
                Arc::new(services::SinksServiceImpl::new()),
                Arc::new(services::PipelineServiceImpl::new()),
                Arc::new(services::PttServiceImpl::new()),
                Arc::new(services::ModelsServiceImpl::new(Arc::clone(&events))),
                Arc::clone(&events),
            );
            // Register context and events with Tauri's state management (accessible to command handlers).
            app.manage(ctx.clone());
            app.manage(events.clone());
            // Register popover blur guard (prevents popover from hiding when opening dialogs).
            app.manage(PopoverBlurGuard::default());

            // Configure platform-specific app settings.
            platform::configure_app(&mut *app);

            eprintln!("[setup] Preloading Candle device...");
            // Preload Candle device on main thread to ensure Metal device is created
            // on the UI thread before WebKit/other subsystems use Metal.
            // This prevents race conditions where Metal device creation happens on wrong thread.
            let _device = keyless_whisper::preload_device();
            eprintln!("[setup] Device preloaded: {:?}", _device);

            eprintln!("[setup] Initializing tray...");
            // Initialize the system tray icon and menu.
            // This must happen on macOS before the app can run as a menubar app.
            if let Err(e) = tray::init(app) {
                eprintln!("[setup] ERROR: Failed to initialize tray: {}", e);
                return Err(e.into());
            }
            eprintln!("[setup] Tray initialized successfully");

            // Create overlay windows (recording indicator and toast notifications).
            // These are separate windows that appear above other applications.
            overlay::create_overlay(app.handle());
            overlay::create_toast(app.handle());

            // Initialize runtime thread on app startup. It will attempt to start the
            // pipeline immediately; if permissions are missing, it logs an error but
            // keeps the thread alive so we can request a restart later.
            // The runtime thread handles audio processing and PTT hotkey listening.
            match crate::runtime::init() {
                Ok(_) => eprintln!("[setup] Runtime init requested at startup"),
                Err(e) => eprintln!("[setup] Runtime init error: {}", e),
            }

            eprintln!("[setup] Setup completed successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::runtime::get_status,
            commands::sinks::list_sinks,
            commands::sinks::select_sink,
            commands::runtime::list_input_devices,
            commands::permissions::check_permissions,
            commands::runtime::initialize_runtime,
            commands::permissions::open_system_settings,
            commands::config::get_config,
            commands::config::update_config,
            commands::config::get_hotkey,
            commands::config::set_hotkey,
            commands::config::set_model,
            commands::config::set_language,
            commands::config::reset_config,
            commands::config::load_settings_bootstrap,
            commands::runtime::runtime_is_ready,
            commands::runtime::suppress_popover_blur,
            commands::runtime::set_popover_visibility,
            commands::runtime::get_arrow_direction,
            commands::models::list_models,
            commands::models::list_models_with_sizes,
            commands::models::start_model_download,
            commands::models::cancel_model_download,
            commands::models::pause_model_download,
            commands::models::resume_model_download,
            commands::models::refresh_model_sizes,
            commands::models::purge_models_cache,
            commands::models::remove_model,
            commands::models::get_model_status,
        ])
        .on_window_event(|window, event| {
            // Handle window blur events for the popover window (main settings window).
            // When the popover loses focus, hide it automatically (menubar app behavior).
            if window.label() == "popover"
                && let tauri::WindowEvent::Focused(false) = event
            {
                // Check if blur suppression is active (e.g., when opening a file dialog).
                // If suppressed, don't hide the window (user is interacting with a dialog).
                let guard = window.state::<PopoverBlurGuard>();
                if guard.is_suppressed() {
                    return;
                }
                // Hide the popover window when it loses focus (standard menubar app behavior).
                let _ = window.hide();
                // Emit a log message for debugging.
                let _ = window
                    .app_handle()
                    .emit("log_message", "popover hidden (blur)");
            }
        });

    eprintln!("[run_inner] Running Tauri app...");
    // Wrap the Tauri run in catch_unwind to convert panics to errors.
    // This ensures we can log panics and exit gracefully instead of crashing.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        builder.run(tauri::generate_context!())
    }));

    // Handle the result: Ok(Ok(_)) means successful run, Ok(Err(_)) means Tauri error,
    // Err(_) means panic was caught.
    match result {
        Ok(Ok(_)) => {
            eprintln!("[run_inner] Tauri app exited normally");
            Ok(())
        }
        Ok(Err(e)) => {
            // Tauri runtime error (e.g., window creation failed, IPC error).
            eprintln!("[run_inner] Tauri run error: {:?}", e);
            eprintln!("[run_inner] Error details: {}", e);
            Err(KeylessError::System(format!("tauri runtime error: {e}")))
        }
        Err(panic) => {
            // Panic was caught (shouldn't happen in production, but handle gracefully).
            eprintln!("[run_inner] PANIC caught: {:?}", panic);
            Err(KeylessError::System(format!(
                "panic during tauri run: {:?}",
                panic
            )))
        }
    }
}

/// Main entry point for the keyless-desktop application.
///
/// Sets up panic handling and delegates to `run_inner()` for the actual Tauri setup.
/// Exits with code 1 on error.
///
/// # Panics
///
/// Sets a panic hook to log panics before they propagate. Panics in `run_inner()` are caught
/// and converted to errors.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Set up a panic hook to log panics before they propagate.
    // This helps with debugging and ensures we can log panics even if they occur
    // outside of catch_unwind blocks.
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[keyless-desktop] PANIC: {:?}", panic_info);
    }));

    eprintln!("[keyless-desktop] Starting...");
    // Delegate to run_inner() for the actual Tauri setup and execution.
    match run_inner() {
        Ok(_) => {
            eprintln!("[keyless-desktop] run_inner() completed successfully");
        }
        Err(e) => {
            // Log fatal error and exit with code 1 (indicates failure).
            // Use stderr for early scaffold; later we'll wire tracing.
            eprintln!("[keyless-desktop] FATAL ERROR: {e}");
            eprintln!("[keyless-desktop] Error details: {:?}", e);
            std::process::exit(1);
        }
    }
    eprintln!("[keyless-desktop] Exiting normally");
}
