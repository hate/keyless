use std::path::{Path, PathBuf};

use candle_core::Device;
use keyless_core::error::{KeylessError, KeylessResult};
use keyless_whisper::inference::{WHISPER_SAMPLE_RATE, run_inference_full};
use keyless_whisper::model::{WhisperModel, load_whisper_model};

fn read_wav_as_f32_mono_16k(path: &Path) -> KeylessResult<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| KeylessError::Audio(format!("failed to open wav file: {}", e)))?;
    let spec = reader.spec();
    let sr = spec.sample_rate as usize;
    let channels = spec.channels as usize;
    let mut pcm_f32: Vec<f32> = Vec::new();

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for f in reader.samples::<f32>() {
                let x = f.map_err(|e| KeylessError::Audio(format!("read sample: {}", e)))?;
                pcm_f32.push(x.clamp(-1.0, 1.0));
            }
        }
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            for s in reader.samples::<i32>() {
                let x =
                    s.map_err(|e| KeylessError::Audio(format!("read sample: {}", e)))? as f32 / max;
                pcm_f32.push(x.clamp(-1.0, 1.0));
            }
        }
    }

    // Downmix stereo to mono if needed
    if channels == 2 {
        let mut mono = Vec::with_capacity(pcm_f32.len() / 2);
        for i in (0..pcm_f32.len()).step_by(2) {
            let l = pcm_f32[i];
            let r = pcm_f32[i + 1];
            mono.push(((l + r) * 0.5).clamp(-1.0, 1.0));
        }
        pcm_f32 = mono;
    }

    // Resample if needed using rubato
    if sr != WHISPER_SAMPLE_RATE {
        use rubato::Resampler;
        match rubato::FftFixedInOut::<f32>::new(sr, WHISPER_SAMPLE_RATE, 1024, 1) {
            Ok(mut resampler) => {
                let out = resampler
                    .process(&[pcm_f32], None)
                    .map_err(|e| KeylessError::Audio(format!("rubato resample failed: {:?}", e)))?;
                return Ok(out[0].clone());
            }
            Err(e) => {
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
    // Initialize device: Metal → CUDA → CPU
    let device = match Device::new_metal(0) {
        Ok(d) => d,
        Err(_) => match Device::new_cuda(0) {
            Ok(d) => d,
            Err(_) => Device::Cpu,
        },
    };
    let cfg = keyless_whisper::WhisperConfig {
        model_path: model.to_path_buf(),
        language: Some("en".to_string()),
        source_sample_hz: 16_000,
    };
    let components = load_whisper_model(&cfg, &device)?;
    Ok((components, device))
}

fn main() -> KeylessResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut args = std::env::args().skip(1);
    let mut wav: Option<PathBuf> = None;
    let mut model: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--wav" => wav = Some(PathBuf::from(args.next().unwrap_or_default())),
            "--model" => model = Some(PathBuf::from(args.next().unwrap_or_default())),
            _ => {}
        }
    }
    let wav = wav.ok_or_else(|| {
        KeylessError::Config("usage: --wav <path.wav> --model <hf_id_or_dir>".to_string())
    })?;
    let model = model.unwrap_or_else(|| PathBuf::from("openai/whisper-base")); // Use multilingual, not .en

    let pcm = read_wav_as_f32_mono_16k(&wav)?;
    let (mut components, device) = load_model(&model)?;

    let text = run_inference_full(&mut components, &pcm, &device)?;

    println!("TEXT: {}", text);
    let bytes = text.as_bytes();
    let preview: String = bytes
        .iter()
        .take(32)
        .map(|b| format!("{:02x}", b))
        .collect();
    println!("UTF8 HEX (head32): {} (len={})", preview, bytes.len());

    Ok(())
}
