#[test]
fn auth_header_token_alias() {
    unsafe {
        std::env::remove_var("HF_TOKEN");
    }
    unsafe {
        std::env::set_var("HUGGINGFACE_HUB_TOKEN", "abc123");
    }
    let hdr = keyless_models::net::auth_header();
    assert!(hdr.is_some());
    unsafe {
        std::env::remove_var("HUGGINGFACE_HUB_TOKEN");
    }
}

#[test]
fn hf_resolve_url_shape() {
    let url = keyless_models::net::hf_resolve_url("openai/whisper-tiny.en", "config.json");
    assert!(url.starts_with("https://huggingface.co/openai/whisper-tiny.en/resolve/main/"));
    assert!(url.ends_with("/config.json"));
}

#[test]
fn head_backoff_invalid_url_returns_none_quickly() {
    let url = "https://invalid.invalid/notfound";
    let t0 = std::time::Instant::now();
    let len = keyless_models::net::head_len_with_backoff(url, &None, 1, 50, 100);
    let elapsed_ms = t0.elapsed().as_millis();
    assert!(len.is_none());
    assert!(elapsed_ms < 2_000, "elapsed {}ms", elapsed_ms);
}
