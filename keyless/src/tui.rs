//! Terminal UI (TUI)
//!
//! This module exposes the public TUI entrypoint `run_with_overrides()` and organizes the
//! implementation into smaller focused modules:
//! - `runtime`: event loop (input → state updates → rendering → runtime wiring)
//! - `dictating`: dictating screen (brand/status/EQ/logs/footer widgets)
//! - `config`: config screen (sinks/models/languages/devices/VAD/footer widgets)
//! - `state`: application state, config hydration, and env/CLI overrides
//! - `widgets`: shared widgets (brand, hotkey helpers)
//!
//! Consumers should call `tui::run_with_overrides()` from the binary entrypoint.

pub mod config;
pub mod dictating;
pub mod runtime;
pub mod state;
pub mod theme;
pub mod widgets;

pub use runtime::controller::run_with_overrides;
