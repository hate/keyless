use std::path::{Path, PathBuf};

use candle_core::Device;
use keyless_core::error::{KeylessError, KeylessResult};
use keyless_whisper::inference::{WHISPER_SAMPLE_RATE, run_inference_voiced};
use keyless_whisper::model::{WhisperModel, load_whisper_model};

fn read_wav_as_f32_mono_16k(path: &Path) -> KeylessResult<Vec<f32>> {
    // Open WAV file and read metadata (sample rate, channels, format).
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| KeylessError::Audio(format!("failed to open wav file: {}", e)))?;
    let spec = reader.spec();
    let sr = spec.sample_rate as usize;
    let channels = spec.channels as usize;
    let mut pcm_f32: Vec<f32> = Vec::new();

    // Handle different sample formats: Float (normalized) vs Int (needs normalization).
    match spec.sample_format {
        hound::SampleFormat::Float => {
            // Float samples are already normalized [-1.0, 1.0]; clamp to ensure range.
            for f in reader.samples::<f32>() {
                let x = f.map_err(|e| KeylessError::Audio(format!("read sample: {}", e)))?;
                // Clamp to [-1.0, 1.0] to handle out-of-range values (defensive).
                pcm_f32.push(x.clamp(-1.0, 1.0));
            }
        }
        hound::SampleFormat::Int => {
            // Calculate max value for normalization (e.g., 16-bit: 2^15 = 32768).
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            for s in reader.samples::<i32>() {
                // Normalize integer sample to [-1.0, 1.0] range (divide by max).
                let x =
                    s.map_err(|e| KeylessError::Audio(format!("read sample: {}", e)))? as f32 / max;
                // Clamp to [-1.0, 1.0] to handle edge cases (defensive).
                pcm_f32.push(x.clamp(-1.0, 1.0));
            }
        }
    }

    // Downmix stereo to mono if needed (Whisper expects mono input).
    if channels == 2 {
        // Pre-allocate mono vector (half the size of stereo).
        let mut mono = Vec::with_capacity(pcm_f32.len() / 2);
        // Average left and right channels (step_by(2) processes interleaved samples).
        for i in (0..pcm_f32.len()).step_by(2) {
            let l = pcm_f32[i];
            let r = pcm_f32[i + 1];
            // Average and clamp (sum may exceed [-1.0, 1.0] if both channels are max).
            mono.push(((l + r) * 0.5).clamp(-1.0, 1.0));
        }
        pcm_f32 = mono;
    }

    // Resample if needed using rubato (Whisper requires 16 kHz).
    if sr != WHISPER_SAMPLE_RATE {
        use rubato::Resampler;
        // FFT-based resampler: 1024 sample chunk size, 1 channel (mono).
        match rubato::FftFixedInOut::<f32>::new(sr, WHISPER_SAMPLE_RATE, 1024, 1) {
            Ok(mut resampler) => {
                // Process entire buffer (float array input, mono channel).
                let out = resampler
                    .process(&[pcm_f32], None)
                    .map_err(|e| KeylessError::Audio(format!("rubato resample failed: {:?}", e)))?;
                // Return first (and only) channel (out is Vec<Vec<f32>>).
                return Ok(out[0].clone());
            }
            Err(e) => {
                // Resampler init failed (unsupported sample rate ratio or invalid params).
                return Err(KeylessError::Audio(format!(
                    "rubato init failed for {} Hz → 16000 Hz: {:?}",
                    sr, e
                )));
            }
        }
    }

    Ok(pcm_f32)
}

fn load_model(model: &Path) -> KeylessResult<(WhisperModel, Device)> {
    // Initialize device: Metal → CUDA → CPU (fallback chain for best performance).
    // Prefer GPU acceleration (Metal on macOS, CUDA on Linux/Windows), fallback to CPU.
    let device = match Device::new_metal(0) {
        Ok(d) => d,
        Err(_) => match Device::new_cuda(0) {
            Ok(d) => d,
            // CPU fallback (always available, but slower).
            Err(_) => Device::Cpu,
        },
    };
    let cfg = keyless_whisper::WhisperConfig {
        model_path: model.to_path_buf(),
        language: Some("en".to_string()),
        // 16 kHz matches WHISPER_SAMPLE_RATE (required for correct transcription).
        source_sample_hz: 16_000,
    };
    // Load model components (weights, tokenizer, mel filters) for specified device.
    let components = load_whisper_model(&cfg, &device)?;
    Ok((components, device))
}

fn main() -> KeylessResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Parse command-line arguments (simple iterator-based parser).
    let mut args = std::env::args().skip(1);
    let mut wav: Option<PathBuf> = None;
    let mut model: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            // Parse --wav flag and next argument (path to WAV file).
            "--wav" => wav = Some(PathBuf::from(args.next().unwrap_or_default())),
            // Parse --model flag and next argument (HF model ID or local path).
            "--model" => model = Some(PathBuf::from(args.next().unwrap_or_default())),
            _ => {}
        }
    }
    // WAV file is required (error if missing).
    let wav = wav.ok_or_else(|| {
        KeylessError::Config("usage: --wav <path.wav> --model <hf_id_or_dir>".to_string())
    })?;
    // Model defaults to multilingual base model (not .en; supports multiple languages).
    let model = model.unwrap_or_else(|| PathBuf::from("openai/whisper-base"));

    let pcm = read_wav_as_f32_mono_16k(&wav)?;
    let (mut components, device) = load_model(&model)?;

    // Run full inference on entire audio (blocking; processes all audio at once).
    let text = run_inference_voiced(&mut components, &pcm, &device)?;

    println!("TEXT: {}", text);
    // Convert text to UTF-8 bytes for hex preview (debugging output).
    let bytes = text.as_bytes();
    // Show first 32 bytes as hex (for debugging encoding issues).
    let preview: String = bytes
        .iter()
        .take(32)
        .map(|b| format!("{:02x}", b))
        .collect();
    println!("UTF8 HEX (head32): {} (len={})", preview, bytes.len());

    Ok(())
}
