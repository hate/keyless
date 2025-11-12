use keyless_desktop_lib::services::permissions::{PermissionsService, PermissionsServiceImpl};

#[test]
fn check_returns_expected_shape() {
    let svc = PermissionsServiceImpl::new();
    let json = match svc.check() {
        Ok(v) => v,
        Err(e) => panic!("permissions check failed: {e}"),
    };
    assert!(json.get("microphone").is_some());
    assert!(json.get("accessibility").is_some());
    assert!(json.get("needsOnboarding").is_some());
    assert!(json["microphone"].is_boolean());
    assert!(json["accessibility"].is_boolean());
    assert!(json["needsOnboarding"].is_boolean());
}
