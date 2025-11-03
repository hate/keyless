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
    sink: &Arc<Box<dyn OutputSink>>,
    tx_log: &mpsc::SyncSender<String>,
    tx_preview: &mpsc::SyncSender<String>,
    sfx: &Arc<SfxPlayer>,
) {
    if let Ok(mut t) = transcriber.lock() {
        while let Some(evt) = t.try_next_event() {
            match evt {
                TranscriptionEvent::Final(text) => {
                    if let Err(e) = sink.send_text(&text) {
                        error!(error = %e, "output sink failed");
                        let _ = tx_log.try_send(format!("sink error: {}", e));
                    } else {
                        sfx.play_final();
                    }
                    info!(text = %text, "final transcription delivered");
                    let _ = tx_log.try_send(format!("Final: {}", text));
                }
                TranscriptionEvent::Partial(text) => {
                    // Show only last 4 words in preview
                    let last4 = extract_last_words(&text, 4);
                    let _ = tx_log.try_send(format!("Partial: {}", last4));
                    let _ = tx_preview.try_send(last4);
                }
            }
        }
    }
}

/// Extract the last N words from text for preview display.
fn extract_last_words(text: &str, n: usize) -> String {
    if n == 0 || text.is_empty() {
        return String::new();
    }
    // Collect up to the last n words without allocating the full split
    let mut tail: smallvec::SmallVec<[&str; 8]> = smallvec::SmallVec::new();
    for w in text.split_whitespace().rev() {
        if tail.len() == n {
            break;
        }
        tail.push(w);
    }
    tail.reverse();
    tail.join(" ")
}
