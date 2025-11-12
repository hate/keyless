//! Service layer abstractions for business logic.
//!
//! This module provides trait-based abstractions for all business logic operations, allowing
//! for testability and clean separation between IPC commands and implementation details.
//! All services are `Send + Sync` to work with Tauri's async command handlers.
//!
//! ## Service Traits
//!
//! - `ConfigService`: Configuration loading, saving, and patching
//! - `ModelsService`: Model catalog, downloads, and status tracking
//! - `PermissionsService`: System permission checking and requests
//! - `PipelineService`: Audio pipeline lifecycle management
//! - `PttService`: Push-to-talk hotkey management
//! - `SinksService`: Output sink enumeration and selection
//!
//! ## Context
//!
//! `DesktopContext` aggregates all service implementations and provides a single state object
//! that can be managed by Tauri and passed to command handlers.

/// Configuration loading, saving, and patching service.
pub mod config;
/// Desktop context and event emitter for Tauri integration.
pub mod context;
/// Model catalog and download management service.
pub mod models;
/// System permissions checking and management service.
pub mod permissions;
/// Audio pipeline lifecycle management service.
pub mod pipeline;
/// Push-to-talk hotkey management service.
pub mod ptt;
/// Output sink enumeration and selection service.
pub mod sinks;

pub use config::{ConfigService, ConfigServiceImpl};
pub use context::{DesktopContext, Events};
pub use models::{ModelsService, ModelsServiceImpl};
pub use permissions::{PermissionsService, PermissionsServiceImpl};
pub use pipeline::{PipelineService, PipelineServiceImpl};
pub use ptt::{PttService, PttServiceImpl};
pub use sinks::{SinksService, SinksServiceImpl};
