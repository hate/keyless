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
        // Enable CLI logging formatting for non-TUI output (stdout is safe here).
        // TUI path avoids stdout logging to prevent tearing alternate screen buffer.
        keyless_logging::init_stdout();
        match keyless_audio::input::list_input_device_names() {
            Ok(list) => {
                // Empty list: show helpful message instead of blank output.
                if list.is_empty() {
                    println!("(no input devices)");
                } else {
                    // Print index:name pairs; enumerate provides 0-based indices for user reference.
                    for (i, n) in list.iter().enumerate() {
                        println!("{}: {}", i, n);
                    }
                }
            }
            Err(e) => {
                // Print error to stdout (non-TUI path); user expects to see it.
                println!("failed to list devices: {}", e);
            }
        }
        // Early return: list-devices handled, don't start TUI.
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
            // Do NOT init stdout logging for the TUI path (prevents alternate screen corruption).
            // Build overrides from CLI flags and ENV vars (CLI takes precedence).
            let ov = keyless::overrides::overrides_from_env_and_cli(&cli);
            if let Err(e) = keyless::tui::run_with_overrides(Some(ov)) {
                // Best-effort error reporting to stderr after TUI exits (terminal restored).
                // Use stderr to avoid mixing with TUI output if terminal wasn't fully restored.
                eprintln!("tui error: {}", e);
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            // Flag parsing error: report to stderr before any TUI initialization.
            eprintln!("flag handling error: {}", e);
            ExitCode::FAILURE
        }
    }
}
