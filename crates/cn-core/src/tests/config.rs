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
