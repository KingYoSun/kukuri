use kukuri_cn_operator::{
    Capability, NodeRole, SAMPLE_CONFIG, build_manifest, check_drift, generate_all,
    load_and_validate, manifest_value, parse_config, resolve_and_validate,
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
fn report_endpoint_emitted_and_available_when_enabled() {
    // #370: report endpoint は実装済み（Phase A）。有効化すると manifest に絶対 URL を出力し、
    // available_enabled（planned ではなく）に入る。
    let yaml = base_config("  report_endpoint: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    let manifest = build_manifest(&resolved);
    assert_eq!(
        manifest.report_endpoint,
        "https://example-kukuri.net/v1/report"
    );

    let m = manifest_value(&resolved);
    let available = m["capability_scope"]["available_enabled"]
        .as_array()
        .unwrap();
    assert!(
        available.iter().any(|v| v == "report_endpoint"),
        "report_endpoint should be available, not planned"
    );
    let planned = m["capability_scope"]["planned_enabled"].as_array().unwrap();
    assert!(planned.iter().all(|v| v != "report_endpoint"));
}

#[test]
fn report_endpoint_absent_when_capability_disabled() {
    // report_endpoint を有効化しない node では空文字を出力し、client は abuse_contact 案内に切替。
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n";
    let resolved = load_and_validate(yaml).unwrap();
    assert_eq!(build_manifest(&resolved).report_endpoint, "");
}

#[test]
fn manifest_has_authority_scope_and_p2p_boundary() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let m = manifest_value(&resolved);

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
        "capability-risk-and-practices.md",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
}

#[test]
fn capability_risk_guide_covers_enabled_and_disabled() {
    // #359: enabled capability は実践ガイドとして、disabled capability は
    // 「引き受けていない責務」として記述される。個人運営を discourage しない。
    let yaml = base_config("  report_endpoint: true\n  analytics: false\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    let guide = doc(&generate_all(&resolved), "capability-risk-and-practices.md");

    // discourage しないトーンの明示。
    assert!(guide.contains("企業だけが担うものとは考えない"));
    // セクション構造。
    assert!(guide.contains("## 有効化している capability"));
    assert!(guide.contains("## 引き受けていない責務（無効な capability）"));
    // 有効化した report_endpoint の実践記述。
    assert!(guide.contains("通報エンドポイント"));
    assert!(guide.contains("authority scope:"));
    assert!(guide.contains("推奨対応:"));
    assert!(guide.contains("scope を狭める / 無効化:"));
    // 無効化した analytics は「引き受けていない責務」側に出る。
    let disabled_section = guide.split("引き受けていない責務").nth(1).unwrap();
    assert!(disabled_section.contains("アナリティクス"));
    // 法的免責が含まれる（header 経由）。
    assert!(guide.contains("法的助言ではありません"));
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

// --- #355: manifest authority scope / P2P boundary / node role ---

#[test]
fn typed_manifest_roundtrips_through_json() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let manifest = build_manifest(&resolved);
    let json = serde_json::to_string(&manifest).unwrap();
    let back: kukuri_cn_operator::CommunityNodeManifest = serde_json::from_str(&json).unwrap();
    // capabilities が型付きで往復できる。
    assert_eq!(
        back.capabilities.iroh_relay,
        manifest.capabilities.iroh_relay
    );
    assert_eq!(back.node_role, manifest.node_role);
}

#[test]
fn node_role_defaults_to_community_node() {
    let yaml = base_config("  iroh_relay: true\n  community_index: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    // 複数 capability を持つため community-node に推定される。
    assert_eq!(build_manifest(&resolved).node_role, NodeRole::CommunityNode);
}

#[test]
fn node_role_infers_relay_assist_for_relay_only() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                features:\n  iroh_relay: true\n";
    let resolved = load_and_validate(yaml).unwrap();
    assert_eq!(build_manifest(&resolved).node_role, NodeRole::RelayAssist);
}

#[test]
fn explicit_node_role_is_respected() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                manifest:\n  node_role: default-onboarding-node\n";
    let resolved = load_and_validate(yaml).unwrap();
    assert_eq!(
        build_manifest(&resolved).node_role,
        NodeRole::DefaultOnboardingNode
    );
}

#[test]
fn default_onboarding_node_distinguished_from_community_node() {
    let onboarding = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                      manifest:\n  node_role: default-onboarding-node\n";
    let community = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                     manifest:\n  node_role: community-node\n";
    let a = build_manifest(&load_and_validate(onboarding).unwrap()).node_role;
    let b = build_manifest(&load_and_validate(community).unwrap()).node_role;
    assert_ne!(a, b);
    assert_eq!(a, NodeRole::DefaultOnboardingNode);
}

#[test]
fn authority_scope_applies_to_derives_from_capabilities() {
    let yaml = base_config("  community_index: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    let m = build_manifest(&resolved);
    assert!(
        m.authority_scope
            .applies_to
            .contains(&"this_node".to_string())
    );
    assert!(
        m.authority_scope
            .applies_to
            .contains(&"communities_indexed_by_this_node".to_string())
    );
}

#[test]
fn operator_can_extend_applies_to() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                manifest:\n  authority_scope:\n    additional_applies_to:\n      - custom_scope\n";
    let resolved = load_and_validate(yaml).unwrap();
    let m = build_manifest(&resolved);
    assert!(
        m.authority_scope
            .applies_to
            .contains(&"custom_scope".to_string())
    );
}

#[test]
fn does_not_apply_to_has_safe_default() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n";
    let resolved = load_and_validate(yaml).unwrap();
    let m = build_manifest(&resolved);
    for expected in [
        "kukuri_network_as_a_whole",
        "user_identity",
        "user_profile_canonical_source",
        "user_social_graph_canonical_source",
        "third_party_nodes",
    ] {
        assert!(
            m.authority_scope
                .does_not_apply_to
                .contains(&expected.to_string()),
            "missing {expected}"
        );
    }
}

#[test]
fn p2p_boundary_is_all_false_invariant() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let b = build_manifest(&resolved).p2p_boundary;
    assert!(!b.identity_authority);
    assert!(!b.profile_canonical_store);
    assert!(!b.social_graph_canonical_store);
    assert!(!b.content_truth_source);
    assert!(!b.network_wide_authority);
}

#[test]
fn generated_docs_reflect_authority_scope() {
    let resolved = load_and_validate(SAMPLE_CONFIG).unwrap();
    let diagram = doc(&generate_all(&resolved), "network-diagram.md");
    assert!(diagram.contains("authority scope"));
    assert!(diagram.contains("does_not_apply_to"));
    assert!(diagram.contains("network-wide authority: false"));
}
