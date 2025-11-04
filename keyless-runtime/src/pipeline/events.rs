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
                    // Send to output sink (paste/clipboard/file); play sound on success.
                    if let Err(e) = sink.send_text(&text) {
                        error!(error = %e, "output sink failed");
                        let _ = tx_log.try_send(format!("sink error: {}", e));
                    } else {
                        // Play beep on successful delivery (user feedback).
                        sfx.play_final();
                    }
                    info!(text = %text, "final transcription delivered");
                    let _ = tx_log.try_send(format!("Final: {}", text));
                }
                TranscriptionEvent::Partial(text) => {
                    // Show only last 4 words in preview (UI space constraint; full text too long).
                    let last4 = extract_last_words(&text, 4);
                    let _ = tx_log.try_send(format!("Partial: {}", last4));
                    // Send to preview channel (non-blocking; may drop if channel full).
                    let _ = tx_preview.try_send(last4);
                }
            }
        }
    }
}

/// Extract the last N words from text for preview display.
fn extract_last_words(text: &str, n: usize) -> String {
    // Early return for edge cases (empty text or zero words requested).
    if n == 0 || text.is_empty() {
        return String::new();
    }
    // Collect up to the last n words without allocating the full split.
    // SmallVec avoids heap allocation for small n (stack-allocated for n <= 8).
    let mut tail: smallvec::SmallVec<[&str; 8]> = smallvec::SmallVec::new();
    // Iterate in reverse (last words first); stop when we have n words.
    for w in text.split_whitespace().rev() {
        if tail.len() == n {
            break;
        }
        tail.push(w);
    }
    // Reverse to restore original word order (was collected backwards).
    tail.reverse();
    tail.join(" ")
}
