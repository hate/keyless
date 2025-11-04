use keyless_core::error::KeylessResult;
use keyless_core::events::TranscriptionEvent;

/// Trait for real-time audio transcription engines.
///
/// Provides a segment-aware interface: accumulate audio during a speaking segment,
/// emit live previews via `Partial` events, and finalize with a single `Final` event
/// when `end_segment()` is called (e.g., on PTT release).
///
/// # Usage Pattern
/// ```rust,no_run
/// # use keyless_whisper::RealtimeTranscriber;
/// # use keyless_core::events::TranscriptionEvent;
/// # use keyless_core::error::KeylessResult;
/// # fn example(transcriber: &mut dyn RealtimeTranscriber) -> KeylessResult<()> {
/// # let frame = vec![0.0f32; 4800];
/// // During speaking (PTT held):
/// transcriber.push_audio(&frame)?;
/// while let Some(evt) = transcriber.try_next_event() {
///     match evt {
///         TranscriptionEvent::Partial(text) => { /* show preview */ }
///         TranscriptionEvent::Final(text) => { /* unexpected during active segment */ }
///     }
/// }
///
/// // On PTT release:
/// transcriber.end_segment()?;
/// while let Some(evt) = transcriber.try_next_event() {
///     if let TranscriptionEvent::Final(text) = evt {
///         // Use final transcription
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub trait RealtimeTranscriber {
    /// Push an audio frame for transcription (non-blocking).
    ///
    /// Audio is accumulated in an internal buffer until `end_segment()` is called.
    /// During accumulation, periodic `Partial` events may be emitted for live preview.
    ///
    /// # Arguments
    /// * `pcm_f32` - Mono audio samples at native sample rate, f32 in [-1.0, 1.0]
    ///
    /// # Errors
    /// Returns `KeylessError::Whisper` if the queue is full (backpressure).
    /// This indicates the worker thread is overloaded; caller should back off.
    fn push_audio(&mut self, pcm_f32: &[f32]) -> KeylessResult<()>;

    /// Poll for the next transcription event (non-blocking).
    ///
    /// Returns `Some(event)` if a new event is available, `None` otherwise.
    /// Should be called frequently (e.g., every audio frame) to drain events and prevent backpressure.
    ///
    /// # Event Types
    /// - `Partial`: Live preview text (last few words) while speaking. For UI feedback only.
    /// - `Final`: Complete transcription after `end_segment()` completes. Deliver to sink.
    fn try_next_event(&mut self) -> Option<TranscriptionEvent>;

    /// Signal the end of the current speaking segment (e.g., PTT released).
    ///
    /// Triggers final inference over the accumulated audio buffer and emits a single
    /// `TranscriptionEvent::Final` with the complete transcription. The buffer is cleared
    /// after finalization; subsequent `push_audio()` calls start a new segment.
    ///
    /// # Errors
    /// Returns `KeylessError::Whisper` if the segment cannot be finalized (e.g., worker shutdown).
    fn end_segment(&mut self) -> KeylessResult<()>;
}
