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
//! - `run_inference_window`: single window (≤30s)
//! - `run_inference_full`: walk multiple windows and concatenate text

use candle_core::{Device, Tensor};

use keyless_core::error::{KeylessError, KeylessResult};

use crate::decode;
use crate::model::WhisperModel;
use crate::preprocessing;

/// Whisper native sample rate.
pub const WHISPER_SAMPLE_RATE: usize = 16_000;
/// Whisper context window in seconds.
pub const WHISPER_WINDOW_S: usize = 30; // 30 seconds
/// Max samples per window.
pub const WHISPER_WINDOW_SAMPLES: usize = WHISPER_SAMPLE_RATE * WHISPER_WINDOW_S;

/// Run Whisper inference on a buffer of PCM audio (internal helper).
///
/// This function executes the complete Whisper pipeline:
/// 1. Convert PCM to mel spectrogram (preprocessing module)
/// 2. Run encoder to get audio embeddings
/// 3. Run decoder autoregressively to generate tokens (decode module)
/// 4. Decode tokens to text using the tokenizer (decode module)
///
/// # Arguments
/// * `components` - Loaded Whisper model, tokenizer, and mel filters
/// * `pcm` - Audio samples at 16 kHz mono, f32 in [-1.0, 1.0]
/// * `device` - Candle device (for tensor operations)
///
/// # Returns
/// Transcribed text as a String.
///
/// # Errors
/// Returns KeylessError::Whisper if any stage of inference fails.
fn run_inference(
    components: &mut WhisperModel,
    pcm: &[f32],
    device: &Device,
) -> KeylessResult<String> {
    // Step 1: Convert PCM to mel spectrogram
    let mel = preprocessing::pcm_to_mel(components.model.config(), pcm, &components.mel_filters);

    // Step 2: Create mel tensor (batch_size=1, num_mel_bins, time_steps)
    let mel_len = mel.len();
    let num_mel_bins = components.model.config().num_mel_bins;
    let time_steps = mel_len / num_mel_bins;

    let mel_tensor = Tensor::from_vec(mel, (1, num_mel_bins, time_steps), device)
        .map_err(|e| KeylessError::Whisper(format!("failed to create mel tensor: {}", e)))?;

    // Step 3: Run encoder to get audio features (once, not twice!)
    let audio_features = components
        .model
        .encoder_forward(&mel_tensor, true) // true = flush KV cache
        .map_err(|e| KeylessError::Whisper(format!("encoder forward failed: {}", e)))?;

    // Step 4: Auto-detect language from features if not specified
    // Note: .en models don't support language tokens, so skip auto-detection for them
    let language_token = if components.language_token.is_none() && !components.is_en_only {
        tracing::debug!("auto-detecting language");
        Some(decode::detect_language_from_features(
            &mut components.model,
            &components.tokenizer,
            &components.tokens,
            &audio_features,
            device,
        )?)
    } else {
        components.language_token
    };

    // Step 5: Decode with temperature fallback for robust transcription
    let result = decode::decode_with_fallback(
        &mut components.model,
        &components.tokenizer,
        &components.tokens,
        &audio_features,
        language_token,
        device,
    )?;

    // Step 6: Skip silent segments based on no-speech probability
    // Note: Use higher threshold (0.65) for .en models as they seem to have slightly different calibration
    let no_speech_threshold = if components.is_en_only { 0.65 } else { 0.6 };
    if result.no_speech_prob > no_speech_threshold {
        tracing::debug!(
            no_speech_prob = result.no_speech_prob,
            threshold = no_speech_threshold,
            "skipping silent segment"
        );
        return Ok(String::new());
    }

    tracing::debug!(
        avg_logprob = result.avg_logprob,
        compression_ratio = result.compression_ratio,
        no_speech_prob = result.no_speech_prob,
        temperature = result.temperature,
        "inference quality metrics"
    );

    Ok(result.text)
}

/// Run Whisper inference on a single window (≤ 30s) of 16 kHz PCM.
///
/// If `pcm_16k` contains more than one window, the excess is ignored (first window used).
///
/// # Performance Note
/// Takes `&mut WhisperModel` to avoid cloning the model (which would copy GB of weights).
/// Safe because WhisperModel is owned by a single inference thread.
pub fn run_inference_window(
    components: &mut WhisperModel,
    pcm_16k: &[f32],
    device: &Device,
) -> KeylessResult<String> {
    let end = pcm_16k.len().min(WHISPER_WINDOW_SAMPLES);
    run_inference(components, &pcm_16k[..end], device)
}

/// Run Whisper inference over an arbitrary length of 16 kHz PCM by walking 30s windows.
///
/// Concatenates the decoded text for each window with a space.
///
/// # Performance Note
/// Takes `&mut WhisperModel` to avoid cloning the model (which would copy GB of weights).
/// Safe because WhisperModel is owned by a single inference thread.
pub fn run_inference_full(
    components: &mut WhisperModel,
    pcm_16k: &[f32],
    device: &Device,
) -> KeylessResult<String> {
    if pcm_16k.is_empty() {
        return Ok(String::new());
    }

    let mut out = String::new();
    let mut start = 0usize;
    let total = pcm_16k.len();
    while start < total {
        let end = (start + WHISPER_WINDOW_SAMPLES).min(total);
        let window = &pcm_16k[start..end];
        let t0 = std::time::Instant::now();
        let text = run_inference(components, window, device)?;
        let dt = t0.elapsed().as_millis();
        tracing::debug!(
            samples = window.len(),
            window_start = start,
            window_end = end,
            duration_ms = dt,
            "window inference completed"
        );
        if !text.trim().is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(text.trim());
        }
        if end == total {
            break;
        }
        start = end;
    }
    Ok(out)
}
