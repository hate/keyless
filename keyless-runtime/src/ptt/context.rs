//! Push-to-talk context (flags, handle, timing).

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, atomic::AtomicU8};
use std::time::Instant;

/// PTT infrastructure and state.
pub struct PttContext {
    /// Shared flag toggled by the hotkey listener when PTT is held.
    pub hold_flag: Arc<AtomicBool>,
    /// 0 = ctrl+option/alt, 1 = ctrl+shift preset.
    pub preset_mode: Arc<AtomicU8>,
    /// Soft stop for the backend loop.
    pub stop_flag: Arc<AtomicBool>,
    /// Whether the listener is enabled.
    pub enabled: Arc<AtomicBool>,
    /// JoinHandle for the listener thread.
    pub handle: std::thread::JoinHandle<()>,
    /// Timestamp of the last key event (used for heartbeat UI).
    pub last_event: Option<Instant>,
}

impl PttContext {
    /// Construct a new PTT context.
    pub fn new(
        hold_flag: Arc<AtomicBool>,
        preset_mode: Arc<AtomicU8>,
        stop_flag: Arc<AtomicBool>,
        enabled: Arc<AtomicBool>,
        handle: std::thread::JoinHandle<()>,
    ) -> Self {
        Self {
            hold_flag,
            preset_mode,
            stop_flag,
            enabled,
            handle,
            last_event: None,
        }
    }
}
