//! Model catalog IPC commands.
//!
//! Provides Tauri command handlers for model management: listing available and installed
//! models, downloading models, pausing/resuming/canceling downloads, and querying model
//! status. All commands delegate to `ModelsService` for business logic.

use super::DesktopState;
use crate::services::models::{ModelInfo, ModelInfoWithSize, ModelStatus};

/// List models known to the app (installed and available).
///
/// Returns a list of all models from the catalog with their download status. Does not
/// include size information.
///
/// # Errors
///
/// Returns an error string if model discovery or catalog loading fails.
#[tauri::command]
pub fn list_models(ctx: DesktopState<'_>) -> Result<Vec<ModelInfo>, String> {
    // Delegate to the models service to list all known models (installed and available).
    // This returns basic model info without size metadata.
    ctx.models.list_models()
}

/// List models with known sizes and install state.
///
/// Returns a list of all models with their download sizes (from metadata cache) and
/// installation status. This is the preferred method for populating the Models UI.
///
/// # Errors
///
/// Returns an error string if model discovery, catalog loading, or size cache loading fails.
#[tauri::command]
pub fn list_models_with_sizes(ctx: DesktopState<'_>) -> Result<Vec<ModelInfoWithSize>, String> {
    // Delegate to the models service to list all models with size information.
    // This includes download sizes from the metadata cache, which allows showing sizes
    // even when offline.
    ctx.models.list_models_with_sizes()
}

/// Start downloading a model by ID.
///
/// Initiates a download for the specified model. The download runs asynchronously and
/// progress is reported via `model_download_progress` events.
///
/// # Errors
///
/// Returns an error string if the model ID is invalid, download is already in progress,
/// or download initialization fails.
#[tauri::command]
pub fn start_model_download(ctx: DesktopState<'_>, model_id: String) -> Result<(), String> {
    // Delegate to the models service to start downloading the specified model.
    // The download runs in a background thread and progress is reported via events.
    ctx.models.start_download(&model_id)
}

/// Cancel an in-progress download by model ID.
///
/// Cancels the download and removes any partially downloaded files. The model will not
/// be installed.
///
/// # Errors
///
/// Returns an error string if the download is not in progress or cancellation fails.
#[tauri::command]
pub fn cancel_model_download(ctx: DesktopState<'_>, model_id: String) -> Result<(), String> {
    // Delegate to the models service to cancel the download and clean up partial files.
    // This stops the download worker and removes any partially downloaded data.
    ctx.models.cancel_download(&model_id)
}

/// Pause an in-progress download by model ID.
///
/// Pauses the download, preserving partially downloaded files. The download can be resumed
/// later with `resume_model_download`.
///
/// # Errors
///
/// Returns an error string if the download is not in progress or pause fails.
#[tauri::command]
pub fn pause_model_download(ctx: DesktopState<'_>, model_id: String) -> Result<(), String> {
    // Delegate to the models service to pause the download.
    // Partial files are preserved, allowing the download to be resumed later.
    ctx.models.pause_download(&model_id)
}

/// Resume a paused download by model ID.
///
/// Resumes a previously paused download from where it left off.
///
/// # Errors
///
/// Returns an error string if the download is not paused or resume fails.
#[tauri::command]
pub fn resume_model_download(ctx: DesktopState<'_>, model_id: String) -> Result<(), String> {
    // Delegate to the models service to resume a paused download.
    // The download worker supports HTTP Range requests, so it resumes from where it stopped.
    ctx.models.resume_download(&model_id)
}

/// Refresh the remote size metadata cache.
///
/// Fetches the latest model size information from remote sources and updates the local
/// cache. This is useful when new models are added to the catalog.
///
/// # Errors
///
/// Returns an error string if the remote fetch or cache update fails.
#[tauri::command]
pub fn refresh_model_sizes(ctx: DesktopState<'_>) -> Result<(), String> {
    // Delegate to the models service to refresh the size metadata cache.
    // This fetches the latest sizes from remote sources and updates the local cache.
    // Size updates are emitted to the frontend via events as they're discovered.
    ctx.models.refresh_sizes()
}

/// Purge the local models cache directory and clear metadata.
///
/// Removes all downloaded models and clears the size metadata cache. This is a destructive
/// operation that cannot be undone.
///
/// # Errors
///
/// Returns an error string if cache directory removal or metadata clearing fails.
#[tauri::command]
pub fn purge_models_cache(ctx: DesktopState<'_>) -> Result<(), String> {
    // Delegate to the models service to purge all downloaded models and clear metadata.
    // This removes the entire cache directory and clears both in-memory and on-disk caches.
    // Warning: This is a destructive operation that cannot be undone.
    ctx.models.purge_models_cache()
}

/// Remove an installed model by ID.
///
/// Deletes the model files from the local cache. If the model is currently selected,
/// the pipeline will need to be restarted with a different model.
///
/// # Errors
///
/// Returns an error string if the model is not installed or removal fails.
#[tauri::command]
pub fn remove_model(ctx: DesktopState<'_>, model_id: String) -> Result<(), String> {
    // Delegate to the models service to remove a specific model from the cache.
    // This deletes the model's directory but preserves its size in the metadata cache
    // (so the UI can still show the size if the user wants to re-download it).
    ctx.models.remove_model(&model_id)
}

/// Get the current status of a model.
///
/// Returns detailed status information including download state, progress (if downloading),
/// and any error messages. This is used to populate the Models UI with real-time status.
///
/// # Errors
///
/// Returns an error string if status query fails.
#[tauri::command]
pub fn get_model_status(ctx: DesktopState<'_>, model_id: String) -> Result<ModelStatus, String> {
    // Delegate to the models service to get the current status of a model.
    // Returns download state, progress (if downloading), errors, and installation status.
    ctx.models.status(&model_id)
}
