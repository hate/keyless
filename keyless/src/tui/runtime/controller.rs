//! Runtime controller: main event loop (refactored).
//!
//! This module is the heart of the TUI runtime. It orchestrates:
//! - Setup via `context::RuntimeContext::build()`
//! - Per-frame channel drains via `drains::*`
//! - Config screen init via `init::init_config_screen()`
//! - Input routing via `input::route_input()`
//! - Periodic tasks via `periodic::tick()`
//! - Teardown via `ctx.teardown()`
//!
//! The main loop is intentionally minimal (~50 lines) for clarity and testability.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event};

use super::{context, drains, init, input, periodic};
use crate::overrides::Overrides;
use crate::tui::state::Screen;

/// Run the TUI with optional non-persistent overrides.
pub fn run_with_overrides(overrides: Option<Overrides>) -> io::Result<()> {
    let mut ctx = context::RuntimeContext::build(overrides.as_ref())?;

    loop {
        // Drain channels every frame
        drains::drain_channels(&mut ctx);
        let _ = drains::drain_download_events(&mut ctx);

        // Config screen initialization (catalog, discovery, restore selections)
        if matches!(ctx.app.screen, Screen::Config) {
            init::init_config_screen(&mut ctx);
        }

        // Update cached runtime_list only if it changed
        if ctx.runtime_list_cache != ctx.app.models.runtime_list {
            ctx.runtime_list_cache = ctx.app.models.runtime_list.clone();
        }

        // Render
        ctx.terminal.draw(|f| match ctx.app.screen {
            Screen::Config => {
                let runtime_refs: Vec<&str> =
                    ctx.runtime_list_cache.iter().map(|s| s.as_str()).collect();
                crate::tui::config::draw_config(f, &ctx.app, ctx.sink_items, &runtime_refs)
            }
            Screen::Dictating => crate::tui::dictating::draw_dictating(
                f,
                &ctx.app,
                ctx.pipeline
                    .as_ref()
                    .map(|p| p.sink_label.as_str())
                    .unwrap_or(""),
            ),
        })?;

        // Poll input
        if crossterm::event::poll(Duration::from_millis(33))?
            && let Event::Key(key) = event::read()?
        {
            match input::route_input(&mut ctx, &key)? {
                input::ControlFlow::Quit => break,
                input::ControlFlow::Continue => {}
            }
        }

        // Periodic tasks
        periodic::tick(&mut ctx);
    }

    ctx.teardown()
}
