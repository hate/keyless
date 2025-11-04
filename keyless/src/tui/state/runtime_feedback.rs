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
            // Placeholder until pipeline starts and provides actual device name.
            mic_name: String::from("(not started)"),
            hold_active: false,
            vad_open: false,
            preview_text: String::new(),
            preview_text_updated_at: None,
            // Capacity 120 ≈ 3 seconds at 40 FPS (assuming one sample per frame).
            // Fixed-size circular buffer; oldest samples evicted via pop_front when full.
            rms_history: VecDeque::with_capacity(120),
            // Spectrum bars populated by audio worker; empty until first update.
            spectrum_bars: Vec::new(),
            // Capacity 200 keeps ~200 log lines in memory; oldest evicted when full.
            // Sized for typical session length without excessive memory usage.
            logs: VecDeque::with_capacity(200),
            session_words: 0,
            session_talk_ms: 0,
            // None until first tick; used to calculate delta for talk time accumulation.
            last_tick: None,
        }
    }
}

impl Default for RuntimeFeedback {
    fn default() -> Self {
        Self::new()
    }
}
