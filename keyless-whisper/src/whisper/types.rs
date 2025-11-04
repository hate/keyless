use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use keyless_core::events::TranscriptionEvent;

/// Candle-based Whisper transcriber.
///
/// Manages a worker thread that runs Whisper inference using the Candle ML framework.
/// The worker owns the model, device, and decoder; the caller interacts via channels.
pub struct Whisper {
    /// Channel to send audio frames to the worker thread.
    pub(crate) tx: mpsc::SyncSender<Vec<f32>>,

    /// Channel to receive transcription events from the worker thread.
    pub(crate) rx_evt: Receiver<TranscriptionEvent>,

    /// Worker thread handle. Joined during drop() for clean shutdown.
    pub(crate) worker: Option<JoinHandle<()>>,

    /// Shutdown signal sender; dropping it signals the worker to exit.
    pub(crate) shutdown_tx: Option<mpsc::Sender<()>>,

    /// Command channel to control the worker (e.g., end of segment).
    pub(crate) cmd_tx: Sender<WhisperCmd>,

    /// Inference worker handle.
    pub(crate) inf_worker: Option<JoinHandle<()>>,
}

/// Commands sent to the worker thread to control segmentation.
/// Control commands sent to the worker thread.
pub(crate) enum WhisperCmd {
    /// Finalize the current segment and emit a Final event.
    EndSegment,
}

/// Requests sent to the inference thread.
/// Inference requests sent to the inference thread.
pub(crate) enum InferReq {
    /// Run a windowed partial inference on the provided PCM samples.
    Partial(Vec<f32>),
    /// Run a full final inference on the provided PCM samples.
    Final(Vec<f32>),
}
