//! UI selection state (sink, model, language, device).

use keyless_audio::input::InputDevice;

use super::SinkChoice;

/// User selections in the Config screen.
pub struct Selections {
    pub sink_choice: SinkChoice,
    pub model_idx: usize,
    pub language_idx: usize,
    pub device_idx: usize,
    pub file_path: String,
    pub file_edit: bool,
    pub devices: Vec<InputDevice>,
    pub device_names: Vec<String>,
}

impl Selections {
    pub fn new(
        sink_choice: SinkChoice,
        file_path: String,
        model_idx: usize,
        language_idx: usize,
        devices: Vec<InputDevice>,
        device_names: Vec<String>,
        device_idx: usize,
    ) -> Self {
        Self {
            sink_choice,
            model_idx,
            language_idx,
            device_idx,
            file_path,
            file_edit: false,
            devices,
            device_names,
        }
    }
}

impl SinkChoice {
    /// Convert to UI list index for rendering.
    pub fn to_index(self) -> usize {
        // Fixed mapping: Paste=0, Clipboard=1, File=2 (matches sink_items array order).
        match self {
            SinkChoice::Paste => 0,
            SinkChoice::Clipboard => 1,
            SinkChoice::File => 2,
        }
    }

    /// Convert from UI list index.
    pub fn from_index(idx: usize) -> Self {
        // Default to File for any index >= 2 (defensive fallback for OOB indices).
        match idx {
            0 => SinkChoice::Paste,
            1 => SinkChoice::Clipboard,
            _ => SinkChoice::File,
        }
    }
}
