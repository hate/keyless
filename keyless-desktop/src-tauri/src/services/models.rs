//! Model catalog and download management service.
//!
//! Provides a trait-based abstraction over `keyless_models` crate functions for model
//! discovery, catalog management, downloads, and status tracking. The implementation manages
//! download workers, progress tracking, and event emission to the frontend.

use crate::services::Events;
use keyless_models::catalog::list_openai_whisper_models;
use keyless_models::discover::discover_local_whisper_repos;
use keyless_models::download::DownloadEvent;
use keyless_models::meta;
use keyless_models::workers;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread;

/// Service trait for model catalog and download management.
///
/// All methods operate on the model catalog and download system. Downloads run
/// asynchronously and progress is reported via events emitted through the `Events` facade.
pub trait ModelsService: Send + Sync {
    /// List all models known to the app (installed and available).
    ///
    /// Returns a list of models with their download status. Does not include size information.
    fn list_models(&self) -> Result<Vec<ModelInfo>, String>;

    /// List all models with known sizes and install state.
    ///
    /// Returns a list of models with their download sizes (from metadata cache) and
    /// installation status. This is the preferred method for populating the Models UI.
    fn list_models_with_sizes(&self) -> Result<Vec<ModelInfoWithSize>, String>;

    /// List all installed model IDs.
    ///
    /// Returns a sorted list of model IDs that are currently installed locally.
    fn list_installed_models(&self) -> Result<Vec<String>, String>;

    /// Start downloading a model by ID.
    ///
    /// Initiates a download for the specified model. The download runs asynchronously and
    /// progress is reported via `model_download_progress` events.
    fn start_download(&self, model_id: &str) -> Result<(), String>;

    /// Cancel an in-progress download by model ID.
    ///
    /// Cancels the download and removes any partially downloaded files.
    fn cancel_download(&self, model_id: &str) -> Result<(), String>;

    /// Pause an in-progress download by model ID.
    ///
    /// Pauses the download, preserving partially downloaded files. Can be resumed later.
    fn pause_download(&self, model_id: &str) -> Result<(), String>;

    /// Resume a paused download by model ID.
    ///
    /// Resumes a previously paused download from where it left off.
    fn resume_download(&self, model_id: &str) -> Result<(), String>;

    /// Refresh the remote size metadata cache.
    ///
    /// Fetches the latest model size information from remote sources and updates the cache.
    fn refresh_sizes(&self) -> Result<(), String>;

    /// Purge the local models cache directory and clear metadata.
    ///
    /// Removes all downloaded models and clears the size metadata cache. Destructive operation.
    fn purge_models_cache(&self) -> Result<(), String>;

    /// Remove an installed model by ID.
    ///
    /// Deletes the model files from the local cache.
    fn remove_model(&self, model_id: &str) -> Result<(), String>;

    /// Get the current status of a model.
    ///
    /// Returns detailed status including download state, progress (if downloading), and errors.
    fn status(&self, model_id: &str) -> Result<ModelStatus, String>;
}

/// Model information without size metadata.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Model ID (e.g., "openai/whisper-base").
    pub id: String,
    /// Whether the model is installed locally.
    pub downloaded: bool,
    /// Whether the model is currently downloading.
    pub downloading: bool,
}

/// Model information with size metadata.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfoWithSize {
    /// Model ID (e.g., "openai/whisper-base").
    pub id: String,
    /// Whether the model is installed locally.
    pub downloaded: bool,
    /// Whether the model is currently downloading.
    pub downloading: bool,
    /// Download size in bytes (from metadata cache), or `None` if unknown.
    pub size_bytes: Option<u64>,
}

/// Detailed status information for a model.
#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    /// Whether the model is installed locally.
    pub downloaded: bool,
    /// Whether the model is currently downloading.
    pub downloading: bool,
    /// Current download stage description (e.g., "downloading", "extracting").
    pub stage: Option<String>,
    /// Download progress information, if downloading.
    pub progress: Option<DownloadProgress>,
    /// Error message, if download failed.
    pub error: Option<String>,
    /// Whether the download is paused.
    pub paused: bool,
}

/// Download progress information.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub bytes: u64,
    /// Total bytes to download, or `None` if unknown.
    pub total: Option<u64>,
    /// Download speed in megabytes per second.
    pub mbps: f64,
    /// Estimated time remaining in seconds.
    pub eta_seconds: f64,
}

/// Payload for download started events.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadStartedPayload {
    /// Model identifier.
    model_id: String,
}

/// Payload for download stage change events.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadStagePayload {
    /// Model identifier.
    model_id: String,
    /// Stage description text.
    text: String,
}

/// Payload for download progress events.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgressPayload {
    /// Model identifier.
    model_id: String,
    /// Current progress information.
    progress: DownloadProgress,
}

/// Payload for download completion events.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadCompletePayload {
    /// Model identifier.
    model_id: String,
}

/// Payload for download error events.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadErrorPayload {
    /// Model identifier.
    model_id: String,
    /// Error message.
    message: String,
}

/// Payload for download cancellation events.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadCancelledPayload {
    /// Model identifier.
    model_id: String,
}

/// Internal tracking entry for an active or queued download.
struct DownloadEntry {
    /// Cancellation flag for the download thread.
    cancel: Arc<AtomicBool>,
    /// Current status of the download.
    status: ModelStatus,
    /// Whether the download is paused.
    paused: bool,
}

impl DownloadEntry {
    /// Create a new download entry with default state.
    fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            status: ModelStatus {
                downloaded: false,
                downloading: false,
                stage: None,
                progress: None,
                error: None,
                paused: false,
            },
            paused: false,
        }
    }
}

/// Trait for emitting JSON events (internal abstraction for testability).
pub trait EventPublisher: Send + Sync {
    /// Emit an event with a JSON payload.
    fn emit_json(&self, name: &str, payload: Value);
}

impl EventPublisher for Events {
    fn emit_json(&self, name: &str, payload: Value) {
        self.emit(name, payload);
    }
}

/// Trait for model catalog and download backend operations (internal abstraction for testability).
pub trait ModelBackend: Send + Sync {
    /// List all remote models from the catalog.
    fn list_remote_models(&self) -> Result<Vec<String>, String>;

    /// Discover locally installed models.
    fn discover_local_models(&self) -> Vec<String>;

    /// Spawn a download worker for a model.
    ///
    /// The download runs in a separate thread and reports progress via the provided channel.
    fn spawn_download(
        &self,
        model_id: String,
        tx: SyncSender<DownloadEvent>,
        cancel: Arc<AtomicBool>,
    );
}

/// Default implementation of `ModelBackend` using `keyless_models` crate.
pub struct DefaultModelBackend;

impl ModelBackend for DefaultModelBackend {
    fn list_remote_models(&self) -> Result<Vec<String>, String> {
        list_openai_whisper_models().map_err(|e| e.to_string())
    }

    fn discover_local_models(&self) -> Vec<String> {
        discover_local_whisper_repos()
    }

    fn spawn_download(
        &self,
        model_id: String,
        tx: SyncSender<DownloadEvent>,
        cancel: Arc<AtomicBool>,
    ) {
        thread::spawn(move || keyless_models::download::ensure_model_cached(model_id, tx, cancel));
    }
}

/// Default implementation of `ModelsService` using `keyless_models` crate.
pub struct ModelsServiceImpl {
    /// Event publisher for emitting download events.
    events: Arc<dyn EventPublisher>,
    /// Backend for listing and downloading models.
    backend: Arc<dyn ModelBackend>,
    /// Active download entries keyed by model ID.
    downloads: Arc<Mutex<HashMap<String, DownloadEntry>>>,
    /// Model size cache (model ID -> size in bytes).
    sizes: Arc<Mutex<HashMap<String, u64>>>,
    /// List of installed model IDs.
    installed: Arc<Mutex<Vec<String>>>,
}

impl ModelsServiceImpl {
    /// Create a new `ModelsServiceImpl` with the default backend.
    ///
    /// Preloads the size cache and discovers installed models.
    pub fn new(events: Arc<Events>) -> Self {
        let publisher: Arc<dyn EventPublisher> = events.clone();
        Self::with_backend(publisher, Arc::new(DefaultModelBackend))
    }

    /// Create a new `ModelsServiceImpl` with a custom backend (for testing).
    ///
    /// Preloads the size cache and discovers installed models using the provided backend.
    pub fn with_backend(events: Arc<dyn EventPublisher>, backend: Arc<dyn ModelBackend>) -> Self {
        // Preload sizes cache for fast initial display / offline support.
        // The cache contains download sizes for known models, allowing the UI to show sizes
        // even when offline or before remote sizes are fetched.
        let (sizes_map, _ts) = meta::load_sizes_cache();
        // Discover locally installed models using the backend.
        let mut installed = backend.discover_local_models();
        // Sort installed models for consistent display.
        installed.sort();
        // Create the service with empty downloads map (downloads are added on-demand).
        Self {
            events,
            backend,
            downloads: Arc::new(Mutex::new(HashMap::new())),
            sizes: Arc::new(Mutex::new(sizes_map)),
            installed: Arc::new(Mutex::new(installed)),
        }
    }
}

impl ModelsService for ModelsServiceImpl {
    fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        // Get list of all remote models from the catalog.
        let remote = self.backend.list_remote_models()?;

        // Get list of locally installed models and convert to HashSet for fast lookup.
        let local_list = self
            .installed
            .lock()
            .map_err(|_| "models: installed list poisoned".to_string())?
            .clone();
        let local: HashSet<String> = local_list.into_iter().collect();

        // Pre-allocate vector with capacity for all remote models.
        let mut infos = Vec::with_capacity(remote.len());

        // Lock downloads map to check download status for each model.
        let downloads_guard = self
            .downloads
            .lock()
            .map_err(|_| "models: download map poisoned".to_string())?;

        // Build ModelInfo for each remote model.
        for id in remote {
            // Check if model is installed locally.
            let downloaded = local.contains(&id);
            // Check if model is currently downloading.
            let downloading = downloads_guard
                .get(&id)
                .map(|entry| entry.status.downloading)
                .unwrap_or(false);
            infos.push(ModelInfo {
                id,
                downloaded,
                downloading,
            });
        }

        Ok(infos)
    }

    fn list_models_with_sizes(&self) -> Result<Vec<ModelInfoWithSize>, String> {
        // Get base model list (without sizes).
        let base = self.list_models()?;

        // Lock sizes cache to look up download sizes.
        let sizes_guard = self
            .sizes
            .lock()
            .map_err(|_| "models: sizes map poisoned".to_string())?;

        // Enrich each model with size information from the cache.
        let infos = base
            .into_iter()
            .map(|m| ModelInfoWithSize {
                size_bytes: sizes_guard.get(&m.id).cloned(), // Look up size, or None if unknown.
                id: m.id,
                downloaded: m.downloaded,
                downloading: m.downloading,
            })
            .collect();
        Ok(infos)
    }

    fn list_installed_models(&self) -> Result<Vec<String>, String> {
        // Get the list of installed model IDs (sorted).
        let installed = self
            .installed
            .lock()
            .map_err(|_| "models: installed list poisoned".to_string())?;
        // Return a clone (caller owns the Vec).
        Ok(installed.clone())
    }

    fn start_download(&self, model_id: &str) -> Result<(), String> {
        // Lock the downloads map to check/create download entry.
        let mut guard = self
            .downloads
            .lock()
            .map_err(|_| "models: download map poisoned".to_string())?;

        // Check if there's already a download entry for this model.
        if let Some(entry) = guard.get_mut(model_id) {
            // Entry exists: check if download is already in progress.
            if entry.status.downloading {
                return Err("download already in progress".into());
            }
            // Reset previous state to allow re-download (e.g., after a failed download).
            entry.status = ModelStatus {
                downloaded: false,
                downloading: false,
                stage: None,
                progress: None,
                error: None,
                paused: false,
            };
            entry.status.downloading = true;
            // Clear cancellation flag and paused state.
            entry.cancel.store(false, Ordering::Relaxed);
            entry.paused = false;
            entry.status.paused = false;
        } else {
            // No entry exists: create a new download entry.
            let mut entry = DownloadEntry::new();
            entry.status.downloading = true;
            guard.insert(model_id.to_string(), entry);
        }

        // Release the lock before spawning threads (avoid holding lock during I/O).
        drop(guard);

        // Clone Arc references needed for the event handler thread.
        let events = Arc::clone(&self.events);
        let downloads = Arc::clone(&self.downloads);
        let installed = Arc::clone(&self.installed);
        let model = model_id.to_string();

        // Get the cancellation flag for the download worker.
        // We need to lock again to get the entry we just created/updated.
        let cancel_flag = {
            let guard = downloads
                .lock()
                .map_err(|_| "models: download map poisoned".to_string())?;
            guard
                .get(&model)
                .map(|entry| Arc::clone(&entry.cancel))
                .ok_or_else(|| "models: missing download entry".to_string())?
        };

        // Create a channel for receiving download events from the worker thread.
        // Buffer size of 32 should be sufficient for progress updates.
        let (tx, rx) = std::sync::mpsc::sync_channel::<DownloadEvent>(32);

        // Spawn the download worker thread (handles actual file download).
        // The worker sends events through `tx` and checks `cancel_flag` for cancellation.
        self.backend.spawn_download(model.clone(), tx, cancel_flag);

        // Spawn an event handler thread that processes download events and updates state.
        thread::spawn(move || {
            let installed_cache = installed;
            // Process events from the download worker until Done is received.
            for evt in rx {
                match evt {
                    DownloadEvent::Started { model: id } => {
                        // Download has started: emit event to frontend.
                        if let Ok(value) = serde_json::to_value(DownloadStartedPayload {
                            model_id: id.clone(),
                        }) {
                            events.emit_json("download_started", value);
                        }
                    }
                    DownloadEvent::Stage { text } => {
                        // Download stage changed (e.g., "downloading", "extracting"): update status.
                        if let Ok(mut guard) = downloads.lock()
                            && let Some(entry) = guard.get_mut(&model)
                        {
                            entry.status.stage = Some(text.clone());
                            entry.status.paused = false; // Stage change means not paused.
                        }
                        // Emit stage change event to frontend.
                        if let Ok(value) = serde_json::to_value(DownloadStagePayload {
                            model_id: model.clone(),
                            text: text.clone(),
                        }) {
                            events.emit_json("download_stage", value);
                        }
                    }
                    DownloadEvent::Progress {
                        bytes,
                        total,
                        mbps,
                        eta_s,
                    } => {
                        // Download progress update: update status and emit to frontend.
                        let progress = DownloadProgress {
                            bytes,
                            total,
                            mbps,
                            eta_seconds: eta_s,
                        };
                        // Update progress in the download entry.
                        if let Ok(mut guard) = downloads.lock()
                            && let Some(entry) = guard.get_mut(&model)
                        {
                            entry.status.progress = Some(progress.clone());
                            entry.status.paused = false; // Progress means not paused.
                        }
                        // Emit progress event to frontend (for progress bar updates).
                        if let Ok(value) = serde_json::to_value(DownloadProgressPayload {
                            model_id: model.clone(),
                            progress,
                        }) {
                            events.emit_json("download_progress", value);
                        }
                    }
                    DownloadEvent::Done(result) => {
                        // Download completed (successfully or with error).
                        let (ok, error_msg) = match result {
                            Ok(()) => (true, None),                 // Success.
                            Err(e) => (false, Some(e.to_string())), // Error.
                        };
                        let mut was_paused = false;
                        // Update download entry status.
                        if let Ok(mut guard) = downloads.lock()
                            && let Some(entry) = guard.get_mut(&model)
                        {
                            // Check if download was paused (paused downloads don't mark as downloaded).
                            was_paused = entry.paused && !ok;
                            entry.status.downloading = false;
                            // Only mark as downloaded if successful and not paused.
                            entry.status.downloaded = ok && !was_paused;
                            // Clear error if paused (paused is not an error).
                            entry.status.error = if was_paused { None } else { error_msg.clone() };
                            entry.status.paused = was_paused;
                        }
                        // Emit appropriate completion event based on outcome.
                        if was_paused {
                            // Download was paused: emit paused event.
                            if let Ok(value) = serde_json::to_value(DownloadCancelledPayload {
                                model_id: model.clone(),
                            }) {
                                events.emit_json("download_paused", value);
                            }
                        } else if ok {
                            // Download succeeded: add to installed list and emit completion event.
                            if let Ok(mut guard) = installed_cache.lock()
                                && !guard.iter().any(|m| m == &model)
                            {
                                // Add to installed list if not already present.
                                guard.push(model.clone());
                                guard.sort(); // Keep list sorted.
                            }
                            if let Ok(value) = serde_json::to_value(DownloadCompletePayload {
                                model_id: model.clone(),
                            }) {
                                events.emit_json("download_complete", value);
                            }
                        } else if let Some(msg) = error_msg
                            && let Ok(value) = serde_json::to_value(DownloadErrorPayload {
                                model_id: model.clone(),
                                message: msg.clone(),
                            })
                        {
                            // Download failed: emit error event with message.
                            events.emit_json("download_error", value);
                        }
                        // Exit the event loop (download is complete).
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    fn cancel_download(&self, model_id: &str) -> Result<(), String> {
        // Lock downloads map to update and remove the entry.
        let mut guard = self
            .downloads
            .lock()
            .map_err(|_| "models: download map poisoned".to_string())?;
        
        // Check if download exists.
        if guard.get(model_id).is_none() {
            return Err("download not found".into());
        }
        
        // Set cancellation flag (download worker will check this and stop).
        if let Some(entry) = guard.get_mut(model_id) {
            entry.cancel.store(true, Ordering::Relaxed);
        }
        
        // Remove the download entry from the map (cleanup).
        guard.remove(model_id);
        
        // Release lock before I/O operations.
        drop(guard);

        // Delete the partially downloaded file (cleanup).
        // Ignore errors here; the file might not exist or might be cleaned up by the worker.
        if let Err(e) = keyless_models::hf::delete_partial_file(model_id) {
            eprintln!("[models] failed to delete partial download for {model_id}: {e}");
        }

        // Emit cancellation event to frontend.
        if let Ok(value) = serde_json::to_value(DownloadCancelledPayload {
            model_id: model_id.to_string(),
        }) {
            self.events.emit_json("download_cancelled", value);
        }
        Ok(())
    }

    fn pause_download(&self, model_id: &str) -> Result<(), String> {
        // Lock downloads map to update the entry.
        let mut guard = self
            .downloads
            .lock()
            .map_err(|_| "models: download map poisoned".to_string())?;
        if let Some(entry) = guard.get_mut(model_id) {
            // Verify download is actually in progress.
            if !entry.status.downloading {
                return Err("not downloading".into());
            }
            // Set cancellation flag (download worker will stop but preserve partial file).
            entry.cancel.store(true, Ordering::Relaxed);
            // Update status to reflect pause (not downloading, but paused).
            entry.status.downloading = false;
            entry.paused = true;
            entry.status.paused = true;
            Ok(())
        } else {
            Err("download not found".into())
        }
    }

    fn resume_download(&self, model_id: &str) -> Result<(), String> {
        // Lock downloads map to update the entry.
        let mut guard = self
            .downloads
            .lock()
            .map_err(|_| "models: download map poisoned".to_string())?;
        if let Some(entry) = guard.get_mut(model_id) {
            // Verify download is actually paused.
            if !entry.paused {
                return Err("not paused".into());
            }
            // Create a fresh cancellation flag (old one was set to true for pause).
            // Mark as not paused and ready to resume.
            entry.cancel = Arc::new(AtomicBool::new(false));
            entry.paused = false;
            entry.status.paused = false;
        } else {
            return Err("download not found".into());
        }
        // Release lock before spawning download thread.
        drop(guard);

        // Spawn download again (the download worker supports HTTP Range requests,
        // so it will resume from where it left off using the partial file).
        self.start_download(model_id)
    }

    fn refresh_sizes(&self) -> Result<(), String> {
        // Get list of all remote models to fetch sizes for.
        let models = self.backend.list_remote_models()?;

        // Create a channel for receiving size updates (model ID -> size in bytes).
        // Buffer size of 64 should handle bursts of size updates.
        let (tx, rx) = std::sync::mpsc::sync_channel::<(String, Option<u64>)>(64);

        // Spawn a thread to emit cached sizes first (fast path, works offline).
        // This provides immediate feedback while remote sizes are being fetched.
        {
            let tx_cached = tx.clone();
            let models_cached = models.clone();
            thread::spawn(move || workers::emit_cached_sizes(models_cached, tx_cached));
        }

        // Spawn a thread to fetch fresh sizes from remote sources (slower, requires network).
        // This updates the cache with the latest sizes.
        thread::spawn(move || workers::fetch_sizes_now(models, tx));

        // Spawn a thread to process size updates and emit events to frontend.
        let sizes_map = Arc::clone(&self.sizes);
        let publisher = Arc::clone(&self.events);
        thread::spawn(move || {
            // Batch updates to avoid flooding the frontend with events.
            let mut updates: Vec<(String, Option<u64>)> = Vec::new();

            // Process all size updates from both cached and remote sources.
            while let Ok((id, size)) = rx.recv() {
                // Update the in-memory sizes cache if we got a valid size.
                if let Ok(mut m) = sizes_map.lock()
                    && let Some(bytes) = size
                {
                    m.insert(id.clone(), bytes);
                }
                // Add to batch for frontend notification.
                updates.push((id, size));

                // Emit batched updates every 16 models (reduces event overhead).
                if updates.len() >= 16 {
                    if let Ok(val) = serde_json::to_value(
                        updates
                            .iter()
                            .map(|(id, s)| serde_json::json!({"modelId": id, "sizeBytes": s}))
                            .collect::<Vec<_>>(),
                    ) {
                        publisher.emit_json("model_sizes_updated", val);
                    }
                    updates.clear();
                }
            }

            // Emit any remaining updates that didn't reach the batch size.
            if !updates.is_empty()
                && let Ok(val) = serde_json::to_value(
                    updates
                        .iter()
                        .map(|(id, s)| serde_json::json!({"modelId": id, "sizeBytes": s}))
                        .collect::<Vec<_>>(),
                )
            {
                publisher.emit_json("model_sizes_updated", val);
            }
        });
        Ok(())
    }

    fn purge_models_cache(&self) -> Result<(), String> {
        // Get the root cache directory where all models are stored.
        let root = keyless_models::hf::keyless_cache_root();

        // Remove the entire cache directory if it exists.
        if root.exists() {
            std::fs::remove_dir_all(&root).map_err(|e| format!("purge: {e}"))?;
        }

        // Recreate the cache directory (empty).
        std::fs::create_dir_all(&root).map_err(|e| format!("recreate cache: {e}"))?;

        // Clear the in-memory sizes cache.
        if let Ok(mut s) = self.sizes.lock() {
            s.clear();
        }

        // Clear the installed models list.
        if let Ok(mut installed) = self.installed.lock() {
            installed.clear();
        }

        // Save empty sizes cache to disk (persist the purge).
        keyless_models::meta::save_sizes_cache(&HashMap::new());

        // Notify frontend that the catalog was refreshed (purged).
        if let Ok(val) = serde_json::to_value(serde_json::json!({"ok": true})) {
            self.events.emit_json("catalog_refreshed", val);
        }
        Ok(())
    }

    fn remove_model(&self, model_id: &str) -> Result<(), String> {
        // Models are stored under "<root>/<org>--<repo>/" (not "<root>/<org>/<repo>/").
        // The double-dash format avoids nested directories. Use the helper to construct the correct path.
        let path = keyless_models::hf::keyless_cache_repo_dir(model_id);

        // Verify the model is actually installed before attempting removal.
        if !path.exists() {
            return Err("model not installed".to_string());
        }

        // Remove the model directory and all its contents.
        std::fs::remove_dir_all(&path).map_err(|e| format!("remove model: {e}"))?;

        // Intentionally DO NOT remove the model's size from the in-memory cache.
        // Keeping the known size allows the UI to continue showing accurate size
        // information when the model becomes "available" again after deletion.
        // This is a UX improvement: users can see how big a model is even after deleting it.
        if let Ok(guard) = self.sizes.lock() {
            // Persist the sizes cache (which still contains this model's size).
            keyless_models::meta::save_sizes_cache(&guard);
        }

        // Remove the model from the installed list.
        if let Ok(mut installed) = self.installed.lock() {
            installed.retain(|m| m != model_id);
        }

        // Notify frontend that the model was removed.
        if let Ok(val) = serde_json::to_value(serde_json::json!({ "modelId": model_id })) {
            self.events.emit_json("model_removed", val);
        }
        Ok(())
    }

    fn status(&self, model_id: &str) -> Result<ModelStatus, String> {
        // Check if there's an active download entry for this model.
        let guard = self
            .downloads
            .lock()
            .map_err(|_| "models: download map poisoned".to_string())?;
        if let Some(entry) = guard.get(model_id) {
            // Return the status from the download entry (includes progress, stage, errors).
            return Ok(entry.status.clone());
        }

        // No download entry: check if the model is installed.
        // This handles the case where a model is installed but not currently downloading.
        let downloaded = {
            let guard = self
                .installed
                .lock()
                .map_err(|_| "models: installed list poisoned".to_string())?;
            guard.iter().any(|m| m == model_id)
        };

        // Return a simple status (installed or not, no download state).
        Ok(ModelStatus {
            downloaded,
            downloading: false,
            stage: None,
            progress: None,
            error: None,
            paused: false,
        })
    }
}
