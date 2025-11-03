use std::collections::HashMap;

#[test]
fn sizes_cache_roundtrip_with_override() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    unsafe {
        std::env::set_var("KEYLESS_CACHE_DIR", dir.path());
    }

    let mut map = HashMap::new();
    map.insert("openai/whisper-tiny".to_string(), 123u64);
    keyless_models::meta::save_sizes_cache(&map);

    let (loaded, ts) = keyless_models::meta::load_sizes_cache();
    assert!(ts > 0, "timestamp written");
    assert_eq!(loaded.get("openai/whisper-tiny"), Some(&123u64));

    let p = keyless_models::meta::sizes_cache_path();
    assert!(p.starts_with(dir.path()));
    assert!(std::fs::metadata(p).is_ok());

    unsafe {
        std::env::remove_var("KEYLESS_CACHE_DIR");
    }
    Ok(())
}
