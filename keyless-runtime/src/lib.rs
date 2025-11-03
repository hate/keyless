//! keyless-runtime
//!
//! Application orchestration layer for keyless. Coordinates audio capture, transcription,
//! and output services into ready-to-use pipelines. Framework-agnostic and suitable for
//! TUI, Tauri, CLI, or other frontends.
//!
//! Architecture: `Config → start_pipeline() → {AudioHandle, WhisperHandle} → Channels → Frontend Event Loop`
//!
//! ## Key Features
//!
//! - **Phased startup**: Non-blocking initialization with granular progress events
//! - **Pipeline orchestration**: Wires audio → VAD → Whisper → output sink
//! - **Worker lifecycle**: Start/stop management for audio and whisper services
//! - **Push-to-talk**: Global hotkey listener with configurable presets
//! - **Channel-based events**: mpsc for async communication between services
//! - **Thread-safe state**: `Arc<AtomicBool>` for PTT hold, cancel flags, enable/disable
//! - **Send-boundary handling**: Safe artifact passing across thread boundaries
//!
//! ## Submodules
//!
//! - `pipeline`: Public API (`start_pipeline`, `build_startup_artifacts`, `finalize_pipeline_from_artifacts`)
//! - `channels`: Semantically-grouped mpsc channels (`AudioChannels`, `UiChannels`, `ControlChannels`)
//! - `workers`: Lifecycle management for audio and whisper services
//! - `ptt`: Global hotkey listener with `PttBackend` trait
//! - `ptt_context`: PTT state bundling (flags, handle, timing)
//!
//! ## Public API
//!
//! - `start_pipeline()`, `build_startup_artifacts()`, `finalize_pipeline_from_artifacts()`
//! - `PipelineChannels`, `PipelineHandles`, `StartupEvent`, `StartupArtifacts`
//! - `Channels`, `ChannelReceivers`
//! - `spawn_listener()`, `spawn_with_backend()`, `PttBackend` trait
//! - `PttContext`
//!
//! ## Quick Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::sync::atomic::AtomicBool;
//! use keyless_runtime::{pipeline, channels};
//! use keyless_core::config::Config;
//! use keyless_core::error::KeylessResult;
//!
//! # fn example() -> KeylessResult<()> {
//! let config = Config::default();
//! let (ch, receivers) = channels::Channels::new();
//! let hold_flag = Arc::new(AtomicBool::new(false));
//!
//! let pipeline_channels = pipeline::PipelineChannels {
//!     tx_level: ch.audio.tx_level,
//!     tx_log: ch.ui.tx_log,
//!     tx_spec: ch.audio.tx_spec,
//!     tx_preview: ch.ui.tx_preview,
//!     tx_vad: ch.control.tx_vad,
//! };
//! let pipeline = pipeline::start_pipeline(
//!     &config,
//!     config.eq.clone(),
//!     config.vad.clone(),
//!     pipeline_channels,
//!     hold_flag,
//!     None, // Use default audio device
//! )?;
//!
//! // Drain events from receivers and update UI
//! while let Ok(log) = receivers.ui.rx_log.try_recv() {
//!     println!("{}", log);
//! }
//! # Ok(())
//! # }
//! ```

#![deny(warnings)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(
    doc,
    deny(
        missing_docs,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_html_tags,
        rustdoc::bare_urls
    )
)]

pub mod channels;
pub mod pipeline;
pub mod ptt;
pub mod workers;

pub use channels::{
    AudioChannelReceivers, AudioChannels, ChannelReceivers, Channels, ControlChannelReceivers,
    ControlChannels, UiChannelReceivers, UiChannels,
};
pub use pipeline::{PipelineChannels, PipelineHandles, start_pipeline};
pub use ptt::{PttBackend, PttContext, spawn_listener, spawn_with_backend};
