//! Main TUI application state.

use super::{ModelCatalog, Overlays, RuntimeFeedback, Selections};
use keyless_audio::input::list_input_devices;
use keyless_core::config::{EqTuning, OutputMode, VadThresholds, storage};

use crate::overrides::{Overrides, apply_overrides};

/// Current screen in the TUI.
#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    /// Configuration screen (model/sink/language/device/VAD selection).
    Config,
    /// Live dictating screen (visualizer, logs, status).
    Dictating,
}

/// Selected output sink (UI representation).
#[derive(Clone, Copy, PartialEq)]
pub enum SinkChoice {
    /// Paste mode (keystroke simulation).
    Paste,
    /// Clipboard mode (copy to system clipboard).
    Clipboard,
    /// File mode (append to file).
    File,
}

/// All state needed to render the TUI and run the live pipeline.
pub struct AppState {
    /// Current screen (Config or Dictating).
    pub screen: Screen,
    /// User selections (sink, model, language, device, file path).
    pub selections: Selections,
    /// VAD thresholds (start/stop dB, timing).
    pub vad: VadThresholds,
    /// EQ tuning parameters (bands, noise reduction, gamma, attack/decay).
    pub eq_tuning: EqTuning,
    /// Runtime feedback (logs, spectrum, VAD state, preview text).
    pub feedback: RuntimeFeedback,
    /// Model catalog (runtime list, discovered, sizes).
    pub models: ModelCatalog,
    /// Overlays (download/loading, expert mode, error messages).
    pub overlays: Overlays,
    /// Permission warnings (macOS Accessibility, microphone).
    pub perm_warnings: Vec<String>,
    /// Active hotkey (e.g., "control+option").
    pub hotkey: String,
    /// Persisted lifetime words counter (accumulates across runs).
    pub lifetime_words: u64,
    /// Persisted milliseconds spent talking across runs.
    pub lifetime_talk_ms: u64,
}

impl AppState {
    /// Create a new AppState by loading config from disk and applying overrides.
    pub fn new(overrides: Option<&Overrides>) -> Self {
        // 1) Hydrate config from disk, then apply non-persistent overrides for the current run
        let mut defaults = storage::load_config().unwrap_or_default();
        if let Some(ov) = overrides {
            apply_overrides(&mut defaults, ov);
        }

        // 2) Derive initial UI selections from config
        let sink_choice = match defaults.output_mode {
            OutputMode::Paste => SinkChoice::Paste,
            OutputMode::Clipboard => SinkChoice::Clipboard,
            OutputMode::File(_) => SinkChoice::File,
        };
        let mut file_path = String::from("/tmp/keyless.txt");
        if let OutputMode::File(p) = &defaults.output_mode {
            file_path = p.display().to_string();
        }

        // Model/language indices will be hydrated in init.rs after catalog is populated
        // Start at 0; init.rs will update to match saved config
        let model_idx = 0;
        let language_idx = 0;

        // 3) Enumerate devices once at startup; store display names for the UI
        let devices = list_input_devices().unwrap_or_default();
        let device_names: Vec<String> = devices
            .iter()
            .map(|d| d.name().unwrap_or_else(|_| "(unknown)".to_string()))
            .collect();
        let device_idx = {
            // Prefer the saved device by name → else fallback to OS default → else index 0
            if let Some(cfg) = defaults.device_name.as_ref() {
                if let Some(pos) = device_names.iter().position(|d| d == cfg) {
                    pos
                } else if let Ok(def) = keyless_audio::input::default_input_device() {
                    if let Ok(name) = def.name() {
                        device_names.iter().position(|d| d == &name).unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else if let Ok(def) = keyless_audio::input::default_input_device() {
                if let Ok(name) = def.name() {
                    device_names.iter().position(|d| d == &name).unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        };

        let selections = Selections::new(
            sink_choice,
            file_path,
            model_idx,
            language_idx,
            devices,
            device_names.clone(),
            device_idx,
        );

        let vad = VadThresholds {
            start_db: defaults.vad.start_db,
            stop_db: defaults.vad.stop_db,
            min_duration_ms: defaults.vad.min_duration_ms,
            max_silence_ms: defaults.vad.max_silence_ms,
        };

        let needs_paste = matches!(sink_choice, SinkChoice::Paste);
        let perm_warnings = keyless_core::permissions::gather_permission_warnings(
            needs_paste,
            !device_names.is_empty(),
        );

        Self {
            screen: Screen::Config,
            selections,
            vad,
            eq_tuning: defaults.eq,
            feedback: RuntimeFeedback::new(),
            models: ModelCatalog::new(),
            overlays: Overlays::new(),
            perm_warnings,
            hotkey: defaults.hotkey,
            lifetime_words: defaults.lifetime_words,
            lifetime_talk_ms: defaults.lifetime_talk_ms,
        }
    }

    /// Convenience: check if currently downloading.
    pub fn is_downloading(&self) -> bool {
        self.overlays.download.is_some()
    }

    /// Convenience: get current download model name.
    pub fn downloading_model(&self) -> Option<&str> {
        self.overlays.download.as_ref().map(|d| d.model.as_str())
    }

    /// Convenience: check if expert mode is open.
    pub fn expert_mode(&self) -> bool {
        self.overlays.expert.is_some()
    }
}
