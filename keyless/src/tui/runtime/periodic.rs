//! Periodic tasks (size refresh, PTT heartbeat).

use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::context::RuntimeContext;

/// Run periodic tasks every ~250ms: size refresh and PTT heartbeat check.
pub fn tick(ctx: &mut RuntimeContext) {
    // Throttle to ~250ms to avoid excessive CPU usage; early return if not enough time passed.
    if ctx.last_tick.elapsed() < Duration::from_millis(250) {
        return;
    }
    // Refresh selected model size from cache and persist.
    // Spawn a worker thread to avoid blocking the UI; channel size 1 is sufficient for single query.
    if ctx.catalog.sizes_started {
        let (tx_sz, rx_sz) = mpsc::sync_channel::<(String, Option<u64>)>(1);
        // Clone model ID to move into thread; fallback to None if index is out of bounds.
        if let Some(id) = ctx
            .app
            .models
            .runtime_list
            .get(ctx.app.selections.model_idx)
            .cloned()
        {
            std::thread::spawn(move || {
                keyless_models::workers::emit_cached_sizes(vec![id], tx_sz);
            });
            // Drain all available size updates non-blockingly; may receive 0 or 1 result.
            while let Ok((id, sz)) = rx_sz.try_recv() {
                if let Some(bytes) = sz {
                    ctx.app.models.sizes.insert(id, bytes);
                }
            }
            // Persist cache after update to ensure size info survives restarts.
            keyless_models::meta::save_sizes_cache(&ctx.app.models.sizes);
        }
    }
    // PTT heartbeat check: detect if global hotkey listener has gone stale (>5s since last event).
    // Only check when PTT is enabled; avoid false positives during normal operation.
    if ctx.ptt.enabled.load(Ordering::Relaxed) {
        // If no event has occurred yet, treat as stale (listener may not have started).
        let stale = ctx
            .ptt
            .last_event
            .map(|t| t.elapsed() >= Duration::from_secs(5))
            .unwrap_or(true);
        if stale {
            // Show permission warning only once; check existing warnings to avoid duplicates.
            let msg = "Global hotkey listener inactive. On macOS, enable Accessibility for keyless (System Settings → Privacy & Security → Accessibility).";
            if !ctx
                .app
                .perm_warnings
                .iter()
                .any(|w| w.contains("Accessibility"))
            {
                ctx.app.perm_warnings.push(msg.to_string());
            }
        }
    }
    // Update tick timestamp after processing to measure next interval from completion time.
    ctx.last_tick = Instant::now();
}
