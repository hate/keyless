//! CPAL-based audio input implementation.
//!
//! Threading model:
//! - OS audio callback runs on a realtime thread (must not block!)
//! - User callback runs on a dedicated worker thread (can block safely)
//!
//! This separation ensures audio I/O never starves, even if user callbacks are slow.

use std::sync::{Arc, Mutex, mpsc};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use keyless_core::error::KeylessResult;
use tracing::{error, info};

use super::api::{AudioInput, FrameCallback, InputDevice};
use crate::config::AudioConfig;

/// CPAL-based audio input implementation.
#[derive(Default)]
pub struct CpalAudioInput {
    /// Active CPAL audio stream (None when stopped).
    stream: Option<cpal::Stream>,
    /// Worker thread handle (None when stopped).
    worker: Option<std::thread::JoinHandle<()>>,
    /// Shutdown signal sender (None when stopped).
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Currently chosen sample rate in Hz (None before start).
    chosen_sample_rate_hz: Option<u32>,
    /// Currently chosen frame size in samples (None before start).
    chosen_frame_samples: Option<usize>,
}

impl CpalAudioInput {
    /// Create a new audio input (not started yet).
    pub fn new() -> Self {
        Self::default()
    }

    /// Start with an explicit CPAL device (internal helper).
    fn start_with_device(
        &mut self,
        device: &cpal::Device,
        config: AudioConfig,
        on_frame: FrameCallback,
    ) -> KeylessResult<()> {
        // Prevent double-start (would leak resources or cause conflicts).
        if self.stream.is_some() {
            return Err(keyless_core::error::KeylessError::Audio(
                "audio already started".to_string(),
            ));
        }

        // Determine the chosen input configuration via three-tier selection.
        let chosen = {
            // Collect supported configs into Vec to iterate multiple times (needed for 48k check).
            let list = match device.supported_input_configs() {
                Ok(iter) => iter.collect::<Vec<_>>(),
                Err(e) => {
                    return Err(keyless_core::error::KeylessError::Audio(format!(
                        "failed to list input configs: {}",
                        e
                    )));
                }
            };

            // Helper to pick 48 kHz if the device supports it (for determinism and EQ quality).
            // 48 kHz is optimal for Whisper (native rate) and avoids resampling overhead.
            let pick_48k = || -> Option<cpal::SupportedStreamConfig> {
                for range in &list {
                    // Check if 48k falls within the device's supported range.
                    if range.min_sample_rate().0 <= 48_000 && 48_000 <= range.max_sample_rate().0 {
                        return Some(range.with_sample_rate(cpal::SampleRate(48_000)));
                    }
                }
                None
            };

            // Tier 1: If explicit rate requested (config.sample_hz > 0), try to honor it.
            let mut selected: Option<cpal::SupportedStreamConfig> = if config.sample_hz > 0 {
                let mut found: Option<cpal::SupportedStreamConfig> = None;
                // First-match wins; break early to avoid unnecessary iteration.
                for range in &list {
                    if range.min_sample_rate().0 <= config.sample_hz
                        && config.sample_hz <= range.max_sample_rate().0
                    {
                        found = Some(range.with_sample_rate(cpal::SampleRate(config.sample_hz)));
                        break;
                    }
                }
                found
            } else {
                // Tier 2: No explicit rate; prefer 48 kHz for optimal quality.
                pick_48k()
            };

            // Tier 3: Fallback to device default if nothing selected yet (handles edge cases).
            if selected.is_none() {
                selected = Some(device.default_input_config().map_err(|e| {
                    keyless_core::error::KeylessError::Audio(format!(
                        "failed to query default input config: {}",
                        e
                    ))
                })?);
            }

            let mut chosen = match selected {
                Some(cfg) => cfg,
                None => {
                    // Shouldn't happen (default_input_config should always return something).
                    return Err(keyless_core::error::KeylessError::Audio(
                        "no supported input configuration available".to_string(),
                    ));
                }
            };

            // Cap excessively high defaults (>48k): prefer 48 kHz to avoid unnecessary bandwidth.
            // Some devices default to 96k/192k which wastes CPU without quality benefit for speech.
            if chosen.sample_rate().0 > 48_000
                && let Some(cfg48) = pick_48k()
            {
                chosen = cfg48;
            }
            chosen
        };

        // Compute final frame size: auto mode = 100ms (sample_rate / 10), else use explicit value.
        // Max(1) prevents zero-sized frames if sample_rate < 10 (shouldn't happen but defensive).
        let chosen_sample_rate = chosen.sample_rate().0;
        let final_frame_samples = if config.frame_samples == 0 {
            (chosen_sample_rate / 10).max(1) as usize
        } else {
            config.frame_samples
        };

        // Sync channel with capacity 64: OS callback can enqueue many frames without blocking.
        // Bounded buffer prevents unbounded memory growth if worker thread is slow.
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(64);
        // Async channel for shutdown (unbounded is fine for single message).
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        // Worker thread runs user callback (can block safely, unlike OS audio callback).
        let worker = std::thread::spawn(move || {
            let mut cb = on_frame;
            loop {
                // Check shutdown signal first (non-blocking) to allow fast exit.
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }
                // Timeout allows periodic shutdown checks even if no frames arrive.
                match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                    Ok(frame) => cb(&frame),
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        // Shared buffer: accumulates samples until we have a complete frame (final_frame_samples).
        // Capacity = 2x frame size to handle variable OS callback sizes without frequent reallocs.
        // Mutex guards access from OS callback thread (must be fast; no blocking allowed).
        let shared = Arc::new(Mutex::new((
            Vec::<f32>::with_capacity(final_frame_samples * 2),
            final_frame_samples,
        )));
        let shared_clone = Arc::clone(&shared);
        // Error callback runs on OS audio thread; just log, don't block or panic.
        let err_fn = |err| {
            error!(error = %err, "cpal stream error");
        };

        // Build stream based on sample format; convert all formats to f32 for uniform processing.
        let stream = match chosen.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &chosen.config(),
                move |data: &[f32], _| {
                    // Fast path: F32 is already normalized [-1.0, 1.0], just accumulate.
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        buf.extend_from_slice(data);
                        // Drain complete frames; may emit multiple frames if buffer accumulated enough.
                        while buf.len() >= frame_samples {
                            let frame: Vec<f32> = buf.drain(0..frame_samples).collect();
                            // Try-send non-blocking; drops frame if channel full (worker slow).
                            let _ = tx.try_send(frame);
                        }
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &chosen.config(),
                move |data: &[i16], _| {
                    // Convert i16 [-32768, 32767] to f32 [-1.0, 1.0] via division by MAX.
                    // Cast to f32 before division to avoid integer overflow.
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        for &s in data.iter() {
                            buf.push(s as f32 / i16::MAX as f32);
                        }
                        // Drain complete frames; may emit multiple frames if buffer accumulated enough.
                        while buf.len() >= frame_samples {
                            let frame: Vec<f32> = buf.drain(0..frame_samples).collect();
                            let _ = tx.try_send(frame);
                        }
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &chosen.config(),
                move |data: &[u16], _| {
                    // Convert u16 [0, 65535] to f32 [-1.0, 1.0]: normalize to [0, 1], then shift.
                    // Formula: (s / MAX) * 2.0 - 1.0 maps 0→-1.0, 32767→0.0, 65535→1.0.
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        for &s in data.iter() {
                            let f = (s as f32 / u16::MAX as f32) * 2.0 - 1.0;
                            buf.push(f);
                        }
                        // Drain complete frames; may emit multiple frames if buffer accumulated enough.
                        while buf.len() >= frame_samples {
                            let frame: Vec<f32> = buf.drain(0..frame_samples).collect();
                            let _ = tx.try_send(frame);
                        }
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I8 => device.build_input_stream(
                &chosen.config(),
                move |data: &[i8], _| {
                    // Convert i8 [-128, 127] to f32 [-1.0, 1.0] via division by MAX.
                    // Cast to f32 before division to avoid integer overflow.
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        for &s in data.iter() {
                            buf.push(s as f32 / i8::MAX as f32);
                        }
                        // Drain complete frames; may emit multiple frames if buffer accumulated enough.
                        while buf.len() >= frame_samples {
                            let frame: Vec<f32> = buf.drain(0..frame_samples).collect();
                            let _ = tx.try_send(frame);
                        }
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U8 => device.build_input_stream(
                &chosen.config(),
                move |data: &[u8], _| {
                    // Convert u8 [0, 255] to f32 [-1.0, 1.0]: normalize to [0, 1], then shift.
                    // Formula: (s / MAX) * 2.0 - 1.0 maps 0→-1.0, 127→0.0, 255.0.
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        for &s in data.iter() {
                            let f = (s as f32 / u8::MAX as f32) * 2.0 - 1.0;
                            buf.push(f);
                        }
                        // Drain complete frames; may emit multiple frames if buffer accumulated enough.
                        while buf.len() >= frame_samples {
                            let frame: Vec<f32> = buf.drain(0..frame_samples).collect();
                            let _ = tx.try_send(frame);
                        }
                    }
                },
                err_fn,
                None,
            ),
            _ => {
                // Reject unsupported formats (I24, I32, F64, etc.) upfront.
                return Err(keyless_core::error::KeylessError::Audio(
                    "unsupported sample format".to_string(),
                ));
            }
        };

        match stream {
            Ok(s) => {
                // Start the stream (begins OS audio callbacks); must succeed or stream is invalid.
                if let Err(e) = s.play() {
                    return Err(keyless_core::error::KeylessError::Audio(format!(
                        "failed to start audio stream: {}",
                        e
                    )));
                }
                info!(
                    sample_rate = chosen_sample_rate,
                    frame_size = final_frame_samples,
                    "audio stream started"
                );
                // Store runtime values for querying after start (useful for logging/debugging).
                self.chosen_sample_rate_hz = Some(chosen_sample_rate);
                self.chosen_frame_samples = Some(final_frame_samples);
                self.stream = Some(s);
                self.worker = Some(worker);
                self.shutdown_tx = Some(shutdown_tx);
                Ok(())
            }
            Err(e) => Err(keyless_core::error::KeylessError::Audio(format!(
                "failed to build input stream: {}",
                e
            ))),
        }
    }
}

impl AudioInput for CpalAudioInput {
    fn start(
        &mut self,
        device: Option<InputDevice>,
        config: AudioConfig,
        on_frame: Box<dyn FnMut(&[f32]) + Send + 'static>,
    ) -> KeylessResult<()> {
        let host = cpal::default_host();
        // Use provided device if available, else fallback to OS default.
        // Wrap default device in Arc to match InputDevice's Arc<Device> type.
        let dev_arc = match device {
            Some(d) => d.inner,
            None => Arc::new(host.default_input_device().ok_or_else(|| {
                keyless_core::error::KeylessError::Audio("no input device available".to_string())
            })?),
        };
        self.start_with_device(&dev_arc, config, on_frame)
    }

    fn stop(&mut self) -> KeylessResult<()> {
        // Idempotent check: prevent errors if stop() called multiple times.
        if self.stream.is_none() {
            return Err(keyless_core::error::KeylessError::Audio(
                "audio not started".to_string(),
            ));
        }
        // Drop stream first (stops OS callbacks) before signaling worker to exit.
        self.stream = None;
        // Signal worker thread to exit; ignore send error (worker may have already exited).
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Wait for worker thread to finish (ensures callback is no longer running).
        // Join may panic if thread panicked, but that's acceptable (we're shutting down anyway).
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
        Ok(())
    }
}

impl CpalAudioInput {
    /// Return the runtime-selected sample rate in Hz (after `start()`), if available.
    pub fn chosen_sample_rate_hz(&self) -> Option<u32> {
        self.chosen_sample_rate_hz
    }
    /// Return the runtime-selected frame size in samples (after `start()`), if available.
    pub fn chosen_frame_samples(&self) -> Option<usize> {
        self.chosen_frame_samples
    }
}
