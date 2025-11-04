use candle_core::{Device, Tensor};

use keyless_core::error::{KeylessError, KeylessResult};

use crate::model::Model;

use super::constants::{
    COMPRESSION_RATIO_THRESHOLD, LOGPROB_THRESHOLD, NO_SPEECH_THRESHOLD, TEMPERATURES,
};
use super::result::DecodingResult;
use super::temperature::decode_with_temperature;

/// Decode with temperature fallback for robust transcription.
///
/// Tries multiple temperatures (0.0, 0.2, 0.4, ..., 1.0) and returns the best result
/// based on quality metrics. Stops early if a good transcription is found.
///
/// # Arguments
/// * `model` - Whisper model
/// * `tokenizer` - Tokenizer
/// * `audio_features` - Encoded audio
/// * `language_token` - Optional language token
/// * `device` - Candle device
///
/// # Returns
/// Best `DecodingResult` across all temperatures.
pub fn decode_with_fallback(
    model: &mut Model,
    tokenizer: &tokenizers::Tokenizer,
    tokens_cfg: &crate::model::WhisperTokens,
    audio_features: &Tensor,
    language_token: Option<u32>,
    device: &Device,
) -> KeylessResult<DecodingResult> {
    // Try temperatures in order (greedy first, then progressively more random).
    for (i, &temp) in TEMPERATURES.iter().enumerate() {
        let result = decode_with_temperature(
            model,
            tokenizer,
            tokens_cfg,
            audio_features,
            language_token,
            temp,
            device,
        );

        // On last temperature, return whatever we got (no more fallback options).
        if i == TEMPERATURES.len() - 1 {
            return result;
        }

        match result {
            Ok(dr) => {
                // Check quality metrics to decide if we need fallback.
                // Compression ratio > 2.4: likely repetitive/hallucinated (too many repeats).
                // Avg logprob < -1.0: low confidence (model uncertain about predictions).
                let needs_fallback = dr.compression_ratio > COMPRESSION_RATIO_THRESHOLD
                    || dr.avg_logprob < LOGPROB_THRESHOLD;

                // If quality is good OR it's silent, use this result (no need to retry).
                // Silent audio (no_speech_prob > 0.6) is acceptable even with poor metrics.
                if !needs_fallback || dr.no_speech_prob > NO_SPEECH_THRESHOLD {
                    return Ok(dr);
                }
                // Otherwise, try next temperature (higher temp may help unstuck generation).
            }
            Err(e) => {
                // On error, try next temperature (transient tensor error, not quality issue).
                tracing::debug!(error = %e, temp, "decode failed, trying next temperature");
                continue;
            }
        }
    }
    Err(KeylessError::Whisper(
        "all temperatures failed (unreachable)".to_string(),
    ))
}
