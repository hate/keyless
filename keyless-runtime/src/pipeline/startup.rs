//! Pipeline startup and finalization logic.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};

use keyless_audio::AudioConfig;
use keyless_core::config::{Config, EqTuning, OutputMode, VadThresholds};
use keyless_core::error::KeylessResult;
use keyless_output::OutputSinkProvider;
use keyless_whisper::{PhaseState, WhisperLoadPhase};
use tracing::{error, info};

use crate::workers;

use super::callback;

/// Channels for pipeline communication.
pub struct PipelineChannels {
    /// RMS level → TUI.
    pub tx_level: mpsc::SyncSender<u16>,
    /// Logs → TUI.
    pub tx_log: mpsc::SyncSender<String>,
    /// Spectrum bars → TUI.
    pub tx_spec: mpsc::SyncSender<Vec<u16>>,
    /// Preview text → TUI.
    pub tx_preview: mpsc::SyncSender<String>,
    /// VAD open/closed → TUI.
    pub tx_vad: mpsc::SyncSender<bool>,
}

/// Handle to running pipeline components.
pub struct PipelineHandles {
    /// Handle to the running audio capture stream.
    pub audio: workers::audio::AudioHandle,
    /// Whisper worker/transcriber handle.
    pub whisper: workers::whisper::WhisperHandle,
    /// Human-readable sink label for UI.
    pub sink_label: String,
    /// Human-readable microphone name for UI.
    pub mic_name: String,
    /// Transcriber handle (alias to `whisper`) kept for compatibility.
    pub transcriber: workers::whisper::WhisperHandle,
}

/// Phased startup events for non-blocking initialization.
pub enum StartupEvent {
    /// A phase has begun (e.g., "mapping weights").
    PhaseStarted(&'static str),
    /// A phase completed successfully.
    PhaseOk(&'static str),
    /// A phase failed with an error message.
    PhaseErr(&'static str, String),
    /// All phases done; artifacts are ready for finalization on the main thread.
    Ready(StartupArtifacts),
}

/// Send-safe artifacts produced by the background worker.
pub struct StartupArtifacts {
    /// Initialized Whisper transcriber.
    pub transcriber: workers::whisper::WhisperHandle,
    /// EQ configuration selected for the session.
    pub eq_cfg: keyless_audio::EqConfig,
}

/// Build send-safe startup artifacts (synchronous).
///
/// Why we split startup into phases:
///
/// Problem: Some components (like cpal::Stream) contain raw pointers that aren't Send.
/// We can't move them across thread boundaries. But we want to run heavy initialization
/// (model loading) in a background thread to keep the UI responsive.
///
/// Solution: Split into two phases:
/// 1. This function (background thread): Initialize Send-safe components (Whisper, EQ config)
/// 2. finalize_pipeline_from_artifacts (main thread): Create non-Send components (audio stream, sink)
///
/// Result: The TUI shows a loading overlay with granular progress, and the user can cancel with ESC.
pub fn build_startup_artifacts(
    config: &Config,
    eq_tuning: &EqTuning,
    mut progress: Option<&mut dyn FnMut(StartupEvent)>,
) -> keyless_core::error::KeylessResult<StartupArtifacts> {
    // Device preload (Metal/CUDA/CPU) happens inside keyless-whisper::Whisper::new.

    // Initialize Whisper transcriber (the heavy part: model loading, tokenizer, mel filters).
    // This is Send-safe because WhisperHandle wraps Arc<Mutex<Whisper>>, which can cross threads.
    let whisper_cfg = keyless_whisper::WhisperConfig {
        model_path: config.model_path.clone(),
        language: config.language.clone(),
        // 48 kHz matches audio input sample rate (required for correct transcription).
        source_sample_hz: 48_000,
    };

    // Bridge progress events from Whisper loading to our startup events (for UI overlay).
    // Map WhisperLoadPhase → StartupEvent so TUI can show granular progress.
    let bridge = progress.as_mut().map(|cb| {
        move |ph: WhisperLoadPhase, st: PhaseState| {
            let label = ph.as_label();
            match st {
                PhaseState::Begin => cb(StartupEvent::PhaseStarted(label)),
                PhaseState::End => cb(StartupEvent::PhaseOk(label)),
            }
        }
    });

    let transcriber = workers::whisper::start_whisper_with_progress(whisper_cfg, bridge)?;

    // EQ config: matches audio input sample rate (48 kHz) for correct frequency analysis.
    let eq_cfg = keyless_audio::EqConfig {
        sample_rate_hz: 48_000.0,
        nfft: 1024,
        bands: eq_tuning.bands as usize,
        start_hz: 120.0,
        noise_reduction: eq_tuning.noise_reduction,
        window_db: eq_tuning.window_db,
        gamma: eq_tuning.gamma,
        attack: eq_tuning.attack,
        decay: eq_tuning.decay,
    };

    Ok(StartupArtifacts {
        transcriber,
        eq_cfg,
    })
}

/// Spawn background worker to build send-safe startup artifacts.
pub fn spawn_startup_worker(
    config: Config,
    eq_tuning: EqTuning,
    tx_log: mpsc::SyncSender<String>,
) -> mpsc::Receiver<StartupEvent> {
    // Sync channel with buffer (32 events) to handle progress bursts without blocking.
    let (tx_evt, rx_evt) = mpsc::sync_channel::<StartupEvent>(32);
    std::thread::spawn(move || {
        // Emit device phase (device preload is quick, just a signal to UI).
        let _ = tx_evt.send(StartupEvent::PhaseStarted("device"));
        let _ = tx_log.try_send("device preloaded".to_string());
        let _ = tx_evt.send(StartupEvent::PhaseOk("device"));

        // Emit whisper phase start (this is the heavy part).
        let _ = tx_evt.send(StartupEvent::PhaseStarted("whisper"));
        // Forward progress events from build_startup_artifacts to UI channel.
        let mut forward = |evt: StartupEvent| {
            let _ = tx_evt.send(evt);
        };
        match build_startup_artifacts(&config, &eq_tuning, Some(&mut forward)) {
            Ok(art) => {
                let _ = tx_log.try_send("whisper initialized".to_string());
                let _ = tx_evt.send(StartupEvent::PhaseOk("whisper"));
                // Send artifacts to main thread for finalization (main thread creates non-Send components).
                let _ = tx_evt.send(StartupEvent::Ready(art));
            }
            Err(e) => {
                // Emit error event (UI will show error overlay and allow retry).
                let _ = tx_evt.send(StartupEvent::PhaseErr("whisper", e.to_string()));
            }
        }
    });
    rx_evt
}

/// Start the audio/whisper pipeline with the given config.
pub fn start_pipeline(
    config: &Config,
    eq_tuning: EqTuning,
    vad_config: VadThresholds,
    channels: PipelineChannels,
    hold_flag: Arc<AtomicBool>,
    selected_device: Option<keyless_audio::input::InputDevice>,
) -> KeylessResult<PipelineHandles> {
    // Validate config (save only elsewhere to avoid blocking)
    config.validate()?;

    // Build artifacts and finalize synchronously
    let artifacts = build_startup_artifacts(config, &eq_tuning, None)?;
    finalize_pipeline_from_artifacts(
        artifacts,
        config,
        vad_config,
        channels,
        hold_flag,
        selected_device,
    )
}

/// Finalize pipeline from prebuilt artifacts (main thread only).
pub fn finalize_pipeline_from_artifacts(
    artifacts: StartupArtifacts,
    config: &Config,
    vad_config: VadThresholds,
    channels: PipelineChannels,
    hold_flag: Arc<AtomicBool>,
    selected_device: Option<keyless_audio::input::InputDevice>,
) -> KeylessResult<PipelineHandles> {
    let PipelineChannels {
        tx_level,
        tx_log,
        tx_spec,
        tx_preview,
        tx_vad,
    } = channels;

    // Create output sink (paste/clipboard/file) based on config.
    let sink_label = match &config.output_mode {
        OutputMode::Paste => "Paste".into(),
        OutputMode::Clipboard => "Clipboard".into(),
        OutputMode::File(p) => format!("File ({})", p.display()),
    };
    let provider = keyless_output::DefaultOutputProvider;
    // Provider creates sink based on output_mode (runtime polymorphism).
    let sink_box: Box<dyn keyless_core::output::OutputSink> = provider.provide(config)?;

    // Extract mic name: prefer config, fallback to system default, fallback to placeholder.
    let mic_name = config
        .device_name
        .clone()
        .or_else(|| keyless_audio::input::default_input_name().ok())
        .unwrap_or_else(|| "No microphone".into());

    // Build callback using provided transcriber and eq cfg
    let transcriber = artifacts.transcriber.clone();
    let sfx = Arc::new(keyless_audio::SfxPlayer::new());
    let sink: Arc<dyn keyless_core::output::OutputSink> = sink_box.into();
    let tx = callback::CallbackTx {
        tx_level: tx_level.clone(),
        tx_log: tx_log.clone(),
        tx_spec,
        tx_preview,
        tx_vad,
    };
    let params = callback::AudioCallbackParams {
        eq_cfg: artifacts.eq_cfg,
        vad: vad_config,
        transcriber: transcriber.clone(),
        sink: Arc::clone(&sink),
        sfx: Arc::clone(&sfx),
        hold_flag,
        tx,
    };
    let callback = callback::create_audio_callback(params);

    // Start audio input stream (non-Send component; must be created on main thread).
    let dev_name_for_log = selected_device
        .as_ref()
        .and_then(|d| d.name().ok())
        .unwrap_or_else(|| "(default)".to_string());
    let audio_handle = match crate::workers::audio::start_audio(
        selected_device,
        AudioConfig {
            // Auto-select sample rate (backend prefers 48 kHz if available).
            sample_hz: 0,
            // Auto-calculate frame size (~100ms at chosen sample rate for low latency).
            frame_samples: 0,
        },
        callback,
    ) {
        Ok(h) => h,
        Err(e) => {
            // Audio start failed (permission issue or device unavailable).
            error!(error = %e, device = %dev_name_for_log, "audio capture start failed");
            let _ = tx_log.try_send(format!("audio start error: {}", e));
            return Err(e);
        }
    };
    info!(device = %dev_name_for_log, "audio capture started");
    let _ = tx_log.try_send(format!("audio init ok (device={})", dev_name_for_log));

    Ok(PipelineHandles {
        audio: audio_handle,
        whisper: transcriber.clone(),
        sink_label,
        mic_name,
        transcriber,
    })
}
