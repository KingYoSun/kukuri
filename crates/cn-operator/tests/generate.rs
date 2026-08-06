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
fn promoted_capability_validates_without_ack() {
    // #617: index / moderation / local trust は提供中へ昇格済み。承認フラグ無しで有効化できる。
    let yaml = base_config("  moderation: true\n", false);
    let resolved = load_and_validate(&yaml).unwrap();
    assert!(resolved.enabled(Capability::Moderation));
    assert!(resolved.enabled_planned_capabilities().is_empty());
}

#[test]
fn acknowledge_flag_remains_accepted_for_compatibility() {
    // 既存 config の後方互換: 承認フラグが設定されていてもエラーにしない。
    let yaml = base_config("  moderation: true\n", true);
    let resolved = load_and_validate(&yaml).unwrap();
    assert!(resolved.enabled(Capability::Moderation));
    assert!(resolved.enabled_planned_capabilities().is_empty());
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

    // capability_scope は available と planned を分離する。#617 の昇格後、moderation は
    // available 側に入り、planned は空になる。
    let scope = &m["capability_scope"];
    assert!(scope["available_enabled"].is_array());
    assert!(scope["planned_enabled"].is_array());
    let available = scope["available_enabled"].as_array().unwrap();
    assert!(available.iter().any(|v| v == "moderation"));
    assert!(scope["planned_enabled"].as_array().unwrap().is_empty());
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

/// safety providers 付きの config（外部送信の動的開示の検証用）。
/// `vlm_hosting_line` は general provider 配下の行（例: `"      hosting: self_host\n"`）か空。
fn config_with_safety_providers(vlm_hosting_line: &str) -> String {
    let base = r#"server:
  domain: example-kukuri.net
  operator_name: Example Operator
  country: JP
features:
  moderation: true
retention:
  connection_logs_days: 30
  moderation_logs_days: 180
safety:
  profile: public-node
  policy_version: 2026-06-public-node-v1
  providers:
    known_csam:
      provider: project-arachnid-shield
      required: true
      credential_secret_id: kukuri-cn-safety-known-csam
    general:
      provider: openai-compatible-vlm
"#;
    format!("{base}{vlm_hosting_line}")
}

#[test]
fn safety_providers_surface_in_external_transmission_notice() {
    // 自前ホスト宣言: 視覚言語モデルは運営者管理基盤、Arachnid は第三者への外部送信。
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let ext = doc(&generate_all(&resolved), "external-transmission-notice.md");
    assert!(ext.contains("安全性走査プロバイダへの送信"));
    assert!(ext.contains("Project Arachnid Shield"));
    assert!(ext.contains("第三者への外部送信"));
    assert!(ext.contains("運営者が管理する視覚言語モデル基盤"));
    assert!(ext.contains("第三者への外部送信ではない"));

    // 外部 API 宣言: 視覚言語モデルも第三者への外部送信として表示される。
    let yaml = config_with_safety_providers("      hosting: external\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let ext = doc(&generate_all(&resolved), "external-transmission-notice.md");
    assert!(ext.contains("外部の視覚言語モデル API"));
    assert!(!ext.contains("運営者が管理する視覚言語モデル基盤"));

    // 未指定は保守側（第三者への外部送信）として扱う。
    let yaml = config_with_safety_providers("");
    let resolved = load_and_validate(&yaml).unwrap();
    let ext = doc(&generate_all(&resolved), "external-transmission-notice.md");
    assert!(ext.contains("外部の視覚言語モデル API"));
}

#[test]
fn moderation_policy_describes_scan_flow_and_appeals() {
    // #617 T6: moderation-policy が「未提供」ではなく走査の流れ・申し立て導線を説明する。
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let policy = doc(&generate_all(&resolved), "moderation-policy.md");
    for needle in [
        "走査と判定の流れ",
        "fail-closed",
        "視覚言語モデル",
        "Match Data",
        "申し立て",
    ] {
        assert!(policy.contains(needle), "missing: {needle}");
    }
    assert!(!policy.contains("未提供"));
}

#[test]
fn generated_docs_contain_no_planned_wording_after_promotion() {
    // #617 T6: 昇格後、全生成物から「計画中」表記が消えている（分離セクション含む）。
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    for file in generate_all(&resolved) {
        assert!(
            !file.content.contains("計画中"),
            "{}: planned wording must not remain",
            file.filename
        );
    }
}

#[test]
fn network_diagram_shows_index_stack_data_flow() {
    // #617 T5: 索引系が有効な node の構成図に、実データフロー（3 ブロック）と境界説明が載る。
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let diagram = doc(&generate_all(&resolved), "network-diagram.md");
    for needle in [
        "構成要素とデータフロー",
        "利用者端末 / 他ピア",
        "Direct P2P",
        "cn-user-api",
        "cn-indexer",
        "Postgres",
        "Valkey",
        "ArcadeDB",
        "関係解析の定期実行",
        "iroh docs / blob ピア",
        "Project Arachnid Shield",
        "運営者が管理する視覚言語モデル基盤",
        "サポート対象（公開トピック）内に",
        "恒久保存しない",
    ] {
        assert!(diagram.contains(needle), "missing: {needle}");
    }

    // 索引系が無効な node には実データフロー節を出さない（過大表示の防止）。
    let yaml = base_config("", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let diagram = doc(&generate_all(&resolved), "network-diagram.md");
    assert!(!diagram.contains("構成要素とデータフロー"));
}

#[test]
fn telecom_notification_carries_service_name_and_server() {
    // 届出様式への転記元: サービス名と使用サーバーの行を持つ。
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let telecom = doc(&generate_all(&resolved), "telecom-notification-draft.md");
    assert!(telecom.contains("提供するサービス: P2P コミュニケーションネットワークの補助サービス"));
    // fixture に cloud_provider が無い場合は行ごと出ない（誤記入の防止）。
    assert!(!telecom.contains("使用するサーバー:"));

    let with_cloud = yaml.replace(
        "  country: JP\n",
        "  country: JP\n  cloud_provider: Google Cloud\n",
    );
    let resolved = load_and_validate(&with_cloud).unwrap();
    let telecom = doc(&generate_all(&resolved), "telecom-notification-draft.md");
    assert!(telecom.contains("使用するサーバー: Google Cloud"));
    let diagram = doc(&generate_all(&resolved), "network-diagram.md");
    assert!(diagram.contains("使用するサーバー: Google Cloud"));
}

#[test]
fn data_retention_lists_storage_classes_for_index_stack() {
    // #617 T4: 索引・モデレーション・信頼の系統が有効な node では、データ区分と保存先・
    // 再構築/バックアップ区分が保持ポリシーへ載る。
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let retention = doc(&generate_all(&resolved), "data-retention-policy.md");
    assert!(retention.contains("データ区分と保存先"));
    for needle in [
        "Postgres",
        "ArcadeDB",
        "Valkey",
        "恒久保存しない",
        "再構築可能",
        "バックアップ対象は Postgres のみ",
        "canonical store ではない",
    ] {
        assert!(retention.contains(needle), "missing: {needle}");
    }

    // 系統が無効な node には索引系の保存先区分を書かない（誤開示防止）。
    let yaml = base_config("", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let retention = doc(&generate_all(&resolved), "data-retention-policy.md");
    assert!(!retention.contains("データ区分と保存先"));
}

#[test]
fn generated_docs_never_contain_private_endpoints_or_secret_ids_values() {
    // 公開資料の非含有監査: URL・secret 値らしき文字列が生成物に出ないこと。
    // （secret は ID のみ config に書かれ、値はそもそも config に無い。ここでは
    //   接続先アドレスの類が漏れないことを固定する）
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    for file in generate_all(&resolved) {
        for needle in ["http://10.", "http://192.168.", "wireguard", "WireGuard"] {
            assert!(
                !file.content.contains(needle),
                "{}: must not leak private endpoints: {needle}",
                file.filename
            );
        }
    }
}

#[test]
fn promoted_capability_metadata_describes_implemented_behavior() {
    // #617 T2: 昇格した 3 capability の説明が「（計画）」ではなく実装済みのデータフローを
    // 記述していることを固定する（開示文書の生成元となる契約）。
    for cap in [
        Capability::CommunityIndex,
        Capability::Moderation,
        Capability::CommunityLocalTrust,
    ] {
        let meta = cap.meta();
        for text in [
            meta.handled_data,
            meta.purpose,
            meta.retention_impact,
            meta.telecom_note,
            meta.privacy_note,
            meta.terms_note,
        ] {
            assert!(
                !text.contains("計画"),
                "{cap}: metadata must not read as planned: {text}"
            );
        }
    }

    // index: 許可 content のみ・真実源と投影の分離・生メディア非保存。
    let index = Capability::CommunityIndex.meta();
    assert!(index.handled_data.contains("公開トピック"));
    assert!(index.handled_data.contains("Postgres"));
    assert!(index.handled_data.contains("ArcadeDB"));
    assert!(index.handled_data.contains("生メディア"));
    assert!(index.purpose.contains("走査を通過した許可"));
    assert!(index.telecom_note.contains("真実源ではない"));

    // moderation: 既知一致 + 分類器・fail-closed・Match Data 非保存・authority 限定。
    let moderation = Capability::Moderation.meta();
    assert!(moderation.purpose.contains("Project Arachnid Shield"));
    assert!(moderation.purpose.contains("視覚言語モデル"));
    assert!(moderation.purpose.contains("fail-closed"));
    assert!(moderation.handled_data.contains("Match Data"));
    assert!(moderation.telecom_note.contains("authority scope"));
    assert!(moderation.terms_note.contains("申し立て"));

    // trust / relation: 双方を含む・node-local advisory・共参加のみ・opt-out 可逆。
    let trust = Capability::CommunityLocalTrust.meta();
    assert!(trust.display_name.contains("relation"));
    assert!(trust.purpose.contains("node-local advisory"));
    assert!(trust.privacy_note.contains("共参加"));
    assert!(trust.privacy_note.contains("プライベートチャンネル"));
    assert!(trust.privacy_note.contains("可逆"));
    assert!(trust.telecom_note.contains("canonical"));
    assert!(trust.terms_note.contains("network-wide command"));
}

#[test]
fn promoted_capability_listed_as_operating() {
    // #617 の昇格後、moderation は運用中の補助機能として記載され、「計画中」分離は出ない。
    let yaml = base_config("  moderation: true\n", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let svc = doc(&generate_all(&resolved), "service-description-draft.md");
    assert!(!svc.contains("計画中（この配布物では未提供）"));
    assert!(svc.contains("モデレーション"));
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
