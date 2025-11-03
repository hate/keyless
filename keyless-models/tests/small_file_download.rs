#[cfg(feature = "net-tests")]
#[test]
fn download_config_json_to_temp_and_verify_size() {
    let url = keyless_models::net::hf_resolve_url("openai/whisper-tiny.en", "config.json");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");

    let (bytes, len_hdr) = rt.block_on(async move {
        let client = keyless_models::net::build_async_client().expect("client");
        let auth = keyless_models::net::auth_header();
        keyless_models::net::retry_with_backoff(
            || {
                let mut req = client.get(url.clone());
                if let Some((h, v)) = &auth {
                    req = req.header(h.clone(), v.clone());
                }
                let url_for_err = url.clone();
                async move {
                    let resp = req
                        .send()
                        .await
                        .map_err(|e| format!("GET {}: {}", url_for_err, e))?
                        .error_for_status()
                        .map_err(|e| format!("{}", e))?;
                    let len = resp.content_length();
                    let b = resp
                        .bytes()
                        .await
                        .map_err(|e| format!("read {}: {}", url_for_err, e))?
                        .to_vec();
                    Ok::<(Vec<u8>, Option<u64>), String>((b, len))
                }
            },
            3,
            200,
            2_000,
        )
        .await
        .expect("get bytes")
    });

    let dir = tempfile::tempdir().expect("tmp");
    let dst = dir.path().join("config.json");
    std::fs::write(&dst, &bytes).expect("write");
    let sz = std::fs::metadata(&dst).expect("meta").len();
    if let Some(cl) = len_hdr {
        assert_eq!(sz, cl, "size must match content-length");
    }

    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(
        v.get("model_type").and_then(|s| s.as_str()),
        Some("whisper")
    );
}
