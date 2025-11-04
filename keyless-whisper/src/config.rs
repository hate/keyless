use std::path::PathBuf;

/// Loading phase identifiers for Whisper model initialization.
#[derive(Clone, Copy, Debug)]
pub enum WhisperLoadPhase {
    /// Resolving model source (local cache path or remote HF ID).
    ModelSource,
    /// Detecting model format (safetensors vs. gguf quantized).
    DetectFormat,
    /// Reading model configuration (config.json).
    ReadConfig,
    /// Parsing model configuration JSON.
    ParseConfig,
    /// Mapping model weights into memory (mmap or gguf loader).
    MapWeights,
    /// Constructing model layers/graph.
    ConstructModel,
    /// Loading tokenizer JSON.
    LoadTokenizer,
    /// Resolving special token IDs (SOT/EOT/etc.).
    ResolveTokens,
    /// Building mel filter bank.
    BuildMelFilters,
}

impl WhisperLoadPhase {
    /// Return a user-facing label for this phase.
    pub fn as_label(&self) -> &'static str {
        match self {
            WhisperLoadPhase::ModelSource => "locating model",
            WhisperLoadPhase::DetectFormat => "detecting model format",
            WhisperLoadPhase::ReadConfig => "reading config",
            WhisperLoadPhase::ParseConfig => "parsing config",
            WhisperLoadPhase::MapWeights => "mapping weights",
            WhisperLoadPhase::ConstructModel => "constructing model",
            WhisperLoadPhase::LoadTokenizer => "loading tokenizer",
            WhisperLoadPhase::ResolveTokens => "resolving tokens",
            WhisperLoadPhase::BuildMelFilters => "building mel filters",
        }
    }
}

/// Loading phase state (begin/end).
#[derive(Clone, Copy, Debug)]
pub enum PhaseState {
    /// Phase begins.
    Begin,
    /// Phase ends successfully.
    End,
}

/// Configuration for the Whisper transcriber.
///
/// Specifies the model source, language hint, and source audio sample rate.
/// The transcriber will handle model downloading and resampling automatically.
#[derive(Clone, Debug)]
pub struct WhisperConfig {
    /// Model source: either a Hugging Face model ID or a local directory path.
    ///
    /// **Hugging Face model IDs** (auto-downloads if not cached):
    /// - "openai/whisper-tiny" (~75 MB, fastest, multilingual, supports auto-detection)
    /// - "openai/whisper-base" (~142 MB, balanced, multilingual, supports auto-detection)
    /// - "openai/whisper-small" (~466 MB, good accuracy, multilingual)
    /// - "openai/whisper-tiny.en" (~75 MB, fastest, English-only)
    /// - "openai/whisper-base.en" (~142 MB, balanced, English-only)
    /// - ...
    ///
    /// **Local directory paths** (must contain config.json, model.safetensors, tokenizer.json):
    /// - Example: "/Users/name/.cache/keyless/models/whisper-tiny"
    ///
    /// If not specified or if the path doesn't exist, defaults to "openai/whisper-tiny".
    pub model_path: PathBuf,

    /// Language code for transcription (ISO 639-1, e.g., "en", "es", "fr").
    /// Providing a hint significantly improves accuracy vs. auto-detection.
    /// `None` = let Whisper auto-detect (slower and less accurate).
    pub language: Option<String>,

    /// Sample rate of the incoming audio (e.g., 48000, 44100, 16000).
    /// The transcriber will resample to 16 kHz mono if necessary.
    pub source_sample_hz: u32,
}
