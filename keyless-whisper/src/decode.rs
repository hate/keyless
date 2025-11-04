//! Token generation and text decoding.
//!
//! This module handles the decoder side of Whisper inference:
//! - Autoregressive token generation using the TextDecoder
//! - Token ID → text conversion using the tokenizer
//! - Filtering of special tokens (SOT, EOT, language tags, etc.)
//! - Quality metrics: no-speech detection, avg log probability, compression ratio
//! - Temperature fallback for difficult audio
//!
//! - Feed full token sequence every step (not last-token-only)
//! - Flush KV cache only on first iteration (`i == 0`)
//! - Use `decoder.final_linear()` for vocab projection
//! - Greedy sampling with repetition guards

use candle_core::{Device, IndexOp, Tensor};
use candle_nn::ops::softmax;

use keyless_core::error::{KeylessError, KeylessResult};

use crate::model::Model;

/// Quality thresholds from OpenAI's Whisper implementation.
/// These are used to determine when to retry with a different temperature.
/// Maximum acceptable compression ratio before triggering fallback.
const COMPRESSION_RATIO_THRESHOLD: f64 = 2.4;
/// Minimum acceptable average log probability before triggering fallback.
const LOGPROB_THRESHOLD: f64 = -1.0;
/// No-speech probability threshold used to accept silence.
const NO_SPEECH_THRESHOLD: f64 = 0.6;

/// Temperature schedule for fallback decoding.
/// Try greedy (0.0) first, then progressively higher temperatures.
const TEMPERATURES: &[f64] = &[0.0, 0.2, 0.4, 0.6, 0.8, 1.0];

/// Result of decoding a single audio segment with quality metrics.
#[derive(Debug, Clone)]
pub struct DecodingResult {
    /// Decoded text.
    pub text: String,
    /// Average log probability across all tokens (quality indicator).
    pub avg_logprob: f64,
    /// Probability that the segment contains no speech (0.0-1.0).
    pub no_speech_prob: f64,
    /// Temperature used for sampling (0.0 = greedy).
    pub temperature: f64,
    /// Text compression ratio (higher = more repetitive/hallucinated).
    pub compression_ratio: f64,
}

/// Check if token generation is stuck in a repetition loop.
///
/// Returns `true` if:
/// - ≥8 of the last 10 tokens are identical, OR
/// - The last 3-gram appears ≥3 times in the recent buffer
fn is_repeating(recent: &[u32]) -> bool {
    // Check for single-token repetition (e.g., "and and and...").
    // If 8+ of last 10 tokens are the same, likely stuck in loop.
    if recent.len() >= 10 {
        let last10 = &recent[recent.len() - 10..];
        let mut max_count = 0usize;
        // Count occurrences of each token in last 10; track max.
        for &t in last10 {
            let c = last10.iter().filter(|&&x| x == t).count();
            if c > max_count {
                max_count = c;
            }
        }
        // Threshold: 8/10 = 80% repetition (highly likely stuck).
        if max_count >= 8 {
            return true;
        }
    }
    // Check for 3-gram repetition (e.g., "thank you thank you thank you").
    // If last 3 tokens appear 3+ times total, likely stuck in pattern.
    if recent.len() >= 12 {
        let tri = &recent[recent.len() - 3..];
        let mut count = 0usize;
        // Count how many times this 3-gram appears in recent buffer.
        for w in recent.windows(3) {
            if w == tri {
                count += 1;
            }
        }
        // Threshold: 3+ occurrences of same 3-gram (repetitive pattern).
        if count >= 3 {
            return true;
        }
    }
    false
}

/// Generate tokens autoregressively using Whisper's TextDecoder with quality metrics.
///
/// This implements temperature-based decoding: greedy (t=0) or sampling (t>0).
/// Computes quality metrics including no-speech probability and average log probability.
///
/// Internal helper used by `decode_with_fallback`.
///
/// # Arguments
/// * `model` - Whisper model (normal or quantized) containing the decoder
/// * `audio_features` - Encoded audio embeddings from the encoder
/// * `language_token` - Optional language token ID to prepend
/// * `temperature` - Sampling temperature (0.0 = greedy, higher = more random)
/// * `device` - Candle device
///
/// # Returns
/// A `DecodingResult` with tokens, text, and quality metrics.
///
/// # Errors
/// Returns KeylessError::Whisper if tensor operations or decoder forward pass fails.
fn decode_with_temperature(
    model: &mut Model,
    tokenizer: &tokenizers::Tokenizer,
    tokens: &crate::model::WhisperTokens,
    audio_features: &Tensor,
    language_token: Option<u32>,
    temperature: f64,
    device: &Device,
) -> KeylessResult<DecodingResult> {
    // Use tokens looked up from tokenizer (IDs vary between .en and multilingual!).
    // Token IDs differ: multilingual uses higher IDs, .en models use lower IDs.
    let sot_token = tokens.sot;
    let eot_token = tokens.eot;
    let transcribe_token = tokens.transcribe;
    let notimestamps_token = tokens.notimestamps;
    // If no_speech token not found, skip no-speech detection (set to NaN).
    // Some tokenizers may not have this token (e.g., older Whisper versions).
    let no_speech_token_opt = tokens.no_speech;

    // Build initial prompt sequence:
    // <|startoftranscript|> [<|language|>] <|transcribe|> <|notimestamps|>
    // Language token is optional (omitted for "auto" detection).
    let mut token_ids = vec![sot_token];
    if let Some(lang_tok) = language_token {
        token_ids.push(lang_tok);
    }
    token_ids.push(transcribe_token);
    token_ids.push(notimestamps_token);

    // Whisper's maximum sequence length (hard limit from model architecture).
    let max_tokens = 224;
    let mut steps = 0usize;
    // Recent token buffer for repetition detection (fixed-size circular buffer).
    let mut recent: Vec<u32> = Vec::with_capacity(64);
    // Accumulate log probabilities for quality metric (average logprob).
    let mut sum_logprob = 0.0f64;
    // Initialize to NaN (only set if no_speech_token exists and is checked).
    let mut no_speech_prob = f64::NAN;

    // Autoregressive generation: predict text one token (word/subword) at a time
    //
    // How it works: We start with a prompt ("<|startoftranscript|> <|transcribe|>...") and predict
    // the next token. Then we append that token and predict again, repeating until we hit the end token.
    //
    // Analogy: Like autocomplete on your phone, but for entire sentences. Each prediction depends
    // on all previous words.
    for i in 0..max_tokens {
        // Feed the FULL token sequence each step (not just the last token).
        // Candle's Whisper decoder manages a KV cache internally. It expects the complete
        // sequence every time, but only computes new values for positions it hasn't seen yet.
        // This is different from some libraries that let you feed just new tokens.
        let token_tensor = Tensor::new(token_ids.as_slice(), device)
            .map_err(|e| KeylessError::Whisper(format!("failed to create token tensor: {}", e)))?
            // unsqueeze(0) adds batch dimension (shape: [seq_len] → [1, seq_len]).
            .unsqueeze(0)
            .map_err(|e| KeylessError::Whisper(format!("failed to unsqueeze: {}", e)))?;

        // Run decoder forward pass.
        // KV cache: The decoder caches intermediate "key" and "value" tensors to avoid recomputing
        // attention for old tokens. We flush it (clear it) only on the first iteration (i == 0),
        // then reuse it for subsequent tokens in this sequence.
        // Think of it like: "I've already processed '<|startoftranscript|>', don't redo that work."
        // i == 0 means flush cache (start fresh for this segment).
        let ys = model
            .decoder_forward(&token_tensor, audio_features, i == 0)
            .map_err(|e| KeylessError::Whisper(format!("decoder forward failed: {}", e)))?;

        // On the first iteration, check if the audio is actually speech or just silence/noise.
        // The decoder outputs a special "<|nospeech|>" token probability. If it's high (>0.6),
        // the audio is probably silent or background noise, so we can skip transcription entirely.
        // This prevents Whisper from hallucinating text like "Thank you for watching" on silence.
        // Steps: Get the decoder's logits (raw scores), convert to probabilities via softmax
        // (squashes scores into 0-1 range that sums to 1), then extract the nospeech token's probability.
        if i == 0
            && let Some(no_speech_token) = no_speech_token_opt
        {
            // Extract logits for first position only (prompt position, before any generated tokens).
            let logits_first = model
                .decoder_final_linear(
                    // ys.i(..1) gets first sequence position (batch index 0, position 0).
                    &ys.i(..1)
                        .map_err(|e| KeylessError::Whisper(format!("ys.i: {}", e)))?,
                )
                .map_err(|e| KeylessError::Whisper(format!("decoder_final_linear: {}", e)))?
                // Chain indexing: batch[0].seq[0] to get single token's logits.
                .i(0)
                .map_err(|e| KeylessError::Whisper(format!("i(0): {}", e)))?
                .i(0)
                .map_err(|e| KeylessError::Whisper(format!("i(0): {}", e)))?;

            // Softmax: Converts raw scores into probabilities (all sum to 1.0).
            // Dimension 0 is vocab dimension (one probability per token).
            let probs = softmax(&logits_first, 0)
                .map_err(|e| KeylessError::Whisper(format!("softmax: {}", e)))?;
            // Extract probability for no_speech token (vocab index).
            no_speech_prob = probs
                .i(no_speech_token as usize)
                .map_err(|e| KeylessError::Whisper(format!("i(token): {}", e)))?
                .to_scalar::<f32>()
                .map_err(|e| KeylessError::Whisper(format!("to_scalar: {}", e)))?
                as f64;
        }

        // Get logits for the last position using decoder_final_linear (Candle API).
        // Last position contains prediction for next token (autoregressive generation).
        let (_, seq_len, _) = ys
            .dims3()
            .map_err(|e| KeylessError::Whisper(format!("ys dims3: {}", e)))?;
        let logits = model
            .decoder_final_linear(
                // Extract last sequence position (seq_len - 1) for next-token prediction.
                &ys.i((.., seq_len - 1..))
                    .map_err(|e| KeylessError::Whisper(format!("ys narrow: {}", e)))?,
            )
            .map_err(|e| KeylessError::Whisper(format!("final_linear: {}", e)))?
            // Chain indexing: batch[0].seq[0] to get single token's logits (vocab-sized vector).
            .i(0)
            .map_err(|e| KeylessError::Whisper(format!("batch index: {}", e)))?
            .i(0)
            .map_err(|e| KeylessError::Whisper(format!("seq index: {}", e)))?;

        // Pick the next token using temperature-controlled sampling.
        // Temperature: Controls randomness vs. determinism in text generation.
        // - temperature = 0.0 (greedy): Always pick the most likely token (deterministic, fast)
        // - temperature > 0.0: Add randomness by "flattening" the probability distribution
        // Why use temperature: Sometimes greedy decoding gets stuck or produces low-quality output.
        // Adding slight randomness (temp=0.2) can help the model "unstuck" itself and try alternatives.
        // How we use it: Try greedy first. If quality metrics are poor, retry with higher temps.
        let next_token: u32 = if temperature > 0.0 {
            // Temperature sampling: divide logits by temperature before softmax.
            // Effect: Higher temp → flatter distribution → more random choices.
            // Example: temp=2.0 means unlikely tokens more likely to be picked.
            let scaled_logits = (&logits / temperature)
                .map_err(|e| KeylessError::Whisper(format!("scale logits: {}", e)))?;
            let probs = softmax(&scaled_logits, 0)
                .map_err(|e| KeylessError::Whisper(format!("softmax: {}", e)))?;
            let probs_v: Vec<f32> = probs
                .to_vec1()
                .map_err(|e| KeylessError::Whisper(format!("probs to_vec1: {}", e)))?;

            // Even with temperature, we still pick the most likely token (greedy from scaled distribution).
            // This is "temperature-greedy" (not true sampling); still deterministic but with temp effect.
            probs_v
                .iter()
                .enumerate()
                .max_by(|(_, u), (_, v)| u.total_cmp(v))
                .map(|(i, _)| i as u32)
                .ok_or_else(|| KeylessError::Whisper("no max in probs".to_string()))?
        } else {
            // Greedy decoding (temperature = 0): pick the single most likely token.
            let probs = softmax(&logits, 0)
                .map_err(|e| KeylessError::Whisper(format!("softmax: {}", e)))?;
            let probs_v: Vec<f32> = probs
                .to_vec1()
                .map_err(|e| KeylessError::Whisper(format!("probs to_vec1: {}", e)))?;
            // Argmax: find index with highest probability (vocab index = token ID).
            probs_v
                .iter()
                .enumerate()
                .max_by(|(_, u), (_, v)| u.total_cmp(v))
                .map(|(i, _)| i as u32)
                .ok_or_else(|| KeylessError::Whisper("no max in probs".to_string()))?
        };

        // Calculate log probability of the selected token (for quality metric).
        // Always use untempered logits for logprob (measures true model confidence, not temp-scaled).
        // This ensures quality metrics are comparable across temperatures.
        let prob = if temperature == 0.0 {
            // Recompute softmax on untempered logits (even though we computed it above for token selection).
            // This keeps code paths consistent and ensures we use the same logits for both selection and metric.
            let probs = softmax(&logits, 0)
                .map_err(|e| KeylessError::Whisper(format!("softmax for logprob: {}", e)))?;
            probs
                .i(next_token as usize)
                .map_err(|e| KeylessError::Whisper(format!("i(next_token): {}", e)))?
                .to_scalar::<f32>()
                .map_err(|e| KeylessError::Whisper(format!("prob to_scalar: {}", e)))?
                as f64
        } else {
            // Temperature>0 path: use untempered logits for logprob (not scaled logits).
            // This measures actual model confidence, not temperature-adjusted confidence.
            let probs = softmax(&logits, 0)
                .map_err(|e| KeylessError::Whisper(format!("softmax for logprob: {}", e)))?;
            probs
                .i(next_token as usize)
                .map_err(|e| KeylessError::Whisper(format!("i(next_token): {}", e)))?
                .to_scalar::<f32>()
                .map_err(|e| KeylessError::Whisper(format!("prob to_scalar: {}", e)))?
                as f64
        };
        // Accumulate log probabilities (ln(prob) because we want log-space for averaging).
        sum_logprob += prob.ln();

        // Append generated token to sequence (for next iteration's full-sequence feed).
        token_ids.push(next_token);
        // Update recent buffer (FIFO: oldest removed when full).
        recent.push(next_token);
        // Maintain fixed-size buffer (64 tokens) for repetition detection (drop oldest).
        if recent.len() > 64 {
            recent.remove(0);
        }
        steps += 1;

        // Stop if we hit end-of-text token (model signals completion).
        if next_token == eot_token {
            break;
        }

        // Early stop guards to prevent infinite loops.
        // Problem: On noisy or very short audio, the model can get stuck repeating the same tokens
        // (e.g., "And and and and...") or never emit an end-of-text token.
        // Solution 1: Hard cap at 160 tokens (reasonable length for speech; ~30 seconds at ~5 tokens/sec).
        if steps >= 160 {
            break;
        }
        // Solution 2: Repetition detection (if we see the same pattern too many times, stop).
        if is_repeating(&recent) {
            break;
        }
    }

    // Decode tokens to text (filters special tokens, converts IDs to string).
    let text = decode_tokens(tokenizer, &token_ids, eot_token)?;

    // Calculate average log probability (quality metric: higher = more confident predictions).
    // max(1.0) prevents division by zero if steps == 0 (shouldn't happen, but defensive).
    let avg_logprob = sum_logprob / (steps as f64).max(1.0);

    // Calculate compression ratio (text length / token count).
    // Higher ratio indicates repetitive/hallucinated output (same tokens → more text = compression).
    // Example: "thank you thank you" has high ratio (few tokens, many chars).
    // max(1.0) prevents division by zero if token_ids is empty (shouldn't happen, but defensive).
    let compression_ratio = text.len() as f64 / (token_ids.len() as f64).max(1.0);

    Ok(DecodingResult {
        text,
        avg_logprob,
        no_speech_prob,
        temperature,
        compression_ratio,
    })
}

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

/// Decode token IDs to text using the tokenizer.
///
/// Filters out Whisper's special tokens (SOT, EOT, language tags, task tokens, etc.)
/// and returns only the transcribed text.
///
/// # Arguments
/// * `tokenizer` - Loaded tokenizer
/// * `tokens` - Generated token IDs
/// * `eot_token` - EOT token ID (varies between .en and multilingual)
///
/// # Returns
/// Decoded text string with special tokens removed.
///
/// # Errors
/// Returns KeylessError::Whisper if tokenizer decoding fails.
fn decode_tokens(
    tokenizer: &tokenizers::Tokenizer,
    tokens: &[u32],
    eot_token: u32,
) -> KeylessResult<String> {
    // Whisper special token ranges (IDs vary between .en and multilingual models):
    // Multilingual: EOT=50257, SOT=50258, langs=50259-50358, tasks/timestamps=50359+
    // .en models:   EOT=50256, SOT=50257, langs=50258-50357, tasks/timestamps=50358+
    // Filter all tokens >= EOT to remove special tokens (EOT is first special token).
    let special_token_threshold = eot_token;

    // Filter special tokens (keep only regular text tokens; IDs < EOT are text tokens).
    let text_tokens: Vec<u32> = tokens
        .iter()
        .filter(|&&t| t < special_token_threshold)
        .copied()
        .collect();

    // Decode using tokenizer (convert token IDs to UTF-8 string).
    // skip_special_tokens=true ensures any remaining special tokens are omitted (defensive).
    let text = tokenizer
        .decode(&text_tokens, true)
        .map_err(|e| KeylessError::Whisper(format!("tokenizer decode failed: {}", e)))?;

    Ok(text)
}
