//! Main TUI application state.

use super::{ModelCatalog, Overlays, RuntimeFeedback, Selections};
use keyless_audio::input::list_input_devices;
use keyless_core::config::{OutputMode, VadThresholds, storage};

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
        // Load config from disk; use defaults if file missing/invalid (allows first-run).
        // Overrides are non-persistent (e.g., CLI flags) and applied on top of saved config.
        let mut defaults = storage::load_config().unwrap_or_default();
        if let Some(ov) = overrides {
            apply_overrides(&mut defaults, ov);
        }

        // Derive sink choice from config output mode; File path extracted separately below.
        let sink_choice = match defaults.output_mode {
            OutputMode::Paste => SinkChoice::Paste,
            OutputMode::Clipboard => SinkChoice::Clipboard,
            OutputMode::File(_) => SinkChoice::File,
        };
        // Default file path if not specified in config; updated if File mode has a path.
        let mut file_path = String::from("/tmp/keyless.txt");
        if let OutputMode::File(p) = &defaults.output_mode {
            // Convert PathBuf to String for UI display; assumes UTF-8 path encoding.
            file_path = p.display().to_string();
        }

        // Model/language indices start at 0; init.rs will hydrate them after catalog discovery.
        // Cannot set here because runtime_list isn't populated until init runs.
        let model_idx = 0;
        let language_idx = 0;

        // Enumerate devices once at startup (OS call); empty vec if enumeration fails.
        // Device names are cloned for UI display; "(unknown)" fallback if name() fails.
        let devices = list_input_devices().unwrap_or_default();
        let device_names: Vec<String> = devices
            .iter()
            .map(|d| d.name().unwrap_or_else(|_| "(unknown)".to_string()))
            .collect();
        let device_idx = {
            // Three-tier fallback: saved device name → OS default → index 0.
            // Position search finds index by name match; falls back to 0 if not found.
            if let Some(cfg) = defaults.device_name.as_ref() {
                // Try saved device name first (most preferred).
                if let Some(pos) = device_names.iter().position(|d| d == cfg) {
                    pos
                } else if let Ok(def) = keyless_audio::input::default_input_device() {
                    // Saved device not found; try OS default by name.
                    if let Ok(name) = def.name() {
                        device_names.iter().position(|d| d == &name).unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else if let Ok(def) = keyless_audio::input::default_input_device() {
                // No saved device; use OS default by name.
                if let Ok(name) = def.name() {
                    device_names.iter().position(|d| d == &name).unwrap_or(0)
                } else {
                    0
                }
            } else {
                // No OS default available; use first device (index 0).
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

        // Copy VAD thresholds from config (Copy types, no clone needed).
        let vad = VadThresholds {
            start_db: defaults.vad.start_db,
            stop_db: defaults.vad.stop_db,
            min_duration_ms: defaults.vad.min_duration_ms,
            max_silence_ms: defaults.vad.max_silence_ms,
        };

        // Check permissions based on sink choice (paste requires Accessibility on macOS).
        // Empty device_names indicates no mic detected (permission issue or hardware missing).
        let needs_paste = matches!(sink_choice, SinkChoice::Paste);
        let perm_warnings = keyless_core::permissions::gather_permission_warnings(
            needs_paste,
            !device_names.is_empty(),
        );

        Self {
            screen: Screen::Config,
            selections,
            vad,
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
