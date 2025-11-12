//! Bridge runtime events to the Tauri frontend (logs, stats, spectrum, preview, VAD).
use super::events::{emit_event, emit_log, with_events};
use super::state::runtime_state;
use keyless_core::config::{Config, storage};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Mutable counters for session/lifetime stats used while bridging events.
#[derive(Clone, Debug, Default)]
struct StatsState {
    /// Words typed in the current session.
    session_words: u64,
    /// Milliseconds spoken in the current session.
    session_talk_ms: u64,
    /// Persisted lifetime word count.
    lifetime_words: u64,
    /// Persisted lifetime talk time in milliseconds.
    lifetime_talk_ms: u64,
}

impl StatsState {
    /// Create new stats counters with existing lifetime totals.
    fn new(lifetime_words: u64, lifetime_talk_ms: u64) -> Self {
        Self {
            session_words: 0,
            session_talk_ms: 0,
            lifetime_words,
            lifetime_talk_ms,
        }
    }

    /// Add words; returns true if a non-zero increment was applied.
    fn add_words(&mut self, words: u64) -> bool {
        if words == 0 {
            return false;
        }
        self.session_words = self.session_words.saturating_add(words);
        self.lifetime_words = self.lifetime_words.saturating_add(words);
        true
    }

    /// Add elapsed talk time; returns true if a non-zero increment was applied.
    fn add_talk_ms(&mut self, delta_ms: u64) -> bool {
        if delta_ms == 0 {
            return false;
        }
        self.session_talk_ms = self.session_talk_ms.saturating_add(delta_ms);
        self.lifetime_talk_ms = self.lifetime_talk_ms.saturating_add(delta_ms);
        true
    }

    /// Get current session words.
    fn session_words(&self) -> u64 {
        self.session_words
    }

    /// Get current session talk milliseconds.
    fn session_talk_ms(&self) -> u64 {
        self.session_talk_ms
    }

    /// Get lifetime words.
    fn lifetime_words(&self) -> u64 {
        self.lifetime_words
    }

    /// Get lifetime talk milliseconds.
    fn lifetime_talk_ms(&self) -> u64 {
        self.lifetime_talk_ms
    }

    /// Build a serializable snapshot payload.
    fn snapshot(&self) -> StatsPayload {
        StatsPayload {
            session_words: self.session_words,
            session_talk_ms: self.session_talk_ms,
            lifetime_words: self.lifetime_words,
            lifetime_talk_ms: self.lifetime_talk_ms,
        }
    }
}

/// Current epoch time in milliseconds.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Serializable stats payload emitted to the frontend.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsPayload {
    /// Session word count.
    session_words: u64,
    /// Session talk time in milliseconds.
    session_talk_ms: u64,
    /// Lifetime word count.
    lifetime_words: u64,
    /// Lifetime talk time in milliseconds.
    lifetime_talk_ms: u64,
}

/// Paste guard payload indicating the guard deadline timestamp (ms since epoch).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PasteGuardPayload {
    /// Deadline (ms since epoch) until paste guard is active; 0 to clear.
    until: u64,
}

/// Permission warning payload emitted when the hotkey listener appears stale.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionWarningPayload<'a> {
    /// Whether the warning is currently active.
    active: bool,
    /// Human-readable warning message.
    message: &'a str,
    /// Timestamp (ms since epoch) when the warning state changed.
    timestamp_ms: u64,
}

/// Persist lifetime statistics to the configuration file.
/// This saves the total word count and talk time across all sessions.
fn persist_lifetime_counters(lifetime_words: u64, lifetime_talk_ms: u64) {
    // Load current config (or defaults if none exists).
    let mut cfg: Config = storage::load_config().unwrap_or_default();
    // Update lifetime counters (these persist across app restarts).
    cfg.lifetime_words = lifetime_words;
    cfg.lifetime_talk_ms = lifetime_talk_ms;
    // Save the updated config to disk.
    if let Err(e) = storage::save_config(&cfg) {
        // Log error but don't crash (stats persistence failure shouldn't break the app).
        let msg = format!("failed to persist lifetime stats: {e}");
        eprintln!("[runtime] {msg}");
        emit_log(msg);
    }
}

/// Bridge pipeline events to Tauri frontend.
pub fn bridge_events(
    rx_level: mpsc::Receiver<u16>,
    rx_log: mpsc::Receiver<String>,
    rx_spec: mpsc::Receiver<Vec<u16>>,
    rx_preview: mpsc::Receiver<String>,
    rx_vad: mpsc::Receiver<bool>,
) {
    let mut eq_stream_started = false;
    static EQ_LOGGED: OnceLock<AtomicBool> = OnceLock::new();
    let mut vad_open = false;

    let mut stats = if let Some(cfg) = storage::load_config() {
        StatsState::new(cfg.lifetime_words, cfg.lifetime_talk_ms)
    } else {
        StatsState::new(0, 0)
    };

    let state = runtime_state();
    state.session_words.store(0, Ordering::Relaxed);
    state.session_talk_ms.store(0, Ordering::Relaxed);

    let mut toast_hide_deadline: Option<Instant> = None;

    state.paste_guard_until_ms.store(0, Ordering::Relaxed);
    let guard_atomic = &state.paste_guard_until_ms;
    let heartbeat_atomic = &state.last_heartbeat_ms;

    let mut last_vad_logged: Option<bool> = None;
    let mut last_guard_ms: u64 = 0;
    let mut last_emitted = (0u64, 0u64, stats.lifetime_words(), stats.lifetime_talk_ms());
    let mut last_tick = Instant::now();
    let mut guard_until: Option<Instant> = None;
    let mut stats_dirty = true;
    let mut stats_dirty_for_talk = false;
    let mut heartbeat_warned = false;
    let mut last_hold = state.listening.load(Ordering::Relaxed);
    // If we receive an explicit "Words: N" message, suppress counting on the next "Final: ..." log.
    let mut suppress_final_once = false;

    // Main event loop: process all pipeline events and forward to frontend.
    loop {
        // Track if we processed any events this iteration (for sleep optimization).
        let mut any_alive = false;

        // Process audio level updates (VU meter).
        if let Ok(level) = rx_level.try_recv() {
            any_alive = true;
            // Forward audio level to frontend for VU meter display.
            emit_event("audio_level", level);
        }

        // Process log messages from the pipeline.
        if let Ok(msg) = rx_log.try_recv() {
            any_alive = true;
            eprintln!("[backend] {}", msg);
            
            // Handle explicit word count messages ("Words: N").
            // These are sent by the pipeline when it knows the exact word count.
            if let Some(n) = msg.strip_prefix("Words: ")
                && let Ok(words) = n.trim().parse::<u64>()
                && stats.add_words(words)
            {
                // Update session counter in runtime state (accessible to other threads).
                state
                    .session_words
                    .store(stats.session_words(), Ordering::Relaxed);
                // Persist lifetime totals to disk immediately.
                persist_lifetime_counters(stats.lifetime_words(), stats.lifetime_talk_ms());
                // Emit stats update to frontend.
                let sw = stats.session_words();
                let stm = stats.session_talk_ms();
                let lw = stats.lifetime_words();
                let ltm = stats.lifetime_talk_ms();
                emit_event(
                    "stats_update",
                    StatsPayload {
                        session_words: sw,
                        session_talk_ms: stm,
                        lifetime_words: lw,
                        lifetime_talk_ms: ltm,
                    },
                );
                last_emitted = (sw, stm, lw, ltm);
                stats_dirty = true;
                // Suppress word counting on the next "Final: ..." message (avoid double-counting).
                suppress_final_once = true;
            } 
            // Handle final transcription messages ("Final: {text}").
            // These are sent when the pipeline outputs the final transcribed text.
            else if let Some(text) = msg.strip_prefix("Final: ") {
                // Check if we should suppress counting (already counted via "Words: N").
                if suppress_final_once {
                    // Already accounted for by a preceding "Words: N" message.
                    suppress_final_once = false;
                } else {
                    // Count words in the final text and update stats.
                    let words = text.split_whitespace().count() as u64;
                    if stats.add_words(words) {
                        // Update session counter immediately.
                        state
                            .session_words
                            .store(stats.session_words(), Ordering::Relaxed);
                        // Persist lifetime totals immediately after final output.
                        persist_lifetime_counters(stats.lifetime_words(), stats.lifetime_talk_ms());
                        // Emit stats update right away (no visible delay).
                        let sw = stats.session_words();
                        let stm = stats.session_talk_ms();
                        let lw = stats.lifetime_words();
                        let ltm = stats.lifetime_talk_ms();
                        emit_event(
                            "stats_update",
                            StatsPayload {
                                session_words: sw,
                                session_talk_ms: stm,
                                lifetime_words: lw,
                                lifetime_talk_ms: ltm,
                            },
                        );
                        last_emitted = (sw, stm, lw, ltm);
                        // Keep dirty flag so later logic can coalesce with talk-time updates in same tick.
                        stats_dirty = true;
                    }
                }
                // Emit toast notification for final transcription (shows transcribed text).
                let sink_name = match storage::load_config() {
                    Some(cfg) => match cfg.output_mode {
                        keyless_core::config::OutputMode::Paste => "Paste".to_string(),
                        keyless_core::config::OutputMode::Clipboard => "Clipboard".to_string(),
                        keyless_core::config::OutputMode::File(_) => "File".to_string(),
                    },
                    None => "Paste".to_string(),
                };
                let toast_payload = serde_json::json!({
                    "text": text,
                    "wordCount": text.split_whitespace().count(),
                    "sink": sink_name,
                });
                eprintln!("[toast] Emitting final_transcription: {:?}", toast_payload);
                emit_event("final_transcription", toast_payload);
                // Hide recording overlay and show toast window.
                with_events(|events| {
                    let app_handle = events.app_handle();
                    crate::overlay::hide_overlay(&app_handle);
                    crate::overlay::show_toast(&app_handle);
                });
                // Schedule toast to auto-hide after 5.3 seconds.
                toast_hide_deadline = Some(Instant::now() + Duration::from_millis(5300));
                
                // Activate paste guard for 750ms after finalization.
                // This prevents accidental pastes immediately after transcription completes.
                let deadline = Instant::now() + Duration::from_millis(750);
                guard_until = Some(deadline);
                let until_ms = now_millis().saturating_add(750);
                guard_atomic.store(until_ms, Ordering::Relaxed);
                // Emit paste guard event if timestamp changed (avoid duplicate events).
                if last_guard_ms != until_ms {
                    emit_event("paste_guard", PasteGuardPayload { until: until_ms });
                    last_guard_ms = until_ms;
                }
            } else if let Some(rest) = msg.strip_prefix("Final → ") {
                // Format from keyless-runtime events: "Final → {sink}: {text}"
                // Extract sink and text
                if let Some(pos) = rest.find(": ") {
                    let sink_name = &rest[..pos];
                    let text = &rest[pos + 2..];

                    // Emit toast notification for final transcription
                    let toast_payload = serde_json::json!({
                        "text": text,
                        "wordCount": text.split_whitespace().count(),
                        "sink": sink_name,
                    });
                    eprintln!("[toast] Emitting final_transcription: {:?}", toast_payload);
                    emit_event("final_transcription", toast_payload);

                    // Hide the pill overlay and show the toast window
                    with_events(|events| {
                        let app_handle = events.app_handle();
                        crate::overlay::hide_overlay(&app_handle);
                        crate::overlay::show_toast(&app_handle);
                    });
                    toast_hide_deadline = Some(Instant::now() + Duration::from_millis(5300));

                    if suppress_final_once {
                        suppress_final_once = false;
                    } else {
                        let words = text.split_whitespace().count() as u64;
                        if stats.add_words(words) {
                            state
                                .session_words
                                .store(stats.session_words(), Ordering::Relaxed);
                            persist_lifetime_counters(
                                stats.lifetime_words(),
                                stats.lifetime_talk_ms(),
                            );
                            let sw = stats.session_words();
                            let stm = stats.session_talk_ms();
                            let lw = stats.lifetime_words();
                            let ltm = stats.lifetime_talk_ms();
                            emit_event(
                                "stats_update",
                                StatsPayload {
                                    session_words: sw,
                                    session_talk_ms: stm,
                                    lifetime_words: lw,
                                    lifetime_talk_ms: ltm,
                                },
                            );
                            last_emitted = (sw, stm, lw, ltm);
                            stats_dirty = true;

                            // Paste guard same as in the "Final: " path
                            let deadline = Instant::now() + Duration::from_millis(750);
                            guard_until = Some(deadline);
                            let until_ms = now_millis().saturating_add(750);
                            guard_atomic.store(until_ms, Ordering::Relaxed);
                            if last_guard_ms != until_ms {
                                emit_event("paste_guard", PasteGuardPayload { until: until_ms });
                                last_guard_ms = until_ms;
                            }
                        }
                    }
                }
            }
            emit_log(msg);
        }

        // Process EQ spectrum updates (frequency bars for visualizer).
        if let Ok(bars) = rx_spec.try_recv() {
            any_alive = true;
            // Log EQ sample bars once (for debugging, avoid spam).
            let once = EQ_LOGGED.get_or_init(|| AtomicBool::new(false));
            if !once.load(Ordering::Relaxed) && !bars.is_empty() {
                eprintln!(
                    "[eq] sample bars: {:?}",
                    bars.iter().take(8).cloned().collect::<Vec<_>>()
                );
                once.store(true, Ordering::Relaxed);
            }
            // Forward EQ bars to frontend for visualizer display.
            emit_event("eq_update", &bars);
            // Log when EQ stream starts (first update received).
            if !eq_stream_started {
                eq_stream_started = true;
                eprintln!("[eq] stream started");
                emit_log("eq_update stream started".to_string());
            }
        }

        // Process partial transcription preview (real-time text as user speaks).
        if let Ok(txt) = rx_preview.try_recv() {
            any_alive = true;
            eprintln!("[preview] {}", txt);
            // Forward partial transcription to frontend (for real-time preview display).
            emit_event("transcript_partial", txt);
        }

        // Process VAD (Voice Activity Detection) state updates.
        if let Ok(speaking) = rx_vad.try_recv() {
            any_alive = true;
            vad_open = speaking;
            eprintln!("[vad] {}", if speaking { "open" } else { "closed" });
            // Forward VAD state to frontend (for UI indicators).
            emit_event("vad_speaking", speaking);
            // If speaking, cancel toast auto-hide and ensure toast is visible.
            if speaking {
                toast_hide_deadline = None;
                with_events(|events| {
                    crate::overlay::show_toast(&events.app_handle());
                });
            }
            // Log VAD state changes (edge detection, avoid spam).
            if last_vad_logged.map(|v| v != speaking).unwrap_or(true) {
                last_vad_logged = Some(speaking);
                let msg = if speaking { "VAD open" } else { "VAD closed" }.to_string();
                emit_log(msg);
            }
        }

        // Periodic processing: handle timers and update talk time.
        let now = Instant::now();
        let delta_ms = now.saturating_duration_since(last_tick).as_millis() as u64;
        last_tick = now;

        if delta_ms > 0 {
            // Check if paste guard has expired (750ms after final transcription).
            if let Some(deadline) = guard_until
                && now >= deadline
            {
                guard_until = None;
                guard_atomic.store(0, Ordering::Relaxed);
                // Emit guard cleared event if it was active.
                if last_guard_ms != 0 {
                    emit_event("paste_guard", PasteGuardPayload { until: 0 });
                    last_guard_ms = 0;
                }
            }

            // Check if toast should be auto-hidden (5.3 seconds after final transcription).
            if let Some(deadline) = toast_hide_deadline
                && now >= deadline
            {
                toast_hide_deadline = None;
                // Hide toast window.
                with_events(|events| {
                    crate::overlay::hide_toast(&events.app_handle());
                });
            }

            // Update talk time if PTT is held and VAD is open (user is speaking).
            // Talk time accumulates while the user is actively speaking.
            if state.listening.load(Ordering::Relaxed) && vad_open && stats.add_talk_ms(delta_ms) {
                // Update session talk time in runtime state.
                state
                    .session_talk_ms
                    .store(stats.session_talk_ms(), Ordering::Relaxed);
                stats_dirty = true;
                // Mark talk time as dirty for persistence on PTT release.
                stats_dirty_for_talk = true;
            }
        }

        // Persist talk time when PTT is released (edge detection).
        // We only persist on release to avoid excessive disk writes during long sessions.
        let hold_now = state.listening.load(Ordering::Relaxed);
        if last_hold && !hold_now && stats_dirty_for_talk {
            // PTT was just released: persist lifetime counters.
            persist_lifetime_counters(stats.lifetime_words(), stats.lifetime_talk_ms());
            stats_dirty_for_talk = false;
        }
        last_hold = hold_now;

        // Emit stats updates if dirty (coalescing: only emit if values changed).
        if stats_dirty {
            let snapshot = stats.snapshot();
            let current = (
                snapshot.session_words,
                snapshot.session_talk_ms,
                snapshot.lifetime_words,
                snapshot.lifetime_talk_ms,
            );
            // Only emit if stats actually changed (avoid duplicate events).
            if current != last_emitted {
                emit_event("stats_update", snapshot);
                last_emitted = current;
            }
            stats_dirty = false;
        }

        // Check for stale PTT heartbeat (indicates missing Accessibility permission).
        // If PTT is enabled but no heartbeats received in 5 seconds, warn the user.
        let now_epoch = now_millis();
        let last_hb = heartbeat_atomic.load(Ordering::Relaxed);
        let enabled = state.ptt_enabled.load(Ordering::Relaxed);
        // Stale if enabled but no heartbeat (last_hb == 0) or heartbeat is >5s old.
        let stale = enabled && (last_hb == 0 || now_epoch.saturating_sub(last_hb) >= 5_000);

        // Emit permission warning if heartbeat is stale and we haven't warned yet.
        if stale && !heartbeat_warned {
            emit_event(
                "permission_warning",
                PermissionWarningPayload {
                    active: true,
                    message: "Global hotkey listener inactive. On macOS, enable Accessibility for keyless (System Settings → Privacy & Security → Accessibility).",
                    timestamp_ms: now_epoch,
                },
            );
            heartbeat_warned = true;
        } 
        // Clear warning if heartbeat is no longer stale or PTT is disabled.
        else if heartbeat_warned && (!stale || !enabled) {
            emit_event(
                "permission_warning",
                PermissionWarningPayload {
                    active: false,
                    message: "",
                    timestamp_ms: now_epoch,
                },
            );
            heartbeat_warned = false;
        }

        // If no events processed this iteration, sleep briefly to avoid busy-waiting.
        if !any_alive {
            thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StatsState;

    #[test]
    fn stats_state_tracks_words_and_talk_time() {
        let mut stats = StatsState::new(10, 100);
        assert!(!stats.add_words(0));
        assert!(stats.add_words(5));
        assert_eq!(stats.session_words(), 5);
        assert_eq!(stats.lifetime_words(), 15);

        assert!(!stats.add_talk_ms(0));
        assert!(stats.add_talk_ms(200));
        assert_eq!(stats.session_talk_ms(), 200);
        assert_eq!(stats.lifetime_talk_ms(), 300);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.session_words, 5);
        assert_eq!(snapshot.session_talk_ms, 200);
        assert_eq!(snapshot.lifetime_words, 15);
        assert_eq!(snapshot.lifetime_talk_ms, 300);
    }
}
