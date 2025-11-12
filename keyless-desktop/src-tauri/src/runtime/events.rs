//! Simple event bus wrapper to publish messages to the frontend window(s).
use crate::services::Events;
use serde::Serialize;
use std::sync::{Arc, OnceLock};

/// Global events emitter shared with runtime threads.
pub static EVENTS: OnceLock<Arc<Events>> = OnceLock::new();

/// Install the global events handle (called from setup).
pub fn set_events(events: Arc<Events>) {
    // Store the events emitter globally (accessible to runtime threads).
    // Uses OnceLock to ensure thread-safe one-time initialization.
    let _ = EVENTS.set(events);
}

/// Borrow the global events handle if present and call a function with it.
pub fn with_events<F>(f: F)
where
    F: FnOnce(&Arc<Events>),
{
    // Get the global events handle and call the provided function with it.
    // If events haven't been set yet, this is a no-op (safe for early initialization).
    if let Some(events) = EVENTS.get() {
        f(events);
    }
}

/// Emit a typed JSON event to all windows.
pub fn emit_event<T>(name: &str, payload: T)
where
    T: Serialize,
{
    // Emit an event with a serializable payload to all Tauri windows.
    // This is thread-safe and can be called from any thread (including runtime threads).
    with_events(|events| events.emit(name, payload));
}

/// Emit a log line to the UI log.
pub fn emit_log(message: impl Into<String>) {
    // Convenience wrapper for emitting log messages to the frontend.
    // The frontend displays these in a log view.
    emit_event("log_message", message.into());
}
