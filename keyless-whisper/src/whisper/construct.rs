use std::sync::mpsc;

use keyless_core::error::KeylessError;
use keyless_core::error::KeylessResult;
use keyless_core::events::TranscriptionEvent;

use crate::config::{PhaseState, WhisperConfig, WhisperLoadPhase};
use crate::device::preload_device;

use super::inference_thread::run_inference_thread;
use super::types::Whisper;
use super::worker_thread::run_worker_thread;

impl Whisper {
    /// Create a new Whisper transcriber and start the worker thread.
    ///
    /// This method:
    /// 1. Spawns a worker thread
    /// 2. Downloads/loads the Whisper model (may take 10-60s on first run)
    /// 3. Sets up resampling if source_sample_hz != 16000
    /// 4. Begins listening for audio frames
    ///
    /// If model loading fails, the worker emits an empty Final event as a signal.
    pub fn new(cfg: WhisperConfig) -> KeylessResult<Self> {
        // Delegate to the progress-enabled constructor to avoid duplication.
        Self::new_with_progress::<fn(WhisperLoadPhase, PhaseState)>(cfg, None)
    }

    /// Create a new Whisper transcriber with a progress callback for loading phases.
    pub fn new_with_progress<F>(cfg: WhisperConfig, mut progress: Option<F>) -> KeylessResult<Self>
    where
        F: FnMut(WhisperLoadPhase, PhaseState),
    {
        // Channel sizes: audio frames (512), events (16), inference (1).
        // 512 audio frames: buffer ~10s at 48kHz/1024-frame chunks (prevents audio dropouts).
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(512);
        // 16 events: enough for bursty Partial/Final events without blocking.
        let (tx_evt, rx_evt) = mpsc::sync_channel::<TranscriptionEvent>(16);
        // Unbounded channel for shutdown (low-frequency, don't want to block drop()).
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        // Unbounded channel for commands (low-frequency, don't want to block end_segment()).
        let (cmd_tx, cmd_rx) = mpsc::channel::<super::types::WhisperCmd>();
        // Size 1 inference channel: serializes inference requests (only one runs at a time).
        let (inf_tx, inf_rx) = mpsc::sync_channel::<super::types::InferReq>(1);

        // Initialize or reuse cached device (Metal > CUDA > CPU) with warm-up
        let device = preload_device();

        // Load the Whisper model with progress
        let mut progress_obj = progress
            .as_mut()
            .map(|p| p as &mut dyn FnMut(WhisperLoadPhase, PhaseState));
        let model_components = match crate::model::load_whisper_model_with_progress(
            &cfg,
            &device,
            progress_obj.as_mut(),
        ) {
            Ok(m) => m,
            Err(e) => {
                return Err(KeylessError::Whisper(format!(
                    "model loading failed: {}",
                    e
                )));
            }
        };

        // Clone inference sender for the worker thread
        let inf_tx_worker = inf_tx.clone();

        // Spawn worker thread (resampling/accumulation)
        let worker = std::thread::spawn(move || {
            run_worker_thread(cfg, rx, shutdown_rx, cmd_rx, inf_tx_worker);
        });

        // Spawn inference thread (dedicated thread for Candle ops; avoids blocking audio worker).
        let tx_evt_clone = tx_evt.clone();
        let infer_worker = std::thread::spawn(move || {
            run_inference_thread(model_components, device, inf_rx, tx_evt_clone);
        });

        Ok(Self {
            tx,
            rx_evt,
            worker: Some(worker),
            shutdown_tx: Some(shutdown_tx),
            cmd_tx,
            inf_worker: Some(infer_worker),
        })
    }
}
