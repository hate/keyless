## keyless (CLI)

_Local real‑time dictation CLI with a full‑screen TUI._

### Overview

Full‑screen terminal UI that wires microphone audio → Whisper transcription → output sinks (Paste, Clipboard, File). On launch you get a Config screen (choose sink, model, language, and device). After starting, the Dictating screen shows the active mic device, an input visualizer, and live logs. Config persist across runs. VAD gating is included to reduce silence processing.

#### How it works (speech → text)
- While you hold the PTT hotkey, we capture 48 kHz mic audio and gate it with VAD so only speech enters the transcription buffer.
- Audio is resampled to 16 kHz via rubato (supports all device sample rates) and accumulated as one "utterance" during the PTT hold.
- **Preview**: Every ~120–200 ms, we run the same voiced‑mask pipeline on the current buffer (≤10s units with 0.5s overlap), reusing cached unit texts via a ~128 ms tail hash. Preview text matches what Final would be now.
- **Final**: When you release PTT, we split the utterance into voiced spans, process each as ≤10s units with overlap, stitch units together with dedupe, and deliver the Final to your selected sink. This prevents empty Finals on long dictations and improves accuracy.

#### Model Recommendations
- **Both multilingual and `.en` models are supported**
- Multilingual models (`openai/whisper-base`, `openai/whisper-small`): Support auto-detection, can switch languages
- English-only `.en` models (`openai/whisper-base.en`): Slightly smaller, English-only, no auto-detection
- Select your language in Config for best accuracy (auto-detection is slower)
- Recommended: `openai/whisper-base` for balanced speed/accuracy/flexibility

### Features

- Real‑time audio capture (cpal) and framing
- Whisper inference via Candle (auto‑download model, cached offline)
- Resumable model downloads with progress overlay (model name, progress bar, MB/s, ETA)
  - `[esc] pause` (keeps partial for resume) or `[backspace] cancel` (deletes partial)
- Phased startup with a "loading" overlay for Whisper initialization
  - Shows granular steps (e.g., "detecting model format", "mapping weights")
  - Live logs streamed into the overlay
  - `[esc] cancel` to abort startup
- Output sinks: Paste into active app, copy to Clipboard, append to File
- Config persistence with validation and atomic save
- Error display on invalid config or missing devices (stays on Config)
- VAD gating (start/stop dB, min duration, silence hangover) via `keyless-audio::VadGate`
- EQ spectrum visualization (FFT-based, log-spaced bands, auto-sensitivity via max normalization) via `keyless-audio::compute_bars()`
- Preview text display: shows full preview text at top of visualizer (truncated to screen width if needed) for immediate feedback while speaking
- Permissions UX: shows mic/accessibility guidance; proactively warns if macOS Accessibility is not granted for Paste sink

### Technologies

- Rust workspace (2024 edition), `tracing` for logs
- `ratatui`, `crossterm` for the TUI
- `cpal` for audio, `candle` (+ tokenizers) for Whisper, `rubato` for high-quality resampling (supports all device sample rates)
- `reqwest` for HTTP (model fetching via `keyless-models`)
- `enigo` (Paste), `arboard` (Clipboard)

### Module structure

- `src/main.rs`: process entry, initializes logging and TUI
- `src/overrides.rs`: CLI/ENV overrides parsing and application (`Overrides`, `apply_overrides()`)
- `src/tui.rs`: Pure re-export module; `run_with_overrides()` re-export
- `src/tui/` (modularized):
  - `runtime/`: event loop
    - `controller.rs`: main event loop (orchestrates drains, init, input, periodic, rendering)
    - `context.rs`: runtime context bundling channels, flags, handles, terminal
    - `drains.rs`: channel drain logic (logs, sizes, download events from keyless-models)
    - `handlers/`: input handling logic (config, dictating, expert mode)
    - `init.rs`: Config screen one-time setup (catalog, discovery, workers)
    - `input.rs`: input routing (Config, Expert Mode, Dictating)
    - `periodic.rs`: 250ms tick tasks (size refresh, PTT heartbeat)
    - `ptt.rs`: global hotkey listener (presets, enable/disable, heartbeat)
    - `workers/`: audio and whisper worker lifecycle (uses keyless-runtime pipeline)
  - `state.rs`: Pure re-export module for TUI application state
    - `state/app_state.rs`: `AppState`, `Screen`, `SinkChoice`
    - `state/model_catalog.rs`: `ModelCatalog`
    - `state/overlays.rs`: `Overlays`, `DownloadState`, `ExpertOverlay`
    - `state/runtime_feedback.rs`: `RuntimeFeedback`
    - `state/selections.rs`: `Selections`
  - `config.rs`: Pure re-export module for config screen rendering
    - `config/draw.rs`: main draw function
    - `config/device.rs`: input device list rendering
    - `config/expert.rs`: expert mode overlay
    - `config/footer.rs`: hotkey hints rendering
    - `config/languages.rs`: language selector rendering
    - `config/models.rs`: model table rendering
    - `config/sinks.rs`: output sink picker rendering
    - `config/vad.rs`: VAD threshold/timing panel rendering
  - `dictating.rs`: Pure re-export module for dictating screen rendering
    - `dictating/draw.rs`: main draw function
    - `dictating/eq.rs`: EQ bars with preview text
    - `dictating/footer.rs`: footer hotkeys for dictating view
    - `dictating/status.rs`: status indicator (listening/idle)
  - `widgets.rs`: Pure re-export module for shared widgets
    - `widgets/brand.rs`: brand header with title + optional hotkey line
    - `widgets/hotkey.rs`: hotkey rendering helpers
    - `widgets/logs.rs`: rolling logs panel with syntax highlighting
  - `permissions.rs`: platform permission helpers (e.g., macOS Accessibility)
  - `logger.rs`: TUI-safe tracing layer (logs → channel)
  - `theme.rs`: color palette and theme utilities
- Depends on crates: `keyless-core`, `keyless-models`, `keyless-audio`, `keyless-whisper`, `keyless-output`, `keyless-runtime`, `keyless-logging`

### Installation

This is a workspace member. Build and run from the repo root:

```
cargo run -p keyless
```

### Usage

Run the TUI (first run downloads the model; subsequent runs are offline):
CLI help and flags (auto‑generated by clap):

```
keyless --help

USAGE:
  keyless [FLAGS] [OPTIONS]

FLAGS:
  -h, --help            Show help and exit
  -V, --version         Show version and exit
      --list-devices    List input devices and exit

OPTIONS (overrides current run only; saved on Enter/q in the TUI):
      --model <ID>                  HF model id (e.g. openai/whisper-base or openai/whisper-base.en)
      --language <CODE>             Language code (e.g. en, fr) - omit for auto-detection
      --sink <MODE>                 Output sink: paste | clipboard | file
      --file <PATH>                 File path when using --sink file
      --device <NAME>               Audio input device name
      --hotkey <HOTKEY>             PTT hotkey (e.g. "control+option", "ctrl+shift+d")
      --vad-start <DB>              VAD start threshold in dB (e.g. -30)
      --vad-stop <DB>               VAD stop threshold in dB (e.g. -35)
      --vad-min-duration <MS>       VAD minimum speech duration in ms
      --vad-max-silence <MS>        VAD maximum silence duration in ms
      --eq-bands <N>                EQ bands (16-128)
      --eq-noise-reduction <N>      EQ noise reduction (0.0-1.0, 0=noisy, 1=smooth)
      --eq-window <DB>              EQ window in dB (10-80)
      --eq-gamma <N>                EQ gamma (0.8-2.0)
      --eq-attack <N>               EQ attack (0.05-1.0)
      --eq-decay <N>                EQ decay (0.05-1.0)
```


```
KEYLESS_LOG=info cargo run -p keyless
```

Config screen (full‑screen):
- ↑/↓: select output sink (Paste | Clipboard | File)
- ←/→: select model (centralized list)
- [/]: select language (centralized list)
- n/m: select input device
- e: toggle file path edit (for File sink)
- backspace: delete selected model from disk (if installed)
- +/-: increase/decrease VAD start dB
- =/_: increase/decrease VAD stop dB
- 1/2: decrease/increase VAD min duration (ms)
- 3/4: decrease/increase VAD silence hangover (ms)
- Enter: save and start
- q: save and quit

Dictating screen:
- Top: brand header (app title + hotkey preset)
- Input card: mic device name, sink, model, language
- Status: LISTENING (green) when PTT hotkey held, IDLE otherwise
- Center: EQ‑style audio visualizer (non‑scrolling)
  - 64 log‑spaced bars across the full width; thin columns (▎)
  - FFT: 1024‑point real FFT with Hann window and auto-sensitivity normalization
  - Preview text: full preview text displayed at top of visualizer (truncated to screen width if needed)
  - Auto-sensitivity: bars normalized against max magnitude for consistent heights
  - Attack (0.35) and decay (0.12) smoothing for fluid animation
  - Tunable via expert mode: noise reduction, gamma curve, window dB, attack/decay
  - Colors per row: bottom blue, middle green, top amber, peak red
  - Bars are flat dark grey when PTT hotkey is not held
- Bottom: compressed, colorized logs (partial/final events, sink sends)
- Footer: hotkey hints ([esc] back to config, [q] quit)
- q: quit (Ctrl+C is ignored; use q)
- esc: return to config screen (stops audio/whisper, resets state)

### Expert Mode (EQ tuning)

Accessible from the Config screen with `x`. Draws a centered overlay with two framed sections:

- eq tuning: bands, noise reduction, window dB, gamma, attack, decay
- tools: open config folder (o), reset config to defaults (r)

Hotkeys while the panel is open:

- esc: close
- ↑/↓: select field
- ←/→: adjust (hold Shift for coarse)
- o: open the config folder in the OS default app (Finder/Explorer)
- r: reset config to safe defaults → confirmation prompt `confirm reset` → `y` to confirm (green), `n`/esc to cancel (red)

Notes:

- The footer dynamically swaps to show Expert Mode hotkeys while the overlay is open, and a confirm-only set (esc, y, n) while the reset prompt is shown.
- Parameter adjustments are clamped to safe bounds and log when clamped (e.g., `eq.gamma clamped to 2.00`).

Environment variables:

```
KEYLESS_LOG=info                # or crate filters, e.g., keyless_whisper=debug,info
RUST_LOG=info                   # fallback if KEYLESS_LOG not set
```

### Configuration

Config is stored in the OS config directory as JSON (e.g., macOS: `~/Library/Application Support/keyless/config.json`; Linux: `~/.config/keyless/config.json`; Windows: `%APPDATA%/keyless/config.json`).
On start, config is validated; on Enter or q, config is saved atomically.

Environment/CLI overrides (do not auto‑save; applied to the current run):
Precedence (applied to current run; saved on Enter/q):

```
CLI flags > ENV vars > saved config
```

Env:
```
KEYLESS_MODEL=openai/whisper-base.en
KEYLESS_LANGUAGE=fr
KEYLESS_SINK=paste|clipboard|file
KEYLESS_FILE=/tmp/out.txt
KEYLESS_DEVICE="USB Mic"
KEYLESS_HOTKEY="control+option"
KEYLESS_VAD_START_DB=-42
KEYLESS_VAD_STOP_DB=-52
KEYLESS_VAD_MIN_DURATION_MS=300
KEYLESS_VAD_MAX_SILENCE_MS=800
KEYLESS_EQ_BANDS=64
KEYLESS_EQ_NOISE_REDUCTION=0.46
KEYLESS_EQ_WINDOW_DB=50.0
KEYLESS_EQ_GAMMA=1.3
KEYLESS_EQ_ATTACK=0.35
KEYLESS_EQ_DECAY=0.12
```

CLI:
```
cargo run -p keyless -- \
  --sink paste \
  --model openai/whisper-base.en \
  --language en \
  --device "USB Mic" \
  --vad-start -42 --vad-stop -52
```

### Logging

The CLI initializes logging via `keyless-logging::init_stdout()` for non‑TUI commands. The TUI path installs a TUI‑safe layer that routes logs to an internal channel to avoid tearing. The Dictating screen shows the last few log lines.

Note: footer hotkeys are measured off‑screen for exact wrapping and keep each token with its label (e.g., `[q] quit`).

#### What we log (TUI)

- Startup & pipeline
  - Global hotkey listener started
  - Whisper startup phases forwarded to UI (loading overlay) and logs
  - Whisper/audio init OK or error details
  - One‑line pipeline configuration summary (sink, model, language, VAD thresholds/timers, EQ params)
- PTT & VAD
  - PTT held/released; VAD OPEN/CLOSED with current dB level
- Interactions in Config
  - Sink/model/language/device changes; VAD parameter changes; hotkey preset swaps; file edit toggles; “starting pipeline…”
- Dictating lifecycle
  - PTT enabled on enter; disabled on exit; quitting/returning to config actions
- Transcription
  - Partial/Final summaries (and sink send errors, if any)

### Push-to-Talk (PTT)

The app uses a global hotkey to gate audio processing and transcription:

- **Behavior**: 
  - While held: audio is captured and processed; status shows "LISTENING"
  - On release: final transcription is delivered to your sink; status shows "IDLE"
  - Whisper processing only occurs while the hotkey is active
  - Multiple dictations can overlap: if you press PTT again before the previous one completes, both will finish and be sent
- **Hotkey presets**: 
  - `control+option` (macOS-friendly default)
  - `control+shift` (alternative)
- **Dynamic swapping**: Press `h` in the Dictating view to toggle between presets. Changes are saved to config.
- **macOS Accessibility**: The global hotkey listener requires Accessibility permission on macOS. If the listener is inactive (no events received for 5+ seconds), a warning appears in the Config header with instructions: "System Settings → Privacy & Security → Accessibility". Enable Accessibility for keyless (or your terminal emulator) to use PTT.
- **Lifecycle**: The listener is enabled when entering the Dictating view and disabled when returning to Config. Only one listener thread is spawned at startup to avoid duplicates.

### Audio visualizer (EQ)

Implementation notes for the EQ meter:

- The audio callback computes RMS for VAD and a 1024‑point Hann‑windowed Real FFT.
- Magnitudes are grouped into 64 log‑spaced bands (≈120 Hz → Nyquist).
- Auto-sensitivity: each band is normalized against the global max for consistent bar heights.
- Tunable filtering and shaping: noise reduction threshold, gamma curve, window dB scaling.
- Attack (0.35) and decay (0.12) smoothing provides fluid animation.
- Bars are mapped to 0..100% and rendered as thin columns with color-coded zones.
- Bars are flat dark grey when PTT hotkey is not held.
- Status shows "LISTENING" (green) when PTT is active and audio is detected, "IDLE" otherwise.

### Testing

Run the integration suite:

```
cargo test -p keyless --test integration_tests
```

Note: microphone/Paste permissions are OS‑managed. On macOS, grant Accessibility to your terminal for Paste.

### Platform support (macOS, Windows, Linux)

- **Microphone**: OS prompts on first use across platforms.
- **Paste (enigo)**: macOS requires Accessibility permission; Windows/Linux typically work without prompts.
- **Push-to-Talk (PTT)**: macOS requires Accessibility permission for global hotkey listening. The app detects when the listener is inactive and shows guidance in the Config header. Windows/Linux typically work without prompts.
- **Model acceleration**:
  - macOS builds auto‑enable Metal; runtime logs “using Metal GPU device”.
  - Windows/Linux default to CPU; enable CUDA with:
    - `cargo run -p keyless --features "keyless-whisper/cuda"`
  - Runtime falls back to CPU if GPU init fails.

### Troubleshooting

**No transcription / empty text:**
- Check language is set correctly in Config (or use "★ auto" for multilingual models)
- Speak for at least 3–4 seconds while holding PTT (minimum context for stable preview)
- Check logs for "window inference completed" and "Final:" lines
- Verify the model downloaded successfully (check `~/.cache/keyless/models/`)

**Gibberish or repetitive text:**
- Switch to a larger model (`whisper-small` or `whisper-medium`)
- Adjust VAD thresholds in Config (try raising start dB by 2-3)
- Ensure you're speaking clearly with PTT held continuously

**Audio not captured:**
- Grant microphone permission when prompted
- Check that the correct device is selected in Config (use `n`/`m` to cycle devices)
- On macOS: grant Accessibility permission for PTT (System Settings → Privacy & Security → Accessibility)

**Performance issues:**
- Use Metal-accelerated models on macOS (automatic)
- On Windows/Linux: enable CUDA with `cargo run -p keyless --features "keyless-whisper/cuda"`
- Try smaller models (`whisper-tiny`) for faster transcription

### License

MIT


