## keyless-core

_Shared types and contracts for the workspace._

### Overview

`keyless-core` provides configuration (`Config`, `OutputMode`), error handling (`KeylessError`, `KeylessResult`), events (`TranscriptionEvent`), options (model/language lists), and the `OutputSink` trait.

### Features

- Strongly‑typed config with serde serialization
- Central error type (`KeylessError`) and result alias (`KeylessResult`)
- Granular error variants: `Audio`, `Whisper`, `Output`, `Config`, `Io`, `InvalidPath`, `System`, `Ptt`, `Permissions`, `Other`
- Event model for Partial/Final transcriptions
- Curated language codes (`LANG_CODES`) and helpers
- Shared utilities: `now_millis()`, `human_size()` for cross-crate use
- OutputSink trait for pluggable output backends

### Installation

Internal workspace dependency:

```
keyless-core = { path = "../keyless-core" }
```

### Usage

```
use keyless_core::config::{Config, OutputMode};
use keyless_core::options::{LANG_CODES, lang_name, is_english_only_model};

let mut config = Config::default();
config.output_mode = OutputMode::File("/tmp/keyless.txt".into());
config.validate()?;
```

### Configuration

- `Config` includes model path/ID, language, hotkey (push-to-talk preset), output mode, and VAD thresholds.
- `Config::validate()` checks file paths and language code format.
- Hotkey format: `"control+option"` or `"control+shift"` (used for PTT gating).

### Logging

This crate does not initialize logging, but it emits `tracing` events in storage helpers:

- `config::storage::config_path()` – trace path computation
- `config::storage::load_config()` – debug on load, warn on parse failure
- `config::storage::save_config()` – debug on successful save (path, byte size)

Initialize logging in the binary (`keyless-logging`) to see these.

### Testing

```
cargo test -p keyless-core
```

### Technologies

- `serde`/`serde_json` for serialization
- `thiserror` for error handling

### Module structure

- `src/config.rs`: Pure re-export module for configuration
  - `config/schema.rs`: `Config`, `OutputMode`, `VadThresholds`, `EqTuning` + validation + constants
  - `config/storage.rs`: Load/save config from disk (JSON, atomic writes)
- `src/error.rs`: `KeylessError` (with `thiserror`), `KeylessResult`
- `src/events.rs`: `TranscriptionEvent` (Partial/Final)
- `src/options.rs`: Curated language codes (`LANG_CODES`, `lang_name()`, `is_english_only_model()`)
- `src/output.rs`: `OutputSink` trait
- `src/permissions.rs`: Platform permission helpers
- `src/utils.rs`: Shared utilities (`now_millis()`, `human_size()`)

Note: CLI/ENV overrides (`Overrides`, `apply_overrides()`) live in the `keyless` binary crate, not here.

### Platform support (macOS, Windows, Linux)

Cross‑platform; no OS‑specific code in this crate.

### License

MIT


