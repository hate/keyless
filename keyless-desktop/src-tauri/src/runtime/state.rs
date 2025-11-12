use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, OnceLock, mpsc};

/// Consolidated runtime state tracking.
pub struct RuntimeState {
    /// Guards concurrent initialization attempts; ensures only one init runs at a time.
    pub initializing: AtomicBool,
    /// Tracks if runtime has been initialized already.
    pub initialized: AtomicBool,
    /// Tracks whether PTT is currently held (listening).
    pub listening: AtomicBool,
    /// Tracks whether the audio pipeline is currently active (started successfully).
    pub pipeline_active: AtomicBool,
    /// Session-level word counter (resets on app start).
    pub session_words: AtomicU64,
    /// Session-level talk-time counter in milliseconds.
    pub session_talk_ms: AtomicU64,
    /// When the paste guard expires (ms since UNIX_EPOCH, 0 = inactive).
    pub paste_guard_until_ms: AtomicU64,
    /// Timestamp of the most recent PTT heartbeat event (ms since UNIX_EPOCH).
    pub last_heartbeat_ms: AtomicU64,
    /// Tracks whether the PTT listener is currently enabled.
    pub ptt_enabled: AtomicBool,
}

impl RuntimeState {
    /// Create a new runtime state with all fields initialized to defaults.
    fn new() -> Self {
        Self {
            initializing: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
            listening: AtomicBool::new(false),
            pipeline_active: AtomicBool::new(false),
            session_words: AtomicU64::new(0),
            session_talk_ms: AtomicU64::new(0),
            paste_guard_until_ms: AtomicU64::new(0),
            last_heartbeat_ms: AtomicU64::new(0),
            ptt_enabled: AtomicBool::new(false),
        }
    }
}

/// Global runtime state (initialized once).
pub static RUNTIME_STATE: OnceLock<Arc<RuntimeState>> = OnceLock::new();

/// Get or initialize the runtime state.
pub fn runtime_state() -> Arc<RuntimeState> {
    // Get the global runtime state, initializing it if this is the first call.
    // Uses OnceLock to ensure thread-safe one-time initialization.
    Arc::clone(RUNTIME_STATE.get_or_init(|| Arc::new(RuntimeState::new())))
}

/// Commands sent to the runtime loop for live control.
pub enum RuntimeCommand {
    /// Request the audio pipeline to restart (e.g., after device change).
    RestartPipeline,
}

/// Sender to post runtime commands from other threads (bridge/UI).
pub static RUNTIME_TX: OnceLock<mpsc::Sender<RuntimeCommand>> = OnceLock::new();

/// PTT controller state (desktop runtime), mirrors the TUI wiring.
pub struct PttController {
    /// Atomic flag set when PTT hotkey is held.
    pub hold_flag: Arc<AtomicBool>,
    /// Current preset mode (0 = disabled, 1 = hold, etc.).
    pub preset_mode: Arc<std::sync::atomic::AtomicU8>,
    /// Flag to signal the listener thread to stop.
    pub stop_flag: Arc<AtomicBool>,
    /// Whether PTT listener is currently enabled.
    pub enabled: Arc<AtomicBool>,
    /// Sender ends used to wire the rdev listener when started.
    pub tx_hold: mpsc::SyncSender<bool>,
    /// Sender for heartbeat events.
    pub tx_evt: mpsc::SyncSender<()>,
    /// Listener thread (spawned lazily after pipeline starts).
    pub listener: Option<std::thread::JoinHandle<()>>,
    /// Internal dispatcher that bridges hold/heartbeat to UI and hold_flag
    pub _dispatch: std::thread::JoinHandle<()>,
}

/// Global PTT controller (initialized once).
pub static PTT: OnceLock<Mutex<PttController>> = OnceLock::new();
