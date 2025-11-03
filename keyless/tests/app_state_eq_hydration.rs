use keyless_core::config::{Config, EqTuning, storage};

#[test]
fn app_state_hydrates_eq_from_config() -> Result<(), Box<dyn std::error::Error>> {
    // write config with custom eq
    let c = Config {
        eq: EqTuning {
            bands: 96,
            noise_reduction: 0.55,
            window_db: 60.0,
            gamma: 1.5,
            attack: 0.40,
            decay: 0.20,
        },
        ..Default::default()
    };
    storage::save_config(&c)?;

    // build app state and assert hydration
    let app = keyless::tui::state::AppState::new(None);
    assert_eq!(app.eq_tuning.bands, 96);
    assert!((app.eq_tuning.noise_reduction - 0.55).abs() < f32::EPSILON);
    assert!((app.eq_tuning.window_db - 60.0).abs() < f32::EPSILON);
    assert!((app.eq_tuning.gamma - 1.5).abs() < f32::EPSILON);
    assert!((app.eq_tuning.attack - 0.40).abs() < f32::EPSILON);
    assert!((app.eq_tuning.decay - 0.20).abs() < f32::EPSILON);
    Ok(())
}
