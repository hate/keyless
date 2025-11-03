//! TUI application state.
//!
//! This module provides all state structures and enums needed to render the TUI and manage
//! the application lifecycle. It organizes the implementation into focused submodules:
//!
//! - `app_state`: Main application state (`AppState`), screen enum (`Screen`), and sink choice (`SinkChoice`)
//! - `selections`: User selections in the Config screen (sink, model, language, device, file path)
//! - `overlays`: Overlay state management (`Overlays`, `DownloadState`, `ExpertOverlay`)
//! - `runtime_feedback`: Runtime feedback channels (logs, spectrum, VAD state, preview text)
//! - `model_catalog`: Model catalog management (cached model list, discovery, size fetching)

mod app_state;
mod model_catalog;
mod overlays;
mod runtime_feedback;
mod selections;

pub use app_state::{AppState, Screen, SinkChoice};
pub use model_catalog::ModelCatalog;
pub use overlays::{DownloadState, ExpertOverlay, Overlays};
pub use runtime_feedback::RuntimeFeedback;
pub use selections::Selections;
