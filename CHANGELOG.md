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

## [0.2.0] - 2025-11-04

### Breaking
- keyless-runtime: `create_audio_callback` now takes `AudioCallbackParams` instead of many args.
- keyless-runtime: sink type simplified to `Arc<dyn OutputSink>`.

[0.2.0]: https://github.com/hate/keyless/releases/tag/v0.2.0

