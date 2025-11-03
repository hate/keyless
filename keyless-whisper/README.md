## keyless-whisper

_Real‑time Whisper transcription with Candle._

### Overview

Provides a real‑time Whisper engine using Hugging Face Candle. Handles model download/cache, mel preprocessing, encoder/decoder inference, and emits Partial/Final events from a worker thread. Uses segment-aware windowing (≤30s per Whisper context) with live preview and final transcription on segment end.

### Features

- Candle‑based Whisper (CPU, Metal, CUDA) device selection
- **Quantized model support** - 3-4x faster inference with GGUF models
- **Quality improvements** - No-speech detection, temperature fallback, quality metrics
- HF Hub auto‑download and local cache
- High‑quality arbitrary-rate resampling via rubato (16k, 32k, 44.1k, 48k, 96k all supported)
- Streaming interface: push audio frames, poll events
- Segment-aware: live preview while speaking; single Final on segment end
- Windowed inference: ≤30s windows for long utterances; automatic concatenation
- **Language auto-detection** - Automatic language identification when not specified
- **Performance optimized** - Zero-copy model access, single encoder pass, rubato resampling

### Supported Models

**✅ Normal models (safetensors):**
- `openai/whisper-tiny` - fastest, 75 MB
- `openai/whisper-base` - good balance, 142 MB
- `openai/whisper-small` - higher accuracy, 466 MB
- ...

**⚡ Quantized models (GGUF) - 3-4x faster:**
- Any model with `.gguf` files in the directory
- Automatically detected and loaded
- ~1% accuracy loss, significant speed gain
- Ideal for lower-end hardware


### Installation

```
keyless-whisper = { path = "../keyless-whisper" }
```

Build backends:

- macOS: Metal is auto‑enabled via target‑specific dependencies; nothing extra to do.
- Windows/Linux with NVIDIA: enable CUDA explicitly:

```
cargo run -p keyless --features "keyless-whisper/cuda"
```

### Usage

```
use keyless_whisper::{Whisper, WhisperConfig, RealtimeTranscriber, WhisperLoadPhase, PhaseState};
use std::path::PathBuf;

let cfg = WhisperConfig {
    model_path: PathBuf::from("openai/whisper-base"),  // or whisper-base.en
    language: Some("en".to_string()),
    source_sample_hz: 48_000,
};
// Basic constructor
let mut t = Whisper::new(cfg.clone())?;
// t.push_audio(&frame)?; // push mono f32 frames
// t.end_segment()?; // on PTT release
// if let Some(evt) = t.try_next_event() { /* handle */ }
```

#### Progress callback during model load (optional)

```rust
let mut t = Whisper::new_with_progress(cfg, Some(|phase: WhisperLoadPhase, state: PhaseState| {
    // Map to your UI/logging; called for Begin/End of each phase
    eprintln!("{:?}: {:?}", phase, state);
}))?;
```

### Configuration

- `WhisperConfig` model path can be an HF ID (auto‑download) or a local directory with `config.json`, `model.safetensors`, `tokenizer.json`.
- **Language:** For multilingual models, provide `language: Some("en")` for English transcription. Auto-detection is slower and less accurate.
- **Model selection:** Both multilingual and `.en` models work. `.en` models are slightly smaller but English-only.

### How It Works

**Audio Flow:**
1. Audio captured at device sample rate → rubato resampler → 16 kHz mono
2. VAD gates speech; only voice frames accumulate in the utterance buffer
3. While speaking (PTT held): emit Partial every ~0.2s from last ≤2s of audio (preview only)
4. On PTT release: emit single Final from entire utterance (walk ≤30s windows, concatenate)

**Decoder:**
- Based on Candle WASM Whisper example pattern
- Feed full token sequence every step (not last-token-only)
- Flush KV cache only on first iteration per window
- Use `decoder.final_linear()` for vocab projection
- Greedy sampling with repetition guards (max steps, identical tokens, tri-grams)

### Learnings from Candle Whisper WASM Example Analysis

We analyzed the [candle-wasm-examples/whisper](https://github.com/huggingface/candle/tree/main/candle-wasm-examples/whisper) reference implementation and extracted several critical improvements:

Auto-detects `.gguf` files and loads appropriate format. No user configuration needed.

#### **1. No-Speech Detection (prevents hallucinations)**

**What we learned:**
- On silent/noise-only segments, Whisper can hallucinate text
- First decoder iteration provides reliable no-speech probability
- Threshold of 0.6 effectively filters silent segments

**Implementation:**
- Extract `no_speech_prob` from first decoder forward pass
- Skip segment if `no_speech_prob > 0.6`
- Prevents "Thank you for watching" hallucinations on silence

#### **2. Temperature Fallback (quality recovery)**

**What we learned:**
- Greedy decoding (temp=0.0) sometimes fails on difficult audio
- Quality metrics indicate when to retry: `compression_ratio`, `avg_logprob`
- Temperature schedule [0.0, 0.2, 0.4, 0.6, 0.8, 1.0] from OpenAI reference

**Implementation:**
- Try greedy first (fastest)
- If quality metrics indicate failure, retry with higher temperature
- Early stop when quality is acceptable (95%+ succeed on first try)

#### **3. Language Auto-Detection**

**What we learned:**
- Whisper can detect language from audio features
- Run one decoder step with just SOT token, examine language token probabilities
- More accurate than specifying wrong language, but slower than correct hint

**Implementation:**
- Optional: auto-detect when language not specified
- Reuses encoder output (no duplicate encoder pass)
- Logs detected language and confidence

### Implementation Notes & Lessons Learned

**Critical fixes during development:**

1. **Vocab Projection Must Use `decoder.final_linear()`**
   - **Problem:** Initially tried manual vocab projection via token embedding
   - **Issue:** This produced gibberish because we were argmaxing over the wrong value
   - **Solution:** Use Candle's built-in `decoder.final_linear()` method which handles the projection correctly

2. **Decoder Loop Pattern: Full Sequence Every Step**
   - **Problem:** Attempted to feed only the last token on subsequent steps (trying to optimize KV cache)
   - **Issue:** Candle's Whisper decoder expects the full token sequence on every forward pass
   - **Solution:** Feed `tokens[..]` every step; KV cache is managed internally, only flush on first iteration (`i == 0`)

3. **Repetition Guards Essential**
   - **Problem:** Short/noisy audio segments cause decoder loops (e.g., "And And And...")
   - **Solution:** Implement guards for identical token repetition (≥8 of last 10) and tri-gram repetition (≥3 occurrences)

**Reference Implementation:**
This implementation follows the pattern from [`candle-wasm-examples/whisper`](https://github.com/huggingface/candle/tree/main/candle-wasm-examples/whisper), which is the official working Candle Whisper example. Note:
- Full token sequence feeding pattern
- Proper use of `decoder.final_linear()` instead of manual projection
- KV cache semantics (flush once per window, not per step)

### Logging

Emits `tracing` logs for device selection, model loading, and inference timing/errors. Libraries do not initialize logging.

### API Notes

- `Whisper::new()` delegates to `new_with_progress(None)`; no duplication.
- `WhisperLoadPhase::as_label()` provides user-friendly labels for UI overlays.

### Testing

```
cargo test -p keyless-whisper
```

Unit tests cover preprocessing and token filtering. Full model inference is exercised in integration flows.

### Technologies

- `candle-core`, `candle-nn`, `candle-transformers` for Whisper
- `tokenizers` for text decoding
- `keyless-models` for cache path resolution (models are downloaded by the TUI)
- `rubato` for high-quality resampling
- `tracing` for logging

### Module structure

- `src/lib.rs`: threading, public API, event channels, rubato resampling
- `src/model.rs`: model/tokenizer download and load
- `src/preprocessing.rs`: mel filter generation and PCM→mel
- `src/decode.rs`: token generation/decoding
- `src/inference.rs`: end‑to‑end inference pipeline

### Platform support (macOS, Windows, Linux)

- Device selection at runtime: Metal (macOS) → CUDA (if enabled) → CPU fallback. The CLI logs the chosen backend during startup.
- Offline-first: models are downloaded and cached by the TUI (via `keyless-models`); this crate loads from the local cache.

### Token ID Differences: .en vs Multilingual (Important!)

**Critical Discovery:** `.en` and multilingual models have **different token IDs** for special tokens:

| Token | Multilingual | `.en` Model |
|-------|--------------|-------------|
| `<|endoftext|>` | 50257 | 50256 |
| `<|startoftranscript|>` | 50258 | 50257 |
| `<|en|>` | 50259 | 50258 |
| `<|transcribe|>` | 50359 | 50358 |
| `<|notimestamps|>` | 50363 | 50362 |

```rust
// From candle-transformers
let sot = tokenizer.token_to_id(m::SOT_TOKEN)?;  // Gets correct ID for any model
let eot = tokenizer.token_to_id(m::EOT_TOKEN)?;
```

This pattern automatically works for both model types.

### License

MIT


