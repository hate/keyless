# keyless-models

HTTP/HF helpers and model metadata/cache utilities for Keyless.

## Features
- Reqwest-only HTTP
- HF token support via env
- Resolve URLs for repo files
- Resumable downloads via HTTP Range headers and `.partial` files
- Connection pooling with sane HTTP timeouts (10s connect, 60s total) for GET/HEAD
- Sizes TTL cache (JSON)
- HF cache path helpers and local repo discovery
- Small helpers for HEAD with backoff and async retry/backoff
  - Defaults: connect_timeout=10s, request_timeout=60s (blocking and async clients)

## Env vars
- `HF_TOKEN` or `HUGGINGFACE_HUB_TOKEN`: bearer token for restricted models
- `KEYLESS_CACHE_DIR`: override cache root (tests/dev)

## Module structure
- `download`: Model download with progress reporting and cancellation (`DownloadEvent`, `ensure_model_cached`)
  - Status-aware resume (206 append, 200 restart/truncate) with periodic progress (≤4 Hz)
  - Organized into submodules: `events`, `model`, `small_files`, `large_file`, `progress`
- `hf.rs`: HF cache path helpers (`keyless_cache_repo_dir`, `get_local_model_size`, `delete_partial_file`)
- `net.rs`: HTTP utilities (auth headers, URL resolution, retry/backoff)
- `meta.rs`: Sizes TTL cache management
- `catalog.rs`: OpenAI Whisper model catalog
- `discover.rs`: Local repo discovery
- `workers.rs`: Background workers for size fetching (TUI integration)

## Cache paths
- Keyless model cache: `~/.cache/keyless/models/<org>--<repo>/...`
  - Contains downloaded `config.json`, `tokenizer.json`, `model.safetensors`
  - Local discovery scans this directory only
- Sizes TTL JSON: `~/.cache/keyless/meta/models.json`

## Public APIs (selected)
- `download::DownloadEvent` - progress events enum
- `download::ensure_model_cached(model_id, tx, cancel)` - download with progress reporting
- `net::auth_header() -> Option<(HeaderName, HeaderValue)>`
- `net::hf_resolve_url(model, file) -> String`
- `net::head_len_with_backoff(url, auth, attempts, initial_ms, max_ms) -> Option<u64>`
- `net::blocking_get_with_backoff(client, url, auth, attempts, initial_ms, max_ms) -> KeylessResult<(Vec<u8>, Option<u64>)>`
- `net::retry_with_backoff(op, attempts, initial_ms, max_ms) -> KeylessResult<T>` (async)
- `meta::{load_sizes_cache, save_sizes_cache, update_saved_size, sizes_cache_path, plan_total_bytes, sizes_ttl_ms, backoff_config}`
- `hf::{keyless_cache_root, keyless_cache_repo_dir, get_local_model_size, delete_partial_file}`
- `catalog::list_openai_whisper_models() -> KeylessResult<Vec<String>>`
- `discover::discover_local_whisper_repos() -> Vec<String>`
- `workers::{emit_cached_sizes, fetch_sizes_now}` (for TUI integration)

## Running tests
- Unit (no network):
  - `cargo test -q -p keyless-models --no-run`
- Network tests (feature-gated):
  - `cargo test -q -p keyless-models --no-run --features net-tests`
  - Includes real GET/HEAD against `openai/whisper-tiny`
  - Tests Range resume (partial → full download)
  - Tests cancel mid-stream behavior

## License

MIT
