//! Config screen initialization.

use std::sync::mpsc;

use super::context::RuntimeContext;

/// Initialize catalog, discovery, and restore saved selections.
/// Called once when entering the Config screen.
pub fn init_config_screen(ctx: &mut RuntimeContext) {
    // Guard: only initialize once per session (prevents re-discovery on every Config screen visit).
    if ctx.catalog.sizes_started {
        return;
    }
    ctx.catalog.sizes_started = true;

    // Load persisted sizes cache: restores model file sizes from disk to avoid re-computation.
    // Timestamp (_ts) is ignored; cache is refreshed periodically by workers.
    let (cached, _ts) = keyless_models::meta::load_sizes_cache();
    // Use entry().or_insert() to avoid overwriting already-loaded sizes (workers may have updated).
    for (k, v) in cached {
        ctx.app.models.sizes.entry(k).or_insert(v);
    }

    // Populate model catalog and discovered locals first (only once per session).
    if !ctx.catalog.repos_started {
        ctx.catalog.repos_started = true;
        // Discover locally installed models first; these get priority in the UI.
        ctx.app.models.discovered = keyless_models::discover::discover_local_whisper_repos();
        // Add discovered models to runtime_list if not already present (avoid duplicates).
        for s in &ctx.app.models.discovered {
            if !ctx.app.models.runtime_list.iter().any(|m| m == s) {
                ctx.app.models.runtime_list.push(s.clone());
            }
        }
        // Fetch remote catalog (openai/whisper* models) and merge into runtime_list.
        // Continue even if fetch fails (network issues shouldn't block UI).
        if let Ok(list) = keyless_models::catalog::list_openai_whisper_models() {
            for s in list {
                if !ctx.app.models.runtime_list.iter().any(|m| m == &s) {
                    ctx.app.models.runtime_list.push(s);
                }
            }
        }
        // Fallback: ensure at least one model exists (default model) if catalog is empty.
        if ctx.app.models.runtime_list.is_empty() {
            ctx.app
                .models
                .runtime_list
                .push(keyless_core::config::DEFAULT_MODEL_ID.to_string());
        }

        // Restore saved model and language selections after catalog is populated.
        // Config is loaded after catalog so saved model ID can be found in runtime_list.
        if let Some(config) = keyless_core::config::storage::load_config() {
            // Find saved model in catalog and update model_idx.
            // Convert PathBuf to String for comparison; if not found, keep default index.
            let saved_model = config.model_path.to_string_lossy().to_string();
            if let Some(pos) = ctx
                .app
                .models
                .runtime_list
                .iter()
                .position(|m| m == &saved_model)
            {
                ctx.app.selections.model_idx = pos;
            }

            // Restore language selection based on model type (affects available languages).
            let is_en_model = keyless_core::options::is_english_only_model(&saved_model);
            if is_en_model {
                // .en models: always English (index 0); no "auto" option available.
                ctx.app.selections.language_idx = 0;
            } else {
                // Multilingual models: 0 = auto, 1+ = LANG_CODES (offset by +1 for "auto").
                ctx.app.selections.language_idx = match config.language.as_deref() {
                    None => 0, // No language saved = auto-detect.
                    Some(code) => keyless_core::options::LANG_CODES
                        .iter()
                        .position(|c| *c == code)
                        .map(|i| i + 1) // Offset +1 because index 0 is reserved for "auto".
                        .unwrap_or(0), // Fallback to auto if saved code not found.
                };
            }
        }

        // Log catalog summary: show first 10 models for user feedback during initialization.
        let mut snapshot = ctx.app.models.runtime_list.clone();
        snapshot.sort();
        let preview = snapshot
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let _ = ctx.channels.ui.tx_log.try_send(format!(
            "catalog: {} models (first: {})",
            snapshot.len(),
            preview
        ));
        // Log installed models separately if any were discovered.
        if !ctx.app.models.discovered.is_empty() {
            let ok = ctx
                .app
                .models
                .discovered
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            let _ = ctx.channels.ui.tx_log.try_send(format!(
                "installed: {} (first: {})",
                ctx.app.models.discovered.len(),
                ok
            ));
        }
    }

    // Kick off cached size emission: fetch sizes from local cache (fast, non-blocking).
    // Channel size 64 allows buffering multiple model sizes without blocking worker.
    let (tx_sz, rx_sz) = mpsc::sync_channel::<(String, Option<u64>)>(64);
    let model_ids: Vec<String> = ctx.app.models.runtime_list.to_vec();
    std::thread::spawn(move || {
        keyless_models::workers::emit_cached_sizes(model_ids, tx_sz);
    });
    ctx.catalog.rx_sizes_cached = Some(rx_sz);

    // Spawn remote size fetch for all catalog (force on first load to refresh stale cache).
    // This runs in parallel with cached fetch; remote results overwrite cached ones.
    let (tx_remote, rx_remote) = mpsc::sync_channel::<(String, Option<u64>)>(64);
    let ids_for_remote = ctx.app.models.runtime_list.clone();
    std::thread::spawn(move || {
        keyless_models::workers::fetch_sizes_now(ids_for_remote, tx_remote);
    });
    ctx.catalog.rx_sizes_remote = Some(rx_remote);

    // Proactively probe and warm up the ML device so first dictation is instant.
    // This avoids cold-start delay when user starts speaking; device initialization is async.
    std::thread::spawn(|| {
        let _ = keyless_whisper::preload_device();
    });
}
