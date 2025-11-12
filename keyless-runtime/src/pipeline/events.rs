//! Transcription event processing.

use std::sync::{Arc, Mutex, mpsc};

use keyless_audio::SfxPlayer;
use keyless_core::events::TranscriptionEvent;
use keyless_core::output::OutputSink;
use keyless_whisper::RealtimeTranscriber;
use tracing::{error, info};

/// Process transcription events: deliver Final events, show Partial as preview.
pub fn process_transcription_events<T: RealtimeTranscriber>(
    transcriber: &Arc<Mutex<T>>,
    sink: &Arc<dyn OutputSink>,
    tx_log: &mpsc::SyncSender<String>,
    tx_preview: &mpsc::SyncSender<String>,
    sfx: &Arc<SfxPlayer>,
) {
    // Lock transcriber to drain events (non-blocking lock; ignore if poisoned).
    if let Ok(mut t) = transcriber.lock() {
        // try_next_event() is non-blocking; drain all available events in one pass.
        while let Some(evt) = t.try_next_event() {
            match evt {
                TranscriptionEvent::Final(text) => {
                    // Emit word count first so UIs can update stats without parsing Final lines.
                    let words = text.split_whitespace().count();
                    let _ = tx_log.try_send(format!("Words: {}", words));
                    let sink_label = sink.label();
                    // Send to output sink (paste/clipboard/file); play sound on success.
                    if let Err(e) = sink.send_text(&text) {
                        error!(error = %e, "output sink failed");
                        let _ = tx_log.try_send(format!("sink error: {}", e));
                    } else {
                        // Play beep on successful delivery (user feedback).
                        sfx.play_final();
                    }
                    info!(text = %text, sink = %sink_label, "final transcription delivered");
                    let _ = tx_log.try_send(format!("Final → {}: {}", sink_label, text));
                }
                TranscriptionEvent::Partial(text) => {
                    // Show full preview text and label as Preview.
                    let _ = tx_log.try_send(format!("Preview: {}", text));
                    // Send full text to preview channel (non-blocking; may drop if channel full).
                    let _ = tx_preview.try_send(text);
                }
            }
        }
    }
}
