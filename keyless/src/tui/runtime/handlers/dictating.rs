//! Dictating screen input handler.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use keyless_core::config::storage;

use crate::tui::runtime::PipelineHandles;
use crate::tui::runtime::handlers::build_config_from_app;
use crate::tui::state::AppState;

#[derive(Debug)]
/// Action produced by the Dictating input handler.
pub enum DictatingAction {
    /// Continue rendering.
    Continue,
    /// Exit the application.
    Quit,
    /// Return to Config screen.
    ReturnToConfig,
}

/// Handle input events for the Dictating screen.
pub fn handle_dictating_input(
    app: &mut AppState,
    key: &crossterm::event::KeyEvent,
    _pipeline: Option<&mut PipelineHandles>,
    ptt_stop: &Arc<AtomicBool>,
    ptt_enabled: &Arc<AtomicBool>,
    preset_mode: &Arc<AtomicU8>,
    tx_log: &std::sync::mpsc::SyncSender<String>,
) -> DictatingAction {
    match key.code {
        crossterm::event::KeyCode::Char('q') => {
            let _ = tx_log.try_send("quitting".to_string());
            ptt_stop.store(true, Ordering::Relaxed);
            return DictatingAction::Quit;
        }
        crossterm::event::KeyCode::Char('h') => {
            app.hotkey = if app.hotkey == "control+option" {
                "control+shift".to_string()
            } else {
                "control+option".to_string()
            };
            if app.hotkey.contains("shift") {
                preset_mode.store(1, Ordering::Relaxed);
            } else {
                preset_mode.store(0, Ordering::Relaxed);
            }
            let config = build_config_from_app(app);
            if config.validate().is_ok() {
                let _ = storage::save_config(&config);
                let _ = tx_log.try_send(format!("hotkey set to {}", app.hotkey));
            }
        }
        crossterm::event::KeyCode::Esc => {
            let _ = tx_log.try_send("returning to config".to_string());
            ptt_enabled.store(false, Ordering::Relaxed);
            return DictatingAction::ReturnToConfig;
        }
        _ => {}
    }
    DictatingAction::Continue
}
