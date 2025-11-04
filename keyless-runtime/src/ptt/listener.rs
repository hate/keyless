//! Global push-to-talk (PTT) listener utilities.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, mpsc::SyncSender};

use keyless_core::error::KeylessResult;

/// Spawn the global hotkey listener thread.
///
/// - `preset_mode`: 0 = ctrl+option/alt, 1 = ctrl+shift
/// - `stop_flag`: when set, the handler becomes a no-op (soft stop)
/// - `tx_hold`: emits `true` when the combo is held, `false` when released
pub trait PttBackend: Send + Sync + 'static {
    /// Run the backend loop and produce hold/heartbeat events.
    ///
    /// Implementations should block and listen for global key events,
    /// sending `true` on hold and `false` on release to `tx_hold`, and
    /// periodically sending an empty event on `tx_evt` as a heartbeat.
    fn run(
        &self,
        preset_mode: Arc<AtomicU8>,
        enabled_flag: Arc<AtomicBool>,
        stop_flag: Arc<AtomicBool>,
        tx_hold: SyncSender<bool>,
        tx_evt: SyncSender<()>,
    ) -> std::thread::JoinHandle<()>;
}

/// Default `rdev`-based backend.
struct RealBackend;

impl PttBackend for RealBackend {
    fn run(
        &self,
        preset_mode: Arc<AtomicU8>,
        enabled_flag: Arc<AtomicBool>,
        stop_flag: Arc<AtomicBool>,
        tx_hold: SyncSender<bool>,
        tx_evt: SyncSender<()>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            use rdev::{Event, EventType, Key, listen};

            // Track modifier key states (updated on every key press/release).
            // Must track all three independently to support both preset combos.
            let mut ctrl = false;
            let mut alt = false;
            let mut shift = false;
            // Previous hotkey combo state (for edge detection; only emit on state change).
            let mut last_state = false;

            let handler = move |event: Event| {
                // Respect stop and enabled flags (let the app disable PTT dynamically).
                // Relaxed ordering is sufficient (no other synchronization needed).
                if stop_flag.load(Ordering::Relaxed) || !enabled_flag.load(Ordering::Relaxed) {
                    return;
                }
                // Heartbeat: any key event = listener is alive (used for UI stale detection).
                let _ = tx_evt.try_send(());

                // Update modifier key states based on press/release.
                // We support two presets:
                // - Preset 0 (mode=0): Control + Option/Alt (MetaLeft/Right on macOS)
                // - Preset 1 (mode=1): Control + Shift
                // We must track Ctrl, Alt, and Shift independently to detect both combos.
                match event.event_type {
                    EventType::KeyPress(k) => match k {
                        // Track both left and right variants (user may press either).
                        Key::ControlLeft | Key::ControlRight => ctrl = true,
                        // Alt/Option keys: AltGr is right Alt, Meta is Option on macOS.
                        Key::Alt | Key::AltGr | Key::MetaLeft | Key::MetaRight => alt = true,
                        Key::ShiftLeft | Key::ShiftRight => shift = true,
                        _ => {}
                    },
                    EventType::KeyRelease(k) => match k {
                        // Reset state on release (combo only active while all modifiers held).
                        Key::ControlLeft | Key::ControlRight => ctrl = false,
                        Key::Alt | Key::AltGr | Key::MetaLeft | Key::MetaRight => alt = false,
                        Key::ShiftLeft | Key::ShiftRight => shift = false,
                        _ => {}
                    },
                    _ => {}
                }

                // Check if the current preset's hotkey combo is active.
                // Read preset_mode atomic (0 or 1) to determine which combo the user wants.
                // Relaxed ordering is sufficient (no other synchronization needed).
                let mode = preset_mode.load(Ordering::Relaxed);
                let need_shift = mode == 1; // Preset 1: Ctrl+Shift
                let need_alt = mode == 0; // Preset 0: Ctrl+Alt/Option

                // Determine if combo is held based on current preset and modifier states.
                let held = if need_shift {
                    ctrl && shift
                } else if need_alt {
                    ctrl && alt
                } else {
                    false
                };

                // Edge detection: only send when state changes (avoid flooding the channel).
                // Pipeline only needs to know when combo transitions (pressed → released or vice versa).
                if held != last_state {
                    last_state = held;
                    // true = pressed (combo active), false = released (combo inactive).
                    let _ = tx_hold.try_send(held);
                }
            };
            let _ = listen(handler);
        })
    }
}

/// Spawn the real global hotkey listener with the default backend.
pub fn spawn_listener(
    preset_mode: Arc<AtomicU8>,
    enabled_flag: Arc<AtomicBool>,
    stop_flag: Arc<AtomicBool>,
    tx_hold: SyncSender<bool>,
    tx_evt: SyncSender<()>,
) -> KeylessResult<std::thread::JoinHandle<()>> {
    // Use real backend by default (rdev-based implementation for production).
    Ok(spawn_with_backend(
        RealBackend,
        preset_mode,
        enabled_flag,
        stop_flag,
        tx_hold,
        tx_evt,
    ))
}

/// Spawn the global hotkey listener with a custom backend (for tests).
pub fn spawn_with_backend<B: PttBackend>(
    backend: B,
    preset_mode: Arc<AtomicU8>,
    enabled_flag: Arc<AtomicBool>,
    stop_flag: Arc<AtomicBool>,
    tx_hold: SyncSender<bool>,
    tx_evt: SyncSender<()>,
) -> std::thread::JoinHandle<()> {
    backend.run(preset_mode, enabled_flag, stop_flag, tx_hold, tx_evt)
}
