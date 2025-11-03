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
        if self.stream.is_some() {
            return Err(keyless_core::error::KeylessError::Audio(
                "audio already started".to_string(),
            ));
        }

        // Determine the chosen input configuration.
        let chosen = {
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
            let pick_48k = || -> Option<cpal::SupportedStreamConfig> {
                for range in &list {
                    if range.min_sample_rate().0 <= 48_000 && 48_000 <= range.max_sample_rate().0 {
                        return Some(range.with_sample_rate(cpal::SampleRate(48_000)));
                    }
                }
                None
            };

            // If an explicit sample rate was requested, honor it when supported.
            let mut selected: Option<cpal::SupportedStreamConfig> = if config.sample_hz > 0 {
                let mut found: Option<cpal::SupportedStreamConfig> = None;
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
                pick_48k()
            };

            // Fallback to device default if nothing selected yet.
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
                    return Err(keyless_core::error::KeylessError::Audio(
                        "no supported input configuration available".to_string(),
                    ));
                }
            };

            // Cap excessively high defaults: pick the highest supported rate ≤ 48 kHz, prefer 48 kHz.
            if chosen.sample_rate().0 > 48_000
                && let Some(cfg48) = pick_48k()
            {
                chosen = cfg48;
            }
            chosen
        };

        // Compute final frame size (auto ≈100ms) if requested.
        let chosen_sample_rate = chosen.sample_rate().0;
        let final_frame_samples = if config.frame_samples == 0 {
            (chosen_sample_rate / 10).max(1) as usize
        } else {
            config.frame_samples
        };

        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(64);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        let worker = std::thread::spawn(move || {
            let mut cb = on_frame;
            loop {
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }
                match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                    Ok(frame) => cb(&frame),
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        let shared = Arc::new(Mutex::new((
            Vec::<f32>::with_capacity(final_frame_samples * 2),
            final_frame_samples,
        )));
        let shared_clone = Arc::clone(&shared);
        let err_fn = |err| {
            error!(error = %err, "cpal stream error");
        };

        let stream = match chosen.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &chosen.config(),
                move |data: &[f32], _| {
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        buf.extend_from_slice(data);
                        while buf.len() >= frame_samples {
                            let frame: Vec<f32> = buf.drain(0..frame_samples).collect();
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
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        for &s in data.iter() {
                            buf.push(s as f32 / i16::MAX as f32);
                        }
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
                    if let Ok(mut state) = shared_clone.lock() {
                        let frame_samples = state.1;
                        let buf = &mut state.0;
                        for &s in data.iter() {
                            let f = (s as f32 / u16::MAX as f32) * 2.0 - 1.0;
                            buf.push(f);
                        }
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
                return Err(keyless_core::error::KeylessError::Audio(
                    "unsupported sample format".to_string(),
                ));
            }
        };

        match stream {
            Ok(s) => {
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
        let dev_arc = match device {
            Some(d) => d.inner,
            None => Arc::new(host.default_input_device().ok_or_else(|| {
                keyless_core::error::KeylessError::Audio("no input device available".to_string())
            })?),
        };
        self.start_with_device(&dev_arc, config, on_frame)
    }

    fn stop(&mut self) -> KeylessResult<()> {
        if self.stream.is_none() {
            return Err(keyless_core::error::KeylessError::Audio(
                "audio not started".to_string(),
            ));
        }
        self.stream = None;
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
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
