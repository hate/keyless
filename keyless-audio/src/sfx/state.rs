//! Internal realtime state for the SFX output callback.

/// Beep variants triggered by the public player API.
#[derive(Clone, Copy)]
pub enum BeepKind {
    /// PTT pressed
    Press,
    /// PTT released
    Release,
    /// Final text delivered (double beep)
    Final,
}

/// Minimal oscillator + envelope state progressed by the audio callback.
pub struct CallbackState {
    /// Audio sample rate in Hz.
    pub sample_rate: f32,
    /// Whether a beep is currently playing.
    pub active: bool,
    /// Current oscillator phase (0 to TAU).
    pub phase: f32,
    /// Phase increment per sample (frequency * TAU / sample_rate).
    pub phase_inc: f32,
    /// Current gain multiplier.
    pub gain: f32,
    /// Total frames for the current beep.
    pub frames_total: usize,
    /// Frames played so far.
    pub frames_played: usize,
    /// Attack duration in frames.
    pub attack_frames: usize,
    /// Release duration in frames.
    pub release_frames: usize,
    /// Whether a pending beep is queued (for double beeps).
    pub pending_active: bool,
    /// Frequency for the pending beep.
    pub pending_freq: f32,
    /// Gain for the pending beep.
    pub pending_gain: f32,
    /// Total frames for the pending beep.
    pub pending_frames_total: usize,
    /// Frames remaining in the gap between double beeps.
    pub gap_frames_remaining: usize,
}

impl CallbackState {
    /// Creates a new callback state with the given sample rate.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            active: false,
            phase: 0.0,
            phase_inc: 0.0,
            gain: 0.0,
            frames_total: 0,
            frames_played: 0,
            // Attack: 2ms ramp-up (prevents clicks); max(1) ensures at least 1 frame.
            attack_frames: ((0.002 * sample_rate).round() as usize).max(1),
            // Release: 30ms fade-out (smooth ending); max(1) ensures at least 1 frame.
            release_frames: ((0.030 * sample_rate).round() as usize).max(1),
            pending_active: false,
            pending_freq: 0.0,
            pending_gain: 0.0,
            pending_frames_total: 0,
            gap_frames_remaining: 0,
        }
    }

    /// Starts a beep of the given kind (press, release, or final).
    pub fn start_beep(&mut self, kind: BeepKind) {
        match kind {
            // Press: higher pitch (1200 Hz), shorter duration (45ms), slightly louder.
            BeepKind::Press => self.set_current_beep(1200.0, 0.045, 0.20),
            // Release: lower pitch (800 Hz), longer duration (60ms), slightly quieter.
            BeepKind::Release => self.set_current_beep(800.0, 0.060, 0.18),
            BeepKind::Final => {
                // Double beep: low then high, with a tiny gap (~20ms) between them.
                // First beep: 780 Hz, 55ms, 0.18 gain.
                self.set_current_beep(780.0, 0.055, 0.18);
                // Queue second beep: 1200 Hz, 60ms, 0.17 gain (starts after gap).
                self.pending_freq = 1200.0;
                self.pending_gain = 0.17;
                self.pending_frames_total = (0.060 * self.sample_rate).max(1.0) as usize;
                self.pending_active = true;
                // Gap duration: 20ms silence between beeps for distinct sound.
                self.gap_frames_remaining = (0.020 * self.sample_rate).round() as usize;
            }
        }
    }

    /// Sets the current beep parameters (frequency, duration, gain).
    pub fn set_current_beep(&mut self, freq: f32, dur_s: f32, gain: f32) {
        // Reset oscillator phase to 0 (start of sine wave cycle).
        self.phase = 0.0;
        // Calculate phase increment: radians per sample = (freq * 2π) / sample_rate.
        // This advances the oscillator by the correct amount each sample to produce the desired frequency.
        self.phase_inc = (freq * std::f32::consts::TAU) / self.sample_rate;
        self.gain = gain;
        // Convert duration (seconds) to frames; max(1.0) prevents zero-length beeps.
        self.frames_total = (dur_s * self.sample_rate).max(1.0) as usize;
        self.frames_played = 0;
        self.active = true;
    }

    /// Generates the next audio sample, advancing the oscillator and envelope.
    pub fn next_sample(&mut self) -> f32 {
        if !self.active {
            // Handle gap between double beeps: count down silence frames.
            if self.gap_frames_remaining > 0 {
                self.gap_frames_remaining = self.gap_frames_remaining.saturating_sub(1);
                return 0.0;
            }
            // After gap (or if no gap), start pending beep if queued (double beep second tone).
            if self.pending_active {
                // Extract pending parameters before clearing flag (avoids borrow checker issues).
                let freq = self.pending_freq;
                let gain = self.pending_gain;
                let total = self.pending_frames_total;
                self.pending_active = false;
                // Convert frames back to seconds for set_current_beep (needs duration in seconds).
                self.set_current_beep(freq, total as f32 / self.sample_rate, gain);
            } else {
                // No active beep and no pending beep: return silence.
                return 0.0;
            }
        }
        // Check if beep duration exceeded; deactivate and return silence.
        if self.frames_played >= self.frames_total {
            self.active = false;
            return 0.0;
        }

        // Generate sine wave sample: sin(phase) produces [-1.0, 1.0] oscillation.
        let s = (self.phase).sin();
        // Advance phase by increment (wraps around TAU to prevent unbounded growth).
        self.phase += self.phase_inc;
        // Wrap phase to [0, TAU) to prevent precision loss from large phase values.
        if self.phase > std::f32::consts::TAU {
            self.phase -= std::f32::consts::TAU;
        }
        // Calculate envelope: attack ramp-up, sustain at 1.0, release fade-out.
        let a = self.frames_played as i32;
        let env = if a < self.attack_frames as i32 {
            // Attack phase: linear ramp from 0 to 1 over attack_frames.
            (a as f32) / (self.attack_frames as f32)
        } else {
            // Sustain or release phase: check if we're in release window.
            let remain = (self.frames_total as i32 - a).max(0) as usize;
            if remain <= self.release_frames {
                // Release phase: linear fade from 1 to 0 over release_frames.
                (remain as f32) / (self.release_frames as f32)
            } else {
                // Sustain phase: full volume (envelope = 1.0).
                1.0
            }
        };
        self.frames_played += 1;
        // Apply gain and envelope, clamp to [-0.8, 0.8] to prevent clipping/distortion.
        // 0.8 is conservative headroom (leaves 20% margin before clipping at ±1.0).
        (s * self.gain * env).clamp(-0.8, 0.8)
    }
}
