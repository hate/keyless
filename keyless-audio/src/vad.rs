//! Voice Activity Detection (VAD) gate.
//!
//! Stateful gate that decides whether to forward a frame based on:
//! - Start/stop thresholds in dB (hysteresis to avoid flicker)
//! - Minimum speech duration (attack) before opening
//! - Maximum silence hangover before closing
//!
//! The runtime drives this gate with per-frame dB levels derived from RMS.

/// Hysteresis/timing-based speech gate for streaming audio.
pub struct VadGate {
    /// Start threshold (dB) to begin counting towards open.
    start_db: f32,
    /// Stop threshold (dB) to begin counting quiet hangover.
    stop_db: f32,
    /// Minimum duration above start (ms) before opening.
    min_ms: f32,
    /// Maximum allowed silence (ms) before closing.
    max_silence_ms: f32,
    /// Frame duration (ms) used for timers.
    frame_ms: f32,
    /// Current open/closed state.
    speaking: bool,
    /// Accumulated time above start threshold.
    above_ms: f32,
    /// Accumulated time below stop threshold.
    quiet_ms: f32,
}

impl VadGate {
    /// Create a new gate with thresholds and timing (in ms) and a fixed frame duration.
    pub fn new(
        start_db: f32,
        stop_db: f32,
        min_ms: u16,
        max_silence_ms: u16,
        frame_ms: f32,
    ) -> Self {
        // Frame time is fixed by the audio frame size; it drives the timers.
        Self {
            start_db,
            stop_db,
            min_ms: min_ms as f32,
            max_silence_ms: max_silence_ms as f32,
            frame_ms,
            speaking: false,
            above_ms: 0.0,
            quiet_ms: 0.0,
        }
    }

    /// Reset the gate to initial state (closed, timers cleared).
    /// Call this when starting a new dictation session.
    pub fn reset(&mut self) {
        self.speaking = false;
        self.above_ms = 0.0;
        self.quiet_ms = 0.0;
    }

    /// Feed one frame's dB level and update internal timers/state.
    /// Returns true if the frame should be forwarded (currently speaking).
    pub fn update(&mut self, level_db: f32) -> bool {
        // When open: accumulate silence until hangover threshold, then close
        if self.speaking {
            if level_db < self.stop_db {
                self.quiet_ms += self.frame_ms;
                if self.quiet_ms >= self.max_silence_ms {
                    self.speaking = false;
                    self.above_ms = 0.0;
                    self.quiet_ms = 0.0;
                }
            } else {
                self.quiet_ms = 0.0;
            }
        } else {
            // When closed: accumulate time above start threshold until min duration, then open
            if level_db > self.start_db {
                self.above_ms += self.frame_ms;
                if self.above_ms >= self.min_ms {
                    self.speaking = true;
                    self.quiet_ms = 0.0;
                }
            } else {
                self.above_ms = 0.0;
            }
        }
        self.speaking
    }
}
