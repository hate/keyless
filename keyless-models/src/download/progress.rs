//! Progress reporting utilities for download streaming.

use super::events::DownloadEvent;
use keyless_core::error::{KeylessError, KeylessResult};
use std::io::{Read, Write};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

/// Stream response chunks to file with progress reporting.
pub(crate) fn stream_with_progress(
    resp: &mut reqwest::blocking::Response,
    file: &mut Box<dyn Write>,
    total: u64,
    mut downloaded: u64,
    tx: &SyncSender<DownloadEvent>,
    cancel: &Arc<AtomicBool>,
) -> KeylessResult<()> {
    // Track time for speed/ETA calculations.
    let start = std::time::Instant::now();
    let mut last_emit = std::time::Instant::now();
    // 64KB buffer: balances memory usage vs I/O syscall overhead.
    let mut buf = vec![0u8; 64 * 1024];

    loop {
        // Check cancellation before reading (allows prompt abort on slow networks).
        if cancel.load(Ordering::Relaxed) {
            return Err(KeylessError::Other("cancelled".into()));
        }
        // Read chunk from response stream (may return < buf.len() at end).
        let n = resp
            .read(&mut buf)
            .map_err(|e| KeylessError::Other(format!("read chunk: {}", e)))?;
        // EOF: n == 0 indicates stream end (normal completion).
        if n == 0 {
            break;
        }
        // Check cancellation after read (allows abort during write).
        if cancel.load(Ordering::Relaxed) {
            return Err(KeylessError::Other("cancelled".into()));
        }
        // Write chunk to file (only write n bytes, not full buffer).
        file.write_all(&buf[..n]).map_err(KeylessError::from)?;
        downloaded += n as u64;
        // Throttle progress events to at most every 250ms (prevents channel saturation).
        if last_emit.elapsed().as_millis() >= 250 {
            last_emit = std::time::Instant::now();
            emit_progress(downloaded, total, start, tx);
        }
    }

    Ok(())
}

/// Emit a progress event with calculated speed and ETA.
fn emit_progress(
    downloaded: u64,
    total: u64,
    start: std::time::Instant,
    tx: &SyncSender<DownloadEvent>,
) {
    if total > 0 && downloaded <= total {
        // Calculate speed (MB/s) and ETA (seconds remaining).
        // max(0.001) prevents division by zero on very fast downloads.
        let secs = start.elapsed().as_secs_f64().max(0.001);
        let mbps = (downloaded as f64 / 1_000_000.0) / secs;
        // saturating_sub prevents underflow if downloaded > total (shouldn't happen).
        let left = (total.saturating_sub(downloaded)) as f64;
        let bps = (downloaded as f64) / secs;
        // ETA = remaining_bytes / bytes_per_second; max(0.0) prevents negative ETA.
        let eta = if bps > 0.0 {
            (left / bps).max(0.0)
        } else {
            0.0
        };
        let _ = tx.try_send(DownloadEvent::Progress {
            bytes: downloaded,
            total: Some(total),
            mbps,
            eta_s: eta,
        });
    } else {
        // Total unknown or downloaded exceeds expected (shouldn't happen).
        let _ = tx.try_send(DownloadEvent::Progress {
            bytes: downloaded,
            total: None,
            mbps: 0.0,
            eta_s: 0.0,
        });
    }
}
