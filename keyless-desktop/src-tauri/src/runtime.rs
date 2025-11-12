//! Runtime thread for the desktop app.
//!
//! Manages the audio pipeline on a dedicated background thread to avoid Send constraints
//! (cpal Stream isn't Send). The runtime thread handles:
//!
//! - Audio pipeline lifecycle (startup, shutdown, restart)
//! - Push-to-talk hotkey listener (spawned at startup, controls audio gating via hold_flag)
//! - Event bridging from workspace crates to Tauri events
//! - Pipeline state coordination with the main thread
//!
//! Architecture: `init() → runtime_loop() → {start_pipeline_internal(), ptt_listener, bridge_events()}`
//!
//! ## Submodules
//!
//! - `api`: Public API for querying and controlling the runtime thread
//! - `bridge`: Event bridging from workspace crates to Tauri events
//! - `events`: Tauri event emitter setup
//! - `loop`: Main runtime loop that coordinates pipeline and PTT
//! - `pipeline`: Pipeline startup and lifecycle management
//! - `ptt`: Push-to-talk hotkey listener management
//! - `startup`: Runtime thread initialization
//! - `state`: Shared state for runtime thread coordination

/// Public API for querying and controlling the runtime thread.
mod api;
/// Event bridging from workspace crates to Tauri events.
mod bridge;
/// Tauri event emitter setup.
mod events;
/// Main runtime loop that coordinates pipeline and PTT.
mod r#loop;
/// Pipeline startup and lifecycle management.
mod pipeline;
/// Push-to-talk hotkey listener management.
mod ptt;
/// Runtime thread initialization.
mod startup;
/// Shared state for runtime thread coordination.
mod state;

use self::r#loop::runtime_loop;
use self::pipeline::start_pipeline_internal;

pub use self::api::{is_initialized, is_pipeline_active, request_restart};
pub use self::events::set_events;
pub use self::ptt::{is_listening, update_ptt_selection};
pub use self::startup::init;
