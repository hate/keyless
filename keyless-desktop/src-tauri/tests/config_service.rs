use keyless_core::config::{Config, OutputMode};
use keyless_desktop_lib::services::config::{ConfigService, ConfigServiceImpl};
use serde_json::json;

#[test]
fn update_changes_sink_to_clipboard() {
    let service = ConfigServiceImpl::new();
    let mut cfg = Config::default();

    let outcome = match service.update(&mut cfg, &json!({ "outputMode": "clipboard" })) {
        Ok(o) => o,
        Err(e) => panic!("unexpected error: {e}"),
    };

    assert!(outcome.sink_changed);
    assert!(matches!(cfg.output_mode, OutputMode::Clipboard));
}

#[test]
fn update_requires_file_path() {
    let service = ConfigServiceImpl::new();
    let mut cfg = Config::default();

    let err = match service.update(&mut cfg, &json!({ "outputMode": "file" })) {
        Ok(_) => panic!("expected error for missing file path"),
        Err(e) => e,
    };

    assert!(err.contains("file sink"), "unexpected error: {err}");
}

#[test]
fn update_marks_hotkey_change() {
    let service = ConfigServiceImpl::new();
    let mut cfg = Config {
        hotkey: "control+option".into(),
        ..Config::default()
    };

    let outcome = match service.update(&mut cfg, &json!({ "hotkey": "control+shift" })) {
        Ok(o) => o,
        Err(e) => panic!("unexpected error: {e}"),
    };

    assert!(outcome.hotkey_changed);
    assert_eq!(cfg.hotkey, "control+shift");
}

#[test]
fn update_marks_device_change() {
    let service = ConfigServiceImpl::new();
    let mut cfg = Config::default();

    let outcome = match service.update(&mut cfg, &json!({ "deviceName": "USB Mic" })) {
        Ok(o) => o,
        Err(e) => panic!("unexpected error: {e}"),
    };

    assert!(outcome.device_changed);
    assert_eq!(cfg.device_name.as_deref(), Some("USB Mic"));

    let outcome = match service.update(&mut cfg, &json!({ "deviceName": null })) {
        Ok(o) => o,
        Err(e) => panic!("unexpected error: {e}"),
    };

    assert!(outcome.device_changed);
    assert!(cfg.device_name.is_none());
}

#[test]
fn update_updates_vad_thresholds() {
    let service = ConfigServiceImpl::new();
    let mut cfg = Config::default();

    match service.update(
        &mut cfg,
        &json!({
            "vad": {
                "startDb": -40.0,
                "stopDb": -55.0,
                "minDurationMs": 400,
                "maxSilenceMs": 1200
            }
        }),
    ) {
        Ok(_) => {}
        Err(e) => panic!("unexpected error: {e}"),
    };

    assert_eq!(cfg.vad.start_db, -40.0);
    assert_eq!(cfg.vad.stop_db, -55.0);
    assert_eq!(cfg.vad.min_duration_ms, 400);
    assert_eq!(cfg.vad.max_silence_ms, 1200);
}

#[test]
fn update_updates_eq_settings() {
    let service = ConfigServiceImpl::new();
    let mut cfg = Config::default();

    match service.update(
        &mut cfg,
        &json!({
            "eq": {
                "bands": 48,
                "noiseReduction": 0.75,
                "windowDb": 40.0,
                "gamma": 1.6,
                "attack": 0.5,
                "decay": 0.25
            }
        }),
    ) {
        Ok(_) => {}
        Err(e) => panic!("unexpected error: {e}"),
    };

    assert_eq!(cfg.eq.bands, 48);
    assert!((cfg.eq.noise_reduction - 0.75).abs() < f32::EPSILON);
    assert_eq!(cfg.eq.window_db, 40.0);
    assert!((cfg.eq.gamma - 1.6).abs() < f32::EPSILON);
    assert!((cfg.eq.attack - 0.5).abs() < f32::EPSILON);
    assert!((cfg.eq.decay - 0.25).abs() < f32::EPSILON);
}
