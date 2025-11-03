#[cfg(feature = "net-tests")]
#[test]
fn resume_from_partial_with_range() {
    let url = keyless_models::net::hf_resolve_url("openai/whisper-tiny.en", "model.safetensors");
    let len = keyless_models::net::head_len_with_backoff(&url, &None, 3, 200, 2_000)
        .expect("content-length");
    let dir = tempfile::tempdir().expect("tmp");
    let tmp = dir.path().join("model.safetensors.partial");

    // Write first slice [0..offset)
    let offset = (len / 10).max(1);
    let client = reqwest::blocking::Client::builder()
        .build()
        .expect("client");
    let req1 = client
        .get(&url)
        .header(reqwest::header::RANGE, format!("bytes=0-{}", offset - 1));
    let mut resp1 = req1
        .send()
        .expect("send1")
        .error_for_status()
        .expect("status1");
    assert_eq!(resp1.status(), reqwest::StatusCode::PARTIAL_CONTENT);
    let mut f = std::fs::File::create(&tmp).expect("create tmp");
    std::io::copy(&mut resp1, &mut f).expect("copy1");
    let sz1 = std::fs::metadata(&tmp).expect("meta1").len();
    assert_eq!(sz1, offset);

    // Resume [offset..)
    let req2 = client
        .get(&url)
        .header(reqwest::header::RANGE, format!("bytes={}-", offset));
    let mut resp2 = req2
        .send()
        .expect("send2")
        .error_for_status()
        .expect("status2");
    assert_eq!(resp2.status(), reqwest::StatusCode::PARTIAL_CONTENT);
    let mut f2 = std::fs::OpenOptions::new()
        .append(true)
        .open(&tmp)
        .expect("open append");
    std::io::copy(&mut resp2, &mut f2).expect("copy2");
    let final_sz = std::fs::metadata(&tmp).expect("meta2").len();
    assert_eq!(final_sz, len);
}

#[cfg(feature = "net-tests")]
#[test]
fn cancel_mid_stream_no_final_file() {
    // We simulate cancellation by breaking after first chunk write and verifying
    // that no final file (renamed) exists; only the partial remains.
    let url = keyless_models::net::hf_resolve_url("openai/whisper-tiny.en", "model.safetensors");
    let dir = tempfile::tempdir().expect("tmp");
    let weights = dir.path().join("model.safetensors");
    let partial = dir.path().join("model.safetensors.partial");

    let client = reqwest::blocking::Client::builder()
        .build()
        .expect("client");
    let req = client.get(&url);
    let mut resp = req
        .send()
        .expect("send")
        .error_for_status()
        .expect("status");
    let mut f = std::fs::File::create(&partial).expect("create partial");
    // Write first small chunk then simulate cancel by stopping early.
    use std::io::Read;
    let mut buf = vec![0u8; 1024 * 32];
    let n = resp.read(&mut buf).expect("read");
    assert!(n > 0);
    std::io::Write::write_all(&mut f, &buf[..n]).expect("write");
    drop(f);
    // Cancel: do not rename to final file
    assert!(partial.exists());
    assert!(!weights.exists());
    // Cleanup
    std::fs::remove_file(&partial).expect("rm partial");
}
