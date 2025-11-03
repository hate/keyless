#[cfg(feature = "net-tests")]
#[test]
fn fetch_whisper_tiny_config_metadata() {
    let url = keyless_models::net::hf_resolve_url("openai/whisper-tiny.en", "config.json");
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect("client");
    let mut req = client.get(&url);
    if let Some((h, v)) = keyless_models::net::auth_header() {
        req = req.header(h, v);
    }
    let bytes = req
        .send()
        .expect("send")
        .error_for_status()
        .expect("status")
        .bytes()
        .expect("bytes");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(
        v.get("model_type").and_then(|s| s.as_str()),
        Some("whisper")
    );
}

#[cfg(feature = "net-tests")]
#[test]
fn head_model_weights_reports_reasonable_size() {
    let url = keyless_models::net::hf_resolve_url("openai/whisper-tiny.en", "model.safetensors");
    let len = keyless_models::net::head_len_with_backoff(&url, &None, 3, 200, 2_000)
        .expect("content-length");
    assert!(len > 1_000_000, "len={}", len);
    assert!(len < 2_000_000_000, "len={}", len);
}

#[cfg(feature = "net-tests")]
#[test]
fn list_openai_whisper_models_contains_tiny() {
    let list = keyless_models::catalog::list_openai_whisper_models().expect("list");
    assert!(list.iter().any(|s| s == "openai/whisper-tiny"));
    assert!(list.iter().any(|s| s == "openai/whisper-tiny.en"));
}
