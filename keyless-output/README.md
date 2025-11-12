## keyless-output

_Clipboard, File, and keystroke output sinks._

### Overview

Implements output sinks for delivering transcribed text. A provider selects the sink from `Config`.

### Features

- `ClipboardSink` (arboard)
- `FileSink` (append one line per Final)
- `PasteSink` (enigo keystrokes)
- `DefaultOutputProvider` maps `Config::output_mode` to a sink
- All sinks implement `label()` method for self-describing output (used in runtime logs)

### Installation

```
keyless-output = { path = "../keyless-output" }
```

### Usage

```
use keyless_core::config::{Config, OutputMode};
use keyless_output::{DefaultOutputProvider, OutputSinkProvider};

let mut config = Config::default();
config.output_mode = OutputMode::Clipboard;
let sink = DefaultOutputProvider.provide(&config)?;
sink.send_text("Hello")?;
```

In the TUI, the sink is selectable on the Config screen (↑/↓), and is shown in the header during the Dictating screen.

### Configuration

`OutputMode` options:
- `Paste` – requires OS permissions (macOS Accessibility)
- `Clipboard` – copies to system clipboard
- `File(PathBuf)` – appends per Final segment

### Logging

Emits `tracing` logs on success/failure for each sink.

### Testing

```
cargo test -p keyless-output
```

Clipboard/Paste tests are tolerance tests due to headless/permission variability.

### Technologies

- `arboard` for Clipboard
- `enigo` for keystrokes (Paste)
- `tracing` for logging

### Module structure

- `src/lib.rs`: re-exports and sinks module entry
- `src/provider.rs`: `DefaultOutputProvider` and `OutputSinkProvider` trait
- `src/sinks/mod.rs`: sink module entry
- `src/sinks/clipboard.rs`: `ClipboardSink`
- `src/sinks/file.rs`: `FileSink`
- `src/sinks/paste.rs`: `PasteSink`

### Platform support (macOS, Windows, Linux)

- Clipboard works across platforms (headless environments may restrict it).
- Paste requires enabling Accessibility on macOS; Windows/Linux generally work without extra setup.

### License

MIT


