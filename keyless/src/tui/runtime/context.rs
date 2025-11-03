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
        // Restore terminal on panic
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

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        let app = AppState::new(overrides);

        // Create channels
        let (channels, receivers) = Channels::new();

        // Shared PTT flags
        let hold_flag = Arc::new(AtomicBool::new(false));
        let preset_mode = Arc::new(std::sync::atomic::AtomicU8::new(
            if app.hotkey.contains("shift") { 1 } else { 0 },
        ));
        let ptt_stop = Arc::new(AtomicBool::new(false));
        let ptt_enabled = Arc::new(AtomicBool::new(false));

        // Route tracing logs into the TUI log pane
        keyless_logging::init_channel(channels.ui.tx_log.clone());

        // Spawn global hotkey listener
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
        if let Some(pipeline) = self.pipeline {
            let _ = pipeline.audio.stop();
            pipeline.transcriber.stop();
        }
        disable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        )?;
        Ok(())
    }
}
