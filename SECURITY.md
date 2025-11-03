# Security Policy

## Supported Versions

We actively support security updates for the latest stable release. Please update to the latest version to ensure you have the latest security patches.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

### For Users

- Always download releases from the [official GitHub releases page](https://github.com/hate/keyless/releases)
- Verify checksums when available
- Keep your system and dependencies updated
- Review permissions requested by the application (microphone access, etc.)

### For Developers

- Follow the [Contributing Guide](CONTRIBUTING.md)
- Run security checks before submitting PRs: `cargo audit` and `cargo deny check`
- Never commit secrets, API keys, or tokens
- Use environment variables for sensitive configuration

## Security Considerations

### Privacy-First Architecture

keyless is designed with privacy as a core principle:

- **100% local processing** - All audio and transcription stays on your device
- **No network required** - After initial model download, works completely offline
- **No telemetry** - Zero tracking or analytics
- **Open source** - Full code auditability

### Data Handling

- Audio is processed in memory and never persisted unless explicitly configured
- Models are cached locally in `~/.cache/keyless/models/`
- Configuration is stored locally in OS-specific config directories
- Log files may contain debugging information but no sensitive audio data

### Permissions

keyless requires:
- **Microphone access** - For audio capture (system-level permission)
- **Accessibility/Automation permissions** - For paste output mode (macOS/Windows)

These permissions are necessary for core functionality and are clearly documented.

## Security Audits

We use automated security scanning:

- `cargo audit` - Checks against RustSec advisory database
- `cargo deny` - License compliance and security checks
- CI/CD runs these checks on every commit

To run locally:
```bash
cargo install cargo-audit cargo-deny --locked
cargo audit
cargo deny check
```

## Known Limitations

- Model downloads are not cryptographically verified (future improvement)