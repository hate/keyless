//! Configuration schema types.

use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::storage::CURRENT_VERSION;
use crate::error::{KeylessError, KeylessResult};

/// Default Whisper model ID.
///
/// Uses multilingual variant for maximum flexibility (auto-detection, language switching).
/// Size: ~75 MB, Speed: Fastest, Languages: All supported by Whisper.
pub const DEFAULT_MODEL_ID: &str = "openai/whisper-tiny";

/// Minimum allowed EQ bands.
pub const EQ_BANDS_MIN: u16 = 16;
/// Maximum allowed EQ bands.
pub const EQ_BANDS_MAX: u16 = 128;
/// Minimum noise reduction threshold.
pub const EQ_NOISE_REDUCTION_MIN: f32 = 0.0;
/// Maximum noise reduction threshold.
pub const EQ_NOISE_REDUCTION_MAX: f32 = 1.0;
/// Minimum EQ window dB scaling.
pub const EQ_WINDOW_DB_MIN: f32 = 10.0;
/// Maximum EQ window dB scaling.
pub const EQ_WINDOW_DB_MAX: f32 = 80.0;
/// Minimum gamma value for perceptual curve.
pub const EQ_GAMMA_MIN: f32 = 0.8;
/// Maximum gamma value for perceptual curve.
pub const EQ_GAMMA_MAX: f32 = 2.0;
/// Minimum attack smoothing factor.
pub const EQ_ATTACK_MIN: f32 = 0.05;
/// Maximum attack smoothing factor.
pub const EQ_ATTACK_MAX: f32 = 1.0;
/// Minimum decay smoothing factor.
pub const EQ_DECAY_MIN: f32 = 0.05;
/// Maximum decay smoothing factor.
pub const EQ_DECAY_MAX: f32 = 1.0;

/// Expert EQ tuning parameters.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EqTuning {
    /// Number of log-spaced EQ bands (16..128).
    pub bands: u16,
    /// Noise reduction threshold (0.0..1.0; 0=noisy, 1=smooth).
    pub noise_reduction: f32,
    /// Window scaling in dB (10..80).
    pub window_db: f32,
    /// Gamma curve shaping (0.8..2.0).
    pub gamma: f32,
    /// Attack smoothing (0.05..1.0).
    pub attack: f32,
    /// Decay smoothing (0.05..1.0).
    pub decay: f32,
}

impl Default for EqTuning {
    fn default() -> Self {
        Self {
            bands: 64,
            noise_reduction: 0.46,
            window_db: 50.0,
            gamma: 1.3,
            attack: 0.35,
            decay: 0.12,
        }
    }
}

impl EqTuning {
    /// Validate bounds for EQ parameters.
    pub fn validate(&self) -> KeylessResult<()> {
        if !(EQ_BANDS_MIN..=EQ_BANDS_MAX).contains(&self.bands) {
            return Err(KeylessError::Config("eq.bands out of range".into()));
        }
        if !(EQ_NOISE_REDUCTION_MIN..=EQ_NOISE_REDUCTION_MAX).contains(&self.noise_reduction) {
            return Err(KeylessError::Config(
                "eq.noise_reduction out of range".into(),
            ));
        }
        if !(EQ_WINDOW_DB_MIN..=EQ_WINDOW_DB_MAX).contains(&self.window_db) {
            return Err(KeylessError::Config("eq.window_db out of range".into()));
        }
        if !(EQ_GAMMA_MIN..=EQ_GAMMA_MAX).contains(&self.gamma) {
            return Err(KeylessError::Config("eq.gamma out of range".into()));
        }
        if !(EQ_ATTACK_MIN..=EQ_ATTACK_MAX).contains(&self.attack) {
            return Err(KeylessError::Config("eq.attack out of range".into()));
        }
        if !(EQ_DECAY_MIN..=EQ_DECAY_MAX).contains(&self.decay) {
            return Err(KeylessError::Config("eq.decay out of range".into()));
        }
        Ok(())
    }
}

/// Output destination for transcribed text.
///
/// This enum determines where final transcription results are sent.
/// - Paste: simulate keystrokes into the active window (most natural UX)
/// - Clipboard: copy to system clipboard (safest fallback)
/// - File: append to a specified file (good for logging/archives)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    /// Simulate keystrokes in the currently focused application.
    /// Requires OS-level accessibility permissions on macOS/Linux.
    Paste,

    /// Copy transcription to the system clipboard.
    /// User must manually paste (Cmd+V or Ctrl+V).
    Clipboard,

    /// Append transcription to a file at the specified path.
    /// Each final result is written on a new line.
    File(PathBuf),
}

impl FromStr for OutputMode {
    type Err = KeylessError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s.trim();
        if v.eq_ignore_ascii_case("paste") {
            Ok(OutputMode::Paste)
        } else if v.eq_ignore_ascii_case("clipboard") {
            Ok(OutputMode::Clipboard)
        } else if v.eq_ignore_ascii_case("file") {
            Err(KeylessError::Config("file sink requires a path".into()))
        } else {
            Err(KeylessError::Config(format!("unknown sink: {}", v)))
        }
    }
}

/// Voice Activity Detection (VAD) configuration.
///
/// VAD determines when to start/stop recording based on audio volume (dB).
/// These thresholds help:
/// - Ignore background noise
/// - Detect speech start/stop automatically
/// - Reduce latency by not processing silence
///
/// Defaults are tuned for typical indoor environments with moderate background noise.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VadThresholds {
    /// dB level above which speech is considered to have started.
    /// Typical range: -50 to -30 dB. Lower = more sensitive.
    pub start_db: f32,

    /// dB level below which speech is considered to have stopped.
    /// Should be lower (more negative) than start_db to avoid flickering.
    pub stop_db: f32,

    /// Minimum duration (ms) to confirm speech start.
    /// Avoids false triggers from brief transient sounds (clicks, taps).
    /// Range: 0-65535ms (u16 max = 65s, plenty for speech)
    pub min_duration_ms: u16,

    /// Maximum silence duration (ms) before stopping.
    /// Longer = more forgiving of pauses mid-sentence; shorter = lower latency.
    /// Range: 0-65535ms (u16 max = 65s, plenty for pauses)
    pub max_silence_ms: u16,
}

impl Default for VadThresholds {
    fn default() -> Self {
        Self {
            start_db: -45.0,      // Moderate sensitivity for typical office/home
            stop_db: -50.0,       // Slightly lower to reduce false stops
            min_duration_ms: 200, // 0.2s minimum to confirm speech
            max_silence_ms: 800,  // 0.8s tolerance for pauses
        }
    }
}

/// Main application configuration.
///
/// This struct holds all user-configurable preferences and is serialized to persistent
/// storage (e.g., Tauri Store, config.json) so config survives app restarts.
///
/// Config can be validated via the `validate()` method before use.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Schema version for future migrations.
    pub version: u32,
    /// Path to the Whisper model file (e.g., ggml-base.en.bin).
    /// The app will fail fast if this file doesn't exist at startup.
    pub model_path: PathBuf,

    /// Language code for transcription (ISO 639-1, e.g., "en", "es", "fr").
    /// `None` = auto-detect language (slower, less accurate).
    /// Whisper performs best with an explicit language hint.
    pub language: Option<String>,

    /// Global hotkey to toggle dictation on/off.
    /// Format: "modifier+key" (e.g., "control+option", "ctrl+shift+d")
    /// Requires OS-level permissions to register global shortcuts.
    pub hotkey: String,

    /// Where to send final transcription results.
    pub output_mode: OutputMode,

    /// Voice Activity Detection thresholds for automatic start/stop.
    /// Future: will be used to gate audio to the transcriber.
    pub vad: VadThresholds,

    /// Optional saved input device name (for UI-selected devices).
    /// Note: we persist a display name for UX; at runtime, if this name is missing
    /// we fall back to the OS default device.
    pub device_name: Option<String>,

    /// Expert EQ tuning parameters (optional, with safe defaults).
    #[serde(default)]
    pub eq: EqTuning,

    /// Total words transcribed across all sessions (persisted).
    #[serde(default)]
    pub lifetime_words: u64,

    /// Total milliseconds spent talking across all sessions (persisted).
    #[serde(default)]
    pub lifetime_talk_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            model_path: default_model_path(),
            language: Some("en".to_string()), // English by default
            hotkey: "control+option".to_string(), // macOS-friendly default
            output_mode: OutputMode::Paste,   // Most natural UX
            vad: VadThresholds::default(),
            device_name: None,
            eq: EqTuning::default(),
            lifetime_words: 0,
            lifetime_talk_ms: 0,
        }
    }
}

impl Config {
    /// Validate config before use.
    ///
    /// Checks:
    /// - File output path parent exists
    /// - Language code is reasonable length
    ///
    /// Does NOT check:
    /// - Model file existence (done separately at startup for better error messages)
    /// - Hotkey validity (platform-specific, checked during registration)
    pub fn validate(&self) -> KeylessResult<()> {
        // If outputting to a file, ensure the parent directory exists
        if let OutputMode::File(path) = &self.output_mode
            && let Some(parent) = path.parent()
            && !parent.exists()
        {
            return Err(KeylessError::InvalidPath(parent.to_path_buf()));
        }

        // Validate language code length (ISO 639-1 = 2 chars, ISO 639-3 = 3 chars)
        if let Some(lang) = &self.language
            && (lang.len() < 2 || lang.len() > 8)
        {
            return Err(KeylessError::Config("invalid language code".into()));
        }

        self.eq.validate()?;

        Ok(())
    }
}

/// Construct the default model identifier.
///
/// Returns a Hugging Face model ID that will be auto-downloaded by the Whisper module
/// on first run. The model is cached in `~/.cache/huggingface/hub/`, so subsequent
/// runs are completely offline.
///
/// Default: "openai/whisper-tiny" (multilingual)
/// - Size: ~75 MB
/// - Speed: Fastest
/// - Language: Multilingual (supports auto-detection)
/// - Accuracy: Good for general use
///
/// The user can override this in Config to use other models:
/// - "openai/whisper-base" (~142 MB, better accuracy, multilingual)
/// - "openai/whisper-small" (~466 MB, best accuracy, multilingual)
/// - "openai/whisper-tiny.en" (~75 MB, English-only)
/// - "openai/whisper-base.en" (~142 MB, English-only)
/// - ...
fn default_model_path() -> PathBuf {
    PathBuf::from(DEFAULT_MODEL_ID)
}
