//! keyless-output
//!
//! Output sinks for delivering transcribed text to various destinations. Provides
//! pluggable backends (Paste, Clipboard, File) with a factory pattern for config-driven selection.
//!
//! ## Key Features
//!
//! - **Paste sink**: Keystroke simulation via enigo (appears where user is typing)
//! - **Clipboard sink**: System clipboard via arboard (user pastes manually)
//! - **File sink**: Append to file (one line per transcription)
//! - **Factory pattern**: `OutputSinkProvider` trait for config-driven selection
//! - **Cross-platform**: Works on macOS, Windows, Linux (permissions vary by sink)
//! - **Send + Sync**: Thread-safe for use in Tauri IPC and concurrent contexts
//!
//! ## Submodules
//!
//! - `provider`: Factory for creating sinks from config (`OutputSinkProvider`, `DefaultOutputProvider`)
//! - `sinks`: Sink implementations (`ClipboardSink`, `FileSink`, `PasteSink`)
//!
//! ## Public API
//!
//! - `OutputSinkProvider` trait, `DefaultOutputProvider`
//! - `ClipboardSink`, `FileSink`, `PasteSink`
//!
//! ## Quick Example
//!
//! ```no_run
//! use keyless_core::config::{Config, OutputMode};
//! use keyless_output::{DefaultOutputProvider, OutputSinkProvider};
//! let mut cfg = Config::default();
//! cfg.output_mode = OutputMode::Clipboard;
//! let sink = DefaultOutputProvider.provide(&cfg)?;
//! sink.send_text("Hello")?;
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

mod provider;
pub mod sinks;

pub use provider::{DefaultOutputProvider, OutputSinkProvider};
pub use sinks::{ClipboardSink, FileSink, PasteSink};
