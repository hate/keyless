//! Periodic tasks (size refresh, PTT heartbeat).

use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::context::RuntimeContext;

/// Run periodic tasks every ~250ms: size refresh and PTT heartbeat check.
pub fn tick(ctx: &mut RuntimeContext) {
    if ctx.last_tick.elapsed() < Duration::from_millis(250) {
        return;
    }
    // Refresh selected model size from cache and persist
    if ctx.catalog.sizes_started {
        let (tx_sz, rx_sz) = mpsc::sync_channel::<(String, Option<u64>)>(1);
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
            while let Ok((id, sz)) = rx_sz.try_recv() {
                if let Some(bytes) = sz {
                    ctx.app.models.sizes.insert(id, bytes);
                }
            }
            keyless_models::meta::save_sizes_cache(&ctx.app.models.sizes);
        }
    }
    // PTT heartbeat check
    if ctx.ptt.enabled.load(Ordering::Relaxed) {
        let stale = ctx
            .ptt
            .last_event
            .map(|t| t.elapsed() >= Duration::from_secs(5))
            .unwrap_or(true);
        if stale {
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
    ctx.last_tick = Instant::now();
}
