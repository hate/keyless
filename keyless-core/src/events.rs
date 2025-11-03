//! Transcription events emitted by the Whisper engine.
//!
//! The transcriber produces two types of events:
//! - **Partial**: unstable, intermediate transcription (for live UI display)
//! - **Final**: stable, committed transcription (sent to output sinks)
//!
//! Event flow:
//! 1. Audio frames → Whisper worker thread
//! 2. Worker runs inference ~every 0.5s → Partial events
//! 3. Worker reaches 2s window → Final event + buffer clear
//! 4. Main thread polls `try_next_event()` and routes based on type

/// Transcription result from the Whisper engine.
///
/// Design rationale:
/// - Partial events are emitted frequently (~2Hz) for low-latency UI feedback
///   but should NEVER be sent to output sinks (they backtrack and change).
/// - Final events are stable and safe to paste/copy/log.
///
/// Future: Partial events could include confidence scores or token-level timing.
#[derive(Clone, Debug, PartialEq)]
pub enum TranscriptionEvent {
    /// Unstable transcription from the current audio window.
    ///
    /// Use case: display last 3 words in a floating overlay for real-time feedback.
    /// Warning: text may change as more audio arrives (backtracking, punctuation shifts).
    /// Never send partials to clipboard/paste/file/webhook.
    Partial(String),

    /// Stable, committed transcription from a completed audio segment.
    ///
    /// Use case: send to the configured output sink (paste, clipboard, file, webhook).
    /// This text won't change; it's safe to commit to external targets.
    Final(String),
}
