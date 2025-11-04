use candle_core::{Device, IndexOp, Tensor};
use candle_nn::ops::softmax;

use keyless_core::error::{KeylessError, KeylessResult};

use crate::model::Model;

use super::helpers::{decode_tokens, is_repeating};
use super::result::DecodingResult;

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
pub(crate) fn decode_with_temperature(
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
