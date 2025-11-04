use std::path::PathBuf;

use candle_core::Device;
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config};
use tracing::{info, warn};

use crate::{PhaseState, WhisperLoadPhase};
use keyless_core::error::{KeylessError, KeylessResult};

use super::files::{detect_quantized, locate_normal_files, locate_quantized_files};
use super::types::{Model, WhisperModel, WhisperTokens};

/// Load a Whisper model from Hugging Face or a local directory.
///
/// If the model path is a Hugging Face ID (contains '/'), downloads it automatically.
/// If the model doesn't exist and isn't a valid HF ID, falls back to "openai/whisper-tiny".
///
/// Automatically detects whether to use quantized (GGUF) or normal (safetensors) format.
/// Models are cached in `~/.cache/huggingface/hub/`, so subsequent runs are offline.
///
/// # Arguments
/// * `cfg` - Whisper configuration with model path/ID
/// * `device` - Candle device (CPU, Metal, or CUDA)
///
/// # Returns
/// A `WhisperModel` with the loaded model, tokenizer, and mel filters.
///
/// # Errors
/// Returns KeylessError::Whisper if model loading fails after trying defaults.
pub fn load_whisper_model(
    cfg: &crate::WhisperConfig,
    device: &Device,
) -> KeylessResult<WhisperModel> {
    load_whisper_model_with_progress::<fn(WhisperLoadPhase, PhaseState)>(cfg, device, None)
}

/// Load Whisper model with an optional progress callback.
pub fn load_whisper_model_with_progress<F>(
    cfg: &crate::WhisperConfig,
    device: &Device,
    mut progress: Option<&mut F>,
) -> KeylessResult<WhisperModel>
where
    F: FnMut(WhisperLoadPhase, PhaseState),
{
    let model_id = cfg.model_path.to_string_lossy();
    // HF IDs contain '/' (e.g., "openai/whisper-tiny"); local paths may not.
    let is_hf_id = model_id.contains('/');

    // Determine the actual model source (HF ID vs local path vs fallback).
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ModelSource, PhaseState::Begin);
    }
    let (final_model_id, is_remote) = if is_hf_id {
        // HF ID: will download/cache if needed (handled by keyless_models crate).
        (model_id.to_string(), true)
    } else if cfg.model_path.exists() && cfg.model_path.is_dir() {
        // Local directory: use as-is (no download needed).
        (model_id.to_string(), false)
    } else {
        // Path doesn't exist and isn't an HF ID; use default (graceful fallback).
        // This prevents errors when user specifies invalid path.
        const DEFAULT: &str = keyless_core::config::DEFAULT_MODEL_ID;
        warn!(path = ?cfg.model_path, default = DEFAULT, "model path not found, using default");
        (DEFAULT.to_string(), true)
    };
    info!(model = %final_model_id, "loading whisper model");
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ModelSource, PhaseState::End);
    }

    // Locate files from local cache or provided dir (models are ensured by the TUI).
    // HF models are cached in ~/.cache/huggingface/hub/ by keyless_models crate.
    let model_dir = if is_remote {
        // Resolve HF ID to cache directory (download happens in keyless_models if needed).
        keyless_models::hf::keyless_cache_repo_dir(&final_model_id)
    } else {
        // Use local path directly (no cache resolution needed).
        PathBuf::from(&final_model_id)
    };

    // Detect model format: quantized (GGUF) vs normal (safetensors).
    // Different formats require different loading paths (GGUF vs mmap safetensors).
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::DetectFormat, PhaseState::Begin);
    }
    // Check for .gguf extension (quantized) vs .safetensors (normal).
    let is_quantized = detect_quantized(&model_dir);
    info!(format = %if is_quantized {"quantized (.gguf)"} else {"normal (.safetensors)"}, "detected model format");
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::DetectFormat, PhaseState::End);
    }

    let (config_path, weights_path, tokenizer_path) = if is_quantized {
        locate_quantized_files(&model_dir)?
    } else {
        locate_normal_files(&model_dir)?
    };

    // Parse config.json (contains model architecture parameters: layers, dimensions, etc.).
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ReadConfig, PhaseState::Begin);
    }
    info!(path = %config_path.display(), "reading config.json");
    // Read entire file as string (config.json is small, < 1KB).
    let config_json = std::fs::read_to_string(&config_path)
        .map_err(|e| KeylessError::Whisper(format!("failed to read config.json: {}", e)))?;
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ReadConfig, PhaseState::End);
    }
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ParseConfig, PhaseState::Begin);
    }
    // Parse JSON into Config struct (deserializes architecture parameters).
    let config: Config = serde_json::from_str(&config_json)
        .map_err(|e| KeylessError::Whisper(format!("failed to parse config.json: {}", e)))?;
    info!("parsed config");
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ParseConfig, PhaseState::End);
    }

    // Load model based on format (different paths for quantized vs normal).
    let model = if is_quantized {
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::MapWeights, PhaseState::Begin);
        }
        info!(path = %weights_path.display(), "mapping quantized weights");
        // GGUF loader: reads quantized weights (int8/int4) and maps to device.
        // Quantized weights are smaller (~3-4x) but require dequantization during inference.
        let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(
            &weights_path,
            device,
        )
        .map_err(|e| KeylessError::Whisper(format!("failed to load GGUF weights: {}", e)))?;
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::MapWeights, PhaseState::End);
        }
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::ConstructModel, PhaseState::Begin);
        }
        // Construct quantized model layers (encoder/decoder with quantized weights).
        let model = m::quantized_model::Whisper::load(&vb, config.clone()).map_err(|e| {
            KeylessError::Whisper(format!("failed to construct quantized model: {}", e))
        })?;
        info!("constructed quantized model");
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::ConstructModel, PhaseState::End);
        }
        Model::Quantized(model)
    } else {
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::MapWeights, PhaseState::Begin);
        }
        info!(path = %weights_path.display(), "mapping safetensors weights");
        // MMAP safetensors: memory-maps weights file (zero-copy, faster load, lower RAM).
        // unsafe required because mmap bypasses Rust's safety guarantees (validated by Candle).
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], m::DTYPE, device).map_err(|e| {
                KeylessError::Whisper(format!("failed to load safetensors weights: {}", e))
            })?
        };
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::MapWeights, PhaseState::End);
        }
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::ConstructModel, PhaseState::Begin);
        }
        // Construct normal model layers (encoder/decoder with full-precision weights).
        let model = m::model::Whisper::load(&vb, config.clone()).map_err(|e| {
            KeylessError::Whisper(format!("failed to construct normal model: {}", e))
        })?;
        info!("constructed model");
        if let Some(p) = progress.as_mut() {
            p(WhisperLoadPhase::ConstructModel, PhaseState::End);
        }
        Model::Normal(model)
    };

    // Load tokenizer
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::LoadTokenizer, PhaseState::Begin);
    }
    info!(path = %tokenizer_path.display(), "loading tokenizer");
    let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| KeylessError::Whisper(format!("failed to load tokenizer: {}", e)))?;
    info!("tokenizer loaded");
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::LoadTokenizer, PhaseState::End);
    }

    // Look up special tokens from tokenizer using Candle's string constants.
    // Token IDs differ between .en and multilingual models (shifted by 1), so we MUST look them up.
    // Hardcoding IDs would break compatibility (e.g., multilingual EOT=50257, .en EOT=50256).
    let sot = tokenizer
        .token_to_id(m::SOT_TOKEN)
        .ok_or_else(|| KeylessError::Whisper("SOT token not found in tokenizer".to_string()))?;
    let eot = tokenizer
        .token_to_id(m::EOT_TOKEN)
        .ok_or_else(|| KeylessError::Whisper("EOT token not found in tokenizer".to_string()))?;
    let transcribe = tokenizer.token_to_id(m::TRANSCRIBE_TOKEN).ok_or_else(|| {
        KeylessError::Whisper("transcribe token not found in tokenizer".to_string())
    })?;
    let notimestamps = tokenizer
        .token_to_id(m::NO_TIMESTAMPS_TOKEN)
        .ok_or_else(|| {
            KeylessError::Whisper("notimestamps token not found in tokenizer".to_string())
        })?;

    // Try to find no_speech token (may not exist in all tokenizers).
    // Some models use "<|nospeech|>", others use "<|nospeech|>" or variant; try all candidates.
    let no_speech = m::NO_SPEECH_TOKENS
        .iter()
        .find_map(|token| tokenizer.token_to_id(token));

    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ResolveTokens, PhaseState::Begin);
    }
    let tokens = WhisperTokens {
        sot,
        eot,
        transcribe,
        notimestamps,
        no_speech,
    };
    info!("token IDs resolved");
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::ResolveTokens, PhaseState::End);
    }

    // Resolve language token based on model type (.en vs multilingual).
    let model_id_hint = cfg.model_path.to_string_lossy();
    // Check for ".en" suffix (e.g., "openai/whisper-base.en").
    let is_en_only = model_id_hint.contains(".en");
    let language_token = if is_en_only {
        // .en models: no language token (3-token prompt: SOT + transcribe + notimestamps).
        // English-only models don't support language selection (fixed to English).
        None
    } else {
        // Multilingual models: use specified language, or None for auto-detection.
        // Format: "<|en|>", "<|es|>", etc. (ISO 639-1 language codes).
        cfg.language
            .as_ref()
            .and_then(|lang| tokenizer.token_to_id(&format!("<|{}|>", lang)))
        // Don't default to English! None = auto-detect in inference.rs (slower but more accurate).
    };

    // Generate mel filters
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::BuildMelFilters, PhaseState::Begin);
    }
    info!("generating mel filters");
    let mel_filters = super::mel_filters::generate_mel_filters(&config);
    info!("mel filters generated");
    if let Some(p) = progress.as_mut() {
        p(WhisperLoadPhase::BuildMelFilters, PhaseState::End);
    }

    Ok(WhisperModel {
        model,
        tokenizer,
        mel_filters,
        language_token,
        tokens,
        is_en_only,
    })
}
