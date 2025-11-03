//! Channel organization for RuntimeContext.

use std::sync::mpsc;

/// All channels used by the TUI runtime, semantically grouped.
pub struct Channels {
    /// Senders for audio-related data (RMS levels, spectrum bars).
    pub audio: AudioChannels,
    /// Senders for UI data (logs, preview text).
    pub ui: UiChannels,
    /// Senders for control signals (VAD state, hold, PTT events).
    pub control: ControlChannels,
}

impl Channels {
    /// Create the full channel set with paired receivers.
    pub fn new() -> (Self, ChannelReceivers) {
        let (tx_level, rx_level) = mpsc::sync_channel::<u16>(64);
        let (tx_spec, rx_spec) = mpsc::sync_channel::<Vec<u16>>(64);
        let (tx_log, rx_log) = mpsc::sync_channel::<String>(256);
        let (tx_preview, rx_preview) = mpsc::sync_channel::<String>(64);
        let (tx_vad, rx_vad) = mpsc::sync_channel::<bool>(16);
        let (tx_hold, rx_hold) = mpsc::sync_channel::<bool>(8);
        let (tx_ptt_evt, rx_ptt_evt) = mpsc::sync_channel::<()>(8);

        let senders = Self {
            audio: AudioChannels { tx_level, tx_spec },
            ui: UiChannels { tx_log, tx_preview },
            control: ControlChannels {
                tx_vad,
                tx_hold,
                tx_ptt_evt,
            },
        };

        let receivers = ChannelReceivers {
            audio: AudioChannelReceivers { rx_level, rx_spec },
            ui: UiChannelReceivers { rx_log, rx_preview },
            control: ControlChannelReceivers {
                rx_vad,
                rx_hold,
                rx_ptt_evt,
            },
        };

        (senders, receivers)
    }
}

/// Audio-related channels (RMS levels, spectrum).
pub struct AudioChannels {
    /// RMS level → TUI (0..100 scaled).
    pub tx_level: mpsc::SyncSender<u16>,
    /// Spectrum bars → TUI.
    pub tx_spec: mpsc::SyncSender<Vec<u16>>,
}

/// UI-related channels (logs, preview text).
pub struct UiChannels {
    /// Log lines → TUI.
    pub tx_log: mpsc::SyncSender<String>,
    /// Preview text → TUI.
    pub tx_preview: mpsc::SyncSender<String>,
}

/// Control-related channels (VAD, hold, PTT events).
pub struct ControlChannels {
    /// VAD open/closed → TUI.
    pub tx_vad: mpsc::SyncSender<bool>,
    /// Hold state (PTT) → TUI.
    pub tx_hold: mpsc::SyncSender<bool>,
    /// PTT event heartbeat → TUI.
    pub tx_ptt_evt: mpsc::SyncSender<()>,
}

/// Receivers (separated for initialization).
pub struct ChannelReceivers {
    /// Receivers for audio-related channels.
    pub audio: AudioChannelReceivers,
    /// Receivers for UI-related channels.
    pub ui: UiChannelReceivers,
    /// Receivers for control-related channels.
    pub control: ControlChannelReceivers,
}

/// Receivers for audio channels.
pub struct AudioChannelReceivers {
    /// RMS level ← audio callback.
    pub rx_level: mpsc::Receiver<u16>,
    /// Spectrum bars ← audio callback.
    pub rx_spec: mpsc::Receiver<Vec<u16>>,
}

/// Receivers for UI channels.
pub struct UiChannelReceivers {
    /// Log lines ← runtime.
    pub rx_log: mpsc::Receiver<String>,
    /// Preview text ← transcriber.
    pub rx_preview: mpsc::Receiver<String>,
}

/// Receivers for control channels.
pub struct ControlChannelReceivers {
    /// VAD open/closed ← audio callback.
    pub rx_vad: mpsc::Receiver<bool>,
    /// Hold flag ← hotkey listener.
    pub rx_hold: mpsc::Receiver<bool>,
    /// PTT heartbeat ← hotkey listener.
    pub rx_ptt_evt: mpsc::Receiver<()>,
}
