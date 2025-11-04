use candle_core::Tensor;
use candle_transformers::models::whisper::Config;

/// Whisper model backend: either normal (safetensors) or quantized (GGUF).
///
/// This enum provides a unified interface for both model types, dispatching
/// encoder and decoder calls to the appropriate implementation.
#[derive(Clone)]
pub enum Model {
    /// Standard Whisper model loaded from safetensors weights.
    Normal(candle_transformers::models::whisper::model::Whisper),
    /// Quantized Whisper model loaded from GGUF weights (3-4x faster, ~1% accuracy loss).
    Quantized(candle_transformers::models::whisper::quantized_model::Whisper),
}

impl Model {
    /// Get the model configuration.
    pub fn config(&self) -> &Config {
        // Dispatch to underlying model (both variants expose config with same structure).
        match self {
            Self::Normal(m) => &m.config,
            Self::Quantized(m) => &m.config,
        }
    }

    /// Run the audio encoder on mel spectrogram features.
    ///
    /// # Arguments
    /// * `x` - Mel spectrogram tensor (batch, mel_bins, time)
    /// * `flush` - Whether to flush the KV cache (true for new sequences)
    pub fn encoder_forward(
        &mut self,
        x: &Tensor,
        flush: bool,
    ) -> Result<Tensor, candle_core::Error> {
        // Dispatch to appropriate encoder (normal vs quantized have different implementations).
        // Both share same API, so enum dispatch provides unified interface.
        match self {
            Self::Normal(m) => m.encoder.forward(x, flush),
            Self::Quantized(m) => m.encoder.forward(x, flush),
        }
    }

    /// Run the text decoder autoregressively.
    ///
    /// # Arguments
    /// * `x` - Token IDs tensor (batch, seq_len)
    /// * `xa` - Audio features from encoder (batch, time, features)
    /// * `flush` - Whether to flush the KV cache (true on first iteration)
    pub fn decoder_forward(
        &mut self,
        x: &Tensor,
        xa: &Tensor,
        flush: bool,
    ) -> Result<Tensor, candle_core::Error> {
        // Dispatch to appropriate decoder (normal vs quantized have different implementations).
        // Both share same API, so enum dispatch provides unified interface.
        match self {
            Self::Normal(m) => m.decoder.forward(x, xa, flush),
            Self::Quantized(m) => m.decoder.forward(x, xa, flush),
        }
    }

    /// Apply final linear projection to get vocabulary logits.
    pub fn decoder_final_linear(&self, x: &Tensor) -> Result<Tensor, candle_core::Error> {
        // Dispatch to appropriate final_linear (normal vs quantized have different implementations).
        // Both share same API, so enum dispatch provides unified interface.
        match self {
            Self::Normal(m) => m.decoder.final_linear(x),
            Self::Quantized(m) => m.decoder.final_linear(x),
        }
    }
}

/// Special token IDs for Whisper decoding.
///
/// Token IDs vary between .en and multilingual models (shifted by 1),
/// so we look them up from the tokenizer instead of hardcoding.
#[derive(Debug, Clone)]
pub struct WhisperTokens {
    /// <|startoftranscript|> token id.
    pub sot: u32,
    /// <|endoftext|> token id.
    pub eot: u32,
    /// <|transcribe|> token id.
    pub transcribe: u32,
    /// <|notimestamps|> token id.
    pub notimestamps: u32,
    /// Optional <|nospeech|> token id (not present in all models).
    pub no_speech: Option<u32>,
}

/// Components needed for Whisper inference.
///
/// This struct bundles the model, tokenizer, and mel filters together
/// so the inference pipeline has everything it needs.
pub struct WhisperModel {
    /// Loaded Whisper model (normal or quantized).
    pub model: Model,
    /// Tokenizer used for text decoding.
    pub tokenizer: tokenizers::Tokenizer,
    /// Mel filter bank coefficients.
    pub mel_filters: Vec<f32>,
    /// Optional language token for multilingual models.
    pub language_token: Option<u32>,
    /// Special token IDs (dynamically looked up from tokenizer).
    pub tokens: WhisperTokens,
    /// Whether this is an English-only `.en` model.
    pub is_en_only: bool,
}
