//! Runtime feedback from workers (for Dictating screen rendering).

use std::collections::VecDeque;
use std::time::Instant;

/// Live feedback from audio/whisper workers.
pub struct RuntimeFeedback {
    pub mic_name: String,
    pub hold_active: bool,
    pub vad_open: bool,
    pub preview_text: String,
    pub preview_text_updated_at: Option<Instant>,
    pub rms_history: VecDeque<u16>,
    pub spectrum_bars: Vec<u16>,
    pub logs: VecDeque<String>,
    /// Count of words transcribed in the current app session.
    pub session_words: u64,
    /// Milliseconds of talk time in the current app session.
    pub session_talk_ms: u64,
    /// Last tick instant used for accumulating talk time.
    pub last_tick: Option<Instant>,
}

impl RuntimeFeedback {
    pub fn new() -> Self {
        Self {
            mic_name: String::from("(not started)"),
            hold_active: false,
            vad_open: false,
            preview_text: String::new(),
            preview_text_updated_at: None,
            rms_history: VecDeque::with_capacity(120),
            spectrum_bars: Vec::new(),
            logs: VecDeque::with_capacity(200),
            session_words: 0,
            session_talk_ms: 0,
            last_tick: None,
        }
    }
}

impl Default for RuntimeFeedback {
    fn default() -> Self {
        Self::new()
    }
}
