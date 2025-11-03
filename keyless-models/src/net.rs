//! HTTP utilities (blocking and async) with sane timeouts and backoff.

use keyless_core::error::{KeylessError, KeylessResult};
use reqwest::header::{AUTHORIZATION, CONTENT_LENGTH, HeaderName, HeaderValue};

/// Build an async reqwest client (10s connect, 60s total timeout).
pub fn build_async_client() -> KeylessResult<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| KeylessError::Config(format!("http client: {}", e)))
}

/// Build a blocking reqwest client (10s connect, 60s total timeout).
pub fn build_blocking_client() -> KeylessResult<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| KeylessError::Config(format!("http client: {}", e)))
}

/// Optional HF auth header from env vars `HF_TOKEN` or `HUGGINGFACE_HUB_TOKEN`.
pub fn auth_header() -> Option<(HeaderName, HeaderValue)> {
    if let Ok(tok) = std::env::var("HF_TOKEN").or_else(|_| std::env::var("HUGGINGFACE_HUB_TOKEN"))
        && !tok.is_empty()
        && let Ok(v) = HeaderValue::from_str(&format!("Bearer {}", tok))
    {
        return Some((AUTHORIZATION, v));
    }
    None
}

/// Resolve a Hugging Face URL for a given model file.
pub fn hf_resolve_url(model_id: &str, file: &str) -> String {
    format!("https://huggingface.co/{}/resolve/main/{}", model_id, file)
}

/// Perform a blocking HEAD and return the content length (if any).
pub fn head_content_length(url: &str, auth: &Option<(HeaderName, HeaderValue)>) -> Option<u64> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .ok()?;
    let mut req = client.head(url);
    if let Some((h, v)) = auth.as_ref() {
        req = req.header(h.clone(), v.clone());
    }
    let resp = req.send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.headers()
        .get(CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

/// HEAD with exponential backoff using a provided blocking client.
pub fn head_len_with_backoff_with_client(
    client: &reqwest::blocking::Client,
    url: &str,
    auth: &Option<(HeaderName, HeaderValue)>,
    attempts: usize,
    initial_ms: u64,
    max_ms: u64,
) -> Option<u64> {
    let mut delay = initial_ms;
    for _ in 0..attempts {
        let mut req = client.head(url);
        if let Some((h, v)) = auth.as_ref() {
            req = req.header(h.clone(), v.clone());
        }
        if let Ok(resp) = req.send()
            && resp.status().is_success()
            && let Some(n) = resp
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
        {
            return Some(n);
        }
        std::thread::sleep(std::time::Duration::from_millis(delay));
        delay = (delay * 2).min(max_ms);
    }
    None
}

/// HEAD with exponential backoff (builds its own blocking client).
pub fn head_len_with_backoff(
    url: &str,
    auth: &Option<(HeaderName, HeaderValue)>,
    attempts: usize,
    initial_ms: u64,
    max_ms: u64,
) -> Option<u64> {
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .ok()?;
    head_len_with_backoff_with_client(&client, url, auth, attempts, initial_ms, max_ms)
}

/// Retry an async operation with exponential backoff.
pub async fn retry_with_backoff<F, Fut, T>(
    mut op: F,
    attempts: usize,
    initial_ms: u64,
    max_ms: u64,
) -> KeylessResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = KeylessResult<T>>,
{
    let mut delay = initial_ms;
    let mut last_err: Option<KeylessError> = None;
    for _ in 0..attempts {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                delay = (delay * 2).min(max_ms);
            }
        }
    }
    Err(last_err
        .unwrap_or_else(|| KeylessError::Config("retry_with_backoff: unknown error".to_string())))
}

/// Blocking GET with exponential backoff.
///
/// Returns `(body_bytes, content_length_header)`.
pub fn blocking_get_with_backoff(
    client: &reqwest::blocking::Client,
    url: &str,
    auth: &Option<(reqwest::header::HeaderName, reqwest::header::HeaderValue)>,
    attempts: usize,
    initial_ms: u64,
    max_ms: u64,
) -> KeylessResult<(Vec<u8>, Option<u64>)> {
    let mut delay = initial_ms;
    let mut last_err: Option<KeylessError> = None;
    for _ in 0..attempts {
        let mut req = client.get(url);
        if let Some((h, v)) = auth.as_ref() {
            req = req.header(h.clone(), v.clone());
        }
        match req.send() {
            Ok(resp) => match resp.error_for_status() {
                Ok(mut ok) => {
                    let len = ok.content_length();
                    let mut buf = Vec::new();
                    use std::io::Read;
                    match ok.read_to_end(&mut buf) {
                        Ok(_) => return Ok((buf, len)),
                        Err(e) => {
                            last_err = Some(KeylessError::Config(format!("read {}: {}", url, e)));
                        }
                    }
                }
                Err(e) => {
                    last_err = Some(KeylessError::Config(e.to_string()));
                }
            },
            Err(e) => {
                last_err = Some(KeylessError::Config(format!("GET {}: {}", url, e)));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(delay));
        delay = (delay * 2).min(max_ms);
    }
    Err(last_err
        .unwrap_or_else(|| KeylessError::Config("blocking_get_with_backoff: failed".to_string())))
}
