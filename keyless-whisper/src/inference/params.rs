//! Shared inference constants for Whisper voiced-mask decoding.
//! Kept as constants for simplicity; can be lifted into a PipelineParams later.

/// Whisper native sample rate.
pub const WHISPER_SAMPLE_RATE: usize = 16_000;
/// Whisper context window in seconds.
pub const WHISPER_WINDOW_S: usize = 30; // 30 seconds
/// Max samples per 30-second context window.
pub const WHISPER_WINDOW_SAMPLES: usize = WHISPER_SAMPLE_RATE * WHISPER_WINDOW_S;

/// Duration (seconds) of each voiced decode unit.
pub const FINAL_UNIT_SECONDS: usize = 10;
/// Samples per voiced decode unit (10 seconds @ 16 kHz).
pub const FINAL_UNIT_SAMPLES: usize = WHISPER_SAMPLE_RATE * FINAL_UNIT_SECONDS;
/// Overlap (0.5 seconds) between voiced decode units to preserve boundary words.
pub const FINAL_OVERLAP_SAMPLES: usize = WHISPER_SAMPLE_RATE / 2;

/// Candle mel spectrograms are stride-2; encoder context is max_source_positions.
pub const MEL_FRAMES_FACTOR: usize = 2;

/// Per-unit silence drop thresholds: high no-speech probability and very low RMS.
pub const SILENCE_DROP_NO_SPEECH_PROB: f64 = 0.85;
/// Minimum RMS below which a unit is considered silence and may be dropped.
pub const SILENCE_DROP_MIN_RMS: f64 = 0.003;

/// Number of tail samples hashed when caching preview units (~128 ms @ 16 kHz).
/// 16000 samples/sec * 0.128 sec = 2048 samples
pub const PREVIEW_HASH_TAIL: usize = 2_048;
