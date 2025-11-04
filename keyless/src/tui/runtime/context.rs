//! Runtime context: channels, flags, and handles for the TUI event loop.

use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::overrides::Overrides;
use crate::tui::state::AppState;

use super::{
    CatalogRuntime, ChannelReceivers, Channels, DownloadContext, PipelineHandles, PttContext,
};

/// Bundled runtime state for the TUI event loop.
pub struct RuntimeContext {
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub app: AppState,
    pub channels: Channels,
    pub receivers: ChannelReceivers,
    pub ptt: PttContext,
    pub pipeline: Option<PipelineHandles>,
    pub download: Option<DownloadContext>,
    pub catalog: CatalogRuntime,
    pub last_tick: std::time::Instant,
    pub sink_items: &'static [&'static str],
    // Cached runtime_list refs to avoid per-frame allocation
    pub runtime_list_cache: Vec<String>,
    // Startup events receiver for phased startup
    pub startup_rx: Option<std::sync::mpsc::Receiver<keyless_runtime::pipeline::StartupEvent>>,
    pub startup_model_cfg: Option<(
        keyless_core::config::Config,
        keyless_core::config::EqTuning,
        keyless_core::config::VadThresholds,
        Option<keyless_audio::input::InputDevice>,
    )>,
    /// Ignore terminal keypresses for a brief window after pasting Finals
    pub paste_guard_until: Option<std::time::Instant>,
}

impl RuntimeContext {
    /// Build runtime context with channels, flags, and terminal setup.
    pub fn build(overrides: Option<&Overrides>) -> io::Result<Self> {
        // Install panic hook to restore terminal state on crash (prevents terminal corruption).
        // Hook disables raw mode and exits alternate screen so shell remains usable.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = crossterm::execute!(
                io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::cursor::Show
            );
            prev_hook(info);
        }));

        // Enable raw mode: disables line buffering and echo for immediate keypress handling.
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        // Enter alternate screen: provides clean canvas for TUI, hides scrollback.
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        // Clear screen to ensure clean initial state (removes any existing content).
        terminal.clear()?;

        let app = AppState::new(overrides);

        // Create channels: bidirectional communication between UI and background workers.
        let (channels, receivers) = Channels::new();

        // Shared PTT flags: atomic flags shared between UI thread and hotkey listener thread.
        let hold_flag = Arc::new(AtomicBool::new(false));
        // Detect shift modifier from hotkey string; preset_mode 1 = coarse adjustment.
        let preset_mode = Arc::new(std::sync::atomic::AtomicU8::new(
            if app.hotkey.contains("shift") { 1 } else { 0 },
        ));
        let ptt_stop = Arc::new(AtomicBool::new(false));
        let ptt_enabled = Arc::new(AtomicBool::new(false));

        // Route tracing logs into the TUI log pane for unified logging.
        keyless_logging::init_channel(channels.ui.tx_log.clone());

        // Spawn global hotkey listener; continue even if spawn fails (user can restart).
        // On failure, spawn a no-op thread to avoid Option handling elsewhere.
        let ptt_handle = super::ptt::spawn_listener(
            Arc::clone(&preset_mode),
            Arc::clone(&ptt_enabled),
            Arc::clone(&ptt_stop),
            channels.control.tx_hold.clone(),
            channels.control.tx_ptt_evt.clone(),
        )
        .unwrap_or_else(|e| {
            let _ = channels
                .ui
                .tx_log
                .try_send(format!("PTT listener failed: {}", e));
            std::thread::spawn(|| {})
        });
        let _ = channels
            .ui
            .tx_log
            .try_send("global hotkey listener started".to_string());

        let ptt = PttContext::new(hold_flag, preset_mode, ptt_stop, ptt_enabled, ptt_handle);

        Ok(Self {
            terminal,
            app,
            channels,
            receivers,
            ptt,
            pipeline: None,
            download: None,
            catalog: CatalogRuntime::new(),
            last_tick: std::time::Instant::now(),
            sink_items: &["paste", "clipboard", "file"],
            runtime_list_cache: Vec::new(),
            startup_rx: None,
            startup_model_cfg: None,
            paste_guard_until: None,
        })
    }

    /// Convenience: log a message to the TUI log pane.
    pub fn log(&self, msg: String) {
        let _ = self.channels.ui.tx_log.try_send(msg);
    }

    /// Teardown: stop workers, disable raw mode, restore terminal.
    pub fn teardown(self) -> io::Result<()> {
        // Stop pipeline workers if active: audio capture and transcription threads.
        // Ignore errors from stop() calls; teardown continues even if workers fail to stop cleanly.
        if let Some(pipeline) = self.pipeline {
            let _ = pipeline.audio.stop();
            pipeline.transcriber.stop();
        }
        // Disable raw mode: restore normal terminal line buffering and echo.
        disable_raw_mode()?;
        // Exit alternate screen and show cursor: restore terminal to pre-TUI state.
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        )?;
        Ok(())
    }
}
