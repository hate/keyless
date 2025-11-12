//! IPC command modules exposed to the frontend.
//!
//! This module contains all Tauri command handlers that the React frontend can invoke via
//! `invoke()`. Commands are organized by domain: config, models, permissions, runtime, and sinks.
//!
//! All commands receive a `DesktopState` parameter containing the `DesktopContext` with access
//! to all service implementations. Commands delegate to services for business logic, keeping
//! the IPC layer thin.
//!
//! ## Command Categories
//!
//! - `config`: Configuration management (get/update config, hotkey, language, model, bootstrap)
//! - `models`: Model catalog and download management (list, download, pause/resume, status)
//! - `permissions`: System permission checking and requests
//! - `runtime`: Runtime status and control (status, initialization, device listing)
//! - `sinks`: Output sink enumeration and selection

use tauri::State;

use crate::services::DesktopContext;

/// Configuration management commands (get/update config, hotkey, language, model, bootstrap).
pub mod config;
/// Model catalog and download management commands (list, download, pause/resume, status).
pub mod models;
/// System permission checking and request commands.
pub mod permissions;
/// Runtime status and control commands (status, initialization, device listing).
pub mod runtime;
/// Output sink enumeration and selection commands.
pub mod sinks;

/// Shared state wrapper passed to command handlers.
///
/// This is a type alias for `State<'a, DesktopContext>` to simplify command handler signatures.
pub type DesktopState<'a> = State<'a, DesktopContext>;
