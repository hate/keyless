## keyless-audio

_Microphone capture and framing via cpal._

### Overview

Captures microphone input using `cpal`, normalizes to `f32`, and delivers fixed‑size frames to a user callback on a worker thread.

### Features

- Unified start API with optional device handle
- Default input device selection
- Format conversion (F32/I16/U16 → f32)
- Frame accumulation and bounded channel backpressure
- Clean start/stop lifecycle
- Voice Activity Detection (VAD) gate with hysteresis and timing
- Device helpers for UI: `InputDevice`, `list_input_devices()`, `default_input_device()`

### Installation

```
keyless-audio = { path = "../keyless-audio" }
```

### Usage

```
use keyless_audio::{AudioConfig, input::AudioInput, input::CpalAudioInput, VadGate, EqConfig, EqState, compute_bars};
use keyless_audio::input::{InputDevice, list_input_devices, default_input_device};

let mut audio = CpalAudioInput::new();

// Start on the system default device (auto sample rate and ~100ms frames)
audio.start(
    None,
    AudioConfig { sample_hz: 0, frame_samples: 0 },
    Box::new(|frame: &[f32]| {
        // handle ~100ms frames at the chosen device rate
    }),
)?;

// later
audio.stop()?;

// Or choose a device explicitly (opaque handle, no cpal types leak)
let devices: Vec<InputDevice> = list_input_devices()?;
let selected = devices.into_iter().next();
let mut audio = CpalAudioInput::new();
audio.start(
    selected,
    AudioConfig { sample_hz: 0, frame_samples: 0 },
    Box::new(|frame: &[f32]| {/* ... */}),
)?;
```

Device helpers for UI layers (no `cpal` exposure at call sites):

```
use keyless_audio::input::{default_input_device, list_input_devices};
let def = default_input_device()?;    // -> InputDevice
let all = list_input_devices()?;      // -> Vec<InputDevice>
let name = def.name()?;               // get display name when needed
```

### Configuration

- `AudioConfig { sample_hz, frame_samples }`:
  - `sample_hz = 0` → auto: use device default; prefers 48 kHz and caps excessively high defaults (≤48 kHz)
  - `frame_samples = 0` → auto: derives ~100 ms frames from the chosen sample rate

### Logging

Emits `tracing` logs for stream errors, start, and backpressure drops.

### Testing

```
cargo test -p keyless-audio
```

Tests avoid real hardware (lifecycle and config checks only).

### Technologies

- `cpal` for cross‑platform audio input/output
- `tracing` for logging

### Module structure

- `src/lib.rs`: re-exports; crate docs
- `src/config.rs`: `AudioConfig`
- `src/input/`: audio input subsystem
  - `api.rs`: `AudioInput` trait, `InputDevice` and helpers
  - `input.rs`: `CpalAudioInput` implementation
- `src/vad.rs`: `VadGate` (voice activity detection gate)
- `src/eq_pipeline.rs`: EQ spectrum pipeline (FFT, log-spaced bands, auto-sensitivity via max normalization, cached buffers for zero per-frame allocations)
- `src/sfx/`: procedural sound effects
  - `player.rs`: `SfxPlayer` public API (press/release/final)
  - `state.rs`: realtime callback state (oscillator/envelope)

Note: Resampling is handled in `keyless-whisper` via rubato, not in this crate.

### SFX quick start

```
use keyless_audio::SfxPlayer;

let sfx = SfxPlayer::new();
sfx.play_press();   // on PTT press
sfx.play_release(); // on PTT release
sfx.play_final();   // after successful Final delivery
```

### Platform support (macOS, Windows, Linux)

- Uses the system default input device via `cpal` (CoreAudio, WASAPI, ALSA/Pulse/Jack).
- Mic access prompts are handled by the OS.

### License

MIT


