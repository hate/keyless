//! Runtime workers (audio, whisper) used by the pipeline.
//!
//! - `audio`: CPAL-based audio capture start/stop helpers
//! - `whisper`: Transcriber startup and handle wrappers
/// Audio capture worker helpers.
pub mod audio;
/// Whisper transcriber worker helpers.
pub mod whisper;
