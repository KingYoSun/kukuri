//! deploy セクション / terraform.tfvars 生成（#380）のテスト。

use kukuri_cn_operator::{
    Capability, build_manifest, generate_all, generate_tfvars, load_and_validate,
};

/// deploy セクション付きの最小 config。
fn config_with_deploy(extra_deploy: &str, extra_features: &str, ack: bool) -> String {
    format!(
        "server:\n\
         \x20 domain: example-kukuri.net\n\
         \x20 operator_name: Example Operator\n\
         \x20 country: JP\n\
         features:\n{extra_features}\
         deploy:\n\
         \x20 profile: low-cost\n\
         \x20 project_id: my-project\n\
         \x20 acme_email: ops@example-kukuri.net\n\
         \x20 jwt_secret_id: kukuri-cn-jwt-secret\n\
         \x20 postgres_password_secret_id: kukuri-cn-postgres-password\n{extra_deploy}\
         acknowledge_planned_capabilities: {ack}\n"
    )
}

#[test]
fn deploy_section_parses_and_is_optional() {
    // deploy 無しでも従来通り docs / manifest を生成できる（後方互換）。
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n";
    let resolved = load_and_validate(yaml).unwrap();
    assert!(resolved.deploy().is_none());
    assert!(!generate_all(&resolved).is_empty());
    let _ = build_manifest(&resolved);
}

#[test]
fn deploy_unknown_key_is_rejected() {
    let yaml = config_with_deploy("  not_a_real_key: true\n", "", false);
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("operator-config.yaml のパース")
            || err.to_string().contains("unknown field"),
        "deny_unknown_fields should reject: {err}"
    );
}

#[test]
fn generate_tfvars_is_deterministic() {
    let yaml = config_with_deploy("  relay_domain: relay.example-kukuri.net\n", "", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let first = generate_tfvars(&resolved).unwrap();
    let second = generate_tfvars(&resolved).unwrap();
    assert_eq!(first, second);
    assert!(first.contains("project_id = \"my-project\""));
    assert!(first.contains("api_domain   = \"example-kukuri.net\""));
}

#[test]
fn blob_cache_enabled_derives_from_features_true() {
    let yaml = config_with_deploy(
        "  relay_domain: relay.example-kukuri.net\n  blob_cache_size_gb: 10\n",
        "  blob_cache: true\n",
        false,
    );
    let resolved = load_and_validate(&yaml).unwrap();
    assert!(resolved.enabled(Capability::BlobCache));
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("blob_cache_enabled   = true"));
    assert!(tfvars.contains("blob_cache_size_gb   = 10"));
}

#[test]
fn blob_cache_enabled_derives_from_features_false() {
    let yaml = config_with_deploy(
        "  relay_domain: relay.example-kukuri.net\n",
        "  blob_cache: false\n",
        false,
    );
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("blob_cache_enabled   = false"));
}

#[test]
fn blob_cache_size_without_feature_is_rejected() {
    // features.blob_cache=false なのに sizing > 0 は矛盾（真実源は features 側）。
    let yaml = config_with_deploy(
        "  relay_domain: relay.example-kukuri.net\n  blob_cache_size_gb: 10\n",
        "  blob_cache: false\n",
        false,
    );
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("features.blob_cache"),
        "blob cache contradiction should be rejected: {err}"
    );
}

#[test]
fn low_cost_without_relay_domain_is_rejected_even_when_iroh_relay_disabled() {
    let yaml = config_with_deploy("", "  iroh_relay: false\n", false);
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("relay_domain"),
        "low-cost requires relay_domain because templates always use it: {err}"
    );
}

#[test]
fn iroh_relay_with_relay_domain_validates() {
    let yaml = config_with_deploy(
        "  relay_domain: relay.example-kukuri.net\n",
        "  iroh_relay: true\n",
        false,
    );
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("relay_domain = \"relay.example-kukuri.net\""));
}

#[test]
fn generate_tfvars_without_deploy_fails() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n";
    let resolved = load_and_validate(yaml).unwrap();
    let err = generate_tfvars(&resolved).unwrap_err();
    assert!(
        err.to_string().contains("deploy"),
        "missing deploy should error: {err}"
    );
}

#[test]
fn managed_db_profile_tfvars_is_unsupported() {
    // managed-db / ha は docs / manifest 生成は可能だが tfvars 生成は拡張点（未対応）。
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                deploy:\n  profile: managed-db\n  project_id: p\n  acme_email: a@b.net\n\
                \x20 jwt_secret_id: jwt\n  postgres_password_secret_id: pg\n";
    let resolved = load_and_validate(yaml).unwrap();
    // docs / manifest は生成できる。
    assert!(!generate_all(&resolved).is_empty());
    // tfvars 生成は error。
    let err = generate_tfvars(&resolved).unwrap_err();
    assert!(
        err.to_string().contains("managed-db") || err.to_string().contains("未対応"),
        "managed-db tfvars should be unsupported: {err}"
    );
}

#[test]
fn deploy_requires_project_id() {
    let yaml = "server:\n  domain: d.net\n  operator_name: Op\n  country: JP\n\
                deploy:\n  profile: low-cost\n  project_id: \"\"\n  acme_email: a@b.net\n\
                \x20 jwt_secret_id: jwt\n  postgres_password_secret_id: pg\n";
    let err = load_and_validate(yaml).unwrap_err();
    assert!(err.to_string().contains("project_id"), "got: {err}");
}

#[test]
fn generate_tfvars_trims_deploy_strings() {
    let yaml = "server:\n  domain: example-kukuri.net\n  operator_name: Op\n  country: JP\n\
                deploy:\n  profile: low-cost\n  project_id: \"  my-project  \"\n\
                \x20 region: \"  asia-northeast1  \"\n  zone: \"  asia-northeast1-a  \"\n\
                \x20 relay_domain: \"  relay.example-kukuri.net  \"\n\
                \x20 acme_email: \"  ops@example-kukuri.net  \"\n\
                \x20 jwt_secret_id: \"  kukuri-cn-jwt-secret  \"\n\
                \x20 postgres_password_secret_id: \"  kukuri-cn-postgres-password  \"\n\
                \x20 cn_user_api_image: \"  ghcr.io/kingyosun/kukuri-cn-user-api:latest  \"\n\
                \x20 machine_type: \"  e2-small  \"\n\
                \x20 blob_cache_path: \"  /var/lib/kukuri/blob-cache  \"\n";
    let resolved = load_and_validate(yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("project_id = \"my-project\""));
    assert!(tfvars.contains("region     = \"asia-northeast1\""));
    assert!(tfvars.contains("relay_domain = \"relay.example-kukuri.net\""));
    assert!(tfvars.contains("blob_cache_path      = \"/var/lib/kukuri/blob-cache\""));
    assert!(!tfvars.contains("  my-project  "));
}

#[test]
fn low_cost_rejects_invalid_deploy_format() {
    let yaml = "server:\n  domain: example-kukuri.net\n  operator_name: Op\n  country: JP\n\
                deploy:\n  profile: low-cost\n  project_id: Invalid_Project\n\
                \x20 relay_domain: relay.example-kukuri.net\n  acme_email: ops@example-kukuri.net\n\
                \x20 jwt_secret_id: kukuri-cn-jwt-secret\n\
                \x20 postgres_password_secret_id: kukuri-cn-postgres-password\n";
    let err = load_and_validate(yaml).unwrap_err();
    assert!(err.to_string().contains("project_id"), "got: {err}");

    let yaml = "server:\n  domain: example-kukuri.net\n  operator_name: Op\n  country: JP\n\
                deploy:\n  profile: low-cost\n  project_id: my-project\n\
                \x20 relay_domain: relay.example-kukuri.net\n  acme_email: ops@example-kukuri.net\n\
                \x20 jwt_secret_id: \"bad secret\"\n\
                \x20 postgres_password_secret_id: kukuri-cn-postgres-password\n";
    let err = load_and_validate(yaml).unwrap_err();
    assert!(err.to_string().contains("jwt_secret_id"), "got: {err}");
}

#[test]
fn tfvars_emits_safety_signing_key_secret_id_when_present() {
    // safety.events.signing_key_secret_id があれば tfvars に secret ID として出力される。
    let yaml = "server:\n  domain: example-kukuri.net\n  operator_name: Op\n  country: JP\n\
                safety:\n  events:\n    signing_key_secret_id: kukuri-cn-safety-signing-key\n\
                deploy:\n  profile: low-cost\n  project_id: my-project\n\
                \x20 relay_domain: relay.example-kukuri.net\n  acme_email: ops@example-kukuri.net\n\
                \x20 jwt_secret_id: kukuri-cn-jwt-secret\n\
                \x20 postgres_password_secret_id: kukuri-cn-postgres-password\n";
    let resolved = load_and_validate(yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(
        tfvars.contains("safety_signing_key_secret_id = \"kukuri-cn-safety-signing-key\""),
        "tfvars:\n{tfvars}"
    );
}

#[test]
fn generated_tfvars_guides_operator_config_path() {
    let yaml = config_with_deploy("  relay_domain: relay.example-kukuri.net\n", "", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("# operator_config_path = \"operator-config.yaml\""));
    assert!(!tfvars.contains("operator_config_file = file("));
}

/// indexer stack（#615）一式を有効化した config。
fn config_with_indexer_stack(extra_deploy: &str, extra_features: &str) -> String {
    format!(
        "server:\n\
         \x20 domain: example-kukuri.net\n\
         \x20 operator_name: Example Operator\n\
         \x20 country: JP\n\
         features:\n\
         \x20 iroh_relay: true\n{extra_features}\
         safety:\n\
         \x20 events:\n\
         \x20   emit_signed_moderation_events: true\n\
         \x20   signing_key_secret_id: kukuri-cn-safety-signing-key\n\
         \x20 providers:\n\
         \x20   known_csam:\n\
         \x20     provider: project-arachnid-shield\n\
         \x20     required: true\n\
         \x20   general:\n\
         \x20     provider: openai-compatible-vlm\n\
         \x20   unknown_csam:\n\
         \x20     provider: openai-compatible-vlm\n\
         deploy:\n\
         \x20 profile: low-cost\n\
         \x20 project_id: my-project\n\
         \x20 relay_domain: relay.example-kukuri.net\n\
         \x20 acme_email: ops@example-kukuri.net\n\
         \x20 jwt_secret_id: kukuri-cn-jwt-secret\n\
         \x20 postgres_password_secret_id: kukuri-cn-postgres-password\n\
         \x20 deploy_indexer_stack: true\n\
         \x20 channel_secret_key_secret_id: kukuri-cn-channel-secret-key\n\
         \x20 arcadedb_password_secret_id: kukuri-cn-arcadedb-password\n\
         \x20 arachnid_username_secret_id: kukuri-cn-arachnid-username\n\
         \x20 arachnid_password_secret_id: kukuri-cn-arachnid-password\n\
         \x20 vlm_api_base_url: http://192.0.2.10:8000\n\
         \x20 vlm_model: inclusionAI/SingGuard-2b\n\
         \x20 vlm_response_format: guard\n{extra_deploy}"
    )
}

#[test]
fn machine_type_defaults_to_e2_medium() {
    let yaml = config_with_deploy("  relay_domain: relay.example-kukuri.net\n", "", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(
        tfvars.contains("machine_type          = \"e2-medium\""),
        "tfvars:\n{tfvars}"
    );
}

#[test]
fn indexer_stack_defaults_to_disabled_with_images() {
    // 既存 config（stack 未指定）でも tfvars には明示的な false と image 既定値が出る。
    let yaml = config_with_deploy("  relay_domain: relay.example-kukuri.net\n", "", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("deploy_indexer_stack = false"));
    assert!(
        tfvars.contains("cn_indexer_image     = \"ghcr.io/kingyosun/kukuri-cn-indexer:latest\"")
    );
    assert!(tfvars.contains("arcadedb_image       = \"arcadedata/arcadedb:26.8.1\""));
    assert!(tfvars.contains("relation_analyze_interval_minutes = 60"));
    assert!(tfvars.contains("safety_provider_known_csam            = \"\""));
}

#[test]
fn indexer_stack_full_config_emits_expected_tfvars() {
    let yaml = config_with_indexer_stack(
        "  indexer_data_disk_gb: 10\n  relation_analyze_interval_minutes: 30\n",
        "",
    );
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("deploy_indexer_stack = true"));
    assert!(tfvars.contains("indexer_data_disk_gb = 10"));
    assert!(tfvars.contains("relation_analyze_interval_minutes = 30"));
    assert!(tfvars.contains("indexer_own_relay           = true"));
    assert!(tfvars.contains("channel_secret_key_secret_id = \"kukuri-cn-channel-secret-key\""));
    assert!(tfvars.contains("arcadedb_password_secret_id  = \"kukuri-cn-arcadedb-password\""));
    assert!(tfvars.contains("arachnid_username_secret_id  = \"kukuri-cn-arachnid-username\""));
    assert!(tfvars.contains("arachnid_password_secret_id  = \"kukuri-cn-arachnid-password\""));
    assert!(tfvars.contains("safety_provider_known_csam            = \"project-arachnid-shield\""));
    assert!(tfvars.contains("safety_provider_known_csam_required   = true"));
    assert!(tfvars.contains("safety_provider_general               = \"openai-compatible-vlm\""));
    assert!(tfvars.contains("safety_emit_signed_events             = true"));
    assert!(tfvars.contains("safety_signing_key_secret_id = \"kukuri-cn-safety-signing-key\""));
    assert!(tfvars.contains("vlm_api_base_url     = \"http://192.0.2.10:8000\""));
    assert!(tfvars.contains("vlm_model            = \"inclusionAI/SingGuard-2b\""));
    assert!(tfvars.contains("vlm_response_format  = \"guard\""));
    // 任意 secret（VLM API key）未指定は空文字（Terraform 側で「fetch しない」の合図）。
    assert!(tfvars.contains("vlm_api_key_secret_id        = \"\""));
}

#[test]
fn indexer_stack_requires_channel_and_arcadedb_secret_ids() {
    let yaml = config_with_indexer_stack("", "").replace(
        "  channel_secret_key_secret_id: kukuri-cn-channel-secret-key\n",
        "",
    );
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("channel_secret_key_secret_id"),
        "got: {err}"
    );

    let yaml = config_with_indexer_stack("", "").replace(
        "  arcadedb_password_secret_id: kukuri-cn-arcadedb-password\n",
        "",
    );
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("arcadedb_password_secret_id"),
        "got: {err}"
    );
}

#[test]
fn indexer_stack_requires_relay() {
    // 自前 relay 無効 + 外部 relay 未指定は config 段階で fail-closed。
    let yaml =
        config_with_indexer_stack("", "").replace("  iroh_relay: true\n", "  iroh_relay: false\n");
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(err.to_string().contains("relay"), "got: {err}");

    // 外部 relay URL があれば成立し、tfvars に出力される。
    let yaml = config_with_indexer_stack(
        "  indexer_external_relay_urls:\n    - https://relay.example.net\n",
        "",
    )
    .replace("  iroh_relay: true\n", "  iroh_relay: false\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("indexer_own_relay           = false"));
    assert!(
        tfvars.contains("indexer_external_relay_urls = [\"https://relay.example.net\"]"),
        "tfvars:\n{tfvars}"
    );
}

#[test]
fn indexer_stack_requires_provider_credentials() {
    // Arachnid provider には username / password の secret ID が必要。
    let yaml = config_with_indexer_stack("", "").replace(
        "  arachnid_username_secret_id: kukuri-cn-arachnid-username\n",
        "",
    );
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("arachnid_username_secret_id"),
        "got: {err}"
    );

    // VLM provider には endpoint / model が必要。
    let yaml = config_with_indexer_stack("", "")
        .replace("  vlm_api_base_url: http://192.0.2.10:8000\n", "");
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(err.to_string().contains("vlm_api_base_url"), "got: {err}");
}

#[test]
fn indexer_stack_requires_signing_key_when_emitting_signed_events() {
    let yaml = config_with_indexer_stack("", "").replace(
        "    signing_key_secret_id: kukuri-cn-safety-signing-key\n",
        "",
    );
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("signing_key_secret_id"),
        "got: {err}"
    );
}

#[test]
fn indexer_stack_rejects_invalid_vlm_response_format_and_zero_interval() {
    let yaml = config_with_indexer_stack("", "").replace(
        "  vlm_response_format: guard\n",
        "  vlm_response_format: xml\n",
    );
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("vlm_response_format"),
        "got: {err}"
    );

    let yaml = config_with_indexer_stack("  relation_analyze_interval_minutes: 0\n", "");
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string()
            .contains("relation_analyze_interval_minutes"),
        "got: {err}"
    );
}

#[test]
fn tfvars_never_contains_secret_values() {
    // deploy は secret ID のみを持つ。tfvars には ID のみ出力され、値は出ない。
    let yaml = config_with_deploy("  relay_domain: relay.example-kukuri.net\n", "", false);
    let resolved = load_and_validate(&yaml).unwrap();
    let tfvars = generate_tfvars(&resolved).unwrap();
    assert!(tfvars.contains("jwt_secret_id               = \"kukuri-cn-jwt-secret\""));
    assert!(tfvars.contains("postgres_password_secret_id = \"kukuri-cn-postgres-password\""));
    // 値らしき文字列が無い（ID 以外の secret keyword を出さない）。
    assert!(!tfvars.contains("jwt_secret ="));
    assert!(!tfvars.contains("postgres_password ="));
}
