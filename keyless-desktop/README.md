## keyless-desktop

_Local real‑time dictation desktop app with a modern GUI (Tauri v2 + React)._

### Overview

Native desktop application that wires microphone audio → Whisper transcription → output sinks (Paste, Clipboard, File). Features a system tray icon with a popover UI for settings and status, plus always-on-top overlay windows for visual feedback. Config persists across runs. VAD gating is included to reduce silence processing.

#### How it works (speech → text)
- While you hold the PTT hotkey, we capture 48 kHz mic audio and gate it with VAD so only speech enters the transcription buffer.
- Audio is resampled to 16 kHz via rubato (supports all device sample rates) and accumulated as one "utterance" during the PTT hold.
- **Preview**: Every ~120–200 ms, we run the same voiced‑mask pipeline on the current buffer (≤10s units with 0.5s overlap), reusing cached unit texts via a ~128 ms tail hash. Preview text matches what Final would be now.
- **Final**: When you release PTT, we split the utterance into voiced spans, process each as ≤10s units with overlap, stitch units together with dedupe, and deliver the Final to your selected sink. This prevents empty Finals on long dictations and improves accuracy.

#### Model Recommendations
- **Both multilingual and `.en` models are supported**
- Multilingual models (`openai/whisper-base`, `openai/whisper-small`): Support auto-detection, can switch languages
- English-only `.en` models (`openai/whisper-base.en`): Slightly smaller, English-only, no auto-detection
- Select your language in Settings for best accuracy (auto-detection is slower)
- Recommended: `openai/whisper-base` for balanced speed/accuracy/flexibility

### Features

- Real‑time audio capture (cpal) and framing
- Whisper inference via Candle (auto‑download model, cached offline)
- Resumable model downloads with progress tracking
- System tray integration with popover UI
- Always-on-top overlay windows for visual feedback
- Comprehensive settings UI (General, AI, VAD) with real-time persistence
- Model management interface with download progress
- Onboarding flow for first-time users
- Output sinks: Paste into active app, copy to Clipboard, append to File
- Config persistence with validation and atomic save
- VAD gating (start/stop dB, min duration, silence hangover) via `keyless-audio::VadGate`
- EQ spectrum visualization (FFT-based, log-spaced bands, auto-sensitivity via max normalization) via `keyless-audio::compute_bars()`
- Visual feedback overlays: Recording Pill (bottom-center) and Toast Notifications (top-right)
- Permissions UX: shows mic/accessibility guidance with system settings links

### Technologies

- Rust workspace (2024 edition), `tracing` for logs
- **Desktop**: Tauri v2, React, TypeScript, Tailwind CSS, Vite
- `cpal` for audio, `candle` (+ tokenizers) for Whisper, `rubato` for high-quality resampling (supports all device sample rates)
- `reqwest` for HTTP (model fetching via `keyless-models`)
- `enigo` (Paste), `arboard` (Clipboard)
- Custom `rdev` fork for macOS hotkeys (key-name lookup disabled to prevent crashes)

### Module structure

**Rust backend (`src-tauri/`):**
- `src/lib.rs`: Main entry point, Tauri builder setup
- `src/commands/`: Tauri IPC command handlers
  - `config.rs`: Configuration loading, saving, patching
  - `models.rs`: Model catalog and download management
  - `permissions.rs`: System permission checking
  - `runtime.rs`: Runtime status and control
  - `sinks.rs`: Output sink enumeration
- `src/services/`: Service layer abstractions
  - `config.rs`: Config service implementation
  - `context.rs`: Desktop context and event emitter
  - `models.rs`: Models service implementation
  - `permissions.rs`: Permissions service implementation
  - `pipeline.rs`: Pipeline service implementation
  - `ptt.rs`: PTT service implementation
  - `sinks.rs`: Sinks service implementation
- `src/runtime/`: Background runtime thread
  - `api.rs`: Runtime API for services
  - `bridge.rs`: Bridge between runtime thread and Tauri events
  - `events.rs`: Event definitions
  - `loop.rs`: Main runtime event loop
  - `pipeline.rs`: Pipeline lifecycle management
  - `ptt.rs`: PTT hotkey handling
  - `startup.rs`: Phased startup logic
  - `state.rs`: Runtime state management
- `src/overlay.rs`: Overlay window management (Recording Pill, Toast)
- `src/popover.rs`: Popover window blur suppression
- `src/tray.rs`: System tray icon and menu
- `tests/`: Service layer unit tests

**React frontend (`src/`):**
- `App.tsx`: Main application component, view orchestration
- `views/`: Main application views
  - `Idle.tsx`: Idle state view
  - `Listening.tsx`: Listening state view with audio visualizer
  - `Settings.tsx`: Settings view (General, AI, VAD sections)
  - `Models.tsx`: Model management view
  - `Onboarding.tsx`: First-time user onboarding
- `components/`: Reusable UI components
  - `settings/`: Settings section components (AISection, GeneralSection, VADSection)
  - `buttons/`: Button components (ColoredButtonGroup, IconButton, LinkButton, SecondaryButton)
  - `fields/`: Form field components (SelectField, SliderField)
  - `layout/`: Layout components (Arrow, Card, SettingsSection)
  - `ui/`: UI utility components (AnimatedNumber, DownloadProgress, ErrorToast, ScrollFade)
  - `AudioVisualizer.tsx`: EQ-style audio visualization
  - `BrandTitle.tsx`: Brand header component
  - `DownloadOverlay.tsx`: Model download progress overlay
  - `ErrorBoundary.tsx`: React error boundary
  - `ModelRow.tsx`: Model list row component
  - `PermissionCard.tsx`: Permission status card
  - `StatusPill.tsx`: Status indicator pill
- `overlay/`: Overlay window components
  - `pill/`: Recording Pill overlay (Pill.tsx, Pill.css)
  - `toast/`: Toast notification overlay (Toast.tsx, Toast.css)
- `hooks/`: React hooks for state management
  - `useAnimation.ts`: Mount animation hook
  - `useDownloads.ts`: Download management hook
  - `useModelEventListeners.ts`: Model event listeners
  - `useModels.ts`: Model catalog hook
  - `useModelsState.ts`: Model state management
  - `useOverlayHeight.ts`: Overlay height calculation
  - `usePermissions.ts`: Permissions checking hook
  - `useScrollFade.ts`: Scroll fade effect hook
  - `useSettings.ts`: Settings management hook
  - `useSettingsEventListeners.ts`: Settings event listeners
  - `useSettingsState.ts`: Settings state management
  - `useStats.ts`: Statistics calculation hook
  - `useTauriStreams.ts`: Tauri event stream hook
  - `useViews.ts`: View state management hook
- `utils/`: Utility functions
  - `configHelpers.ts`, `configParsing.ts`, `configPatches.ts`: Config utilities
  - `errorHandling.ts`, `errorNotifications.ts`: Error handling
  - `eventListeners.ts`: Event listener utilities
  - `format.ts`: Formatting utilities
  - `hotkeyParsing.ts`: Hotkey parsing
  - `logger.ts`: Logging utilities
  - `modelStateHelpers.ts`, `modelStatus.ts`, `modelStatusParsing.ts`: Model utilities
  - `numberHelpers.ts`: Number utilities
  - `payloadHelpers.ts`: Payload parsing utilities
  - `tauriHelpers.ts`: Tauri API helpers
- `constants/`: Application constants
  - `colors.ts`: Color palette
  - `views.ts`: View type definitions
- `types/`: TypeScript type definitions

**Entry points:**
- `index.html`: Main popover window
- `overlay.html`: Recording Pill overlay window
- `toast.html`: Toast notification overlay window

- Depends on crates: `keyless-core`, `keyless-models`, `keyless-audio`, `keyless-whisper`, `keyless-output`, `keyless-runtime`, `keyless-logging`

### Installation

**Prerequisites:**
- Node.js (v18 or later)
- pnpm (`npm install -g pnpm`)
- Rust (stable, 2024 edition)
- Tauri CLI (`cargo install tauri-cli`)

**Build from source:**
```bash
cd keyless-desktop
pnpm install
pnpm tauri dev    # Development mode
pnpm tauri build  # Production build
```

**First run:**
1. Grant macOS permissions when prompted:
   - **Microphone**: System Settings → Privacy & Security → Microphone
   - **Accessibility**: System Settings → Privacy & Security → Accessibility (required for PTT hotkeys and Paste sink)
2. Download a Whisper model (Settings → Models)
3. Configure hotkey in Settings → General
4. Select output sink (Paste/Clipboard/File) in Settings → General

### Usage

**System tray:**
- Click tray icon to open/close popover
- Right-click for context menu (Settings, Output Mode, Quit)

**Popover views:**
- **Idle**: Shows status, word count, talk time, quick access to Settings
- **Listening**: Shows audio visualizer, live preview text, status indicator
- **Settings**: General, AI, and VAD configuration with real-time persistence
- **Models**: Model catalog, download progress, installation status
- **Onboarding**: First-time user flow (permissions, model download, quick start)

**Visual feedback overlays:**
- **Recording Pill** (bottom-center): Appears when PTT is pressed, shows EQ bars and VAD state
- **Toast Notifications** (top-right): Shows preview text while speaking, final transcription with word count and sink indicator

**Push-to-Talk (PTT):**
- Hold configured hotkey (default: `control+option`) to start dictating
- Release to finalize and deliver transcription
- Status shows "LISTENING" when active, "IDLE" when not

### Configuration

Config is stored in the OS config directory as JSON (e.g., macOS: `~/Library/Application Support/keyless/config.json`; Linux: `~/.config/keyless/config.json`; Windows: `%APPDATA%/keyless/config.json`).

Settings are persisted in real-time as you adjust them in the Settings view.

**Settings sections:**
- **General**: Output sink, hotkey, input device
- **AI**: Model selection, language
- **VAD**: Voice activity detection thresholds and timing

### Logging

The desktop app uses structured JSON logging via `keyless-logging::init_json()`. Logs are written to `~/.cache/keyless/session.log` and also streamed to the backend for error notifications.

**What we log:**
- Startup & pipeline initialization
- PTT & VAD state changes
- Transcription events (Partial/Final)
- Model downloads and status
- Configuration changes
- Error notifications

### Push-to-Talk (PTT)

The app uses a global hotkey to gate audio processing and transcription:

- **Behavior**: 
  - While held: audio is captured and processed; status shows "LISTENING"
  - On release: final transcription is delivered to your sink; status shows "IDLE"
  - Whisper processing only occurs while the hotkey is active
- **Hotkey presets**: 
  - `control+option` (macOS-friendly default)
  - `control+shift` (alternative)
- **Configuration**: Set in Settings → General
- **macOS Accessibility**: Required for global hotkey listening. The app checks permissions and shows guidance if not granted.
- **macOS PTT Crash Fix**: We use a custom `rdev` fork with key-name lookup disabled to prevent crashes after WebKit/IMK activation. See "PTT on macOS: Queue Assertion Crash (Resolved)" section below.

### Audio visualizer (EQ)

The Listening view includes an EQ-style audio visualizer:

- 64 log‑spaced bars visualizing real-time microphone input
- FFT: 1024‑point real FFT with Hann window and auto-sensitivity normalization
- Auto-sensitivity: bars normalized against max magnitude for consistent heights
- Attack and decay smoothing for fluid animation
- Color-coded zones: gradient from green (low) to red (high)
- Responds to VAD state: shows activity when VAD is open

### UI Overlays

The app includes two always-on-top overlay windows for visual feedback:

**Recording Pill Overlay:**
- **Location**: Bottom-center of screen
- **Trigger**: Appears when PTT hotkey is pressed, stays visible while finalizing transcription
- **Features**:
  - Animated EQ bars visualizing real-time microphone input
  - Idle state (grey circles) when no speech detected
  - Active state (animated gradient bars) responding to audio levels via VAD
  - Finalizing state (white center-out pulse) while waiting for final transcription
  - Horizontal scale-in/out animations
- **Implementation**: Separate transparent Tauri window (`overlay.html`), non-focusable and click-through

**Toast Notification:**
- **Location**: Top-right of screen
- **Trigger**: Appears when preview text starts, continuously updates with partial transcripts while speaking, then flashes sink branding once final output arrives and auto-dismisses ~5 seconds later
- **Features**:
  - Streams preview text in real time while speaking
  - Locks in final transcription with word count and output destination after PTT release
  - Sink branding (PASTE/CLIPBOARD/FILE) flashes in background only on final event
  - Slide-up entrance and exit animations
- **Implementation**: Separate transparent Tauri window (`toast.html`), non-focusable and click-through

**Technical Details:**
- Both overlays are independent webview windows created in `src-tauri/src/overlay.rs`
- Event-driven: Rust backend emits events (`status_changed`, `vad_speaking`, `eq_update`, `final_transcription`), React components listen and update UI
- Cross-platform positioning using monitor work area calculations
- Capabilities configured in `src-tauri/capabilities/default.json`
- Build configuration includes both HTML entry points in `vite.config.ts`

### Testing

Run the service layer unit tests:
```bash
cargo test -p keyless-desktop
```

Note: Microphone/Paste permissions are OS‑managed. On macOS, grant Accessibility to the app for Paste and PTT.

### Platform support (macOS, Windows, Linux)

- **Microphone**: OS prompts on first use across platforms.
- **Paste (enigo)**: macOS requires Accessibility permission; Windows/Linux typically work without prompts.
- **Push-to-Talk (PTT)**: macOS requires Accessibility permission for global hotkey listening. The app checks permissions and shows guidance if not granted. Windows/Linux typically work without prompts.
- **Model acceleration**:
  - macOS builds auto‑enable Metal; runtime logs "using Metal GPU device".
  - Windows/Linux default to CPU; enable CUDA with feature flag (requires rebuilding).
  - Runtime falls back to CPU if GPU init fails.

### Platform Support Status

**macOS**: ✅ Fully tested and supported
- All features working (autostart, system settings links, tray icon)
- PTT crash fix implemented and tested

**Windows/Linux**: ⚠️ Core functionality should work, but:
- **Autostart**: Implemented but may need testing on various distributions/Windows versions
- **System settings links**: Implemented but may not work on all Linux distributions
- **UI behaviors**: Some macOS-specific behaviors (activation policy, menubar-only mode) don't apply

**Known Limitations:**
- System settings opening on Linux depends on desktop environment (GNOME preferred)
- Autostart on Linux uses systemd (may not work on non-systemd distributions)
- Some UI polish may differ between platforms

**Contributions welcome**: If you test on Windows/Linux and find issues, please report them!

### Troubleshooting

**No transcription / empty text:**
- Check language is set correctly in Settings → AI (or use "Auto" for multilingual models)
- Speak for at least 3–4 seconds while holding PTT (minimum context for stable preview)
- Check logs in `~/.cache/keyless/session.log` for errors
- Verify the model downloaded successfully (Settings → Models)

**Gibberish or repetitive text:**
- Switch to a larger model (`whisper-small` or `whisper-medium`) in Settings → AI
- Adjust VAD thresholds in Settings → VAD (try raising start dB by 2-3)
- Ensure you're speaking clearly with PTT held continuously

**Audio not captured:**
- Grant microphone permission when prompted (System Settings → Privacy & Security → Microphone)
- Check that the correct device is selected in Settings → General
- On macOS: grant Accessibility permission for PTT (System Settings → Privacy & Security → Accessibility)

**PTT not working:**
- On macOS: grant Accessibility permission (System Settings → Privacy & Security → Accessibility)
- Verify hotkey is configured correctly in Settings → General
- Check logs for PTT listener errors

**Performance issues:**
- Use Metal-accelerated models on macOS (automatic)
- Try smaller models (`whisper-tiny`) for faster transcription
- Check system resource usage (Activity Monitor / Task Manager)

### PTT on macOS: Queue Assertion Crash (Resolved)

**Symptoms:**
- After launching, PTT works. After opening the popover (WebKit) at least once, pressing a modifier (Ctrl/Alt/Shift/⌘) could crash the app with `EXC_BREAKPOINT` in `libdispatch.dylib::_dispatch_assert_queue_fail`.

**Backtrace highlights:**
- `libdispatch._dispatch_assert_queue_fail`
- `HIToolbox.islGetInputSourceListWithAdditions → TSMGetInputSourceProperty`
- `rdev::macos::keyboard::string_from_code/create_string_for_key`
- `rdev::macos::common::convert(FlagsChanged) → listen::raw_callback`

**Root cause:**
- The rdev macOS path tries to build a human‑readable key name via HIToolbox/TIS from the CGEvent tap callback thread. After WebKit/IMK is active (once the popover loads), Apple enforces a main‑queue precondition for these APIs and aborts with a dispatch assertion.

**Fix implemented:**
- We use a fork of `rdev` with key‑name generation feature‑gated on macOS, and we disable it in this workspace. PTT only needs modifier states, not per‑key "names".
  - Cargo wiring (root `Cargo.toml`):
    - `[patch.crates-io] rdev = { git = "https://github.com/hate/rdev", branch = "macos-no-key-names" }`
  - Runtime dependency (`keyless-runtime/Cargo.toml`):
    - `rdev = { version = "0.5", default-features = false }` (turns off `key_names`)

**Debug notes:**
- If you ever need to investigate again:
  - Attach LLDB: `lldb -p $(pgrep -n keyless-desktop)`
  - Breakpoints: `b __abort_with_payload`, `b __assert_rtn`, `b objc_exception_throw`
  - On stop: `thread select <id>` then `bt 50`
  - Expect no frames in `TSMGetInputSourceProperty` now.

**Behavior after fix:**
- PTT works before/after opening the popover. No crashes on modifiers. Logs show "PTT pressed/released" and `status_changed: listening/idle`.

### License

MIT
