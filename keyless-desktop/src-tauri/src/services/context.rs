//! Desktop context and event emitter for Tauri integration.
//!
//! Provides `DesktopContext` which aggregates all service implementations into a single
//! state object that can be managed by Tauri and passed to command handlers. Also provides
//! `Events` for thread-safe event emission from runtime threads to Tauri windows.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::{
    config::ConfigService, models::ModelsService, permissions::PermissionsService,
    pipeline::PipelineService, ptt::PttService, sinks::SinksService,
};

/// Event emitter facade used by runtime threads to notify Tauri windows.
///
/// Provides thread-safe event emission by serializing payloads and dispatching them
/// to the main thread via `run_on_main_thread`. This allows runtime threads to emit
/// Tauri events without direct access to `AppHandle`.
#[derive(Clone)]
pub struct Events {
    /// Tauri app handle for emitting events to windows.
    app: AppHandle,
}

impl Events {
    /// Create a new `Events` instance with the given app handle.
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Get a clone of the underlying app handle.
    pub fn app_handle(&self) -> AppHandle {
        self.app.clone()
    }

    /// Emit an event to all Tauri windows on the main thread.
    ///
    /// This method is thread-safe and can be called from background threads (like the runtime thread).
    /// It serializes the payload to JSON and dispatches it to the main thread, where Tauri events
    /// must be emitted (Tauri's event system is not thread-safe).
    pub fn emit<T>(&self, name: &str, payload: T)
    where
        T: Serialize,
    {
        // Serialize the payload to JSON first (this can happen on any thread).
        match serde_json::to_value(&payload) {
            Ok(value) => {
                // Clone values needed for the closure that will run on the main thread.
                let event_name = name.to_string();
                let app_handle = self.app.clone();
                let payload_value = value.clone();
                let event_name_for_emit = event_name.clone();

                // Dispatch the actual event emission to the main thread.
                // Tauri's emit() must be called on the main thread, so we use run_on_main_thread
                // to safely emit from background threads.
                let result = self.app.run_on_main_thread(move || {
                    // Emit the event to all Tauri windows.
                    // Ignore the result here; we'll check it below.
                    let _ = app_handle.emit(&event_name_for_emit, payload_value);
                });

                // Log errors but don't fail (event emission failures shouldn't crash the app).
                if let Err(e) = result {
                    eprintln!("[events] failed to dispatch {event_name}: {e}");
                }
            }
            // If serialization fails, log the error but don't crash.
            // This can happen if the payload contains non-serializable types.
            Err(e) => eprintln!("[events] failed to serialize payload for {name}: {e}"),
        }
    }
}

/// Aggregated dependencies for desktop modules.
///
/// Contains all service implementations and the event emitter, providing a single state
/// object that can be managed by Tauri and passed to command handlers. All services are
/// `Arc<dyn Trait>` to allow for testability and clean separation of concerns.
#[derive(Clone)]
pub struct DesktopContext {
    /// System permissions service.
    pub permissions: Arc<dyn PermissionsService>,
    /// Configuration service.
    pub config: Arc<dyn ConfigService>,
    /// Output sink service.
    pub sinks: Arc<dyn SinksService>,
    /// Audio pipeline service.
    pub pipeline: Arc<dyn PipelineService>,
    /// Push-to-talk service.
    pub ptt: Arc<dyn PttService>,
    /// Model catalog service.
    pub models: Arc<dyn ModelsService>,
    /// Event emitter for runtime threads.
    pub events: Arc<Events>,
}

impl DesktopContext {
    /// Create a new `DesktopContext` with the given service implementations.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        permissions: Arc<dyn PermissionsService>,
        config: Arc<dyn ConfigService>,
        sinks: Arc<dyn SinksService>,
        pipeline: Arc<dyn PipelineService>,
        ptt: Arc<dyn PttService>,
        models: Arc<dyn ModelsService>,
        events: Arc<Events>,
    ) -> Self {
        Self {
            permissions,
            config,
            sinks,
            pipeline,
            ptt,
            models,
            events,
        }
    }
}
