//! Shared widgets.
//!
//! This module provides reusable, view-agnostic widgets used across multiple TUI screens.
//! It organizes the implementation into focused submodules:
//!
//! - `brand`: Brand header widget with title and optional hotkey line (used in Config and Dictating screens)
//! - `hotkey`: Hotkey rendering helpers for `[key]` tokens and layout estimation (footer width calculation)
//! - `logs`: Rolling logs panel with syntax highlighting (used in Config and Dictating screens)

pub mod brand;
pub mod hotkey;
pub mod logs;
