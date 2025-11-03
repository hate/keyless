//! Audio input subsystem: public API and CPAL implementation.

pub mod api;
pub mod cpal;

pub use api::{
    AudioInput, FrameCallback, InputDevice, default_input_device, default_input_name,
    list_input_device_names, list_input_devices,
};
pub use cpal::CpalAudioInput;
