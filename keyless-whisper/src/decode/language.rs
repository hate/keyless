use candle_core::{Device, IndexOp, Tensor};
use candle_nn::ops::softmax;

use keyless_core::error::{KeylessError, KeylessResult};

use crate::model::Model;

/// Detect the language from pre-computed audio features.
///
/// Examines decoder output to determine which language the audio contains.
/// Use this instead of running the encoder separately for language detection.
///
/// # Arguments
/// * `model` - Whisper model
/// * `tokenizer` - Tokenizer for decoding language tokens
/// * `tokens_cfg` - Token IDs (vary between .en and multilingual)
/// * `audio_features` - Pre-computed audio features from encoder
/// * `device` - Candle device
///
/// # Returns
/// Language token ID
pub fn detect_language_from_features(
    model: &mut Model,
    tokenizer: &tokenizers::Tokenizer,
    tokens_cfg: &crate::model::WhisperTokens,
    audio_features: &Tensor,
    device: &Device,
) -> KeylessResult<u32> {
    // Language tokens: <|en|>, <|es|>, etc. (Whisper supports ~100 languages).
    // Find the range by looking up first language token (English is always first).
    let first_lang = tokenizer
        .token_to_id("<|en|>")
        .ok_or_else(|| KeylessError::Whisper("English token not found".to_string()))?;
    // Assume 100 language tokens (standard Whisper; languages are consecutive IDs).
    // Range: first_lang..first_lang+100 covers all language tokens.
    let language_token_ids: Vec<u32> = (first_lang..first_lang + 100).collect();

    // Create initial prompt with just SOT token (language detection uses minimal prompt).
    let tokens = Tensor::new(&[[tokens_cfg.sot]], device)
        .map_err(|e| KeylessError::Whisper(format!("create sot tensor: {}", e)))?;

    // Run one decoder step (SOT → language prediction; decoder outputs language probabilities).
    // true = flush KV cache (fresh start for language detection).
    let ys = model
        .decoder_forward(&tokens, audio_features, true)
        .map_err(|e| KeylessError::Whisper(format!("decoder forward: {}", e)))?;

    // Get logits for language tokens (vocab projection from decoder output).
    // Extract first (and only) position: batch[0].seq[0].
    let logits = model
        .decoder_final_linear(
            &ys.i(..1)
                .map_err(|e| KeylessError::Whisper(format!("ys.i: {}", e)))?,
        )
        .map_err(|e| KeylessError::Whisper(format!("final_linear: {}", e)))?
        .i(0)
        .map_err(|e| KeylessError::Whisper(format!("i(0): {}", e)))?
        .i(0)
        .map_err(|e| KeylessError::Whisper(format!("i(0): {}", e)))?;

    // Get probabilities for each language (softmax converts logits to probabilities).
    let probs =
        softmax(&logits, 0).map_err(|e| KeylessError::Whisper(format!("softmax: {}", e)))?;
    let probs_v: Vec<f32> = probs
        .to_vec1()
        .map_err(|e| KeylessError::Whisper(format!("probs to_vec1: {}", e)))?;

    // Find the language with highest probability (argmax over language token IDs).
    let mut max_prob = 0.0f32;
    // Default to English (first language token) if no valid probability found.
    let mut detected_lang_id = first_lang;

    // Iterate over language token IDs; track highest probability.
    for &lang_id in &language_token_ids {
        // Check if lang_id is valid vocab index and probability exceeds current max.
        if let Some(&p) = probs_v.get(lang_id as usize)
            && p > max_prob
        {
            max_prob = p;
            detected_lang_id = lang_id;
        }
    }

    // Log the detected language if possible
    if let Some(token_str) = tokenizer.id_to_token(detected_lang_id) {
        tracing::debug!(language = %token_str, prob = max_prob, "detected language");
    }

    Ok(detected_lang_id)
}
