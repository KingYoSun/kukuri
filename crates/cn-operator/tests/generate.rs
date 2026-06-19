use kukuri_cn_operator::{
    Capability, SAMPLE_CONFIG, build_manifest, check_drift, generate_all, load_and_validate,
    parse_config, resolve_and_validate,
};

fn base_config(extra_features: &str, ack: bool) -> String {
    format!(
        "server:\n\
         \x20 domain: example-kukuri.net\n\
         \x20 operator_name: Example Operator\n\
         \x20 country: JP\n\
         \x20 cloud_provider: AWS\n\
         \x20 region: ap-northeast-1\n\
         features:\n{extra_features}\
         retention:\n\
         \x20 connection_logs_days: 30\n\
         \x20 moderation_logs_days: 180\n\
         acknowledge_planned_capabilities: {ack}\n"
    )
}

#[test]
fn sample_config_is_valid() {
    let resolved = load_and_validate(SAMPLE_CONFIG).expect("sample config must validate");
    assert!(resolved.enabled(Capability::IrohRelay));
    assert!(
        resolved.enabled(Capability::AuthConsent),
        "auth_consent is baseline"
    );
}

#[test]
fn profiles_are_defined() {
    for key in ["minimal", "relay-enabled", "full-service"] {
        let yaml = format!(
            "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
             profile: {key}\nacknowledge_planned_capabilities: true\n"
        );
        let resolved = load_and_validate(&yaml).expect("profile config validates");
        assert!(resolved.enabled(Capability::BootstrapAssist));
    }
}

#[test]
fn relay_enabled_profile_turns_on_relay() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                profile: relay-enabled\nacknowledge_planned_capabilities: true\n";
    let resolved = load_and_validate(yaml).unwrap();
    assert!(resolved.enabled(Capability::IrohRelay));
    assert!(resolved.enabled(Capability::TrafficRelayFallback));
}

#[test]
fn explicit_feature_overrides_profile() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                profile: relay-enabled\nfeatures:\n  iroh_relay: false\n\
                acknowledge_planned_capabilities: true\n";
    let resolved = load_and_validate(yaml).unwrap();
    assert!(!resolved.enabled(Capability::IrohRelay));
}

#[test]
fn planned_capability_without_ack_fails() {
    let yaml = base_config("  moderation: true\n", false);
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("acknowledge_planned_capabilities"),
        "error should explain the Phase B ack guard: {err}"
    );
}

#[test]
fn planned_capability_with_ack_validates() {
    let yaml = base_config("  moderation: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    assert!(resolved.enabled(Capability::Moderation));
    assert_eq!(
        resolved.enabled_planned_capabilities(),
        vec![Capability::Moderation]
    );
}

#[test]
fn unknown_feature_key_is_rejected() {
    let yaml = base_config("  not_a_real_feature: true\n", true);
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(err.to_string().contains("未知のキー"), "got: {err}");
}

#[test]
fn missing_required_fields_fail() {
    let yaml = "server:\n  domain: \"\"\n  operator_name: Op\n  country: JP\n";
    assert!(load_and_validate(yaml).is_err());
}

#[test]
fn manifest_has_authority_scope_and_p2p_boundary() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let m = build_manifest(&resolved);

    // P2P boundary は identity / profile / social graph / network authority を false 宣言。
    let boundary = &m["p2p_boundary"];
    assert_eq!(boundary["identity_authority"], false);
    assert_eq!(boundary["profile_canonical_store"], false);
    assert_eq!(boundary["social_graph_canonical_store"], false);
    assert_eq!(boundary["content_truth_source"], false);
    assert_eq!(boundary["network_wide_authority"], false);

    // authority scope の does_not_apply_to に user identity 等が含まれる。
    let does_not = m["authority_scope"]["does_not_apply_to"]
        .as_array()
        .unwrap();
    assert!(does_not.iter().any(|v| v == "user_identity"));
    assert!(does_not.iter().any(|v| v == "kukuri_network_as_a_whole"));

    // capability_scope は available と planned を分離する。
    let scope = &m["capability_scope"];
    assert!(scope["available_enabled"].is_array());
    assert!(scope["planned_enabled"].is_array());
    let planned = scope["planned_enabled"].as_array().unwrap();
    assert!(planned.iter().any(|v| v == "moderation"));
}

#[test]
fn all_expected_docs_are_generated() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let files = generate_all(&resolved);
    let names: Vec<&str> = files.iter().map(|f| f.filename.as_str()).collect();
    for expected in [
        "server-manifest.json",
        "network-diagram.md",
        "telecom-notification-draft.md",
        "service-description-draft.md",
        "terms.md",
        "privacy-policy.md",
        "external-transmission-notice.md",
        "abuse-policy.md",
        "moderation-policy.md",
        "data-retention-policy.md",
        "prior-consultation-email.md",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
}

#[test]
fn generated_docs_contain_legal_disclaimer() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    for file in generate_all(&resolved) {
        if file.filename.ends_with(".md") {
            assert!(
                file.content.contains("法的助言ではありません"),
                "{} should contain legal disclaimer",
                file.filename
            );
        }
    }
}

fn doc(files: &[kukuri_cn_operator::GeneratedFile], name: &str) -> String {
    files
        .iter()
        .find(|f| f.filename == name)
        .unwrap_or_else(|| panic!("missing {name}"))
        .content
        .clone()
}

#[test]
fn relay_enabled_explains_encrypted_traffic_fallback() {
    let yaml = base_config("  iroh_relay: true\n  traffic_relay_fallback: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    let files = generate_all(&resolved);
    let telecom = doc(&files, "telecom-notification-draft.md");
    assert!(telecom.contains("暗号化済み"));
    let ext = doc(&files, "external-transmission-notice.md");
    assert!(ext.contains("relay"));
}

#[test]
fn analytics_disabled_omits_analytics_destination() {
    let yaml = base_config("  analytics: false\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    let ext = doc(&generate_all(&resolved), "external-transmission-notice.md");
    // 「現在の外部送信先」セクションにアナリティクスが運用中として出ないこと。
    let active_section = ext.split("送信していない").next().unwrap();
    assert!(!active_section.contains("### アナリティクスプロバイダ"));
    // 無効として明示はされる。
    assert!(ext.contains("アナリティクスプロバイダ: 該当機能が無効"));
}

#[test]
fn cloudflare_enabled_emits_external_transmission() {
    let yaml = base_config("  cloudflare_proxy: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    let ext = doc(&generate_all(&resolved), "external-transmission-notice.md");
    let active_section = ext.split("送信していない").next().unwrap();
    assert!(active_section.contains("Cloudflare"));
}

#[test]
fn planned_capability_marked_as_planned_not_operating() {
    let yaml = base_config("  moderation: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    let svc = doc(&generate_all(&resolved), "service-description-draft.md");
    assert!(svc.contains("計画中（この配布物では未提供）"));
    // moderation は「運用中の補助機能」セクションに出ない（capability が Planned のため）。
    let operating_section = svc.split("計画中").next().unwrap();
    assert!(!operating_section.contains("### モデレーション"));
}

#[test]
fn output_is_deterministic() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let first = generate_all(&resolved);
    let second = generate_all(&resolved);
    assert_eq!(first, second);
}

#[test]
fn drift_check_detects_changes_and_clean() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let dir = tempfile::tempdir().unwrap();

    // 生成前は missing。
    let report = check_drift(&resolved, dir.path()).unwrap();
    assert!(!report.is_clean());
    assert!(!report.missing.is_empty());

    // 生成後は clean。
    for file in generate_all(&resolved) {
        std::fs::write(dir.path().join(&file.filename), &file.content).unwrap();
    }
    let report = check_drift(&resolved, dir.path()).unwrap();
    assert!(report.is_clean(), "{}", report.summary());

    // 改変すると changed 検出。
    std::fs::write(dir.path().join("terms.md"), "tampered").unwrap();
    let report = check_drift(&resolved, dir.path()).unwrap();
    assert!(report.changed.contains(&"terms.md".to_string()));
}

#[test]
fn parse_then_resolve_roundtrip() {
    let cfg = parse_config(SAMPLE_CONFIG).unwrap();
    assert_eq!(cfg.server.country, "JP");
    let resolved = resolve_and_validate(cfg).unwrap();
    assert!(resolved.enabled(Capability::CloudflareProxy));
}
