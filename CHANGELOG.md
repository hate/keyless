# Changelog

All notable changes to keyless will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-11-03

### Added
- Initial release
- Quantized model support (GGUF auto-detection)
- No-speech detection (prevents hallucinations on silence)
- Temperature fallback decoding (6-temp schedule for quality)
- Language auto-detection (99 languages supported)
- Quality metrics tracking (avg_logprob, compression_ratio, no_speech_prob)
- .en model support (proper token ID handling)
- Complete CLI (16 configuration options via flags and ENV vars)
- Comprehensive logging (tracing in all crates, session.log)
- Multi-platform CI (Linux, macOS, Windows)
- Automated CD (4 platform binaries on release)
- Issue & PR templates
- Preview text display: last 4 recognized words shown at top of EQ visualizer
- Preview text auto-clears after 500ms of no updates for cleaner UI
- HTTP timeouts (10s connect, 60s request) for GET/HEAD

### Changed
- EQ visualizer now uses auto-sensitivity (max normalization) for consistent bar heights
- EQ visualizer uses 8-level precision (NINE_LEVELS bar set) for smoother animation (matches spectroscope reference)
- Preview text moved to top of visualizer (prevents bars from jumping when text appears/disappears)
- Replaced `margin_db` parameter with `noise_reduction` (0.0-1.0 range)
- Simplified EQ tuning with more intuitive noise reduction threshold
- Status-aware resume: handle 206 (append) and 200 (restart/truncate) to avoid `.partial` corruption
- Reuse blocking HTTP client for HEAD size checks (connection pooling)
- Runtime: Non-blocking, phased pipeline startup with status overlay; unified init path; device preload warm‑up
- TUI: Startup uses a dedicated "loading" overlay (not download overlay)
  - Shows granular Whisper load phases with human-friendly labels
  - Live logs streamed into the overlay message
  - `[esc] cancel` cancels startup

### Fixed
- Download overlay no longer appears for already-cached models
- VAD state now properly resets between dictation sessions
- Model selection now persists across app launches (remembers last selected model)
- Removed duplicate constructor logic in `keyless-whisper` (single path via `new_with_progress`)

### Performance
- EQ pipeline: 80-90% faster via FFT planner and buffer caching (zero per-frame allocations)
- Audio callback: cached zero spectrum vector (eliminates ~10 allocations/sec when idle)
- TUI rendering: cached runtime_list to avoid per-frame allocation

[0.1.0]: https://github.com/hate/keyless/releases/tag/v0.1.0

## [0.3.0] - 2025-11-11

### Added

#### Desktop App (Beta)
- **Complete desktop application** built with Tauri v2 + React + TypeScript
  - Native macOS/Windows/Linux desktop app with system tray integration
  - Modern, responsive UI with smooth animations and transitions
  - Full feature parity with TUI version
- **Settings UI** with comprehensive configuration management
  - General settings: hotkey configuration, output mode selection, file path
  - AI settings: model selection, language detection, quality parameters
  - VAD settings: noise reduction, thresholds, and audio tuning
  - Real-time settings persistence and validation
- **Model management interface**
  - Visual model catalog with size information and download status
  - Download progress tracking with pause/resume/cancel controls
  - Model installation status indicators
  - Automatic model discovery and catalog updates
- **Onboarding flow** for first-time users
  - Permission checks (microphone, accessibility)
  - Model download guidance
  - Quick start instructions
- **Visual feedback overlays**
  - **Recording Pill Overlay**: Bottom-center indicator with animated EQ bars showing real-time audio levels and VAD state
    - Idle state (grey circles) when no speech detected
    - Active state (animated gradient bars) responding to audio levels via VAD
    - Finalizing state (white center-out pulse) while waiting for backend transcription
  - **Toast Notifications**: Top-right notifications displaying transcription results
    - Appears when preview text starts, continuously updates with partial transcripts while speaking
    - Shows final transcription with word count and output destination (paste/clipboard/file) after PTT release
    - Sink branding (PASTE/CLIPBOARD/FILE) flashes in background on final delivery
    - Auto-dismisses after ~5 seconds
- **System tray integration**
  - Tray icon with context menu
  - Quick access to settings, output mode selection, and quit
  - Always-on-top popover window for main UI
- **Runtime pipeline integration**
  - Full audio pipeline lifecycle management
  - Push-to-talk hotkey support (configurable: `control+option`, `control+shift`)
  - Real-time status updates (idle, listening, finalizing)
  - Event-driven architecture with Tauri IPC bridge
- **Developer experience**
  - Comprehensive inline documentation (Rust + TypeScript)
  - Type-safe IPC commands and events
  - Error boundary components for graceful error handling
  - DevTools console logging for backend events
  - React component tests with Vitest and Testing Library
- **CI/CD integration**
  - Automated desktop app builds for macOS/Windows/Linux
  - Release workflow with artifact generation
  - TypeScript type checking and tests in CI
  - Updated CI check scripts (`check-ci.sh`, `check-ci.ps1`) to support desktop app builds

#### Core Improvements
- **Audio API enhancements**
  - Microphone permission/availability probe (`can_record`) for better permission handling
  - Improved audio device detection and selection
- **Output sink improvements**
  - `OutputSink` trait now exposes `label()` method for self-describing sinks
  - Runtime logs use sink labels for clearer delivery tracking
  - Sink selection persists across app launches
- **Whisper engine enhancements**
  - **Unified Preview = Final pipeline**: Preview runs the same voiced‑mask pipeline incrementally
  - **Voiced span detection and unitization**: New `build_voiced_spans` system that splits audio into ≤10s voiced units with 0.5s overlap
    - Hysteresis thresholds (open/close dB) for robust speech detection
    - Min speech duration and max silence gaps for natural segmentation
    - Symmetric padding around speech segments for context preservation
    - Unit tests for voiced span detection, merging, and padding behavior
  - Preview cache with ~128 ms tail‑hash cache for unit text reuse (no more 1s snippet path)
  - Voiced‑mask single pass for Finals and offline (`run_inference_voiced`)
- **Runtime improvements**
  - More granular Whisper load phase telemetry (`locating model`, `mapping weights`, etc.)
  - Full preview text logging (no last‑N truncation)
  - Improved event bridge for desktop app integration
- **Documentation**
  - Comprehensive inline comments throughout Rust backend (`keyless-desktop/src-tauri/src`)
  - Detailed TypeScript/React frontend documentation (`keyless-desktop/src`)
  - All modules, functions, hooks, and components include purpose, implementation details, and usage patterns

### Changed
- keyless-core/output: `OutputSink` trait now exposes `label()` so sinks label themselves
- keyless-output: Paste/Clipboard/File sinks implement `label()`; runtime logs final deliveries using the sink's own label
- keyless-runtime: `process_transcription_events` now queries `sink.label()`; `AudioCallbackParams` no longer needs a `sink_label` field
- Whisper: Removed global no‑speech gating; now uses voiced span unitization with per‑unit silence‑drop (RMS + `no_speech_prob`) and overlap dedupe
- Whisper: Renamed `decode_units_no_gate` → `decode_voiced_units`; `forward_decode_with_metrics` → `decode_mel_tensor`
- Runtime: Logs label "Preview:" and send full preview text (no last‑N truncation)

### Breaking
- **keyless-whisper**: Removed `InferReq::Partial`; use `InferReq::Preview(Vec<f32>)` (whole buffer)
- **keyless-whisper**: Removed `run_inference_window()` and legacy `run_inference()` helpers; use `run_inference_voiced()` (offline) or the Preview/Final paths
- **keyless-runtime**: `create_audio_callback` now takes `AudioCallbackParams` instead of many args
- **keyless-runtime**: sink type simplified to `Arc<dyn OutputSink>`

### Fixed
- **Whisper: Fixed encoder crashes on long audio** by capping mel spectrogram frames to 3000 (30 seconds)
  - **Problem**: Candle's mel builder sometimes produced more than 3000 frames due to internal padding, causing encoder tensor shape mismatches ("narrow" errors)
  - **Solution**: Cap mel frames to `2 × max_source_positions` (3000 frames = full 30s Whisper context) before passing to encoder
  - **Result**: Encoder no longer crashes on long audio segments; all audio is safely processed within Whisper's native context window
- **Whisper: Fixed critical empty Final bug on long dictations**
  - **Problem**: After extended PTT holds (>30s), Final transcription would be empty or transcription would stop working entirely
  - **Root cause**: Monolithic 30s windows with oversized mel frames caused encoder panics and greedy decoding bias toward short polite priors
  - **Solution**: Implemented voiced span unitization system
    - Split audio into ≤10s voiced units with 0.5s overlap using `build_voiced_spans`
    - Process each unit independently with voiced‑mask decoding
    - Stitch units together with overlap dedupe
    - Apply per‑unit silence‑drop (RMS + `no_speech_prob`) to prevent hallucinations
  - **Result**: Long dictations now produce accurate, complete transcriptions without empty Finals or transcription failures
- keyless-core: Improved macOS accessibility permission checks using `AXIsProcessTrustedWithOptions(NULL)` for accurate detection

### Performance
- Whisper: Preview decodes only tail units when speaking thanks to per‑unit cache; most units are reused between updates

[0.3.0]: https://github.com/hate/keyless/releases/tag/v0.3.0

