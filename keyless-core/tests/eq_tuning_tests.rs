use keyless_core::config::EqTuning;

#[test]
fn default_validates() {
    let t = EqTuning::default();
    assert!(t.validate().is_ok());
}

#[test]
fn bands_out_of_range_fails() {
    let t = EqTuning {
        bands: 8,
        ..EqTuning::default()
    };
    assert!(t.validate().is_err());
    let t = EqTuning {
        bands: 256,
        ..EqTuning::default()
    };
    assert!(t.validate().is_err());
}

#[test]
fn floats_out_of_range_fail() {
    assert!(
        EqTuning {
            noise_reduction: -0.1,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            noise_reduction: 1.1,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            window_db: 5.0,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            window_db: 90.0,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            gamma: 0.7,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            gamma: 2.1,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            attack: 0.01,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            attack: 2.0,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            decay: 0.01,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        EqTuning {
            decay: 2.0,
            ..EqTuning::default()
        }
        .validate()
        .is_err()
    );
}
