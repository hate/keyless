//! Input routing and handling for Config and Dictating screens.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use crossterm::event::{KeyCode, KeyEvent};

use super::DownloadContext;
use super::context::RuntimeContext;
use super::handlers;
use crate::tui::config::expert::EqParameter;
use crate::tui::state::{ExpertOverlay, Screen};
use keyless_core::config::{Config, DEFAULT_MODEL_ID, storage};
use keyless_models::DownloadEvent;

/// Control flow directive after handling a key.
pub enum ControlFlow {
    /// Continue the event loop.
    Continue,
    /// Exit the application.
    Quit,
}

/// Route input based on current screen and overlays.
pub fn route_input(ctx: &mut RuntimeContext, key: &KeyEvent) -> std::io::Result<ControlFlow> {
    // Delegate to screen-specific handlers; each screen has its own input mapping.
    match ctx.app.screen {
        Screen::Config => handle_config_screen(ctx, key),
        Screen::Dictating => handle_dictating_screen(ctx, key),
    }
}

/// Handle keys while on the Config screen, including overlays.
fn handle_config_screen(ctx: &mut RuntimeContext, key: &KeyEvent) -> std::io::Result<ControlFlow> {
    // Priority 1: overlays: handle loading overlay ESC to cancel startup, or download pause/cancel.
    // Overlays block normal input until dismissed or confirmed.
    if ctx.app.is_downloading() {
        // Distinguish loading vs downloading by overlay title ("loading" = startup phase).
        let is_loading = ctx
            .app
            .overlays
            .download
            .as_ref()
            .map(|d| d.model.as_str() == "loading")
            .unwrap_or(false);
        if is_loading {
            if let KeyCode::Esc = key.code {
                // Cancel startup overlay and drop startup receiver (stops background worker).
                ctx.app.overlays.download = None;
                ctx.startup_rx = None;
                ctx.startup_model_cfg = None;
                let _ = ctx
                    .channels
                    .ui
                    .tx_log
                    .try_send("startup cancelled".to_string());
                return Ok(ControlFlow::Continue);
            }
            // Ignore Backspace/etc while loading overlay is active (only ESC cancels).
            return Ok(ControlFlow::Continue);
        }
        match key.code {
            KeyCode::Esc => {
                // Pause: keep partial file for resume (download can continue later).
                ctx.app.overlays.download = None;
                if let Some(dl_ctx) = ctx.download.take() {
                    // Signal cancellation to download worker; .partial file remains on disk.
                    dl_ctx.cancel.store(true, Ordering::Relaxed);
                }
                let _ = ctx
                    .channels
                    .ui
                    .tx_log
                    .try_send("download paused (resume on restart)".to_string());
                return Ok(ControlFlow::Continue);
            }
            KeyCode::Backspace => {
                // Cancel: delete partial file (irreversible; user must re-download).
                if let Some(model_id) = ctx.app.downloading_model() {
                    let model_id = model_id.to_string();
                    ctx.app.overlays.download = None;
                    if let Some(dl_ctx) = ctx.download.take() {
                        dl_ctx.cancel.store(true, Ordering::Relaxed);
                    }
                    // Delete the partial file to free disk space.
                    match keyless_models::hf::delete_partial_file(&model_id) {
                        Ok(true) => {
                            ctx.log("download cancelled (partial deleted)".to_string());
                        }
                        Ok(false) => {
                            // Partial file didn't exist (maybe already completed or never started).
                            ctx.log("download cancelled".to_string());
                        }
                        Err(e) => {
                            // Log error but continue; cancellation is still effective.
                            ctx.log(format!(
                                "download cancelled; failed to delete partial: {}",
                                e
                            ));
                        }
                    }
                } else {
                    // No model ID available; cancel download context anyway.
                    ctx.app.overlays.download = None;
                    if let Some(dl_ctx) = ctx.download.take() {
                        dl_ctx.cancel.store(true, Ordering::Relaxed);
                    }
                    ctx.log("download cancelled".to_string());
                }
                return Ok(ControlFlow::Continue);
            }
            _ => {}
        }
    }

    // Priority 2: Expert Mode toggle (x opens) unless editing File path in Config.
    if let KeyCode::Char('x') = key.code {
        // When the user is typing the File sink path, 'x' is literal input; don't open Expert.
        // This prevents mode switching mid-typing.
        if !ctx.app.selections.file_edit {
            if !ctx.app.expert_mode() {
                ctx.app.overlays.expert = Some(ExpertOverlay::new());
                ctx.log("expert mode OPEN".to_string());
            }
            return Ok(ControlFlow::Continue);
        }
    }

    // Priority 3: Expert Mode confirmations (block all other input until resolved).
    // Confirmations are modal dialogs that require y/n/esc before continuing.
    if let Some(expert) = ctx.app.overlays.expert.as_ref() {
        if expert.confirm_reset {
            return handle_expert_confirm_reset(ctx, key);
        }
        if expert.confirm_purge {
            return handle_expert_confirm_purge(ctx, key);
        }
        // Expert Mode normal input: EQ adjustments, navigation, etc.
        return handle_expert_mode(ctx, key);
    }

    // Priority 4: normal Config input (navigation, selections, start pipeline).
    match handlers::handle_config_input(&mut ctx.app, key, ctx.sink_items, &ctx.channels.ui.tx_log)
    {
        handlers::ConfigAction::Continue => Ok(ControlFlow::Continue),
        handlers::ConfigAction::Quit => Ok(ControlFlow::Quit),
        handlers::ConfigAction::StartPipeline => {
            // Start pipeline: ensure model is cached, then initialize.
            // Guard prevents multiple concurrent downloads/startups.
            if ctx.download.is_none() {
                // Build config from current UI state (selections, EQ, VAD, etc.).
                let pending_config = handlers::build_config_from_app(&ctx.app);
                let pending_device_idx = ctx.app.selections.device_idx;
                // Create download event channel for progress updates.
                let (tx_dl, rx_dl) = mpsc::sync_channel::<DownloadEvent>(32);
                // Shared cancellation flag: can be set from UI (pause/cancel) or on error.
                let cancel_flag = Arc::new(AtomicBool::new(false));

                // Stash download context: receiver, cancel flag, and pending config/device.
                ctx.download = Some(DownloadContext::new(
                    rx_dl,
                    cancel_flag.clone(),
                    pending_config,
                    pending_device_idx,
                ));

                // Get selected model ID; fallback to default if index is out of bounds.
                let model_id = ctx
                    .app
                    .models
                    .runtime_list
                    .get(ctx.app.selections.model_idx)
                    .cloned()
                    .unwrap_or_else(|| DEFAULT_MODEL_ID.to_string());

                // Don't show download overlay yet - only show it if we actually need to download.
                // The download worker will emit DownloadEvent::Started if model needs downloading.
                // If model is already cached, worker completes immediately without overlay.
                std::thread::spawn(move || {
                    keyless_models::ensure_model_cached(model_id, tx_dl, cancel_flag);
                });
            }
            Ok(ControlFlow::Continue)
        }
    }
}

/// Handle the expert mode "reset config" confirmation dialog.
fn handle_expert_confirm_reset(
    ctx: &mut RuntimeContext,
    key: &KeyEvent,
) -> std::io::Result<ControlFlow> {
    match key.code {
        KeyCode::Char('y') => {
            // Confirm reset: save default config to disk, then reset UI state.
            if let Err(e) = storage::save_config(&Config::default()) {
                ctx.log(format!("failed to reset config: {}", e));
            } else {
                // Preserve runtime state (catalog, expert overlay, catalog runtime in ctx).
                // These are session-scoped and shouldn't be reset (discovery/catalog work persists).
                let keep_expert = ctx.app.overlays.expert.take();
                let keep_models = std::mem::take(&mut ctx.app.models);

                // Reset to defaults: creates fresh AppState with default selections/config.
                ctx.app = crate::tui::state::AppState::new(None);

                // Restore runtime state: put back catalog and expert overlay.
                ctx.app.models = keep_models;
                ctx.app.overlays.expert = keep_expert;
                // Clear confirmation flag so overlay returns to normal expert mode.
                if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                    expert.confirm_reset = false;
                }
                ctx.log("config reset to defaults".to_string());
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            // Cancel reset: just dismiss the confirmation dialog.
            if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                expert.confirm_reset = false;
            }
            ctx.log("reset cancelled".to_string());
            Ok(ControlFlow::Continue)
        }
        // Ignore other keys while confirmation is active (modal behavior).
        _ => Ok(ControlFlow::Continue),
    }
}

/// Handle the expert mode "delete models" confirmation dialog.
fn handle_expert_confirm_purge(
    ctx: &mut RuntimeContext,
    key: &KeyEvent,
) -> std::io::Result<ControlFlow> {
    match key.code {
        KeyCode::Char('y') => {
            // Confirm purge: delete entire models cache directory (destructive, irreversible).
            let root = keyless_models::hf::keyless_cache_root();
            match std::fs::remove_dir_all(&root) {
                Ok(_) => {
                    // Recreate empty directory so cache location exists for future downloads.
                    let _ = std::fs::create_dir_all(&root);
                    // Clear UI state: discovered models, sizes cache, and reset catalog workers.
                    ctx.app.models.discovered.clear();
                    ctx.app.models.sizes.clear();
                    // Drop receivers to stop size workers (they're no longer needed).
                    ctx.catalog.rx_sizes_cached = None;
                    ctx.catalog.rx_sizes_remote = None;
                    ctx.catalog.sizes_started = false;
                    if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                        expert.confirm_purge = false;
                    }
                    ctx.log(format!("models cache cleared at {:?}", root));
                }
                Err(e) => {
                    // Purge failed: log error but dismiss confirmation dialog anyway.
                    if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                        expert.confirm_purge = false;
                    }
                    ctx.log(format!("failed to clear models cache {:?}: {}", root, e));
                }
            }
            Ok(ControlFlow::Continue)
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            // Cancel purge: just dismiss the confirmation dialog.
            if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                expert.confirm_purge = false;
            }
            ctx.log("delete models cancelled".to_string());
            Ok(ControlFlow::Continue)
        }
        // Ignore other keys while confirmation is active (modal behavior).
        _ => Ok(ControlFlow::Continue),
    }
}

/// Handle expert mode adjustments and navigation.
fn handle_expert_mode(ctx: &mut RuntimeContext, key: &KeyEvent) -> std::io::Result<ControlFlow> {
    match key.code {
        KeyCode::Up => {
            // Navigate to previous EQ parameter (wraps around from Bands to Decay).
            if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                expert.selected = expert.selected.prev();
            }
        }
        KeyCode::Down => {
            // Navigate to next EQ parameter (wraps around from Decay to Bands).
            if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                expert.selected = expert.selected.next();
            }
        }
        KeyCode::Esc => {
            // Close expert mode overlay and return to normal Config screen.
            ctx.app.overlays.expert = None;
            ctx.log("expert mode CLOSED".to_string());
            return Ok(ControlFlow::Continue);
        }
        KeyCode::Char('o') => {
            // Open config folder in system file manager (platform-specific commands).
            let path = storage::config_path();
            // Use parent directory if path is a file; otherwise use path as-is.
            let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or(path);
            let result = if cfg!(target_os = "macos") {
                std::process::Command::new("open").arg(&dir).spawn()
            } else if cfg!(target_os = "windows") {
                std::process::Command::new("cmd")
                    .args(["/C", "start", dir.to_string_lossy().as_ref()])
                    .spawn()
            } else {
                std::process::Command::new("xdg-open").arg(&dir).spawn()
            };
            match result {
                Ok(_) => {
                    ctx.log("opened config folder".to_string());
                }
                Err(e) => {
                    // Log error but continue; file manager launch failure isn't fatal.
                    ctx.log(format!("failed to open config folder: {}", e));
                }
            }
        }
        KeyCode::Char('d') => {
            // Trigger delete models confirmation dialog (destructive action).
            if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                expert.confirm_purge = true;
            }
            ctx.log("delete all models? y/n".to_string());
            return Ok(ControlFlow::Continue);
        }
        KeyCode::Char('r') => {
            // Trigger reset config confirmation dialog.
            if let Some(expert) = ctx.app.overlays.expert.as_mut() {
                expert.confirm_reset = true;
            }
            ctx.log("confirm reset? y/n".to_string());
            return Ok(ControlFlow::Continue);
        }
        KeyCode::Left | KeyCode::Right => {
            // Adjust selected EQ parameter (decrease/increase).
            handle_expert_adjust(ctx, key);
        }
        // Ignore other keys in expert mode (no action defined).
        _ => {}
    }
    Ok(ControlFlow::Continue)
}

/// Adjust an EQ parameter in expert mode based on arrow keys.
fn handle_expert_adjust(ctx: &mut RuntimeContext, key: &KeyEvent) {
    // Detect shift modifier for coarse adjustment (larger steps).
    let coarse = key
        .modifiers
        .contains(crossterm::event::KeyModifiers::SHIFT);
    // Right = increase, Left = decrease.
    let dir = matches!(key.code, KeyCode::Right);
    // Step sizes: small (1.0) for fine, large (3.0) for coarse adjustment.
    let (small, large) = (1.0f32, 3.0f32);
    let step = if coarse { large } else { small };
    // Clamp helper: ensures value stays within [lo, hi] bounds.
    let clamp = |v: f32, lo: f32, hi: f32| v.max(lo).min(hi);

    let Some(expert) = ctx.app.overlays.expert.as_mut() else {
        return;
    };

    match expert.selected {
        EqParameter::Bands => {
            // Bands: adjust by 8 (fine) or 24 (coarse), clamped to [16, 128].
            // Cast through i32 to handle negative delta safely.
            let prev = ctx.app.eq_tuning.bands;
            let delta = if dir { 8i32 } else { -8i32 } * if coarse { 3 } else { 1 };
            let b = (ctx.app.eq_tuning.bands as i32 + delta).clamp(16, 128);
            ctx.app.eq_tuning.bands = b as u16;
            // Log when clamping occurs (reached min/max boundary).
            if ctx.app.eq_tuning.bands != prev
                && (ctx.app.eq_tuning.bands == 16 || ctx.app.eq_tuning.bands == 128)
            {
                ctx.log(format!("eq.bands clamped to {}", ctx.app.eq_tuning.bands));
            }
        }
        EqParameter::NoiseReduction => {
            // Noise reduction: step by step size, clamped to [0.0, 1.0] (unitless factor).
            let next = clamp(
                ctx.app.eq_tuning.noise_reduction + if dir { step } else { -step },
                0.0,
                1.0,
            );
            ctx.app.eq_tuning.noise_reduction = next;
        }
        EqParameter::WindowDb => {
            // Window dB: step by 2x step size (faster adjustment), clamped to [10.0, 80.0] dB.
            let next = clamp(
                ctx.app.eq_tuning.window_db + if dir { 2.0 * step } else { -2.0 * step },
                10.0,
                80.0,
            );
            ctx.app.eq_tuning.window_db = next;
        }
        EqParameter::Gamma => {
            // Gamma: step by 0.05 * step (fractional steps), clamped to [0.8, 2.0].
            // Higher gamma emphasizes louder frequency bins.
            let next = clamp(
                ctx.app.eq_tuning.gamma + if dir { 0.05 * step } else { -0.05 * step },
                0.8,
                2.0,
            );
            ctx.app.eq_tuning.gamma = next;
        }
        EqParameter::Attack => {
            // Attack: step by 0.05 * step (seconds), clamped to [0.05, 1.0] seconds.
            // Attack time constant for envelope follower.
            let next = clamp(
                ctx.app.eq_tuning.attack + if dir { 0.05 * step } else { -0.05 * step },
                0.05,
                1.0,
            );
            ctx.app.eq_tuning.attack = next;
        }
        EqParameter::Decay => {
            // Decay: step by 0.05 * step (seconds), clamped to [0.05, 1.0] seconds.
            // Decay time constant for envelope follower.
            let next = clamp(
                ctx.app.eq_tuning.decay + if dir { 0.05 * step } else { -0.05 * step },
                0.05,
                1.0,
            );
            ctx.app.eq_tuning.decay = next;
        }
    }
}

/// Handle keys while on the Dictating screen.
fn handle_dictating_screen(
    ctx: &mut RuntimeContext,
    key: &KeyEvent,
) -> std::io::Result<ControlFlow> {
    // Paste guard: ignore hotkey toggles immediately after we paste Finals via Paste sink.
    // When Paste sink types into terminal, those characters can trigger our own hotkeys.
    // Guard window (750ms) prevents feedback loop where pasted text triggers PTT toggle.
    if let crossterm::event::KeyCode::Char('h') = key.code
        && let Some(until) = ctx.paste_guard_until
        && std::time::Instant::now() < until
    {
        // Swallow this keypress silently (pretend it never happened).
        return Ok(ControlFlow::Continue);
    }
    // Delegate to dictating input handler for screen-specific key mappings.
    let action = handlers::dictating::handle_dictating_input(
        &mut ctx.app,
        key,
        ctx.pipeline.as_mut(),
        &ctx.ptt.stop_flag,
        &ctx.ptt.enabled,
        &ctx.ptt.preset_mode,
        &ctx.channels.ui.tx_log,
    );

    match action {
        handlers::DictatingAction::Continue => Ok(ControlFlow::Continue),
        handlers::DictatingAction::Quit => Ok(ControlFlow::Quit),
        handlers::DictatingAction::ReturnToConfig => {
            // Stop pipeline workers: audio capture and transcription threads.
            // Ignore errors from stop() calls; teardown continues even if workers fail.
            if let Some(pipeline) = ctx.pipeline.take() {
                let _ = pipeline.audio.stop();
                pipeline.transcriber.stop();
            }
            // Switch back to Config screen.
            ctx.app.screen = Screen::Config;
            // Disable PTT: hotkey listener stops listening (frees system resources).
            ctx.ptt.enabled.store(false, Ordering::Relaxed);
            // Re-hydrate static permission guidance for the current sink/device state.
            // Permission warnings may change based on sink choice (Paste needs Accessibility).
            let needs_paste = matches!(
                ctx.app.selections.sink_choice,
                crate::tui::state::SinkChoice::Paste
            );
            ctx.app.perm_warnings = keyless_core::permissions::gather_permission_warnings(
                needs_paste,
                !ctx.app.selections.device_names.is_empty(),
            );
            ctx.log("PTT disabled".to_string());
            Ok(ControlFlow::Continue)
        }
    }
}
