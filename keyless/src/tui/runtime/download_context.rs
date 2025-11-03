//! Download context (channel, cancellation, pending pipeline config).

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};

use keyless_core::config::Config;
use keyless_models::DownloadEvent;

/// Active download context.
pub struct DownloadContext {
    pub rx: mpsc::Receiver<DownloadEvent>,
    pub cancel: Arc<AtomicBool>,
    pub pending_config: Config,
    pub pending_device_idx: usize,
}

impl DownloadContext {
    pub fn new(
        rx: mpsc::Receiver<DownloadEvent>,
        cancel: Arc<AtomicBool>,
        pending_config: Config,
        pending_device_idx: usize,
    ) -> Self {
        Self {
            rx,
            cancel,
            pending_config,
            pending_device_idx,
        }
    }
}
