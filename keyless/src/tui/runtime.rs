//! Runtime (tui::runtime)
//!
//! Orchestrates the TUI runtime: event loop, input handling, push‑to‑talk (PTT),
//! and worker lifecycles.
//!
//! Submodules:
//! - `controller`: main event loop (50 lines; delegates to helpers)
//! - `context`: runtime context bundling channels, flags, handles, and terminal
//! - `drains`: channel drain logic (logs, spectrum, VAD, sizes, download events)
//! - `init`: one-time Config screen initialization (catalog, discovery, size workers)
//! - `input`: input routing and handling (Config, Expert Mode, Dictating)
//! - `periodic`: periodic tasks (size refresh, PTT heartbeat check)
//! - `ptt`: global hotkey listener (presets, enable/disable, heartbeat)
//! - `workers`: audio and whisper orchestration (start/stop handles)
//!
//! Public entrypoint: `tui::run_with_overrides` re‑exported from `tui::mod`.

mod catalog_runtime;
pub mod context;
pub mod controller;
mod download_context;
mod drains;
mod handlers;
mod init;
mod input;
mod periodic;

pub use keyless_runtime::{
    AudioChannels, ChannelReceivers, Channels, ControlChannels, PipelineHandles, PttContext,
    UiChannels, pipeline, ptt, workers,
};

pub use catalog_runtime::CatalogRuntime;
pub use download_context::DownloadContext;
