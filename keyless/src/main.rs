//! keyless
//!
//! A local‑only, real‑time speech‑to‑text dictation app with a full‑screen TUI.
//!
//! Data flow: `Mic → AudioInput → push_audio() → Whisper Worker → Events → OutputSink`
//!
//! Orchestrates:
//! - Interactive configuration (sink/model/language/device/VAD) in the terminal
//! - Audio capture with push‑to‑talk (PTT) hotkey gating
//! - Real‑time transcription via Whisper with EQ visualizer
//! - Event routing (Partials for logs, Finals to sinks)
//! - Clean shutdown (Ctrl+C ignored in TUI; use 'q')
//!
//! CLI & Overrides:
//! - Flags parsed with `clap` (see `--help`) and applied for the current run
//! - Precedence: CLI flags > ENV vars > saved config

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

use std::process::ExitCode;

// tracing not used in TUI path to avoid tearing; use eprintln! for fatal errors.
use clap::Parser;

use keyless::Cli;

/// Handle top-level CLI flags that short-circuit TUI (e.g., --list-devices).
fn handle_top_level_flags() -> Result<Option<Cli>, String> {
    let cli = Cli::parse();
    if cli.list_devices {
        // Enable CLI logging formatting for non-TUI output
        keyless_logging::init_stdout();
        match keyless_audio::input::list_input_device_names() {
            Ok(list) => {
                if list.is_empty() {
                    println!("(no input devices)");
                } else {
                    for (i, n) in list.iter().enumerate() {
                        println!("{}: {}", i, n);
                    }
                }
            }
            Err(e) => {
                println!("failed to list devices: {}", e);
            }
        }
        return Ok(None);
    }
    Ok(Some(cli))
}

fn main() -> ExitCode {
    // For list-devices (non-TUI), human-friendly CLI logging is fine.
    // For the TUI, avoid initializing a stdout logger to prevent tearing the alt-screen.
    match handle_top_level_flags() {
        Ok(None) => ExitCode::SUCCESS, // handled and printed, exit early
        Ok(Some(cli)) => {
            // Do NOT init stdout logging for the TUI path
            let ov = keyless::overrides::overrides_from_env_and_cli(&cli);
            if let Err(e) = keyless::tui::run_with_overrides(Some(ov)) {
                // Best-effort error reporting to stderr after TUI exits
                eprintln!("tui error: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("flag handling error: {}", e);
            ExitCode::FAILURE
        }
    }
}
