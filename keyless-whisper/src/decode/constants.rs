/// Quality thresholds from OpenAI's Whisper implementation.
/// These are used to determine when to retry with a different temperature.
/// Maximum acceptable compression ratio before triggering fallback.
pub(crate) const COMPRESSION_RATIO_THRESHOLD: f64 = 2.4;
/// Minimum acceptable average log probability before triggering fallback.
pub(crate) const LOGPROB_THRESHOLD: f64 = -1.0;
/// No-speech probability threshold used to accept silence.
pub(crate) const NO_SPEECH_THRESHOLD: f64 = 0.6;

/// Temperature schedule for fallback decoding.
/// Try greedy (0.0) first, then progressively higher temperatures.
pub(crate) const TEMPERATURES: &[f64] = &[0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
