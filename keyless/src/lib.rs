//! keyless
//!
//! Local‑only, real‑time speech‑to‑text dictation with a full‑screen TUI. This binary crate
//! orchestrates the entire application: config screens, model downloads, phased startup,
//! push-to-talk hotkey, audio capture, Whisper transcription, and output delivery.
//!
//! Architecture: `main.rs → TUI Event Loop → {Config Screen, Dictating Screen} → Runtime Context → Pipeline/Workers → Channels`
//!
//! ## Key Features
//!
//! - **Interactive TUI**: Config screen (sink/model/language/device/VAD) and Dictating screen (live EQ, logs, status)
//! - **Phased startup**: Non-blocking Whisper load with "loading" overlay and granular progress
//! - **Model downloads**: Resumable downloads with progress overlay; \[esc] pause, \[backspace] cancel
//! - **Expert mode**: EQ tuning panel with real-time adjustments (bands, noise reduction, gamma, attack/decay)
//! - **Push-to-talk**: Global hotkey with configurable presets (control+option, control+shift)
//! - **Live preview**: Last 4 words shown while speaking; auto-clears after 0.5s
//! - **Permissions UX**: Proactive warnings for macOS Accessibility (PTT, Paste)
//! - **Config persistence**: Atomic save/load with validation; CLI/env overrides
//!
//! ## Submodules
//!
//! - `tui`: TUI screens, widgets, runtime controller, state management
//! - `overrides`: CLI and env var parsing for non-persistent config overrides
//!
//! ## Public API
//!
//! - `Cli`: clap-based CLI parser (16 options for model/language/sink/VAD/EQ/device/hotkey, plus --list-devices flag)
//! - `tui::run_with_overrides()`: Main TUI entry point
//!
//! ## Quick Example
//!
//! ```no_run
//! // Run with defaults (loads saved config or creates one)
//! keyless::tui::run_with_overrides(None)?;
//! # Ok::<(), std::io::Error>(())
//! ```

pub mod overrides;
pub mod tui;

use clap::Parser;

/// CLI argument parser for keyless.
#[derive(Parser, Debug, Default)]
#[command(name = "keyless", version, about = "Local real-time speech-to-text")]
pub struct Cli {
    /// List available audio input devices and exit
    #[arg(long = "list-devices")]
    pub list_devices: bool,

    /// Model ID or path (e.g., "openai/whisper-base" or "openai/whisper-base.en")
    #[arg(long = "model")]
    pub model: Option<String>,

    /// Language code (e.g., "en", "es", "fr") - omit for auto-detection
    #[arg(long = "language")]
    pub language: Option<String>,

    /// Output sink: "paste", "clipboard", or "file"
    #[arg(long = "sink")]
    pub sink: Option<String>,

    /// File path when using --sink file
    #[arg(long = "file")]
    pub file: Option<String>,

    /// Audio input device name
    #[arg(long = "device")]
    pub device: Option<String>,

    /// VAD start threshold in dB (e.g., -30)
    #[arg(long = "vad-start")]
    pub vad_start: Option<f32>,

    /// VAD stop threshold in dB (e.g., -35)
    #[arg(long = "vad-stop")]
    pub vad_stop: Option<f32>,

    /// VAD minimum speech duration in ms (max 65535ms = 65s)
    #[arg(long = "vad-min-duration")]
    pub vad_min_duration: Option<u16>,

    /// VAD maximum silence duration in ms (max 65535ms = 65s)
    #[arg(long = "vad-max-silence")]
    pub vad_max_silence: Option<u16>,

    /// Push-to-talk hotkey (e.g., "control+option", "ctrl+shift+d")
    #[arg(long = "hotkey")]
    pub hotkey: Option<String>,

    /// EQ bands (16-128)
    #[arg(long = "eq-bands")]
    pub eq_bands: Option<u16>,

    /// EQ noise reduction (0.0-1.0, 0=noisy, 1=smooth)
    #[arg(long = "eq-noise-reduction")]
    pub eq_noise_reduction: Option<f32>,

    /// EQ window in dB (10-80)
    #[arg(long = "eq-window")]
    pub eq_window: Option<f32>,

    /// EQ gamma (0.8-2.0)
    #[arg(long = "eq-gamma")]
    pub eq_gamma: Option<f32>,

    /// EQ attack (0.05-1.0)
    #[arg(long = "eq-attack")]
    pub eq_attack: Option<f32>,

    /// EQ decay (0.05-1.0)
    #[arg(long = "eq-decay")]
    pub eq_decay: Option<f32>,
}
