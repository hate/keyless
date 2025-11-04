//! Channel drain helpers: non-blocking pulls from worker queues into AppState.

use std::sync::atomic::Ordering;
use std::time::Instant;

use super::context::RuntimeContext;
use crate::tui::runtime::handlers::build_config_from_app;
use crate::tui::state::DownloadState;
use keyless_core::config::storage;
use keyless_models::DownloadEvent;

/// Result of a drain pass controlling overlay state.
pub enum DrainResult {
    /// Continue normal operation.
    Continue,
    /// Clear download overlay now.
    ClearDownload,
}

/// Drain all non-download channels (logs, spectrum, VAD, sizes, hold, PTT events).
pub fn drain_channels(ctx: &mut RuntimeContext) {
    // Accumulate talk time based on current gate and hold state.
    // Only count time when PTT is held AND VAD is open (speech detected).
    let now = Instant::now();
    if let Some(prev) = ctx.app.feedback.last_tick {
        // Saturating_duration_since prevents panic if clock goes backwards (shouldn't happen).
        let delta_ms = now.saturating_duration_since(prev).as_millis() as u64;
        // Both conditions must be true: PTT held (user wants to speak) AND VAD open (audio detected).
        if ctx.app.feedback.hold_active && ctx.app.feedback.vad_open && delta_ms > 0 {
            // Saturating_add prevents overflow; counters are u64 and unlikely to wrap.
            ctx.app.feedback.session_talk_ms =
                ctx.app.feedback.session_talk_ms.saturating_add(delta_ms);
            ctx.app.lifetime_talk_ms = ctx.app.lifetime_talk_ms.saturating_add(delta_ms);
        }
    }
    // Update tick timestamp for next frame's delta calculation.
    ctx.app.feedback.last_tick = Some(now);

    // Logs and telemetry: drain all available messages non-blockingly.
    // RMS level history: maintain fixed-size circular buffer for audio level visualization.
    while let Ok(level) = ctx.receivers.audio.rx_level.try_recv() {
        // Remove oldest entry if at capacity (FIFO eviction).
        if ctx.app.feedback.rms_history.len() == ctx.app.feedback.rms_history.capacity() {
            let _ = ctx.app.feedback.rms_history.pop_front();
        }
        ctx.app.feedback.rms_history.push_back(level);
    }
    // Log messages: drain all available log lines and process special events.
    while let Ok(line) = ctx.receivers.ui.rx_log.try_recv() {
        // Maintain fixed-size log buffer; oldest entries are evicted when full.
        if ctx.app.feedback.logs.len() == ctx.app.feedback.logs.capacity() {
            let _ = ctx.app.feedback.logs.pop_front();
        }
        // Clone line since we need it for word counting and paste guard checks.
        ctx.app.feedback.logs.push_back(line.clone());
        // Count words on Final events and persist lifetime total.
        // "Final: " prefix indicates completed transcription ready for output.
        if let Some(text) = line.strip_prefix("Final: ") {
            // Split by whitespace and count; cast to u64 for counter (u64 overflow unlikely).
            let words = text.split_whitespace().count() as u64;
            if words > 0 {
                // Saturating_add prevents overflow; both counters are u64.
                ctx.app.feedback.session_words =
                    ctx.app.feedback.session_words.saturating_add(words);
                ctx.app.lifetime_words = ctx.app.lifetime_words.saturating_add(words);
                // Persist config immediately after word count update to save progress.
                let config = build_config_from_app(&ctx.app);
                let _ = storage::save_config(&config);
            }
        }
        // Paste guard: when a Final is delivered, the Paste sink may type into the terminal.
        // If the terminal is focused, those characters can trigger our own hotkeys.
        // Detect Final logs and ignore hotkey input briefly (750ms) to prevent feedback loop.
        if let Some(last) = ctx.app.feedback.logs.back()
            && (last.starts_with("Final") || last.contains("final transcription delivered"))
        {
            ctx.paste_guard_until = Some(Instant::now() + std::time::Duration::from_millis(750));
        }
        // Update loading overlay message from log events during startup phase.
        if let Some(dl) = ctx.app.overlays.download.as_mut()
            && dl.model == "loading"
            && let Some(last) = ctx.app.feedback.logs.back()
        {
            dl.message = last.clone();
        }
    }
    // Spectrum bars: update visualization data; use latest available (drops older if multiple).
    while let Ok(b) = ctx.receivers.audio.rx_spec.try_recv() {
        ctx.app.feedback.spectrum_bars = b;
    }
    // Preview text: live transcription updates before Final event.
    while let Ok(pre) = ctx.receivers.ui.rx_preview.try_recv() {
        ctx.app.feedback.preview_text = pre.clone();
        // Track update timestamp for auto-clear; None if empty (no preview active).
        ctx.app.feedback.preview_text_updated_at = if pre.is_empty() {
            None
        } else {
            Some(Instant::now())
        };
    }

    // Auto-clear preview text after 500ms of no updates (transcription likely stalled or stopped).
    if let Some(last_update) = ctx.app.feedback.preview_text_updated_at
        && last_update.elapsed().as_millis() > 500
        && !ctx.app.feedback.preview_text.is_empty()
    {
        ctx.app.feedback.preview_text.clear();
        ctx.app.feedback.preview_text_updated_at = None;
    }
    // Size updates: drain from both cached and remote sources.
    // Cached sizes are fast (local disk); remote sizes refresh stale cache.
    if let Some(rx) = ctx.catalog.rx_sizes_cached.as_ref() {
        while let Ok((id, sz)) = rx.try_recv() {
            // Only insert if size is Some; None indicates model not found (expected for some models).
            if let Some(bytes) = sz {
                ctx.app.models.sizes.insert(id, bytes);
            }
        }
    }
    // Remote sizes overwrite cached ones when available (more up-to-date).
    if let Some(rx) = ctx.catalog.rx_sizes_remote.as_ref() {
        while let Ok((id, sz)) = rx.try_recv() {
            if let Some(bytes) = sz {
                ctx.app.models.sizes.insert(id, bytes);
            }
        }
    }
    // VAD and hold: control state from pipeline workers.
    // VAD state: whether speech threshold is currently exceeded (gate open/closed).
    while let Ok(v) = ctx.receivers.control.rx_vad.try_recv() {
        ctx.app.feedback.vad_open = v;
    }
    // Hold state: whether PTT key is currently pressed (user holding hotkey).
    while let Ok(held) = ctx.receivers.control.rx_hold.try_recv() {
        ctx.app.feedback.hold_active = held;
        // Sync hold_flag for pipeline workers (shared atomic across threads).
        ctx.ptt.hold_flag.store(held, Ordering::Relaxed);
        if !held {
            // On PTT release: close VAD immediately (no speech possible without PTT held).
            ctx.app.feedback.vad_open = false;
            // Persist lifetime counters on PTT release to save progress incrementally.
            let config = build_config_from_app(&ctx.app);
            let _ = storage::save_config(&config);
        }
        // Fallback: if mic label not yet set but pipeline exists, sync from pipeline.
        // Pipeline may have mic name before UI state is updated (race condition mitigation).
        if ctx.app.feedback.mic_name == "(not started)"
            && let Some(p) = ctx.pipeline.as_ref()
        {
            ctx.app.feedback.mic_name = p.mic_name.clone();
        }
    }
    // PTT events: heartbeat events from hotkey listener (proves listener is alive).
    // Just update timestamp; we don't need the event payload.
    while ctx.receivers.control.rx_ptt_evt.try_recv().is_ok() {
        ctx.ptt.last_event = Some(Instant::now());
    }
}

/// Drain download events and handle state transitions.
/// Downloads can be paused (ESC, keeps .partial) or cancelled (Backspace, deletes .partial).
pub fn drain_download_events(ctx: &mut RuntimeContext) -> DrainResult {
    let mut clear_dl = false;
    let mut events = Vec::new();

    // Collect events first to avoid borrow issues (can't borrow ctx.download mutably while
    // iterating receiver). Collecting into Vec allows processing without holding receiver ref.
    if let Some(dl_ctx) = ctx.download.as_ref() {
        while let Ok(evt) = dl_ctx.rx.try_recv() {
            events.push(evt);
        }
    }

    // Process events: update overlay state and trigger pipeline startup on completion.
    for evt in events {
        match evt {
            DownloadEvent::Started { model } => {
                ctx.log(format!("preparing model {}", model));
                // Show download overlay with initial "starting..." message.
                ctx.app.overlays.download =
                    Some(DownloadState::new(model.clone(), "starting...".to_string()));
            }
            DownloadEvent::Stage { text } => {
                // Update overlay message with current download stage (e.g., "downloading weights...").
                if let Some(dl_state) = ctx.app.overlays.download.as_mut() {
                    dl_state.message = text;
                }
            }
            DownloadEvent::Progress {
                bytes,
                total,
                mbps,
                eta_s,
            } => {
                let model_prefix = ctx.app.downloading_model().unwrap_or("model");
                let text = match total {
                    Some(t) if t > 0 => {
                        // Calculate percentage; clamp to 100% to handle edge cases (slight overshoot).
                        let pct = ((bytes as f64 / t as f64) * 100.0).min(100.0);
                        let downloaded_mb = bytes as f64 / 1_000_000.0;
                        let total_mb = t as f64 / 1_000_000.0;
                        // Format ETA: show minutes+seconds if >=60s, else seconds only.
                        let eta_display = if eta_s >= 60.0 {
                            format!("{}m {}s", (eta_s as u32) / 60, (eta_s as u32) % 60)
                        } else {
                            format!("{:.0}s", eta_s)
                        };
                        // Store percentage for gauge rendering but don't show in text.
                        // Format includes percentage, MB progress, speed, and ETA.
                        format!(
                            "{:.0}% {} ({:.1} / {:.1} MB) • {:.1} MB/s • ETA {}",
                            pct, model_prefix, downloaded_mb, total_mb, mbps, eta_display
                        )
                    }
                    // Unknown total: show downloaded MB only (indeterminate progress).
                    _ => format!("{} {:.1} MB...", model_prefix, bytes as f64 / 1_000_000.0),
                };
                if let Some(dl_state) = ctx.app.overlays.download.as_mut() {
                    dl_state.message = text;
                }
            }
            DownloadEvent::Done(Ok(())) => {
                // Add downloaded model to discovered list for checkmark in models table.
                if let Some(model_id) = ctx.app.downloading_model()
                    && !ctx.app.models.discovered.contains(&model_id.to_string())
                {
                    ctx.app.models.discovered.push(model_id.to_string());
                }
                ctx.app.overlays.download = None;
                clear_dl = true;
            }
            DownloadEvent::Done(Err(e)) => {
                // Download failed: clear overlay and show error message.
                ctx.app.overlays.download = None;
                clear_dl = true;
                ctx.app.overlays.error_message = Some(e);
            }
        }
    }

    // Handle pipeline start after processing all events (download complete or cancelled).
    if clear_dl {
        if let Some(dl_ctx) = ctx.download.take() {
            // Show loading overlay and spawn worker for phased startup (model load, device probe, etc.).
            // Download overlay transitions to loading overlay seamlessly.
            ctx.app.overlays.download = Some(DownloadState::new(
                "loading".to_string(),
                "probing device…".to_string(),
            ));
            // Extract device selection and config from download context (stashed when download started).
            let selected_device = ctx
                .app
                .selections
                .devices
                .get(dl_ctx.pending_device_idx)
                .cloned();
            let config = dl_ctx.pending_config.clone();
            let eq_tuning = ctx.app.eq_tuning.clone();
            let vad = ctx.app.vad.clone();
            // Spawn startup worker: loads model, probes device, initializes GPU/CPU backend.
            // Returns receiver for phased startup events (PhaseStarted, PhaseOk, Ready, etc.).
            let rx = keyless_runtime::pipeline::spawn_startup_worker(
                config.clone(),
                eq_tuning.clone(),
                ctx.channels.ui.tx_log.clone(),
            );
            ctx.startup_rx = Some(rx);
            // Stash inputs for finalization once Ready arrives (device may be None if auto-selected).
            ctx.startup_model_cfg = Some((config, eq_tuning, vad, selected_device));
        }
        DrainResult::ClearDownload
    } else {
        // If a startup is in progress, process startup events and finalize when ready.
        // This runs every frame until startup completes or fails.
        if ctx.startup_rx.is_some() {
            let mut clear_rx = false;
            // Drain all available startup events non-blockingly.
            while let Some(rx) = ctx.startup_rx.as_ref() {
                if let Ok(evt) = rx.try_recv() {
                    match evt {
                        keyless_runtime::pipeline::StartupEvent::PhaseStarted(name) => {
                            // Update overlay message with current phase (e.g., "loading model…").
                            if let Some(dl) = ctx.app.overlays.download.as_mut() {
                                dl.message = format!("{}…", name);
                            }
                        }
                        keyless_runtime::pipeline::StartupEvent::PhaseOk(name) => {
                            // Phase completed successfully; show confirmation.
                            if let Some(dl) = ctx.app.overlays.download.as_mut() {
                                dl.message = format!("{} ok", name);
                            }
                        }
                        keyless_runtime::pipeline::StartupEvent::PhaseErr(name, err) => {
                            // Phase failed: clear overlay and show error; stop processing startup events.
                            ctx.app.overlays.download = None;
                            ctx.app.overlays.error_message =
                                Some(keyless_core::error::KeylessError::Other(format!(
                                    "{} failed: {}",
                                    name, err
                                )));
                            clear_rx = true;
                        }
                        keyless_runtime::pipeline::StartupEvent::Ready(artifacts) => {
                            // Startup complete: finalize pipeline with loaded artifacts.
                            // Take stashed config; if missing, abort (shouldn't happen).
                            let Some((config, _eq_tuning, vad, selected_device)) =
                                ctx.startup_model_cfg.take()
                            else {
                                clear_rx = true;
                                break;
                            };
                            // Build channel handles for pipeline workers (one-way communication).
                            let pipeline_channels = keyless_runtime::pipeline::PipelineChannels {
                                tx_level: ctx.channels.audio.tx_level.clone(),
                                tx_log: ctx.channels.ui.tx_log.clone(),
                                tx_spec: ctx.channels.audio.tx_spec.clone(),
                                tx_preview: ctx.channels.ui.tx_preview.clone(),
                                tx_vad: ctx.channels.control.tx_vad.clone(),
                            };
                            match keyless_runtime::pipeline::finalize_pipeline_from_artifacts(
                                artifacts,
                                &config,
                                vad,
                                pipeline_channels,
                                ctx.ptt.hold_flag.clone(),
                                selected_device,
                            ) {
                                Ok(ph) => {
                                    ctx.app.overlays.download = None;
                                    // Update mic name for UI before moving handle (avoids borrowing issues).
                                    ctx.app.feedback.mic_name = ph.mic_name.clone();
                                    ctx.pipeline = Some(ph);
                                    // Switch to Dictating screen; user can now start speaking.
                                    ctx.app.screen = crate::tui::state::Screen::Dictating;
                                    // Enable PTT: hotkey listener becomes active.
                                    ctx.ptt.enabled.store(true, Ordering::Relaxed);
                                    ctx.log("PTT enabled".to_string());
                                    // Reset last_event to start fresh heartbeat monitoring.
                                    ctx.ptt.last_event = None;
                                }
                                Err(e) => {
                                    // Finalization failed: clear overlay and show error.
                                    ctx.app.overlays.download = None;
                                    ctx.app.overlays.error_message = Some(e);
                                }
                            }
                            clear_rx = true;
                        }
                    }
                } else {
                    // No more events available; break and check again next frame.
                    break;
                }
            }
            // Clear receiver if startup completed or failed (prevents checking on every frame).
            if clear_rx {
                ctx.startup_rx = None;
            }
        }
        DrainResult::Continue
    }
}
