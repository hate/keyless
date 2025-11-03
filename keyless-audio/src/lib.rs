//! keyless-audio
//!
//! Audio capture subsystem using cpal (Cross-Platform Audio Library). Provides
//! device‑agnostic, format‑agnostic, realtime‑friendly microphone input with frame‑based
//! delivery on a worker thread.
//!
//! Architecture: `Mic → OS Audio Thread (cpal) → Accumulation Buffer → Bounded Channel → Worker Thread → on_frame(frame)`
//!
//! ## Key Features
//!
//! - **Cross-platform capture**: Works on macOS (CoreAudio), Windows (WASAPI), Linux (ALSA/Pulse/Jack)
//! - **Format-agnostic**: Auto-converts F32/I16/U16 to normalized f32
//! - **Frame-based delivery**: Fixed-size frames delivered to callback on worker thread
//! - **VAD gating**: Hysteresis-based voice activity detection with timing controls
//! - **EQ visualization**: FFT-based spectrum with log-spaced bands and auto-sensitivity
//! - **Zero per-frame allocations**: Cached FFT buffers for 80-90% speedup
//! - **Device enumeration**: Opaque handles for UI integration without leaking cpal types
//!
//! ## Submodules
//!
//! - `config`: `AudioConfig` (sampling and frame configuration)
//! - `input`: `api` (traits/helpers) and `cpal` (`CpalAudioInput` implementation)
//! - `vad`: `VadGate` (voice activity detection gate with hysteresis and timing)
//! - `eq_pipeline`: EQ spectrum pipeline (FFT, log-spaced bands, auto-sensitivity via max normalization)
//! - `sfx`: Procedural SFX beeps for PTT press/release/final
//!
//! ## Public API
//!
//! - `AudioInput`, `CpalAudioInput`, `AudioConfig`
//! - `VadGate`
//! - `EqConfig`, `EqState`, `compute_bars()`
//! - `InputDevice`, `default_input_device()`, `list_input_devices()`
//!
//! ## Quick Example
//!
//! ```no_run
//! use keyless_audio::{AudioConfig, CpalAudioInput, AudioInput};
//! let mut input = CpalAudioInput::new();
//! input.start(
//!     None,
//!     AudioConfig { sample_hz: 0, frame_samples: 0 },
//!     Box::new(|_frame: &[f32]| {/* handle ~100ms frames */}),
//! )?;
//! input.stop()?;
//! # Ok::<(), keyless_core::error::KeylessError>(())
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

pub mod config;
pub mod eq_pipeline;
pub mod input;
pub mod sfx;
pub mod vad;

pub use config::AudioConfig;
pub use eq_pipeline::{EqConfig, EqState, compute_bars};
pub use input::{AudioInput, CpalAudioInput};
pub use sfx::SfxPlayer;
pub use vad::VadGate;
