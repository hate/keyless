//! PTT state transition handlers.

use std::sync::{Arc, Mutex, mpsc};

use keyless_audio::SfxPlayer;
use keyless_core::events::TranscriptionEvent;
use keyless_core::output::OutputSink;
use keyless_whisper::RealtimeTranscriber;
use tracing::{debug, error, info};

/// Handle PTT release: finalize segment and drain/deliver Final events.
pub fn handle_ptt_released<T: RealtimeTranscriber>(
    transcriber: &Arc<Mutex<T>>,
    sink: &Arc<dyn OutputSink>,
    tx_log: &mpsc::SyncSender<String>,
    tx_preview: &mpsc::SyncSender<String>,
    sfx: &Arc<SfxPlayer>,
) {
    debug!("PTT released, ending segment");
    let _ = tx_log.try_send("PTT released: gating audio".to_string());

    // Signal end of segment to transcriber (triggers finalization, emits Final event).
    // Non-blocking lock; ignore if poisoned (best-effort finalization).
    if let Ok(mut t) = transcriber.lock()
        && let Err(e) = t.end_segment()
    {
        error!(error = %e, "failed to end transcription segment");
        let _ = tx_log.try_send(format!("end_segment error: {}", e));
    }

    // Clear preview text (PTT released = session ended, no more preview needed).
    let _ = tx_preview.try_send(String::new());

    // Drain and deliver any pending Final events (non-blocking; may arrive asynchronously).
    // Final events are emitted by inference thread after end_segment(), so drain here.
    if let Ok(mut t) = transcriber.lock() {
        while let Some(evt) = t.try_next_event() {
            if let TranscriptionEvent::Final(text) = evt {
                // Send to output sink (paste/clipboard/file); play sound on success.
                if let Err(e) = sink.send_text(&text) {
                    error!(error = %e, "output sink failed");
                    let _ = tx_log.try_send(format!("sink error: {}", e));
                } else {
                    // Play beep on successful delivery (user feedback).
                    sfx.play_final();
                    // Immediately report word count so UIs can bump session stats without waiting.
                    let words = text.split_whitespace().count() as u64;
                    if words > 0 {
                        let _ = tx_log.try_send(format!("Words: {}", words));
                    }
                }
                info!(text = %text, "final transcription delivered");
                let _ = tx_log.try_send(format!("Final: {}", text));
            }
        }
    }
}

/// Handle PTT press: start new dictation session.
pub fn handle_ptt_pressed(
    tx_log: &mpsc::SyncSender<String>,
    tx_preview: &mpsc::SyncSender<String>,
) {
    debug!("PTT pressed, listening with VAD gating");
    let _ = tx_log.try_send("PTT held: listening (VAD gated)".to_string());

    // Clear preview text for new session (fresh start, no leftover text from previous session).
    let _ = tx_preview.try_send(String::new());
}
