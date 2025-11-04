/// Whisper construction and initialization.
mod construct;
/// Inference thread implementation.
mod inference_thread;
/// Trait implementations (RealtimeTranscriber, Drop).
mod trait_impl;
/// Type definitions (Whisper struct, WhisperCmd, InferReq).
mod types;
/// Worker thread implementation (resampling, accumulation).
mod worker_thread;

pub use types::Whisper;
