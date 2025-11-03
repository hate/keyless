//! Simple catalog lookup via Hugging Face REST API.

use keyless_core::error::{KeylessError, KeylessResult};

/// Fetch list of public OpenAI Whisper models via Hugging Face REST API.
/// Returns sorted unique model ids, e.g., "openai/whisper-tiny", "openai/whisper-tiny.en".
/// List public OpenAI Whisper model IDs from Hugging Face.
pub fn list_openai_whisper_models() -> KeylessResult<Vec<String>> {
    let url = "https://huggingface.co/api/models?search=openai/whisper";
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| KeylessError::Config(format!("http client: {}", e)))?;
    let mut req = client.get(url);
    if let Some((h, v)) = crate::net::auth_header() {
        req = req.header(h, v);
    }
    let resp = req
        .send()
        .map_err(|e| KeylessError::Config(format!("GET {}: {}", url, e)))?
        .error_for_status()
        .map_err(|e| KeylessError::Config(e.to_string()))?;
    let text = resp
        .text()
        .map_err(|e| KeylessError::Config(format!("read response: {}", e)))?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| KeylessError::Config(format!("parse json: {}", e)))?;
    let mut out: Vec<String> = Vec::new();
    if let Some(arr) = v.as_array() {
        for item in arr {
            if let Some(id) = item.get("modelId").and_then(|s| s.as_str())
                && id.starts_with("openai/whisper")
            {
                out.push(id.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}
