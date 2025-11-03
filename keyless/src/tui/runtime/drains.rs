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
    // Accumulate talk time based on current gate and hold state
    let now = Instant::now();
    if let Some(prev) = ctx.app.feedback.last_tick {
        let delta_ms = now.saturating_duration_since(prev).as_millis() as u64;
        if ctx.app.feedback.hold_active && ctx.app.feedback.vad_open && delta_ms > 0 {
            ctx.app.feedback.session_talk_ms =
                ctx.app.feedback.session_talk_ms.saturating_add(delta_ms);
            ctx.app.lifetime_talk_ms = ctx.app.lifetime_talk_ms.saturating_add(delta_ms);
        }
    }
    ctx.app.feedback.last_tick = Some(now);

    // Logs and telemetry
    while let Ok(level) = ctx.receivers.audio.rx_level.try_recv() {
        if ctx.app.feedback.rms_history.len() == ctx.app.feedback.rms_history.capacity() {
            let _ = ctx.app.feedback.rms_history.pop_front();
        }
        ctx.app.feedback.rms_history.push_back(level);
    }
    while let Ok(line) = ctx.receivers.ui.rx_log.try_recv() {
        if ctx.app.feedback.logs.len() == ctx.app.feedback.logs.capacity() {
            let _ = ctx.app.feedback.logs.pop_front();
        }
        ctx.app.feedback.logs.push_back(line.clone());
        // Count words on Final events and persist lifetime total
        if let Some(text) = line.strip_prefix("Final: ") {
            let words = text.split_whitespace().count() as u64;
            if words > 0 {
                ctx.app.feedback.session_words =
                    ctx.app.feedback.session_words.saturating_add(words);
                ctx.app.lifetime_words = ctx.app.lifetime_words.saturating_add(words);
                let config = build_config_from_app(&ctx.app);
                let _ = storage::save_config(&config);
            }
        }
        // Paste guard: when a Final is delivered, the Paste sink may type into the terminal
        // If the terminal is focused, those characters can trigger our own hotkeys.
        // Detect Final logs and ignore hotkey input briefly.
        if let Some(last) = ctx.app.feedback.logs.back()
            && (last.starts_with("Final") || last.contains("final transcription delivered"))
        {
            ctx.paste_guard_until = Some(Instant::now() + std::time::Duration::from_millis(750));
        }
        if let Some(dl) = ctx.app.overlays.download.as_mut()
            && dl.model == "loading"
            && let Some(last) = ctx.app.feedback.logs.back()
        {
            dl.message = last.clone();
        }
    }
    while let Ok(b) = ctx.receivers.audio.rx_spec.try_recv() {
        ctx.app.feedback.spectrum_bars = b;
    }
    while let Ok(pre) = ctx.receivers.ui.rx_preview.try_recv() {
        ctx.app.feedback.preview_text = pre.clone();
        ctx.app.feedback.preview_text_updated_at = if pre.is_empty() {
            None
        } else {
            Some(Instant::now())
        };
    }

    // Auto-clear preview text after 500ms of no updates
    if let Some(last_update) = ctx.app.feedback.preview_text_updated_at
        && last_update.elapsed().as_millis() > 500
        && !ctx.app.feedback.preview_text.is_empty()
    {
        ctx.app.feedback.preview_text.clear();
        ctx.app.feedback.preview_text_updated_at = None;
    }
    // Size updates
    if let Some(rx) = ctx.catalog.rx_sizes_cached.as_ref() {
        while let Ok((id, sz)) = rx.try_recv() {
            if let Some(bytes) = sz {
                ctx.app.models.sizes.insert(id, bytes);
            }
        }
    }
    if let Some(rx) = ctx.catalog.rx_sizes_remote.as_ref() {
        while let Ok((id, sz)) = rx.try_recv() {
            if let Some(bytes) = sz {
                ctx.app.models.sizes.insert(id, bytes);
            }
        }
    }
    // VAD and hold
    while let Ok(v) = ctx.receivers.control.rx_vad.try_recv() {
        ctx.app.feedback.vad_open = v;
    }
    while let Ok(held) = ctx.receivers.control.rx_hold.try_recv() {
        ctx.app.feedback.hold_active = held;
        ctx.ptt.hold_flag.store(held, Ordering::Relaxed);
        if !held {
            ctx.app.feedback.vad_open = false;
            // Persist lifetime counters on PTT release
            let config = build_config_from_app(&ctx.app);
            let _ = storage::save_config(&config);
        }
        // Fallback: if mic label not yet set but pipeline exists, sync from pipeline
        if ctx.app.feedback.mic_name == "(not started)"
            && let Some(p) = ctx.pipeline.as_ref()
        {
            ctx.app.feedback.mic_name = p.mic_name.clone();
        }
    }
    while ctx.receivers.control.rx_ptt_evt.try_recv().is_ok() {
        ctx.ptt.last_event = Some(Instant::now());
    }
}

/// Drain download events and handle state transitions.
/// Downloads can be paused (ESC, keeps .partial) or cancelled (Backspace, deletes .partial).
pub fn drain_download_events(ctx: &mut RuntimeContext) -> DrainResult {
    let mut clear_dl = false;
    let mut events = Vec::new();

    // Collect events first to avoid borrow issues
    if let Some(dl_ctx) = ctx.download.as_ref() {
        while let Ok(evt) = dl_ctx.rx.try_recv() {
            events.push(evt);
        }
    }

    // Process events
    for evt in events {
        match evt {
            DownloadEvent::Started { model } => {
                ctx.log(format!("preparing model {}", model));
                ctx.app.overlays.download =
                    Some(DownloadState::new(model.clone(), "starting...".to_string()));
            }
            DownloadEvent::Stage { text } => {
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
                        let pct = ((bytes as f64 / t as f64) * 100.0).min(100.0);
                        let downloaded_mb = bytes as f64 / 1_000_000.0;
                        let total_mb = t as f64 / 1_000_000.0;
                        let eta_display = if eta_s >= 60.0 {
                            format!("{}m {}s", (eta_s as u32) / 60, (eta_s as u32) % 60)
                        } else {
                            format!("{:.0}s", eta_s)
                        };
                        // Store percentage for gauge rendering but don't show in text
                        format!(
                            "{:.0}% {} ({:.1} / {:.1} MB) • {:.1} MB/s • ETA {}",
                            pct, model_prefix, downloaded_mb, total_mb, mbps, eta_display
                        )
                    }
                    _ => format!("{} {:.1} MB...", model_prefix, bytes as f64 / 1_000_000.0),
                };
                if let Some(dl_state) = ctx.app.overlays.download.as_mut() {
                    dl_state.message = text;
                }
            }
            DownloadEvent::Done(Ok(())) => {
                // Add downloaded model to discovered list for checkmark
                if let Some(model_id) = ctx.app.downloading_model()
                    && !ctx.app.models.discovered.contains(&model_id.to_string())
                {
                    ctx.app.models.discovered.push(model_id.to_string());
                }
                ctx.app.overlays.download = None;
                clear_dl = true;
            }
            DownloadEvent::Done(Err(e)) => {
                ctx.app.overlays.download = None;
                clear_dl = true;
                ctx.app.overlays.error_message = Some(e);
            }
        }
    }

    // Handle pipeline start after processing all events
    if clear_dl {
        if let Some(dl_ctx) = ctx.download.take() {
            // Show loading overlay and spawn worker for phased startup
            ctx.app.overlays.download = Some(DownloadState::new(
                "loading".to_string(),
                "probing device…".to_string(),
            ));
            let selected_device = ctx
                .app
                .selections
                .devices
                .get(dl_ctx.pending_device_idx)
                .cloned();
            let config = dl_ctx.pending_config.clone();
            let eq_tuning = ctx.app.eq_tuning.clone();
            let vad = ctx.app.vad.clone();
            let rx = keyless_runtime::pipeline::spawn_startup_worker(
                config.clone(),
                eq_tuning.clone(),
                ctx.channels.ui.tx_log.clone(),
            );
            ctx.startup_rx = Some(rx);
            // Stash inputs for finalization once Ready arrives
            ctx.startup_model_cfg = Some((config, eq_tuning, vad, selected_device));
        }
        DrainResult::ClearDownload
    } else {
        // If a startup is in progress, process startup events and finalize when ready
        if ctx.startup_rx.is_some() {
            let mut clear_rx = false;
            while let Some(rx) = ctx.startup_rx.as_ref() {
                if let Ok(evt) = rx.try_recv() {
                    match evt {
                        keyless_runtime::pipeline::StartupEvent::PhaseStarted(name) => {
                            if let Some(dl) = ctx.app.overlays.download.as_mut() {
                                dl.message = format!("{}…", name);
                            }
                        }
                        keyless_runtime::pipeline::StartupEvent::PhaseOk(name) => {
                            if let Some(dl) = ctx.app.overlays.download.as_mut() {
                                dl.message = format!("{} ok", name);
                            }
                        }
                        keyless_runtime::pipeline::StartupEvent::PhaseErr(name, err) => {
                            ctx.app.overlays.download = None;
                            ctx.app.overlays.error_message =
                                Some(keyless_core::error::KeylessError::Other(format!(
                                    "{} failed: {}",
                                    name, err
                                )));
                            clear_rx = true;
                        }
                        keyless_runtime::pipeline::StartupEvent::Ready(artifacts) => {
                            let Some((config, _eq_tuning, vad, selected_device)) =
                                ctx.startup_model_cfg.take()
                            else {
                                clear_rx = true;
                                break;
                            };
                            // Build channels
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
                                    // Update mic name for UI before moving handle
                                    ctx.app.feedback.mic_name = ph.mic_name.clone();
                                    ctx.pipeline = Some(ph);
                                    ctx.app.screen = crate::tui::state::Screen::Dictating;
                                    ctx.ptt.enabled.store(true, Ordering::Relaxed);
                                    ctx.log("PTT enabled".to_string());
                                    ctx.ptt.last_event = None;
                                }
                                Err(e) => {
                                    ctx.app.overlays.download = None;
                                    ctx.app.overlays.error_message = Some(e);
                                }
                            }
                            clear_rx = true;
                        }
                    }
                } else {
                    break;
                }
            }
            if clear_rx {
                ctx.startup_rx = None;
            }
        }
        DrainResult::Continue
    }
}
