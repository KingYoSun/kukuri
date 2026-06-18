#[test]
fn database_init_mode_defaults_to_require_ready() {
    let parsed = crate::DatabaseInitMode::parse("").expect("parse");
    assert_eq!(parsed, crate::DatabaseInitMode::RequireReady);
}

#[test]
fn database_init_mode_accepts_prepare() {
    let parsed = crate::DatabaseInitMode::parse("prepare").expect("parse");
    assert_eq!(parsed, crate::DatabaseInitMode::Prepare);
}

#[test]
fn auth_rollout_defaults_to_off() {
    let rollout = crate::AuthRolloutConfig::default();
    assert!(!rollout.requires_auth(chrono::Utc::now().timestamp()));
}

#[test]
fn jwt_secret_validation_accepts_strong_secret() {
    crate::config::validate_jwt_secret("0123456789abcdef0123456789abcdef")
        .expect("32-byte secret should be accepted");
}

#[test]
fn jwt_secret_validation_rejects_short_secret() {
    let error = crate::config::validate_jwt_secret("too-short-secret")
        .expect_err("secret shorter than the minimum should be rejected");
    assert!(
        error.to_string().contains("at least"),
        "unexpected error: {error}"
    );
}

#[test]
fn jwt_secret_validation_rejects_placeholder_value() {
    let error = crate::config::validate_jwt_secret("dev-jwt-change-me-32-bytes-minimum")
        .expect_err("placeholder secret should be rejected");
    assert!(
        error.to_string().contains("placeholder"),
        "unexpected error: {error}"
    );
}
