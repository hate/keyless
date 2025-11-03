//! Sinks (keyless-output::sinks)
//!
//! Output sink implementations for delivering transcribed text:
//! - `clipboard`: `ClipboardSink` (copy to system clipboard)
//! - `file`: `FileSink` (append to a file)
//! - `paste`: `PasteSink` (simulate keystrokes)
//!
//! Public API: `ClipboardSink`, `FileSink`, `PasteSink`

mod clipboard;
mod file;
mod paste;

pub use clipboard::ClipboardSink;
pub use file::FileSink;
pub use paste::PasteSink;
