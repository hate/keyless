//! Start the audio pipeline and spawn event bridges to the frontend.
use super::bridge::bridge_events;
use super::events::emit_log;
use keyless_audio::input;
use keyless_core::config::{self, Config, storage};
use keyless_core::error::KeylessResult;
use keyless_runtime::pipeline::{self, PipelineChannels, PipelineHandles};
use std::sync::{Arc, mpsc};

/// Start the pipeline and spawn event bridge (called from runtime thread).
pub fn start_pipeline_internal(
    hold_flag: Arc<std::sync::atomic::AtomicBool>,
) -> KeylessResult<PipelineHandles> {
    // Load configuration (or use defaults if no config exists).
    let cfg: Config = storage::load_config().unwrap_or_default();

    // Create channels for pipeline events (these are consumed by the bridge thread):
    // - level: audio level updates (for VU meter).
    let (tx_level, rx_level) = mpsc::sync_channel(8);
    // - log: log messages from the pipeline.
    let (tx_log, rx_log) = mpsc::sync_channel(64);
    // - spec: EQ spectrum bars (for visualizer).
    let (tx_spec, rx_spec) = mpsc::sync_channel(8);
    // - preview: partial transcription text (real-time preview).
    let (tx_preview, rx_preview) = mpsc::sync_channel(64);
    // - vad: voice activity detection state (speaking/not speaking).

    let (tx_vad, rx_vad) = mpsc::sync_channel(8);

    // Bundle channels into a PipelineChannels struct for passing to the pipeline.
    let channels = PipelineChannels {
        tx_level,
        tx_log,
        tx_spec,
        tx_preview,
        tx_vad,
    };

    // Resolve the selected input device by name if configured.
    // If no device is configured, use the OS default (None).
    let selected_device = if let Some(ref name) = cfg.device_name {
        match input::list_input_devices() {
            Ok(devices) => {
                // Search for a device matching the configured name.
                let mut found: Option<input::InputDevice> = None;
                for dev in devices {
                    if let Ok(dev_name) = dev.name()
                        && &dev_name == name
                    {
                        found = Some(dev);
                        break; // Found matching device, stop searching.
                    }
                }
                // Log whether the device was found or not.
                let target = name.clone();
                let msg = if found.is_some() {
                    format!("input device resolved: {}", target)
                } else {
                    format!("input device '{}' not found; using OS default", target)
                };
                emit_log(msg);
                found
            }
            Err(_) => None, // Device listing failed, use OS default.
        }
    } else {
        None // No device configured, use OS default.
    };

    // Log sink information before starting the pipeline.
    let sink_info = match &cfg.output_mode {
        config::OutputMode::Paste => "sink: Paste".to_string(),
        config::OutputMode::Clipboard => "sink: Clipboard".to_string(),
        config::OutputMode::File(p) => format!("sink: File ({})", p.display()),
    };
    eprintln!("[runtime] {}", sink_info);
    emit_log(sink_info);

    // Start the audio pipeline (this initializes audio input, Whisper model, and output sink).
    // The pipeline reads hold_flag to gate audio recording (only records when PTT is held).
    let handles = pipeline::start_pipeline(
        &cfg,
        cfg.eq.clone(),
        cfg.vad.clone(),
        channels,
        hold_flag,
        selected_device,
    )?;

    // Spawn a bridge thread that consumes pipeline events and forwards them to the frontend.
    // This thread runs for the lifetime of the pipeline and handles all event forwarding.
    std::thread::spawn(move || {
        bridge_events(rx_level, rx_log, rx_spec, rx_preview, rx_vad);
    });

    Ok(handles)
}
