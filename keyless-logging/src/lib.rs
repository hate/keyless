//! keyless-logging
//!
//! Logging initialization for the keyless workspace. Centralizes tracing subscriber
//! setup for binaries. Libraries emit `tracing` events; binaries initialize exactly once.
//!
//! ## Key Features
//!
//! - **Stdout formatter**: Human-friendly logs for CLI (honors `KEYLESS_LOG` or `RUST_LOG`)
//! - **JSON formatter**: Structured logs for programmatic consumption
//! - **Channel writer**: Routes logs to mpsc channel for TUI integration
//! - **Env-based filtering**: Supports crate-level filters (e.g., `keyless_whisper=debug,info`)
//! - **Single initialization**: Libraries never init; binaries call once at startup
//!
//! ## Public API
//!
//! - `init_stdout()`: Human-friendly formatting to stdout
//! - `init_json()`: Structured JSON output
//! - `init_channel()`: Channel-based writer for TUI
//!
//! ## Quick Example
//!
//! ```no_run
//! // In a binary (main.rs):
//! keyless_logging::init_stdout();
//! tracing::info!("app started");
//!
//! // In libraries:
//! tracing::debug!("processing frame");
//! ```

#![cfg_attr(
    doc,
    deny(
        missing_docs,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_html_tags,
        rustdoc::bare_urls
    )
)]

use std::fs::{File, OpenOptions};
use std::io::{Result as IoResult, Write};
use std::sync::{Mutex, mpsc::SyncSender};

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Get log level from environment variables.
///
/// Prefers `KEYLESS_LOG` over `RUST_LOG`, defaulting to `"info"`.
fn env_log_level() -> String {
    std::env::var("KEYLESS_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "info".to_string())
}

/// Initialize human-friendly logging to stdout.
///
/// Outputs pretty-formatted logs suitable for terminal display.
/// Honors `KEYLESS_LOG` first, then `RUST_LOG`, defaulting to `info`.
/// Example: `KEYLESS_LOG=keyless_whisper=debug,info`.
pub fn init_stdout() {
    let env_level = env_log_level();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(env_level))
        .with_target(false)
        .with_level(true)
        .with_timer(fmt::time::uptime())
        .compact()
        .try_init();
}

/// Initialize structured logging suitable for a GUI/backend (e.g., Tauri).
/// Outputs JSON lines for machine consumption.
pub fn init_json() {
    let env_level = env_log_level();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(env_level))
        .with_target(true)
        .with_level(true)
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(true)
        .try_init();
}

/// A writer that forwards bytes to a channel.
struct ChannelWriter {
    /// Destination channel for log lines.
    tx: SyncSender<String>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if !buf.is_empty() {
            // Best-effort UTF-8 conversion; lossy is fine for logs
            let s = String::from_utf8_lossy(buf).to_string();
            let _ = self.tx.try_send(s);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

#[derive(Clone)]
/// Factory that produces `ChannelWriter`s bound to a given channel.
struct ChannelWriterFactory {
    /// Target channel for each produced writer.
    tx: SyncSender<String>,
}

impl<'a> MakeWriter<'a> for ChannelWriterFactory {
    type Writer = ChannelWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ChannelWriter {
            tx: self.tx.clone(),
        }
    }
}

/// Get the log file path in the keyless cache.
fn log_file_path() -> std::path::PathBuf {
    if let Some(base) = dirs_next::cache_dir() {
        base.join("keyless").join("session.log")
    } else {
        std::path::PathBuf::from(".cache")
            .join("keyless")
            .join("session.log")
    }
}

/// Writer that appends to a log file.
struct FileWriter {
    /// Underlying log file with interior mutability for concurrent writes.
    file: Mutex<File>,
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.flush();
        }
        Ok(())
    }
}

#[derive(Clone)]
/// Factory for `FileWriter`s bound to a specific file path.
struct FileWriterFactory {
    /// Path to the log file to append to.
    file_path: std::path::PathBuf,
}

impl<'a> MakeWriter<'a> for FileWriterFactory {
    type Writer = FileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        // Try to open the configured log file, with platform-aware fallbacks
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .or_else(|_| {
                // Fallback 1: platform-specific null device (should always exist)
                #[cfg(unix)]
                let null_path = "/dev/null";
                #[cfg(windows)]
                let null_path = "NUL";
                OpenOptions::new().write(true).open(null_path)
            })
            .or_else(|_| {
                // Fallback 2: temp file in current directory
                File::create(".keyless.log.tmp")
            })
            .unwrap_or_else(|e| {
                // If we can't create ANY file (even /dev/null or NUL), the system is broken.
                // Panic with a platform-aware message rather than silently losing logs.
                #[cfg(unix)]
                let attempted = "session.log, /dev/null, .keyless.log.tmp";
                #[cfg(windows)]
                let attempted = "session.log, NUL, .keyless.log.tmp";
                #[cfg(not(any(unix, windows)))]
                let attempted = "session.log, .keyless.log.tmp";

                panic!(
                    "Fatal: cannot create any log file (tried {}): {}",
                    attempted, e
                )
            });

        FileWriter {
            file: Mutex::new(file),
        }
    }
}

/// Initialize channel-based logging that writes to the given channel and log file.
///
/// This is framework-agnostic and can be used by TUI, Tauri, or other frontends
/// that need to capture logs via mpsc channels for display in their UI.
///
/// The channel receives all logs, while the file is filtered to:
/// - INFO+ for dependencies (reqwest, hyper, h2, rustls, etc.)
/// - DEBUG+ for keyless crates
///
/// Safe to call once at startup. Subsequent calls will no-op if a subscriber exists.
pub fn init_channel(tx: SyncSender<String>) {
    // Ensure log directory exists
    let log_path = log_file_path();
    if let Some(dir) = log_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    // Shared filter: quiet dependencies, verbose for keyless crates
    // This applies to both TUI and file to keep them consistent
    let log_filter = EnvFilter::new(
        "info,\
         keyless=debug,\
         keyless_whisper=debug,\
         keyless_audio=debug,\
         keyless_output=debug,\
         keyless_models=debug,\
         keyless_runtime=debug,\
         keyless_core=debug",
    );

    // Channel writer (for TUI display)
    let channel_layer = tracing_subscriber::fmt::layer()
        .with_writer(ChannelWriterFactory { tx })
        .without_time()
        .with_ansi(false)
        .compact()
        .with_filter(log_filter.clone());

    // File writer (same filter as TUI for consistency)
    let file_writer = FileWriterFactory {
        file_path: log_path,
    };

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .without_time()
        .with_ansi(false)
        .compact()
        .with_filter(log_filter);

    // Ignore error if already set (e.g., in tests); we want best-effort setup
    let _ = tracing_subscriber::registry()
        .with(channel_layer)
        .with(file_layer)
        .try_init();
}
