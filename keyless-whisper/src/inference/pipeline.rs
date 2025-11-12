//! Inference pipeline helpers for voiced-mask windowing and stitching.
//! Exposes functions used by higher-level APIs to build units and merge text.
use std::borrow::Cow;
use std::ops::Range;

use candle_core::{Device, Tensor};
use candle_transformers::models::whisper::N_SAMPLES as MODEL_SAMPLES;
use tracing::info;

use keyless_core::error::{KeylessError, KeylessResult};

use crate::decode::{self, DecodingResult};
use crate::inference::params::{
    FINAL_OVERLAP_SAMPLES, MEL_FRAMES_FACTOR, SILENCE_DROP_MIN_RMS, SILENCE_DROP_NO_SPEECH_PROB,
    WHISPER_SAMPLE_RATE, WHISPER_WINDOW_SAMPLES,
};
use crate::inference::util::compute_rms;
use crate::inference::voiced::{VoicedCfg, build_voiced_spans, units_from_spans};
use crate::model::WhisperModel;
use crate::preprocessing;

/// Core inference: PCM → mel → encoder → decode_with_fallback
pub(crate) fn run_inference_impl(
    components: &mut WhisperModel,
    pcm: &[f32],
    device: &Device,
) -> KeylessResult<DecodingResult> {
    let mel = preprocessing::pcm_to_mel(components.model.config(), pcm, &components.mel_filters);

    let num_mel_bins = components.model.config().num_mel_bins;
    let time_steps = mel.len() / num_mel_bins;
    tracing::debug!(
        pcm_samples = pcm.len(),
        mel_frames_initial = time_steps,
        "prepared mel spectrogram"
    );
    // Cap mel frames to model context (30s = 2 * max_source_positions).
    // Candle's mel builder can produce >3000 frames due to internal padding logic.
    // Encoder applies stride-2, so seq_len = time_steps / 2 must be <= max_source_positions.
    let max_ctx = components.model.config().max_source_positions;
    let max_mel_frames = max_ctx.saturating_mul(MEL_FRAMES_FACTOR);
    let (mel_data, frames) = if time_steps > max_mel_frames {
        let truncate_to = num_mel_bins * max_mel_frames;
        let mut mel_owned = mel;
        mel_owned.truncate(truncate_to);
        (mel_owned, max_mel_frames)
    } else {
        (mel, time_steps)
    };

    let mel_tensor = Tensor::from_vec(mel_data, (1, num_mel_bins, frames), device)
        .map_err(|e| KeylessError::Whisper(format!("failed to create mel tensor: {}", e)))?;
    decode_mel_tensor(components, mel_tensor, device)
}

/// Decode a precomputed mel spectrogram tensor to text with fallback decoding.
pub(crate) fn decode_mel_tensor(
    components: &mut WhisperModel,
    mel_tensor: Tensor,
    device: &Device,
) -> KeylessResult<DecodingResult> {
    let audio_features = components
        .model
        .encoder_forward(&mel_tensor, true)
        .map_err(|e| KeylessError::Whisper(format!("encoder forward failed: {}", e)))?;

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

    let result = decode::decode_with_fallback(
        &mut components.model,
        &components.tokenizer,
        &components.tokens,
        &audio_features,
        language_token,
        device,
    )?;

    tracing::debug!(
        avg_logprob = result.avg_logprob,
        compression_ratio = result.compression_ratio,
        no_speech_prob = result.no_speech_prob,
        temperature = result.temperature,
        "inference quality metrics"
    );

    Ok(result)
}

/// Decode voiced units without no-speech gating and with overlap deduplication.
pub fn decode_voiced_units(
    components: &mut WhisperModel,
    pcm: &[f32],
    units: &[Range<usize>],
    device: &Device,
) -> KeylessResult<String> {
    if pcm.is_empty() || units.is_empty() {
        return Ok(String::new());
    }

    let mut out = String::new();
    for unit in units {
        if unit.is_empty() || unit.start >= pcm.len() {
            continue;
        }
        let end = unit.end.min(pcm.len());
        if end <= unit.start {
            continue;
        }
        let slice = &pcm[unit.start..end];
        let slice_rms = compute_rms(slice);
        tracing::debug!(
            unit_start = unit.start,
            unit_end = end,
            samples = slice.len(),
            rms = slice_rms,
            "prepared voiced unit for final decode"
        );
        let pcm_aligned: Cow<[f32]> = if slice.len() > MODEL_SAMPLES {
            Cow::Owned(slice[..MODEL_SAMPLES].to_vec())
        } else {
            Cow::Borrowed(slice)
        };
        let result = run_inference_impl(components, pcm_aligned.as_ref(), device)?;
        tracing::debug!(
            unit_start = unit.start,
            unit_end = end,
            avg_logprob = result.avg_logprob,
            compression_ratio = result.compression_ratio,
            no_speech_prob = result.no_speech_prob,
            temperature = result.temperature,
            preview = %result.text,
            "voiced unit decode metrics"
        );
        // Drop unit if very low RMS and high no-speech probability.
        if result.no_speech_prob.is_finite()
            && result.no_speech_prob > SILENCE_DROP_NO_SPEECH_PROB
            && slice_rms < SILENCE_DROP_MIN_RMS
        {
            info!(
                unit_start = unit.start,
                unit_end = end,
                rms = slice_rms,
                no_speech_prob = result.no_speech_prob,
                "dropping unit as likely silence (high no-speech, very low RMS)"
            );
            continue;
        }
        let mut text = result.text;
        if text.is_empty() {
            info!(
                unit_start = unit.start,
                unit_end = end,
                "voiced unit decoded empty text"
            );
            continue;
        }
        normalize_whitespace(&mut text);
        if text.is_empty() {
            info!(
                unit_start = unit.start,
                unit_end = end,
                "voiced unit text collapsed to empty after normalization"
            );
            continue;
        }

        if !out.is_empty() {
            remove_overlap(&out, &mut text);
        }

        if text.is_empty() {
            info!(
                unit_start = unit.start,
                unit_end = end,
                "voiced unit text fully removed after overlap adjustment"
            );
            continue;
        }

        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&text);
        info!(
            unit_start = unit.start,
            unit_end = end,
            appended = %text,
            current_final = %out
        );
    }

    let final_text = out.trim().to_string();
    info!(len = final_text.len(), "voiced decode complete");
    Ok(final_text)
}

/// Collapse all whitespace in-place to single spaces for clean stitching.
fn normalize_whitespace(text: &mut String) {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    text.clear();
    text.push_str(&normalized);
}

/// Remove repeated word overlap at the boundary between `existing` and `next`.
/// Keeps the earliest occurrence and trims the duplicated prefix from `next`.
pub fn remove_overlap(existing: &str, next: &mut String) {
    const MAX_OVERLAP_WORDS: usize = 6;
    if existing.is_empty() {
        return;
    }

    let existing_words: Vec<&str> = existing.split_whitespace().collect();
    let next_words: Vec<&str> = next.split_whitespace().collect();

    for n in (1..=MAX_OVERLAP_WORDS).rev() {
        if existing_words.len() < n || next_words.len() < n {
            continue;
        }
        if existing_words[existing_words.len() - n..] == next_words[..n] {
            let remainder = next_words[n..].join(" ");
            next.clear();
            next.push_str(remainder.trim_start());
            return;
        }
    }
}

/// Public helper for batch/offline transcription using voiced-mask single pass.
pub fn run_inference_voiced(
    components: &mut WhisperModel,
    pcm_16k: &[f32],
    device: &Device,
) -> KeylessResult<String> {
    if pcm_16k.is_empty() {
        return Ok(String::new());
    }
    let spans = build_voiced_spans(pcm_16k, WHISPER_SAMPLE_RATE, &VoicedCfg::default());
    let spans = if spans.is_empty() {
        std::iter::once(0..pcm_16k.len()).collect::<Vec<_>>()
    } else {
        spans
    };
    let units = units_from_spans(&spans, WHISPER_WINDOW_SAMPLES, FINAL_OVERLAP_SAMPLES);
    decode_voiced_units(components, pcm_16k, &units, device)
}
