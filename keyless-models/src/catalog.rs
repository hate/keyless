//! Simple catalog lookup via Hugging Face REST API.

use keyless_core::error::{KeylessError, KeylessResult};

/// Fetch list of public OpenAI Whisper models via Hugging Face REST API.
/// Returns sorted unique model ids, e.g., "openai/whisper-tiny", "openai/whisper-tiny.en".
/// List public OpenAI Whisper model IDs from Hugging Face.
pub fn list_openai_whisper_models() -> KeylessResult<Vec<String>> {
    let url = "https://huggingface.co/api/models?search=openai/whisper";
    // Use blocking client (synchronous API; this function is not async).
    // Limit redirects to 10 to prevent infinite loops (safety measure).
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| KeylessError::Config(format!("http client: {}", e)))?;
    let mut req = client.get(url);
    // Add auth header if available (HF token from env); works without token for public models.
    if let Some((h, v)) = crate::net::auth_header() {
        req = req.header(h, v);
    }
    // Send request and check HTTP status (4xx/5xx treated as errors).
    let resp = req
        .send()
        .map_err(|e| KeylessError::Config(format!("GET {}: {}", url, e)))?
        .error_for_status()
        .map_err(|e| KeylessError::Config(e.to_string()))?;
    // Read response body as UTF-8 string (assumes HF API returns UTF-8).
    let text = resp
        .text()
        .map_err(|e| KeylessError::Config(format!("read response: {}", e)))?;
    // Parse JSON into generic Value (flexible parsing; don't assume exact schema).
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| KeylessError::Config(format!("parse json: {}", e)))?;
    let mut out: Vec<String> = Vec::new();
    // Expect response to be array of model objects (handle non-array gracefully).
    if let Some(arr) = v.as_array() {
        for item in arr {
            // Extract "modelId" field, filter to only OpenAI Whisper models (prefix check).
            // and_then chains: get("modelId") → as_str() → Some(id) if all succeed.
            if let Some(id) = item.get("modelId").and_then(|s| s.as_str())
                && id.starts_with("openai/whisper")
            {
                out.push(id.to_string());
            }
        }
    }
    // Sort for deterministic output (alphabetical order).
    out.sort();
    // Remove duplicates (same model ID may appear multiple times in HF response).
    out.dedup();
    Ok(out)
}
