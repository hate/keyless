use keyless_core::error::{KeylessError, KeylessResult};

/// Check if token generation is stuck in a repetition loop.
///
/// Returns `true` if:
/// - ≥8 of the last 10 tokens are identical, OR
/// - The last 3-gram appears ≥3 times in the recent buffer
pub(crate) fn is_repeating(recent: &[u32]) -> bool {
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
pub(crate) fn decode_tokens(
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
