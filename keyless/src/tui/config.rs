//! Config screen rendering.
//!
//! This module provides the configuration screen UI where users select output sink, model,
//! language, input device, and VAD parameters. It organizes the implementation into focused
//! submodules:
//!
//! - `draw`: Main draw function that orchestrates all panels and overlays
//! - `sinks`: Output sink picker rendering (Paste, Clipboard, File)
//! - `models`: Model table rendering with download status and size information
//! - `languages`: Language selector rendering with curated language codes
//! - `devices`: Input device list rendering with device enumeration
//! - `vad`: VAD threshold/timing panel rendering (start/stop dB, min duration, max silence)
//! - `footer`: Hotkey hints rendering for the config screen
//! - `expert`: Expert mode overlay for EQ tuning and config management

/// Devices submodule (input device list rendering).
mod device;
/// Main draw function.
mod draw;
/// Expert mode overlay.
pub mod expert;
/// Footer submodule (hotkey hints rendering).
mod footer;
/// Languages submodule (language selector rendering).
mod languages;
/// Models submodule (model table rendering).
mod models;
/// Sinks submodule (output sink picker rendering).
mod sinks;
/// VAD submodule (threshold/timing panel rendering).
mod vad;

pub use draw::draw_config;
pub use expert::EqParameter;
