//! Config screen input handler.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use keyless_core::config::storage;
use keyless_core::config::{Config, DEFAULT_MODEL_ID, OutputMode};
use keyless_core::options::{LANG_CODES, is_english_only_model, lang_name};

use crate::tui::state::{AppState, SinkChoice};

/// Build Config from AppState (for saving).
pub fn build_config_from_app(app: &AppState) -> Config {
    let output_mode = match app.selections.sink_choice {
        SinkChoice::Paste => OutputMode::Paste,
        SinkChoice::Clipboard => OutputMode::Clipboard,
        // Clone path string since Config::File takes ownership; conversion to PathBuf via Into.
        SinkChoice::File => OutputMode::File(app.selections.file_path.clone().into()),
    };
    // Fallback to default model if index is out of bounds (shouldn't happen but defensive).
    // Clone String since Config needs owned PathBuf; conversion via Into trait.
    let model_path = app
        .models
        .runtime_list
        .get(app.selections.model_idx)
        .cloned()
        .unwrap_or_else(|| DEFAULT_MODEL_ID.to_string())
        .into();

    // Language index mapping:
    // - Multilingual: 0 = "auto", 1+ = LANG_CODES offset by 1
    // - .en models: 0 = English (no "auto" entry), 1+ = LANG_CODES (but locked to English anyway)
    // Borrow model ID string slice; no allocation needed for detection check.
    let selected_model = app
        .models
        .runtime_list
        .get(app.selections.model_idx)
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_MODEL_ID);
    let is_en_model = is_english_only_model(selected_model);

    let language = if is_en_model {
        // .en models: always English (index 0 = English, no "auto")
        Some(String::from("en"))
    } else if app.selections.language_idx == 0 {
        // Multilingual: index 0 = "auto" (no language code = auto-detect).
        None
    } else {
        // Multilingual: index 1+ maps to LANG_CODES with off-by-one offset (idx 1 -> LANG_CODES[0]).
        // Assumes language_idx is in bounds [1, LANG_CODES.len()]; upstream must enforce.
        Some(LANG_CODES[app.selections.language_idx - 1].to_string())
    };

    // Clone device name if available; None if index out of bounds or device_names empty.
    let device_name = app
        .selections
        .device_names
        .get(app.selections.device_idx)
        .cloned();

    // Load stored EQ tuning from config to preserve custom settings.
    // Since AppState no longer tracks eq_tuning, we must retrieve it from storage.
    // Fallback to default if config is missing or invalid (first run).
    let stored_eq = storage::load_config().map(|c| c.eq).unwrap_or_default();

    // Clone owned data for Config; lifetime_words/talk_ms are Copy types (no clone needed).
    Config {
        version: keyless_core::config::storage::CURRENT_VERSION,
        model_path,
        language,
        hotkey: app.hotkey.clone(),
        output_mode,
        vad: app.vad.clone(),
        device_name,
        eq: stored_eq,
        lifetime_words: app.lifetime_words,
        lifetime_talk_ms: app.lifetime_talk_ms,
    }
}

#[derive(Debug)]
/// Action produced by the Config input handler.
pub enum ConfigAction {
    /// Continue rendering.
    Continue,
    /// Exit the application.
    Quit,
    /// Begin pipeline startup.
    StartPipeline,
}

/// Handle input events for the Config screen.
pub fn handle_config_input(
    app: &mut AppState,
    key: &crossterm::event::KeyEvent,
    sink_items: &[&str],
    tx_log: &std::sync::mpsc::SyncSender<String>,
) -> ConfigAction {
    // File edit mode takes priority: intercepts all input until ESC exits.
    // This prevents navigation keys from modifying file path accidentally.
    if app.selections.file_edit {
        match key.code {
            KeyCode::Esc => {
                app.selections.file_edit = false;
            }
            KeyCode::Char(c) => {
                // Filter control characters (tab, newline, etc.) to prevent invalid paths.
                if !c.is_control() {
                    app.selections.file_path.push(c);
                }
            }
            KeyCode::Backspace => {
                // Pop last char; ignore result (returns None if string already empty).
                let _ = app.selections.file_path.pop();
            }
            _ => {}
        }
        return ConfigAction::Continue;
    }

    match key.code {
        // 'd' handled in controller under Expert Mode with confirmation
        KeyCode::Char('q') => {
            // Save config to disk before quitting; ignore save errors (non-blocking).
            // Log messages use try_send (non-blocking) to avoid stalling UI thread.
            let _ = tx_log.try_send("saving config and quitting".to_string());
            let config = build_config_from_app(app);
            let _ = storage::save_config(&config);
            return ConfigAction::Quit;
        }
        KeyCode::Left => {
            // Saturating subtract prevents underflow (wraps to 0 if already at first sink).
            let idx = app.selections.sink_choice.to_index().saturating_sub(1);
            app.selections.sink_choice = SinkChoice::from_index(idx);
            // Re-check permissions when sink changes; paste requires Accessibility on macOS.
            let needs_paste = matches!(app.selections.sink_choice, SinkChoice::Paste);
            app.perm_warnings = keyless_core::permissions::gather_permission_warnings(
                needs_paste,
                !app.selections.device_names.is_empty(),
            );
            let sink_label = match app.selections.sink_choice {
                SinkChoice::Paste => "Paste",
                SinkChoice::Clipboard => "Clipboard",
                SinkChoice::File => "File",
            };
            let _ = tx_log.try_send(format!("sink -> {}", sink_label));
        }
        KeyCode::Right => {
            // Clamp to last valid index to prevent OOB; min ensures we stay within sink_items bounds.
            let idx = (app.selections.sink_choice.to_index() + 1).min(sink_items.len() - 1);
            app.selections.sink_choice = SinkChoice::from_index(idx);
            // Re-check permissions when sink changes; paste requires Accessibility on macOS.
            let needs_paste = matches!(app.selections.sink_choice, SinkChoice::Paste);
            app.perm_warnings = keyless_core::permissions::gather_permission_warnings(
                needs_paste,
                !app.selections.device_names.is_empty(),
            );
            let sink_label = match app.selections.sink_choice {
                SinkChoice::Paste => "Paste",
                SinkChoice::Clipboard => "Clipboard",
                SinkChoice::File => "File",
            };
            let _ = tx_log.try_send(format!("sink -> {}", sink_label));
        }
        KeyCode::Up => {
            // Saturating subtract prevents underflow (wraps to 0 if already at first model).
            app.selections.model_idx = app.selections.model_idx.saturating_sub(1);
            // Borrow string slice for model detection; empty string fallback if OOB.
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            // Auto-adjust language when switching to .en model (forces English, no "auto" option).
            if is_english_only_model(id) {
                // .en models: set to English (index 0 since no "auto" entry)
                // Find "en" in LANG_CODES; fallback to 0 if not found (shouldn't happen).
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
            }
            let _ = tx_log.try_send(format!("model -> {}", id));
        }
        KeyCode::Down => {
            // Saturating_sub(1) handles empty list case (max becomes 0, min clamps idx to 0).
            let max = app.models.runtime_list.len().saturating_sub(1);
            app.selections.model_idx = (app.selections.model_idx + 1).min(max);
            // Borrow string slice for model detection; empty string fallback if OOB.
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            // Auto-adjust language when switching to .en model (forces English, no "auto" option).
            if is_english_only_model(id) {
                // .en models: set to English (index 0 since no "auto" entry)
                // Find "en" in LANG_CODES; fallback to 0 if not found (shouldn't happen).
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
            }
            let _ = tx_log.try_send(format!("model -> {}", id));
        }
        KeyCode::Char('[') => {
            // Borrow model ID to check if navigation should be locked (English-only models).
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            if is_english_only_model(id) {
                // .en models: English only, no navigation
                // Force English index; find "en" in LANG_CODES (fallback to 0 if missing).
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
                let _ = tx_log.try_send("language -> English (locked for .en model)".to_string());
            } else {
                // Saturating subtract prevents underflow (wraps to 0 = "auto").
                app.selections.language_idx = app.selections.language_idx.saturating_sub(1);
                let msg = if app.selections.language_idx == 0 {
                    String::from("language -> auto")
                } else {
                    // Off-by-one offset: idx 1 -> LANG_CODES[0], idx 2 -> LANG_CODES[1], etc.
                    // Empty string fallback if index somehow out of bounds (defensive).
                    let code = LANG_CODES
                        .get(app.selections.language_idx - 1)
                        .copied()
                        .unwrap_or("");
                    format!("language -> {} ({})", code, lang_name(code))
                };
                let _ = tx_log.try_send(msg);
            }
        }
        KeyCode::Char(']') => {
            // Borrow model ID to check if navigation should be locked (English-only models).
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            if is_english_only_model(id) {
                // .en models: English only, no navigation
                // Force English index; find "en" in LANG_CODES (fallback to 0 if missing).
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
                let _ = tx_log.try_send("language -> English (locked for .en model)".to_string());
            } else {
                // Max is LANG_CODES.len() (not len-1) because index 0 = "auto", so valid range is 0..=len.
                // Min clamps to prevent OOB; max value represents last language in LANG_CODES.
                let max = LANG_CODES.len(); // +1 auto; indices 0..=max
                app.selections.language_idx = (app.selections.language_idx + 1).min(max);
                let msg = if app.selections.language_idx == 0 {
                    String::from("language -> auto")
                } else {
                    // Off-by-one offset: idx 1 -> LANG_CODES[0], idx 2 -> LANG_CODES[1], etc.
                    // Empty string fallback if index somehow out of bounds (defensive).
                    let code = LANG_CODES
                        .get(app.selections.language_idx - 1)
                        .copied()
                        .unwrap_or("");
                    format!("language -> {} ({})", code, lang_name(code))
                };
                let _ = tx_log.try_send(msg);
            }
        }
        KeyCode::Char('k') => {
            // Duplicate of '[' handler: alternative keybinding for language navigation.
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            if is_english_only_model(id) {
                // .en models: English only, no navigation
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
                let _ = tx_log.try_send("language -> English (locked for .en model)".to_string());
            } else {
                app.selections.language_idx = app.selections.language_idx.saturating_sub(1);
                let msg = if app.selections.language_idx == 0 {
                    String::from("language -> auto")
                } else {
                    let code = LANG_CODES
                        .get(app.selections.language_idx - 1)
                        .copied()
                        .unwrap_or("");
                    format!("language -> {} ({})", code, lang_name(code))
                };
                let _ = tx_log.try_send(msg);
            }
        }
        KeyCode::Char('l') => {
            // Duplicate of ']' handler: alternative keybinding for language navigation.
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            if is_english_only_model(id) {
                // .en models: English only, no navigation
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
                let _ = tx_log.try_send("language -> English (locked for .en model)".to_string());
            } else {
                let max = LANG_CODES.len();
                app.selections.language_idx = (app.selections.language_idx + 1).min(max);
                let msg = if app.selections.language_idx == 0 {
                    String::from("language -> auto")
                } else {
                    let code = LANG_CODES
                        .get(app.selections.language_idx - 1)
                        .copied()
                        .unwrap_or("");
                    format!("language -> {} ({})", code, lang_name(code))
                };
                let _ = tx_log.try_send(msg);
            }
        }
        KeyCode::Char('e') => {
            // Only enable edit mode when File sink is selected (makes sense contextually).
            if matches!(app.selections.sink_choice, SinkChoice::File) {
                app.selections.file_edit = true;
                let _ = tx_log.try_send("file path edit ON".to_string());
            }
        }
        KeyCode::Char('n') => {
            // Explicit bounds check: only decrement if not already at index 0 (prevents underflow).
            if app.selections.device_idx > 0 {
                app.selections.device_idx -= 1;
            }
            // Clone device name for log message; "(unknown)" fallback if index OOB (defensive).
            let name = app
                .selections
                .device_names
                .get(app.selections.device_idx)
                .cloned()
                .unwrap_or_else(|| "(unknown)".to_string());
            let _ = tx_log.try_send(format!("device -> {}", name));
        }
        KeyCode::Char('m') => {
            // Explicit bounds check: only increment if next index is within bounds (prevents OOB).
            if app.selections.device_idx + 1 < app.selections.device_names.len() {
                app.selections.device_idx += 1;
            }
            // Clone device name for log message; "(unknown)" fallback if index OOB (defensive).
            let name = app
                .selections
                .device_names
                .get(app.selections.device_idx)
                .cloned()
                .unwrap_or_else(|| "(unknown)".to_string());
            let _ = tx_log.try_send(format!("device -> {}", name));
        }
        KeyCode::Char('-') => {
            // Increase start threshold (harder to trigger); no upper bound (f64 can handle large values).
            app.vad.start_db += 1.0;
            let _ = tx_log.try_send(format!("VAD start dB -> {:.1}", app.vad.start_db));
        }
        KeyCode::Char('_') => {
            // Decrease start threshold (easier to trigger); no lower bound (f64 can handle negative).
            app.vad.start_db -= 1.0;
            let _ = tx_log.try_send(format!("VAD start dB -> {:.1}", app.vad.start_db));
        }
        KeyCode::Char('=') => {
            // Increase stop threshold (harder to stop); no upper bound (f64 can handle large values).
            app.vad.stop_db += 1.0;
            let _ = tx_log.try_send(format!("VAD stop dB -> {:.1}", app.vad.stop_db));
        }
        KeyCode::Char('+') => {
            // Decrease stop threshold (easier to stop); no lower bound (f64 can handle negative).
            app.vad.stop_db -= 1.0;
            let _ = tx_log.try_send(format!("VAD stop dB -> {:.1}", app.vad.stop_db));
        }
        KeyCode::Char('1') => {
            // Decrease min duration (shorter clips); saturating_sub prevents underflow (min is 0).
            app.vad.min_duration_ms = app.vad.min_duration_ms.saturating_sub(50);
            let _ = tx_log.try_send(format!(
                "VAD min duration ms -> {}",
                app.vad.min_duration_ms
            ));
        }
        KeyCode::Char('2') => {
            // Increase min duration (longer clips); clamp to 5000ms max to prevent excessive values.
            app.vad.min_duration_ms = app.vad.min_duration_ms.saturating_add(50).min(5000);
            let _ = tx_log.try_send(format!(
                "VAD min duration ms -> {}",
                app.vad.min_duration_ms
            ));
        }
        KeyCode::Char('3') => {
            // Decrease silence hangover (stops faster); saturating_sub prevents underflow (min is 0).
            app.vad.max_silence_ms = app.vad.max_silence_ms.saturating_sub(50);
            let _ = tx_log.try_send(format!(
                "VAD silence hangover ms -> {}",
                app.vad.max_silence_ms
            ));
        }
        KeyCode::Char('4') => {
            // Increase silence hangover (waits longer before stopping); clamp to 5000ms max.
            app.vad.max_silence_ms = app.vad.max_silence_ms.saturating_add(50).min(5000);
            let _ = tx_log.try_send(format!(
                "VAD silence hangover ms -> {}",
                app.vad.max_silence_ms
            ));
        }
        KeyCode::Char('h') => {
            // Toggle between two hotkey presets (control+option <-> control+shift).
            // Always allocates new String; acceptable given infrequent usage.
            app.hotkey = if app.hotkey == "control+option" {
                "control+shift".to_string()
            } else {
                "control+option".to_string()
            };
            let _ = tx_log.try_send(format!("hotkey preset -> {}", app.hotkey));
        }
        KeyCode::Enter => {
            // Start pipeline: transition from Config screen to Dictating screen.
            // Controller will handle the actual startup sequence.
            let _ = tx_log.try_send("starting pipeline...".to_string());
            return ConfigAction::StartPipeline;
        }
        KeyCode::Backspace => {
            // Delete selected model from disk if installed.
            // Clone model ID for file operations; empty string fallback if index OOB.
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .cloned()
                .unwrap_or_default();
            // Check if model is actually installed (exists in discovered list).
            let is_installed = app.models.discovered.iter().any(|m| m == &id);
            if !is_installed {
                let _ = tx_log.try_send(format!("model not installed: {}", id));
                return ConfigAction::Continue;
            }
            // Remove entire model directory (contains all model files); may fail if locked/in-use.
            let dir = keyless_models::hf::keyless_cache_repo_dir(&id);
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => {
                    // Remove from discovered list to sync UI state with filesystem.
                    app.models.discovered.retain(|m| m != &id);
                    let _ = tx_log.try_send(format!("deleted model: {}", id));
                }
                Err(e) => {
                    // Log error but continue; user can retry or delete manually.
                    let _ = tx_log.try_send(format!("failed to delete model {}: {}", id, e));
                }
            }
        }
        _ => {
            // Intercept Ctrl+C to prevent accidental terminal termination; show helpful message.
            // Pattern match on cloned KeyEvent to avoid borrow checker issues.
            if let Event::Key(k) = Event::Key(*key)
                && k.modifiers.contains(KeyModifiers::CONTROL)
                && let KeyCode::Char('c') = k.code
            {
                let _ = tx_log.try_send("Ctrl+C ignored. Press 'q' to quit.".to_string());
            }
        }
    }
    ConfigAction::Continue
}
