## keyless-logging

_Shared logging initialization helpers._

### Overview

Centralizes logging subscriber initialization. Libraries emit `tracing` events; binaries initialize a subscriber exactly once.

### Features

- `init_stdout()` – compact, human‑friendly formatting to stdout; respects `KEYLESS_LOG` or `RUST_LOG`
- `init_json()` – structured JSON output for GUI/backend use

### Installation

```
keyless-logging = { path = "../keyless-logging" }
```

### Usage

```
// in the binary (CLI or Tauri backend)
keyless_logging::init_stdout();
// or
keyless_logging::init_json();

// in libraries
tracing::info!("something happened");
```

TUI: the CLI initializes logging once at startup; the Dictating screen shows a rolling subset of log lines for quick inspection.

Env filters:

```
KEYLESS_LOG=keyless_whisper=debug,keyless_output=info
RUST_LOG=info  # fallback
```

## Testing

This crate contains initialization helpers only; no tests are required.

### Technologies

- `tracing` for instrumentation
- `tracing-subscriber` for subscribers and formatting (human and JSON)

### Module structure

- `src/lib.rs`: `init_stdout()`, `init_json()`, and `init_channel()` helpers

### Platform support (macOS, Windows, Linux)

Outputs to stdout/stderr across platforms.

### License

MIT


