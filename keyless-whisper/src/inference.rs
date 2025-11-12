//! Whisper inference pipeline.
//!
//! This module ties together audio preprocessing, encoding, and decoding
//! to run end-to-end speech-to-text inference using Candle.
//!
//! Pipeline:
//! ```text
//! PCM audio → Mel spectrogram → AudioEncoder → Features → TextDecoder (loop) → Tokens → Text
//! ```
//!
//! Windowing:
//! - Whisper is trained on 30s context; longer inputs are split into ≤30s windows
//! - `run_inference_voiced`: voiced-mask driven windowing for complete transcripts

pub mod params;
pub mod pipeline;
pub mod util;
pub mod voiced;
pub use crate::inference::params::{
    FINAL_OVERLAP_SAMPLES, FINAL_UNIT_SAMPLES, FINAL_UNIT_SECONDS, MEL_FRAMES_FACTOR,
    PREVIEW_HASH_TAIL, SILENCE_DROP_MIN_RMS, SILENCE_DROP_NO_SPEECH_PROB, WHISPER_SAMPLE_RATE,
    WHISPER_WINDOW_S, WHISPER_WINDOW_SAMPLES,
};
pub use crate::inference::pipeline::{decode_voiced_units, remove_overlap, run_inference_voiced};
pub use crate::inference::voiced::units_from_spans;
