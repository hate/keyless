//! Audio pipeline lifecycle management service.
//!
//! Provides a trait-based abstraction for controlling the audio pipeline lifecycle:
//! initialization, status checking, and restart requests. The implementation delegates
//! to the runtime thread API.

/// Service trait for audio pipeline lifecycle management.
///
/// All methods operate on the runtime thread's pipeline state. Pipeline operations
/// happen asynchronously on the runtime thread.
pub trait PipelineService: Send + Sync {
    /// Initialize the runtime thread and start the pipeline if not already active.
    ///
    /// Ensures the runtime thread is initialized and requests a pipeline restart if needed.
    fn init(&self) -> Result<(), String>;

    /// Check whether the audio pipeline is currently active.
    fn is_active(&self) -> bool;

    /// Request the runtime thread to restart the pipeline.
    ///
    /// The restart happens asynchronously. This is typically called after configuration
    /// changes that require a pipeline restart (e.g., device change, model change).
    fn request_restart(&self);
}

/// Default implementation of `PipelineService` using the runtime thread API.
pub struct PipelineServiceImpl;

impl PipelineServiceImpl {
    /// Create a new `PipelineServiceImpl` instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PipelineServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineService for PipelineServiceImpl {
    fn init(&self) -> Result<(), String> {
        // Ensure the runtime thread is initialized (creates the background thread if needed).
        if !crate::runtime::is_initialized() {
            crate::runtime::init().map_err(|e| format!("runtime init: {e}"))?;
        }
        // If the pipeline is not active, request a restart to start it.
        // This handles the case where the runtime thread exists but the pipeline hasn't started yet.
        if !crate::runtime::is_pipeline_active() {
            crate::runtime::request_restart();
        }
        Ok(())
    }

    fn is_active(&self) -> bool {
        // Delegate to the runtime API to check if the pipeline is currently running.
        // Returns true if audio is being processed, false otherwise.
        crate::runtime::is_pipeline_active()
    }

    fn request_restart(&self) {
        // Request the runtime thread to restart the pipeline.
        // This is asynchronous: the restart happens on the runtime thread, not immediately.
        // Used after config changes that require pipeline reconfiguration (device, model, etc.).
        crate::runtime::request_restart();
    }
}
