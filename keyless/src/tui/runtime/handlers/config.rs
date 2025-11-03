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
        SinkChoice::File => OutputMode::File(app.selections.file_path.clone().into()),
    };
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
        // Multilingual: index 0 = "auto"
        None
    } else {
        // Multilingual: index 1+ maps to LANG_CODES
        Some(LANG_CODES[app.selections.language_idx - 1].to_string())
    };

    let device_name = app
        .selections
        .device_names
        .get(app.selections.device_idx)
        .cloned();

    Config {
        version: keyless_core::config::storage::CURRENT_VERSION,
        model_path,
        language,
        hotkey: app.hotkey.clone(),
        output_mode,
        vad: app.vad.clone(),
        device_name,
        eq: app.eq_tuning.clone(),
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
    // Handle edit mode first - ESC exits, characters append to path
    if app.selections.file_edit {
        match key.code {
            KeyCode::Esc => {
                app.selections.file_edit = false;
            }
            KeyCode::Char(c) => {
                if !c.is_control() {
                    app.selections.file_path.push(c);
                }
            }
            KeyCode::Backspace => {
                let _ = app.selections.file_path.pop();
            }
            _ => {}
        }
        return ConfigAction::Continue;
    }

    match key.code {
        // 'd' handled in controller under Expert Mode with confirmation
        KeyCode::Char('q') => {
            // save + quit
            let _ = tx_log.try_send("saving config and quitting".to_string());
            let config = build_config_from_app(app);
            let _ = storage::save_config(&config);
            return ConfigAction::Quit;
        }
        KeyCode::Left => {
            let idx = app.selections.sink_choice.to_index().saturating_sub(1);
            app.selections.sink_choice = SinkChoice::from_index(idx);
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
            let idx = (app.selections.sink_choice.to_index() + 1).min(sink_items.len() - 1);
            app.selections.sink_choice = SinkChoice::from_index(idx);
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
            app.selections.model_idx = app.selections.model_idx.saturating_sub(1);
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            if is_english_only_model(id) {
                // .en models: set to English (index 0 since no "auto" entry)
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
            }
            let _ = tx_log.try_send(format!("model -> {}", id));
        }
        KeyCode::Down => {
            let max = app.models.runtime_list.len().saturating_sub(1);
            app.selections.model_idx = (app.selections.model_idx + 1).min(max);
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            if is_english_only_model(id) {
                // .en models: set to English (index 0 since no "auto" entry)
                let en_idx = LANG_CODES.iter().position(|&c| c == "en").unwrap_or(0);
                app.selections.language_idx = en_idx;
            }
            let _ = tx_log.try_send(format!("model -> {}", id));
        }
        KeyCode::Char('[') => {
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
        KeyCode::Char(']') => {
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
                let max = LANG_CODES.len(); // +1 auto; indices 0..=max
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
        KeyCode::Char('k') => {
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
            if matches!(app.selections.sink_choice, SinkChoice::File) {
                app.selections.file_edit = true;
                let _ = tx_log.try_send("file path edit ON".to_string());
            }
        }
        KeyCode::Char('n') => {
            if app.selections.device_idx > 0 {
                app.selections.device_idx -= 1;
            }
            let name = app
                .selections
                .device_names
                .get(app.selections.device_idx)
                .cloned()
                .unwrap_or_else(|| "(unknown)".to_string());
            let _ = tx_log.try_send(format!("device -> {}", name));
        }
        KeyCode::Char('m') => {
            if app.selections.device_idx + 1 < app.selections.device_names.len() {
                app.selections.device_idx += 1;
            }
            let name = app
                .selections
                .device_names
                .get(app.selections.device_idx)
                .cloned()
                .unwrap_or_else(|| "(unknown)".to_string());
            let _ = tx_log.try_send(format!("device -> {}", name));
        }
        KeyCode::Char('-') => {
            app.vad.start_db += 1.0;
            let _ = tx_log.try_send(format!("VAD start dB -> {:.1}", app.vad.start_db));
        }
        KeyCode::Char('_') => {
            app.vad.start_db -= 1.0;
            let _ = tx_log.try_send(format!("VAD start dB -> {:.1}", app.vad.start_db));
        }
        KeyCode::Char('=') => {
            app.vad.stop_db += 1.0;
            let _ = tx_log.try_send(format!("VAD stop dB -> {:.1}", app.vad.stop_db));
        }
        KeyCode::Char('+') => {
            app.vad.stop_db -= 1.0;
            let _ = tx_log.try_send(format!("VAD stop dB -> {:.1}", app.vad.stop_db));
        }
        KeyCode::Char('1') => {
            app.vad.min_duration_ms = app.vad.min_duration_ms.saturating_sub(50);
            let _ = tx_log.try_send(format!(
                "VAD min duration ms -> {}",
                app.vad.min_duration_ms
            ));
        }
        KeyCode::Char('2') => {
            app.vad.min_duration_ms = app.vad.min_duration_ms.saturating_add(50).min(5000);
            let _ = tx_log.try_send(format!(
                "VAD min duration ms -> {}",
                app.vad.min_duration_ms
            ));
        }
        KeyCode::Char('3') => {
            app.vad.max_silence_ms = app.vad.max_silence_ms.saturating_sub(50);
            let _ = tx_log.try_send(format!(
                "VAD silence hangover ms -> {}",
                app.vad.max_silence_ms
            ));
        }
        KeyCode::Char('4') => {
            app.vad.max_silence_ms = app.vad.max_silence_ms.saturating_add(50).min(5000);
            let _ = tx_log.try_send(format!(
                "VAD silence hangover ms -> {}",
                app.vad.max_silence_ms
            ));
        }
        KeyCode::Char('h') => {
            app.hotkey = if app.hotkey == "control+option" {
                "control+shift".to_string()
            } else {
                "control+option".to_string()
            };
            let _ = tx_log.try_send(format!("hotkey preset -> {}", app.hotkey));
        }
        KeyCode::Enter => {
            let _ = tx_log.try_send("starting pipeline...".to_string());
            return ConfigAction::StartPipeline;
        }
        KeyCode::Backspace => {
            // Delete selected model from disk if installed
            let id = app
                .models
                .runtime_list
                .get(app.selections.model_idx)
                .cloned()
                .unwrap_or_default();
            let is_installed = app.models.discovered.iter().any(|m| m == &id);
            if !is_installed {
                let _ = tx_log.try_send(format!("model not installed: {}", id));
                return ConfigAction::Continue;
            }
            let dir = keyless_models::hf::keyless_cache_repo_dir(&id);
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => {
                    app.models.discovered.retain(|m| m != &id);
                    let _ = tx_log.try_send(format!("deleted model: {}", id));
                }
                Err(e) => {
                    let _ = tx_log.try_send(format!("failed to delete model {}: {}", id, e));
                }
            }
        }
        _ => {
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
