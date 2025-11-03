//! Push-to-talk (PTT) subsystem.
//!
//! This module provides global hotkey listening and PTT state management for controlling
//! when audio is captured and transcribed. It organizes the implementation into focused submodules:
//!
//! - `listener`: Global hotkey listener thread (rdev integration) with configurable presets
//!   (ctrl+option, ctrl+shift) and heartbeat detection
//! - `context`: PTT context bundling (flags, handle, timing) for lifecycle management
//! - `handlers`: PTT press/release event handlers (clear preview, finalize segment, deliver output)

/// PTT state bundling (flags, handle, timing).
pub mod context;
/// PTT press/release handlers.
pub mod handlers;
/// Global hotkey listener.
pub mod listener;

pub use context::PttContext;
pub use listener::{PttBackend, spawn_listener, spawn_with_backend};
