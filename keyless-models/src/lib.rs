//! keyless-models
//!
//! HTTP/HF helpers and model metadata/cache utilities for keyless. Manages model
//! downloads, local cache discovery, and metadata persistence without depending on hf-hub.
//!
//! Architecture: `reqwest (blocking + async) → HF Hub REST API → local cache (~/.cache/keyless/models/) → typed JSON metadata`
//!
//! ## Key Features
//!
//! - **Resumable downloads**: HTTP Range headers with status-aware resume (206 append, 200 restart)
//! - **Progress reporting**: `DownloadEvent` stream (Started, Stage, Progress, Done)
//! - **Cancellation support**: Atomic flag for mid-download abort
//! - **Connection pooling**: Reusable HTTP clients with sane timeouts (10s connect, 60s total)
//! - **Exponential backoff**: Retry logic for transient network failures
//! - **Typed metadata cache**: TTL-based size cache with env overrides
//! - **Local discovery**: Scan for installed models without network
//!
//! ## Submodules
//!
//! - `download`: Model download with progress (`ensure_model_cached`, `DownloadEvent`)
//! - `hf`: HF cache path helpers (`keyless_cache_repo_dir`, `get_local_model_size`, `delete_partial_file`)
//! - `net`: HTTP utilities (auth, URL resolution, retry/backoff, client builders)
//! - `meta`: Sizes TTL cache management (typed `SizesCache` struct)
//! - `catalog`: OpenAI Whisper model catalog
//! - `discover`: Local repo discovery
//! - `workers`: Background workers for size fetching (TUI integration)
//!
//! ## Public API
//!
//! - `ensure_model_cached()`, `DownloadEvent`
//! - `keyless_cache_root()`, `keyless_cache_repo_dir()`, `get_local_model_size()`, `delete_partial_file()`
//! - `list_openai_whisper_models()`, `discover_local_whisper_repos()`
//! - `load_sizes_cache()`, `save_sizes_cache()`, `update_saved_size()`
//!
//! ## Quick Example
//!
//! ```no_run
//! use std::sync::{mpsc::sync_channel, Arc};
//! use std::sync::atomic::{AtomicBool, Ordering};
//! use keyless_models::{ensure_model_cached, DownloadEvent};
//! let (tx, rx) = sync_channel::<DownloadEvent>(32);
//! let cancel = Arc::new(AtomicBool::new(false));
//! std::thread::spawn({
//!     let tx = tx.clone();
//!     let cancel = cancel.clone();
//!     move || ensure_model_cached("openai/whisper-tiny".to_string(), tx, cancel)
//! });
//! while let Ok(evt) = rx.recv() {
//!     match evt { DownloadEvent::Done(_) => break, _ => {} }
//! }
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

/// Model catalog utilities.
pub mod catalog;
/// Local model discovery utilities.
pub mod discover;
pub mod download;
/// HF cache path helpers and local size helpers.
pub mod hf;
/// Typed sizes TTL cache.
pub mod meta;
/// HTTP helpers (timeouts, backoff, URL resolution).
pub mod net;
/// Background workers to emit sizes (TUI integration).
pub mod workers;

// Re-export for convenience
pub use download::{DownloadEvent, ensure_model_cached};
