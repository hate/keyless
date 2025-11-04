//! CLI/ENV overrides parsing and application.
//!
//! Parses command-line arguments and environment variables into `Overrides`
//! that can be applied to config for the current run (non-persistent).

use std::path::PathBuf;
use std::str::FromStr;

use crate::Cli;
use keyless_core::config::{
    Config, EQ_ATTACK_MAX, EQ_ATTACK_MIN, EQ_BANDS_MAX, EQ_BANDS_MIN, EQ_DECAY_MAX, EQ_DECAY_MIN,
    EQ_GAMMA_MAX, EQ_GAMMA_MIN, EQ_NOISE_REDUCTION_MAX, EQ_NOISE_REDUCTION_MIN, EQ_WINDOW_DB_MAX,
    EQ_WINDOW_DB_MIN, OutputMode,
};

/// Non-persistent overrides applied on top of saved config for the current run only.
///
/// These are typically parsed from CLI arguments or environment variables and allow
/// users to temporarily override config without modifying the persistent configuration.
#[derive(Clone, Debug, Default)]
pub struct Overrides {
    /// Model override (HF ID or local path).
    pub model: Option<String>,
    /// Language override (ISO 639-1 code) or None for auto.
    pub language: Option<String>,
    /// Sink override (paste|clipboard|file).
    pub sink: Option<String>,
    /// File path override when sink=file.
    pub file: Option<String>,
    /// Input device display name override.
    pub device: Option<String>,
    /// Hotkey preset override (e.g., "control+option").
    pub hotkey: Option<String>,
    /// VAD start dB override.
    pub vad_start: Option<f32>,
    /// VAD stop dB override.
    pub vad_stop: Option<f32>,
    /// VAD min duration (ms) override.
    pub vad_min_duration: Option<u16>,
    /// VAD max silence (ms) override.
    pub vad_max_silence: Option<u16>,
    /// EQ bands override.
    pub eq_bands: Option<u16>,
    /// EQ noise reduction override.
    pub eq_noise_reduction: Option<f32>,
    /// EQ window dB override.
    pub eq_window: Option<f32>,
    /// EQ gamma override.
    pub eq_gamma: Option<f32>,
    /// EQ attack override.
    pub eq_attack: Option<f32>,
    /// EQ decay override.
    pub eq_decay: Option<f32>,
}

/// Apply overrides to config (non-persistent, for current run only).
///
/// Modifies the config in-place according to the provided overrides. Any field
/// set in `ov` will override the corresponding field in `config`.
pub fn apply_overrides(config: &mut Config, ov: &Overrides) {
    if let Some(v) = ov.model.as_ref() {
        // Trim whitespace from model ID/path; reject empty strings (defensive).
        let t = v.trim();
        if !t.is_empty() {
            config.model_path = PathBuf::from(t);
        }
    }
    if let Some(v) = ov.language.as_ref() {
        // Trim whitespace from language code; reject empty strings (None = auto-detect).
        let t = v.trim();
        if !t.is_empty() {
            config.language = Some(t.to_string());
        }
    }
    if let Some(v) = ov.sink.as_ref() {
        // Parse sink string (paste/clipboard/file); ignore parse errors (invalid strings ignored).
        // File mode not set here (handled by file override below); only Paste/Clipboard applied.
        match OutputMode::from_str(v.trim()) {
            Ok(OutputMode::Paste) => config.output_mode = OutputMode::Paste,
            Ok(OutputMode::Clipboard) => config.output_mode = OutputMode::Clipboard,
            Err(_) => {}
            Ok(OutputMode::File(_)) => {}
        }
    }
    if let Some(p) = ov.file.as_ref() {
        // File path override: trim whitespace, reject empty strings.
        // Takes precedence over sink=file parsing above (explicit path provided).
        let t = p.trim();
        if !t.is_empty() {
            config.output_mode = OutputMode::File(PathBuf::from(t));
        }
    }
    if let Some(v) = ov.device.as_ref() {
        // Trim whitespace from device name; reject empty strings (None = use default).
        let t = v.trim();
        if !t.is_empty() {
            config.device_name = Some(t.to_string());
        }
    }
    if let Some(v) = ov.hotkey.as_ref() {
        // Trim whitespace from hotkey string; reject empty strings (prevent invalid config).
        let t = v.trim();
        if !t.is_empty() {
            config.hotkey = t.to_string();
        }
    }
    // VAD overrides: direct assignment (no clamping; user may want extreme values).
    if let Some(f) = ov.vad_start {
        config.vad.start_db = f;
    }
    if let Some(f) = ov.vad_stop {
        config.vad.stop_db = f;
    }
    if let Some(d) = ov.vad_min_duration {
        config.vad.min_duration_ms = d;
    }
    if let Some(s) = ov.vad_max_silence {
        config.vad.max_silence_ms = s;
    }
    // EQ overrides: clamp to valid ranges to prevent invalid audio processing parameters.
    if let Some(b) = ov.eq_bands {
        config.eq.bands = b.clamp(EQ_BANDS_MIN, EQ_BANDS_MAX);
    }
    if let Some(m) = ov.eq_noise_reduction {
        config.eq.noise_reduction = m.clamp(EQ_NOISE_REDUCTION_MIN, EQ_NOISE_REDUCTION_MAX);
    }
    if let Some(w) = ov.eq_window {
        config.eq.window_db = w.clamp(EQ_WINDOW_DB_MIN, EQ_WINDOW_DB_MAX);
    }
    if let Some(g) = ov.eq_gamma {
        config.eq.gamma = g.clamp(EQ_GAMMA_MIN, EQ_GAMMA_MAX);
    }
    if let Some(a) = ov.eq_attack {
        config.eq.attack = a.clamp(EQ_ATTACK_MIN, EQ_ATTACK_MAX);
    }
    if let Some(d) = ov.eq_decay {
        config.eq.decay = d.clamp(EQ_DECAY_MIN, EQ_DECAY_MAX);
    }
    // Validate config after applying all overrides; ignore result (errors logged internally).
    let _ = config.validate();
}

/// Overrides source used by the TUI; constructed in main from CLI/ENV.
#[allow(clippy::module_name_repetitions)]
pub fn overrides_from_env_and_cli(cli: &Cli) -> Overrides {
    // Precedence: CLI > ENV (CLI flags override environment variables).
    // Clone CLI Option<String> to move value into closure (needs owned String).
    let pick_str = |cli_opt: &Option<String>, env_key: &str| -> Option<String> {
        if let Some(v) = cli_opt.clone() {
            Some(v)
        } else {
            // Fallback to ENV var if CLI flag not set; ok() converts Result to Option.
            std::env::var(env_key).ok()
        }
    };

    // Parse f32 from ENV if CLI flag not set; ignore parse errors (invalid strings = None).
    let pick_f32 = |cli_opt: Option<f32>, env_key: &str| -> Option<f32> {
        cli_opt.or_else(|| {
            std::env::var(env_key)
                .ok()
                .and_then(|s| s.parse::<f32>().ok())
        })
    };

    // Parse u16 from ENV if CLI flag not set; ignore parse errors (invalid strings = None).
    let pick_u16 = |cli_opt: Option<u16>, env_key: &str| -> Option<u16> {
        cli_opt.or_else(|| {
            std::env::var(env_key)
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
        })
    };

    Overrides {
        model: pick_str(&cli.model, "KEYLESS_MODEL"),
        language: pick_str(&cli.language, "KEYLESS_LANGUAGE"),
        sink: pick_str(&cli.sink, "KEYLESS_SINK"),
        file: pick_str(&cli.file, "KEYLESS_FILE"),
        device: pick_str(&cli.device, "KEYLESS_DEVICE"),
        hotkey: pick_str(&cli.hotkey, "KEYLESS_HOTKEY"),
        vad_start: pick_f32(cli.vad_start, "KEYLESS_VAD_START_DB"),
        vad_stop: pick_f32(cli.vad_stop, "KEYLESS_VAD_STOP_DB"),
        vad_min_duration: pick_u16(cli.vad_min_duration, "KEYLESS_VAD_MIN_DURATION_MS"),
        vad_max_silence: pick_u16(cli.vad_max_silence, "KEYLESS_VAD_MAX_SILENCE_MS"),
        eq_bands: pick_u16(cli.eq_bands, "KEYLESS_EQ_BANDS"),
        eq_noise_reduction: pick_f32(cli.eq_noise_reduction, "KEYLESS_EQ_NOISE_REDUCTION"),
        eq_window: pick_f32(cli.eq_window, "KEYLESS_EQ_WINDOW_DB"),
        eq_gamma: pick_f32(cli.eq_gamma, "KEYLESS_EQ_GAMMA"),
        eq_attack: pick_f32(cli.eq_attack, "KEYLESS_EQ_ATTACK"),
        eq_decay: pick_f32(cli.eq_decay, "KEYLESS_EQ_DECAY"),
    }
}
