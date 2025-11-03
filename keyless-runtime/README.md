# keyless-runtime

Application orchestration layer for keyless.

## Purpose

This crate provides the business logic for coordinating audio capture, voice activity detection, transcription, and output delivery. It's framework-agnostic and can be used by multiple frontend implementations (TUI, Tauri desktop app, CLI).

## What's Inside

- **Pipeline orchestration** (modular structure):
  - `src/pipeline.rs`: Pure re-export module
  - `src/pipeline/startup.rs`: Phased startup (`StartupEvent`, `build_startup_artifacts()`, `finalize_pipeline_from_artifacts()`, `start_pipeline()`)
  - `src/pipeline/callback.rs`: Audio frame callback with PTT state machine
  - `src/pipeline/events.rs`: Event processing and preview formatting
- **Push-to-talk (PTT)** (modular structure):
  - `src/ptt.rs`: Pure re-export module
  - `src/ptt/listener.rs`: Global hotkey listener (rdev integration)
  - `src/ptt/context.rs`: PTT context (flags, handle, timing)
  - `src/ptt/handlers.rs`: PTT press/release event handlers
- **Worker lifecycle**: Start/stop management for audio and whisper services
- **Channel-based events**: mpsc channels for async communication between services
- **Runtime state**: Catalog, download context, PTT context management

## Pipeline Architecture

The audio frame callback is organized as a state machine:

1. **PTT Released** → finalize current segment, deliver Final transcription
2. **PTT Pressed** → clear any in-progress transcription, start fresh
3. **PTT Held + VAD Open** → push audio, process Partial/Final events

Each state transition is handled in a dedicated module for clarity and testability.

### Phased Startup

To keep the UI responsive and avoid moving non-Send types across threads (e.g., `cpal::Stream`), pipeline startup is split into two phases:

1. `build_startup_artifacts(config, eq_tuning, progress)` (background thread)
   - Initializes Send-safe components (e.g., Whisper transcriber)
   - Emits `StartupEvent::PhaseStarted/PhaseOk/PhaseErr` for granular progress (forwarded from `keyless-whisper` load phases)
2. `finalize_pipeline_from_artifacts(artifacts, ...)` (main thread)
   - Constructs non-Send components (audio stream, output sink)

`spawn_startup_worker` runs phase 1 in the background and sends `StartupEvent`s to the TUI, which renders a "loading" overlay with live log lines and allows `[esc]` to cancel.

## Dependencies

- `keyless-core`: Domain types, config, errors
- `keyless-audio`: Audio capture and processing
- `keyless-whisper`: Transcription engine
- `keyless-output`: Output sink implementations
- `keyless-models`: Model discovery and download
- `rdev`: Global keyboard event listening (for PTT)

## Usage

See the main crate documentation for usage examples.

## License

MIT

