//! Model download with progress reporting and cancellation support.
//!
//! Downloads use Range headers to resume from `.partial` files if paused.
//! Progress events are emitted via channel for UI integration.

mod events;
mod large_file;
mod model;
mod progress;
mod small_files;

pub use events::DownloadEvent;
pub use model::ensure_model_cached;
