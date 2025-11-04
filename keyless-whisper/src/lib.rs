//! keyless-whisper
//!
//! Real‑time speech‑to‑text transcription using Whisper via Candle. Provides a clean,
//! modular Whisper transcription engine using Hugging Face's Candle ML framework.
//!
//! Architecture: `Audio Thread → push_audio() → Queue → Worker Thread (resample/accumulate) → Inference Thread (Candle) → Event Queue → try_next_event()`
//!
//! ## Key Features
//!
//! - **Quantized model support**: 3-4x faster inference with GGUF models
//! - **Quality improvements**: No-speech detection, temperature fallback, quality metrics
//! - **Performance optimized**: Zero-copy model access, single encoder pass
//! - **Language auto-detection**: Automatic language identification when not specified
//! - **Segment-aware**: Live preview while speaking; single Final on PTT release
//! - **Windowed inference**: ≤30s windows for long utterances (Whisper's native context)
//! - **Non-blocking**: Audio/resample never blocks; inference on dedicated thread
//! - **High-quality resampling**: Arbitrary-rate via rubato (16k, 32k, 44.1k, 48k all supported)
//!
//! ## Submodules
//!
//! - `model`: Model downloading/loading, quantized support (GGUF + safetensors)
//! - `preprocessing`: PCM audio preprocessing and mel spectrogram computation
//! - `decode`: Token generation, temperature fallback, no-speech detection, quality metrics
//! - `inference`: Encoder‑decoder pipeline with windowing and language auto-detection
//!
//! ## Public API
//!
//! - `Whisper`: Main transcriber implementation
//! - `RealtimeTranscriber`: Trait for real-time transcription
//! - `WhisperConfig`: Configuration (model path, language, sample rate)
//!
//! ## Quick Example
//!
//! ```rust,no_run
//! use keyless_whisper::{Whisper, WhisperConfig, RealtimeTranscriber};
//! use std::path::PathBuf;
//!
//! let cfg = WhisperConfig {
//!     model_path: PathBuf::from("openai/whisper-base"),  // Multilingual recommended
//!     language: Some("en".to_string()),                   // Manual faster than auto-detect
//!     source_sample_hz: 48_000,
//! };
//! let mut transcriber = Whisper::new(cfg)?;
//! // transcriber.push_audio(&frame)?;
//! // if let Some(event) = transcriber.try_next_event() { /* handle */ }
//! # Ok::<(), keyless_core::error::KeylessError>(())
//! ```
//!

#![deny(warnings)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![cfg_attr(
    doc,
    deny(
        missing_docs,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_html_tags,
        rustdoc::bare_urls
    )
)]

mod decode;
pub mod inference;
pub mod model;
mod preprocessing;

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use candle_core::{DType, Device, Tensor};
use rubato::Resampler;
use tracing::{error, info};

use keyless_core::error::KeylessError;
use keyless_core::error::KeylessResult;
use keyless_core::events::TranscriptionEvent;

/// Loading phase identifiers for Whisper model initialization.
#[derive(Clone, Copy, Debug)]
pub enum WhisperLoadPhase {
    /// Resolving model source (local cache path or remote HF ID).
    ModelSource,
    /// Detecting model format (safetensors vs. gguf quantized).
    DetectFormat,
    /// Reading model configuration (config.json).
    ReadConfig,
    /// Parsing model configuration JSON.
    ParseConfig,
    /// Mapping model weights into memory (mmap or gguf loader).
    MapWeights,
    /// Constructing model layers/graph.
    ConstructModel,
    /// Loading tokenizer JSON.
    LoadTokenizer,
    /// Resolving special token IDs (SOT/EOT/etc.).
    ResolveTokens,
    /// Building mel filter bank.
    BuildMelFilters,
}

impl WhisperLoadPhase {
    /// Return a user-facing label for this phase.
    pub fn as_label(&self) -> &'static str {
        match self {
            WhisperLoadPhase::ModelSource => "locating model",
            WhisperLoadPhase::DetectFormat => "detecting model format",
            WhisperLoadPhase::ReadConfig => "reading config",
            WhisperLoadPhase::ParseConfig => "parsing config",
            WhisperLoadPhase::MapWeights => "mapping weights",
            WhisperLoadPhase::ConstructModel => "constructing model",
            WhisperLoadPhase::LoadTokenizer => "loading tokenizer",
            WhisperLoadPhase::ResolveTokens => "resolving tokens",
            WhisperLoadPhase::BuildMelFilters => "building mel filters",
        }
    }
}

/// Loading phase state (begin/end).
#[derive(Clone, Copy, Debug)]
pub enum PhaseState {
    /// Phase begins.
    Begin,
    /// Phase ends successfully.
    End,
}

static DEVICE_CACHE: std::sync::OnceLock<Device> = std::sync::OnceLock::new();

/// Probe and cache the preferred Candle device (Metal > CUDA > CPU) and perform a tiny warm-up.
///
/// Safe to call multiple times; the device is selected only once per process.
pub fn preload_device() -> Device {
    // Check cache first (device selection is expensive; avoid repeated GPU probes).
    if let Some(d) = DEVICE_CACHE.get() {
        return d.clone();
    }
    // Fallback chain: Metal (macOS) → CUDA (Linux/Windows) → CPU (always available).
    let device = match Device::new_metal(0) {
        Ok(d) => {
            info!("using Metal GPU device");
            d
        }
        Err(_) => match Device::new_cuda(0) {
            Ok(d) => {
                info!("using CUDA GPU device");
                d
            }
            // CPU fallback (slower but always works).
            Err(_) => {
                info!("using CPU device");
                Device::Cpu
            }
        },
    };
    // Best-effort kernel warm-up (first tensor op often slower; this primes the driver).
    // Ignore errors (warm-up failure doesn't prevent inference).
    let _ = Tensor::zeros((1,), DType::F32, &device);
    // Cache device for reuse (OnceLock ensures single initialization).
    let _ = DEVICE_CACHE.set(device.clone());
    device
}

/// Configuration for the Whisper transcriber.
///
/// Specifies the model source, language hint, and source audio sample rate.
/// The transcriber will handle model downloading and resampling automatically.
#[derive(Clone, Debug)]
pub struct WhisperConfig {
    /// Model source: either a Hugging Face model ID or a local directory path.
    ///
    /// **Hugging Face model IDs** (auto-downloads if not cached):
    /// - "openai/whisper-tiny" (~75 MB, fastest, multilingual, supports auto-detection)
    /// - "openai/whisper-base" (~142 MB, balanced, multilingual, supports auto-detection)
    /// - "openai/whisper-small" (~466 MB, good accuracy, multilingual)
    /// - "openai/whisper-tiny.en" (~75 MB, fastest, English-only)
    /// - "openai/whisper-base.en" (~142 MB, balanced, English-only)
    /// - ...
    ///
    /// **Local directory paths** (must contain config.json, model.safetensors, tokenizer.json):
    /// - Example: "/Users/name/.cache/keyless/models/whisper-tiny"
    ///
    /// If not specified or if the path doesn't exist, defaults to "openai/whisper-tiny".
    pub model_path: PathBuf,

    /// Language code for transcription (ISO 639-1, e.g., "en", "es", "fr").
    /// Providing a hint significantly improves accuracy vs. auto-detection.
    /// `None` = let Whisper auto-detect (slower and less accurate).
    pub language: Option<String>,

    /// Sample rate of the incoming audio (e.g., 48000, 44100, 16000).
    /// The transcriber will resample to 16 kHz mono if necessary.
    pub source_sample_hz: u32,
}

/// Trait for real-time audio transcription engines.
///
/// Provides a segment-aware interface: accumulate audio during a speaking segment,
/// emit live previews via `Partial` events, and finalize with a single `Final` event
/// when `end_segment()` is called (e.g., on PTT release).
///
/// # Usage Pattern
/// ```rust,no_run
/// # use keyless_whisper::RealtimeTranscriber;
/// # use keyless_core::events::TranscriptionEvent;
/// # use keyless_core::error::KeylessResult;
/// # fn example(transcriber: &mut dyn RealtimeTranscriber) -> KeylessResult<()> {
/// # let frame = vec![0.0f32; 4800];
/// // During speaking (PTT held):
/// transcriber.push_audio(&frame)?;
/// while let Some(evt) = transcriber.try_next_event() {
///     match evt {
///         TranscriptionEvent::Partial(text) => { /* show preview */ }
///         TranscriptionEvent::Final(text) => { /* unexpected during active segment */ }
///     }
/// }
///
/// // On PTT release:
/// transcriber.end_segment()?;
/// while let Some(evt) = transcriber.try_next_event() {
///     if let TranscriptionEvent::Final(text) = evt {
///         // Use final transcription
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub trait RealtimeTranscriber {
    /// Push an audio frame for transcription (non-blocking).
    ///
    /// Audio is accumulated in an internal buffer until `end_segment()` is called.
    /// During accumulation, periodic `Partial` events may be emitted for live preview.
    ///
    /// # Arguments
    /// * `pcm_f32` - Mono audio samples at native sample rate, f32 in [-1.0, 1.0]
    ///
    /// # Errors
    /// Returns `KeylessError::Whisper` if the queue is full (backpressure).
    /// This indicates the worker thread is overloaded; caller should back off.
    fn push_audio(&mut self, pcm_f32: &[f32]) -> KeylessResult<()>;

    /// Poll for the next transcription event (non-blocking).
    ///
    /// Returns `Some(event)` if a new event is available, `None` otherwise.
    /// Should be called frequently (e.g., every audio frame) to drain events and prevent backpressure.
    ///
    /// # Event Types
    /// - `Partial`: Live preview text (last few words) while speaking. For UI feedback only.
    /// - `Final`: Complete transcription after `end_segment()` completes. Deliver to sink.
    fn try_next_event(&mut self) -> Option<TranscriptionEvent>;

    /// Signal the end of the current speaking segment (e.g., PTT released).
    ///
    /// Triggers final inference over the accumulated audio buffer and emits a single
    /// `TranscriptionEvent::Final` with the complete transcription. The buffer is cleared
    /// after finalization; subsequent `push_audio()` calls start a new segment.
    ///
    /// # Errors
    /// Returns `KeylessError::Whisper` if the segment cannot be finalized (e.g., worker shutdown).
    fn end_segment(&mut self) -> KeylessResult<()>;
}

/// Candle-based Whisper transcriber.
///
/// Manages a worker thread that runs Whisper inference using the Candle ML framework.
/// The worker owns the model, device, and decoder; the caller interacts via channels.
pub struct Whisper {
    /// Channel to send audio frames to the worker thread.
    tx: mpsc::SyncSender<Vec<f32>>,

    /// Channel to receive transcription events from the worker thread.
    rx_evt: Receiver<TranscriptionEvent>,

    /// Worker thread handle. Joined during drop() for clean shutdown.
    worker: Option<JoinHandle<()>>,

    /// Shutdown signal sender; dropping it signals the worker to exit.
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Command channel to control the worker (e.g., end of segment).
    cmd_tx: Sender<WhisperCmd>,

    /// Inference worker handle.
    inf_worker: Option<JoinHandle<()>>,
}

/// Commands sent to the worker thread to control segmentation.
/// Control commands sent to the worker thread.
enum WhisperCmd {
    /// Finalize the current segment and emit a Final event.
    EndSegment,
}

/// Requests sent to the inference thread.
/// Inference requests sent to the inference thread.
enum InferReq {
    /// Run a windowed partial inference on the provided PCM samples.
    Partial(Vec<f32>),
    /// Run a full final inference on the provided PCM samples.
    Final(Vec<f32>),
}

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
        let (cmd_tx, cmd_rx) = mpsc::channel::<WhisperCmd>();
        // Size 1 inference channel: serializes inference requests (only one runs at a time).
        let (inf_tx, inf_rx) = mpsc::sync_channel::<InferReq>(1);

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
            // Set up resampling if the microphone doesn't run at 16 kHz
            //
            // Why resample: Whisper expects exactly 16,000 samples per second (16 kHz). If the mic
            // is 48 kHz, we have too many samples. If it's 44.1 kHz, we have the wrong count.
            //
            // How rubato works: Uses FFT-based sinc interpolation with anti-aliasing. This prevents
            // high-frequency content from "folding back" (aliasing) when we downsample.
            //
            // Think of it like: resizing an image. You can't just delete pixels—you need to blend
            // nearby values smoothly. Same idea for audio.
            let needs_resample = cfg.source_sample_hz != 16000;
            let mut resampler: Option<rubato::FftFixedInOut<f32>> = if needs_resample {
                info!(
                    source_hz = cfg.source_sample_hz,
                    target_hz = 16_000,
                    "initializing rubato resampler"
                );
                match rubato::FftFixedInOut::<f32>::new(
                    cfg.source_sample_hz as usize,
                    16_000,
                    1024, // chunk size (balanced for latency/quality)
                    1,    // mono
                ) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        error!(error = ?e, "rubato init failed; falling back to passthrough");
                        None
                    }
                }
            } else {
                info!("no resampling needed (source is 16 kHz)");
                None
            };

            // Processing loop state.
            // input_accum: source-rate (e.g., 48 kHz) accumulator used to feed rubato in fixed-size chunks.
            // Rubato requires exact chunk sizes (1024 samples); accumulate until we have enough.
            let mut input_accum: Vec<f32> = Vec::new();
            // buffer: 16 kHz domain accumulation used by inference (resampled audio).
            // Grows during speaking segment; cleared on end_segment().
            let mut buffer: Vec<f32> = Vec::new();
            // Preview cadence: emit every ~0.1s of new audio (1600 samples at 16 kHz).
            // Partial stride: minimum new samples before emitting next preview.
            let partial_stride = 16_000 / 10; // 0.1s partials
            let mut last_partial_emit_at: usize = 0;
            // Time gate: prevent partial spam (minimum 120ms between previews).
            let mut last_partial_time: Instant = Instant::now();

            // Main loop: receive audio, resample, accumulate, infer.
            loop {
                // Check shutdown signal (non-blocking; allows graceful exit).
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }
                // Check commands (non-blocking; allows end_segment() to interrupt audio processing).
                if let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        WhisperCmd::EndSegment => {
                            // Clone buffer for final inference (worker continues, inference runs async).
                            let pcm = buffer.clone();
                            // Send final request (blocking; ensures request is queued).
                            if let Err(e) = inf_tx_worker.send(InferReq::Final(pcm)) {
                                error!(error = %e, "failed to send final inference request");
                            }
                            // Clear buffer for next segment (start fresh after PTT release).
                            buffer.clear();
                            last_partial_emit_at = 0;
                            last_partial_time = Instant::now();
                        }
                    }
                }
                // Receive audio frame with timeout (allows periodic shutdown/cmd checks).
                // Timeout: 1s (balance between responsiveness and CPU usage).
                let frame = match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(f) => f,
                    // Timeout: loop back to check shutdown/cmd (keeps loop responsive).
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    // Disconnected: audio thread stopped, exit worker.
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };

                if let Some(r) = resampler.as_mut() {
                    // Accumulate source-rate samples, feed rubato with the next requested block size.
                    // Rubato needs exact chunk sizes (1024 samples at source rate); accumulate until enough.
                    input_accum.extend_from_slice(&frame);
                    loop {
                        // Query rubato: how many input frames needed for next output chunk?
                        let need = r.input_frames_next();
                        // If we don't have enough, break and wait for next frame.
                        if input_accum.len() < need {
                            break;
                        }
                        // Take exactly the number of frames rubato expects this call.
                        // drain(0..need) removes from front (FIFO order preserves audio continuity).
                        let chunk: Vec<f32> = input_accum.drain(0..need).collect();
                        match r.process(&[&chunk], None) {
                            Ok(out) => {
                                let out0 = &out[0];
                                // One-time confirmation log for sizes/ratio (debugging resampling).
                                static ONCE: std::sync::Once = std::sync::Once::new();
                                ONCE.call_once(|| {
                                    tracing::info!(
                                        in_len = need,
                                        out_len = out0.len(),
                                        "rubato processed chunk"
                                    );
                                });
                                // Append resampled output to 16 kHz buffer (ready for inference).
                                buffer.extend_from_slice(out0);
                            }
                            Err(e) => {
                                error!(error = ?e, "rubato process failed; skipping chunk");
                                // On failure, break to avoid tight error loop; keep remaining samples for next round.
                                // Remaining samples in input_accum will be processed on next frame.
                                break;
                            }
                        }
                    }
                } else {
                    // Already 16 kHz: append frame directly into 16 kHz buffer (no resampling needed).
                    buffer.extend_from_slice(&frame);
                }

                // Check if enough new samples accumulated since last partial (sample-based gate).
                // saturating_sub prevents underflow if buffer was cleared.
                let enough_new_samples =
                    buffer.len().saturating_sub(last_partial_emit_at) >= partial_stride;
                // Time gate: minimum spacing between previews (120ms prevents spam).
                // Dual gate (samples + time) ensures reasonable preview cadence.
                let enough_time = last_partial_time.elapsed() >= Duration::from_millis(120);

                // Emit partial previews for live feedback while the user is still speaking.
                // Strategy: Every ~0.2s, transcribe just the last ~1s of audio and show a preview.
                // Why short windows: Faster inference = lower perceived latency. We don't need perfect
                // accuracy for previews (they're just "last few words" hints).
                // Why not the full buffer: Transcribing 10+ seconds every 0.2s would be too slow.
                // Instead, we only look at recent context, then do a full transcription on PTT release.
                if enough_new_samples && enough_time && !buffer.is_empty() {
                    // Last 1 second of audio (enough context for preview, fast inference).
                    const PARTIAL_WINDOW_S: usize = 1;

                    // Grab the last N seconds (partial window).
                    // saturating_sub ensures we don't go negative if buffer < 1s.
                    let max_samples = crate::inference::WHISPER_SAMPLE_RATE * PARTIAL_WINDOW_S;
                    let start = buffer.len().saturating_sub(max_samples);
                    // Clone slice for inference thread (worker continues, inference runs async).
                    let pcm = buffer[start..].to_vec();

                    // Send to inference thread (non-blocking; skip if inference is busy).
                    // try_send fails if inference is still processing previous request (size=1 channel).
                    // Silently skip (preview is best-effort; don't block audio processing).
                    if let Err(_e) = inf_tx_worker.try_send(InferReq::Partial(pcm)) {}
                    // Update tracking: mark this position as last emit point.
                    last_partial_emit_at = buffer.len();
                    last_partial_time = Instant::now();
                }
            }
        });

        // Spawn inference thread (dedicated thread for Candle ops; avoids blocking audio worker).
        let tx_evt_clone = tx_evt.clone();
        let infer_worker = std::thread::spawn(move || {
            // Make mutable for &mut inference calls (model state may change during inference).
            let mut model_components = model_components;
            loop {
                // Blocking receive: inference thread waits for requests (serialized by size=1 channel).
                match inf_rx.recv() {
                    Ok(InferReq::Partial(pcm)) => {
                        // Run windowed inference (last 1s for preview; fast, low latency).
                        let text = match inference::run_inference_window(
                            &mut model_components,
                            &pcm,
                            &device,
                        ) {
                            Ok(t) => t,
                            Err(e) => {
                                error!(error = %e, "inference error on partial");
                                // Return empty string on error (preview is best-effort).
                                String::new()
                            }
                        };
                        // Send Partial event (best-effort; ignore send failure).
                        let _ = tx_evt_clone.send(TranscriptionEvent::Partial(text));
                    }
                    Ok(InferReq::Final(mut pcm)) => {
                        // Add 120ms silence tail (catches trailing speech after PTT release).
                        // Some users release PTT slightly before finishing words; tail captures this.
                        let tail = (0.12 * 16_000.0) as usize;
                        let new_len = pcm.len() + tail;
                        pcm.resize(new_len, 0.0f32);
                        // Enforce minimum 3s length (Whisper works better with sufficient context).
                        // Very short segments (< 3s) may produce lower quality transcriptions.
                        let min_final = 16_000 * 3;
                        if pcm.len() < min_final {
                            pcm.resize(min_final, 0.0f32);
                        }
                        // Run full inference (all windows, complete transcription).
                        let text = match inference::run_inference_full(
                            &mut model_components,
                            &pcm,
                            &device,
                        ) {
                            Ok(t) => t,
                            Err(e) => {
                                error!(error = %e, "inference error on end-segment final");
                                // Return empty string on error (caller should handle gracefully).
                                String::new()
                            }
                        };
                        // Send Final event (best-effort; ignore send failure).
                        let _ = tx_evt_clone.send(TranscriptionEvent::Final(text));
                    }
                    // Channel disconnected: exit inference thread (clean shutdown).
                    Err(_) => break,
                }
            }
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

impl RealtimeTranscriber for Whisper {
    fn push_audio(&mut self, pcm_f32: &[f32]) -> KeylessResult<()> {
        // Non-blocking send: try_send fails if queue is full (backpressure).
        // Clone frame (Vec<f32>) for worker thread (audio thread doesn't block).
        match self.tx.try_send(pcm_f32.to_vec()) {
            Ok(()) => Ok(()),
            // Queue full: worker thread is overloaded; return error (caller should back off).
            Err(_) => Err(keyless_core::error::KeylessError::Whisper(
                "transcriber queue full".to_string(),
            )),
        }
    }

    fn try_next_event(&mut self) -> Option<TranscriptionEvent> {
        // Non-blocking receive: returns None if no events available.
        // Should be called frequently (every audio frame) to drain events and prevent backpressure.
        self.rx_evt.try_recv().ok()
    }

    fn end_segment(&mut self) -> KeylessResult<()> {
        // Blocking send: ensures command is queued (don't want to lose end_segment()).
        // Unbounded channel won't block unless receiver is dropped (worker shutdown).
        match self.cmd_tx.send(WhisperCmd::EndSegment) {
            Ok(()) => Ok(()),
            // Channel disconnected: worker thread already shut down.
            Err(e) => Err(KeylessError::Whisper(format!(
                "failed to send EndSegment: {}",
                e
            ))),
        }
    }
}

impl Drop for Whisper {
    fn drop(&mut self) {
        // Signal worker to shutdown (non-blocking; best-effort).
        // Worker checks shutdown_rx in main loop and exits gracefully.
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Drop inference sender to stop inference thread (channel disconnect signals exit).
        // inf_tx is cloned; dropping original causes inf_rx.recv() to return Err.
        // Inference thread exits when it sees channel disconnect.
        // Wait for worker to exit cleanly (join ensures no resource leaks).
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
        // Wait for inference worker to exit cleanly (ensures model cleanup).
        if let Some(h) = self.inf_worker.take() {
            let _ = h.join();
        }
    }
}
