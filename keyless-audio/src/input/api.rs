//! Audio input API: traits and device helpers.
//!
//! Provides the `AudioInput` trait, `InputDevice` opaque handle, and convenience
//! functions for enumerating/selecting input devices without leaking CPAL types.

use std::sync::Arc;

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
