//! Pipeline orchestration.
//!
//! This module coordinates audio capture, transcription, and output delivery through a phased
//! startup mechanism. It organizes the implementation into focused submodules:
//!
//! - `startup`: Phased startup logic (`build_startup_artifacts`, `finalize_pipeline_from_artifacts`,
//!   `start_pipeline`) with non-blocking initialization and progress events
//! - `callback`: Audio frame callback with PTT state machine and VAD gating
//! - `events`: Event processing and preview text formatting

mod callback;
mod events;
mod startup;

pub use startup::{
    PipelineChannels, PipelineHandles, StartupArtifacts, StartupEvent, build_startup_artifacts,
    finalize_pipeline_from_artifacts, spawn_startup_worker, start_pipeline,
};
