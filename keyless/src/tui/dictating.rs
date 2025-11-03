//! Dictating screen rendering.
//!
//! This module provides the live dictation screen UI that displays the audio visualizer,
//! transcription preview, logs, and status indicators. It organizes the implementation into
//! focused submodules:
//!
//! - `draw`: Main draw function that orchestrates all widgets (brand, status, EQ, logs, footer)
//! - `eq`: EQ bars rendering with preview text overlay (last 4 recognized words)
//! - `status`: Status indicator rendering (LISTENING when PTT held, IDLE otherwise)
//! - `footer`: Footer hotkeys rendering for the dictating view (esc back, q quit)

/// Main draw function.
mod draw;
/// EQ bars with preview text.
mod eq;
/// Footer hotkeys for dictating view.
mod footer;
/// Status indicator (listening/idle).
mod status;

pub use draw::draw_dictating;
