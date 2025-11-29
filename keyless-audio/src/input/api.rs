//! Audio input API: traits and device helpers.
//!
//! Provides the `AudioInput` trait, `InputDevice` opaque handle, and convenience
//! functions for enumerating/selecting input devices without leaking CPAL types.

use std::sync::Arc;

use cpal::traits::StreamTrait;
use cpal::traits::{DeviceTrait, HostTrait};
use keyless_core::error::KeylessResult;

use crate::config::AudioConfig;

/// Type alias for audio frame callback.
pub type FrameCallback = Box<dyn FnMut(&[f32]) + Send + 'static>;

/// Audio input abstraction.
///
/// Implementations include the CPAL backend (`CpalAudioInput`).
pub trait AudioInput {
    /// Start capturing audio and invoke the callback for each frame.
    fn start(
        &mut self,
        device: Option<InputDevice>,
        config: AudioConfig,
        on_frame: FrameCallback,
    ) -> KeylessResult<()>;

    /// Stop audio capture and clean up resources.
    fn stop(&mut self) -> KeylessResult<()>;
}

#[derive(Clone)]
/// Opaque input device handle (avoids leaking CPAL types across crate boundaries).
pub struct InputDevice {
    /// Internal CPAL device handle.
    pub(crate) inner: Arc<cpal::Device>,
}

impl InputDevice {
    /// Get the human-readable display name of this input device.
    pub fn name(&self) -> keyless_core::error::KeylessResult<String> {
        self.inner.name().map_err(|e| {
            keyless_core::error::KeylessError::Audio(format!(
                "failed to query input device name: {}",
                e
            ))
        })
    }
}

/// Return the display name of the default input (microphone) device.
pub fn default_input_name() -> keyless_core::error::KeylessResult<String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| {
        keyless_core::error::KeylessError::Audio("no input device available".to_string())
    })?;
    match device.name() {
        Ok(n) => Ok(n),
        Err(e) => Err(keyless_core::error::KeylessError::Audio(format!(
            "failed to query input device name: {}",
            e
        ))),
    }
}

/// List the display names of all available input devices.
pub fn list_input_device_names() -> keyless_core::error::KeylessResult<Vec<String>> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    // Iterate all input devices; skip devices where name() fails (may be unplugged/permission issue).
    for dev in host.input_devices().map_err(|e| {
        keyless_core::error::KeylessError::Audio(format!(
            "failed to enumerate input devices: {}",
            e
        ))
    })? {
        // Silently skip devices that can't provide a name (defensive; better than failing entire list).
        if let Ok(name) = dev.name() {
            out.push(name);
        }
    }
    Ok(out)
}

/// Enumerate available input devices and return opaque handles.
pub fn list_input_devices() -> keyless_core::error::KeylessResult<Vec<InputDevice>> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    // Wrap each device in Arc for shared ownership (allows cloning InputDevice handles).
    for dev in host.input_devices().map_err(|e| {
        keyless_core::error::KeylessError::Audio(format!(
            "failed to enumerate input devices: {}",
            e
        ))
    })? {
        out.push(InputDevice {
            inner: Arc::new(dev),
        });
    }
    Ok(out)
}

/// Probe whether we can actually open and play a microphone stream (permission + availability).
/// Returns `true` if a CPAL input stream can be created and started.
pub fn can_record() -> bool {
    eprintln!("[can_record] Probing microphone access...");
    let host = cpal::default_host();
    let Some(device) = host.default_input_device() else {
        eprintln!("[can_record] No default input device found");
        return false;
    };

    if let Ok(name) = device.name() {
        eprintln!("[can_record] Found device: {}", name);
    }

    let Ok(cfg) = device.default_input_config() else {
        eprintln!("[can_record] Could not get device config");
        return false;
    };

    fn try_build<T>(device: &cpal::Device, config: cpal::StreamConfig) -> bool
    where
        T: cpal::Sample + cpal::SizedSample,
    {
        use std::sync::{Arc, Mutex};

        // Track if we actually receive audio data (proves permission is granted)
        let data_received = Arc::new(Mutex::new(false));
        let data_received_clone = Arc::clone(&data_received);

        let builder = device.build_input_stream(
            &config,
            move |_data: &[T], _| {
                // If this callback fires, we have real microphone access.
                // Avoid panicking on poisoned mutex; best-effort set to true.
                if let Ok(mut guard) = data_received_clone.lock() {
                    *guard = true;
                }
            },
            |e| {
                eprintln!("[can_record] Stream error: {}", e);
            },
            None,
        );

        match builder {
            Ok(stream) => {
                if let Err(e) = stream.play() {
                    eprintln!("[can_record] Stream play failed: {}", e);
                    return false;
                }

                // Wait briefly to see if we receive any audio data
                std::thread::sleep(std::time::Duration::from_millis(100));

                let received = match data_received.lock() {
                    Ok(guard) => *guard,
                    Err(_) => {
                        // Treat as not received if lock is poisoned; conservative false.
                        false
                    }
                };
                eprintln!("[can_record] Stream played, data received: {}", received);

                // Return true only if we actually received audio data
                received
            }
            Err(e) => {
                eprintln!("[can_record] Stream build failed: {}", e);
                false
            }
        }
    }

    let sc: cpal::StreamConfig = cfg.config().clone();
    let result = match cfg.sample_format() {
        cpal::SampleFormat::I8 => try_build::<i8>(&device, sc),
        cpal::SampleFormat::U8 => try_build::<u8>(&device, sc),
        cpal::SampleFormat::I16 => try_build::<i16>(&device, sc),
        cpal::SampleFormat::U16 => try_build::<u16>(&device, sc),
        cpal::SampleFormat::F32 => try_build::<f32>(&device, sc),
        _ => {
            eprintln!("[can_record] Unsupported sample format");
            false
        }
    };

    eprintln!("[can_record] Final result: {}", result);
    result
}

/// Get the system default input device as an opaque handle.
pub fn default_input_device() -> keyless_core::error::KeylessResult<InputDevice> {
    let host = cpal::default_host();
    // OS chooses default device (usually the primary microphone or first available).
    let dev = host.default_input_device().ok_or_else(|| {
        keyless_core::error::KeylessError::Audio("no input device available".to_string())
    })?;
    // Wrap in Arc for shared ownership (allows cloning the handle).
    Ok(InputDevice {
        inner: Arc::new(dev),
    })
}
