//! Config screen initialization.

use std::sync::mpsc;

use super::context::RuntimeContext;

/// Initialize catalog, discovery, and restore saved selections.
/// Called once when entering the Config screen.
pub fn init_config_screen(ctx: &mut RuntimeContext) {
    if ctx.catalog.sizes_started {
        return;
    }
    ctx.catalog.sizes_started = true;

    // Load persisted sizes cache
    let (cached, _ts) = keyless_models::meta::load_sizes_cache();
    for (k, v) in cached {
        ctx.app.models.sizes.entry(k).or_insert(v);
    }

    // Populate model catalog and discovered locals first
    if !ctx.catalog.repos_started {
        ctx.catalog.repos_started = true;
        // discovered locals
        ctx.app.models.discovered = keyless_models::discover::discover_local_whisper_repos();
        for s in &ctx.app.models.discovered {
            if !ctx.app.models.runtime_list.iter().any(|m| m == s) {
                ctx.app.models.runtime_list.push(s.clone());
            }
        }
        // remote catalog (openai/whisper*)
        if let Ok(list) = keyless_models::catalog::list_openai_whisper_models() {
            for s in list {
                if !ctx.app.models.runtime_list.iter().any(|m| m == &s) {
                    ctx.app.models.runtime_list.push(s);
                }
            }
        }
        if ctx.app.models.runtime_list.is_empty() {
            ctx.app
                .models
                .runtime_list
                .push(keyless_core::config::DEFAULT_MODEL_ID.to_string());
        }

        // Restore saved model and language selections after catalog is populated
        if let Some(config) = keyless_core::config::storage::load_config() {
            // Find saved model in catalog and update model_idx
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

            // Restore language selection based on model type
            let is_en_model = keyless_core::options::is_english_only_model(&saved_model);
            if is_en_model {
                // .en models: always English (index 0)
                ctx.app.selections.language_idx = 0;
            } else {
                // Multilingual models: 0 = auto, 1+ = LANG_CODES
                ctx.app.selections.language_idx = match config.language.as_deref() {
                    None => 0,
                    Some(code) => keyless_core::options::LANG_CODES
                        .iter()
                        .position(|c| *c == code)
                        .map(|i| i + 1)
                        .unwrap_or(0),
                };
            }
        }

        // Log catalog summary
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

    // Kick off cached size emission
    let (tx_sz, rx_sz) = mpsc::sync_channel::<(String, Option<u64>)>(64);
    let model_ids: Vec<String> = ctx.app.models.runtime_list.to_vec();
    std::thread::spawn(move || {
        keyless_models::workers::emit_cached_sizes(model_ids, tx_sz);
    });
    ctx.catalog.rx_sizes_cached = Some(rx_sz);

    // Spawn remote size fetch for all catalog (force on first load)
    let (tx_remote, rx_remote) = mpsc::sync_channel::<(String, Option<u64>)>(64);
    let ids_for_remote = ctx.app.models.runtime_list.clone();
    std::thread::spawn(move || {
        keyless_models::workers::fetch_sizes_now(ids_for_remote, tx_remote);
    });
    ctx.catalog.rx_sizes_remote = Some(rx_remote);

    // Proactively probe and warm up the ML device so first dictation is instant
    std::thread::spawn(|| {
        let _ = keyless_whisper::preload_device();
    });
}
