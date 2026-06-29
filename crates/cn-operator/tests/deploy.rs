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
