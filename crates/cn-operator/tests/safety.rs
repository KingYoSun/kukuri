use std::fs;
use std::process::Command;

use kukuri_cn_operator::{
    READINESS_CHECK_IDS, ReadinessStatus, SafetyErrorAction, evaluate_public_node_readiness,
    load_and_validate,
};

fn config_with_safety(safety: &str) -> String {
    format!(
        "server:\n  domain: example-kukuri.net\n  operator_name: Example Operator\n  country: JP\n{safety}"
    )
}

fn complete_safety() -> &'static str {
    "safety:\n  profile: public-node\n  policy_version: 2026-06-public-node-v1\n  indexing:\n    index_before_scan: false\n    on_scan_error: hold\n  storage:\n    permanent_blob_storage: false\n  events:\n    emit_signed_moderation_events: true\n    signing_key_secret_id: kukuri-cn-safety-signing-key\n  providers:\n    known_csam:\n      provider: project_arachnid_shield\n      required: true\n      credential_secret_id: kukuri-cn-safety-known-csam\n"
}

#[test]
fn safety_config_parses_structured_schema() {
    let resolved = load_and_validate(&config_with_safety(complete_safety())).unwrap();
    let safety = resolved.raw.safety.as_ref().unwrap();
    assert_eq!(safety.profile.as_deref(), Some("public-node"));
    assert!(!safety.indexing.index_before_scan);
    assert_eq!(safety.indexing.on_scan_error, SafetyErrorAction::Hold);
    assert!(!safety.storage.permanent_blob_storage);
    assert!(safety.events.emit_signed_moderation_events);
    assert_eq!(
        safety
            .providers
            .known_csam
            .as_ref()
            .unwrap()
            .credential_secret_id
            .as_deref(),
        Some("kukuri-cn-safety-known-csam")
    );
}

#[test]
fn safety_unknown_key_is_rejected() {
    let yaml =
        config_with_safety("safety:\n  profile: public-node\n  not_a_real_safety_key: true\n");
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("operator-config.yaml のパース")
            || err.to_string().contains("unknown field"),
        "unexpected error: {err}"
    );
}

#[test]
fn safety_invalid_secret_id_is_rejected() {
    let yaml = config_with_safety(
        "safety:\n  providers:\n    known_csam:\n      provider: project_arachnid_shield\n      required: true\n      credential_secret_id: \"bad secret\"\n",
    );
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("credential_secret_id"),
        "unexpected error: {err}"
    );
}

#[test]
fn readiness_complete_static_config_has_unknown_runtime_checks() {
    let resolved = load_and_validate(&config_with_safety(complete_safety())).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    // 完全な static config でも runtime 未確定の unknown が残るため is_ready は false。
    assert!(!report.is_ready());
    // ただし static check に fail は無いので static_checks_pass は true（CLI は SUCCESS を返す）。
    assert!(report.static_checks_pass());
    assert!(!report.has_blocking_failures());
    assert_eq!(report.fail_count(), 0);
    assert_eq!(report.unknown_count(), 2);
    assert_check(&report, "safety_config_present", ReadinessStatus::Pass);
    assert_check(
        &report,
        "known_csam_provider_configured",
        ReadinessStatus::Pass,
    );
    assert_check(
        &report,
        "known_csam_provider_required",
        ReadinessStatus::Pass,
    );
    assert_check(&report, "index_before_scan_disabled", ReadinessStatus::Pass);
    assert_check(&report, "scan_error_fail_closed", ReadinessStatus::Pass);
    assert_check(
        &report,
        "signed_moderation_events_enabled",
        ReadinessStatus::Pass,
    );
    assert_check(
        &report,
        "signing_key_secret_configured",
        ReadinessStatus::Pass,
    );
    assert_check(
        &report,
        "permanent_blob_storage_disabled",
        ReadinessStatus::Pass,
    );
    assert_check(
        &report,
        "known_csam_credential_secret_configured",
        ReadinessStatus::Pass,
    );
    assert_check(
        &report,
        "provider_credential_valid",
        ReadinessStatus::Unknown,
    );
    assert_check(
        &report,
        "scan_coverage_metrics_available",
        ReadinessStatus::Unknown,
    );
}

#[test]
fn readiness_missing_safety_section_fails_closed() {
    let resolved = load_and_validate(&config_with_safety("")).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert!(!report.is_ready());
    assert!(report.has_blocking_failures());
    assert!(!report.static_checks_pass());
    assert_eq!(report.fail_count(), report.checks.len());
    assert_eq!(report.unknown_count(), 0);
}

#[test]
fn readiness_check_ids_are_consistent_across_paths() {
    // 通常経路と safety 欠落経路が、READINESS_CHECK_IDS と同一の id を同順で網羅する。
    let complete = load_and_validate(&config_with_safety(complete_safety())).unwrap();
    let present = evaluate_public_node_readiness(&complete, "public-node");
    let present_ids: Vec<&str> = present.checks.iter().map(|c| c.id).collect();
    assert_eq!(present_ids, READINESS_CHECK_IDS.to_vec());

    let missing_cfg = load_and_validate(&config_with_safety("")).unwrap();
    let missing = evaluate_public_node_readiness(&missing_cfg, "public-node");
    let missing_ids: Vec<&str> = missing.checks.iter().map(|c| c.id).collect();
    assert_eq!(missing_ids, READINESS_CHECK_IDS.to_vec());
}

#[test]
fn readiness_non_public_node_profile_fails() {
    // public-node 以外の profile を要求すると profile check が fail し、blocking failure になる。
    let resolved = load_and_validate(&config_with_safety(complete_safety())).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "minimal");
    assert_eq!(report.profile, "minimal");
    assert_check(&report, "safety_profile_public_node", ReadinessStatus::Fail);
    assert!(report.has_blocking_failures());
    assert!(!report.is_ready());
}

#[test]
fn readiness_empty_profile_defaults_to_public_node() {
    // 空 profile は public-node として扱う。
    let resolved = load_and_validate(&config_with_safety(complete_safety())).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "");
    assert_eq!(report.profile, "public-node");
    assert_check(&report, "safety_profile_public_node", ReadinessStatus::Pass);
}

#[test]
fn safety_readiness_cli_succeeds_when_only_unknown_remains() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("operator-config.yaml");
    fs::write(&config_path, config_with_safety(complete_safety())).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cn-operator"))
        .args(["safety", "readiness", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "stdout:\n{stdout}");
    assert!(stdout.contains("ready=false"), "stdout:\n{stdout}");
    assert!(stdout.contains("static_ok=true"), "stdout:\n{stdout}");
    assert!(stdout.contains("NOTE:"), "stdout:\n{stdout}");
}

#[test]
fn safety_readiness_cli_fails_on_static_failure() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("operator-config.yaml");
    fs::write(&config_path, config_with_safety("")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cn-operator"))
        .args(["safety", "readiness", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!output.status.success(), "stdout:\n{stdout}");
    assert!(stdout.contains("static_ok=false"), "stdout:\n{stdout}");
    assert!(stdout.contains("fail=12"), "stdout:\n{stdout}");
}

#[test]
fn readiness_fails_when_index_before_scan_is_true() {
    let yaml = config_with_safety(
        &complete_safety().replace("index_before_scan: false", "index_before_scan: true"),
    );
    let resolved = load_and_validate(&yaml).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert_check(&report, "index_before_scan_disabled", ReadinessStatus::Fail);
}

#[test]
fn readiness_fails_when_on_scan_error_allows() {
    let yaml = config_with_safety(
        &complete_safety().replace("on_scan_error: hold", "on_scan_error: allow"),
    );
    let resolved = load_and_validate(&yaml).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert_check(&report, "scan_error_fail_closed", ReadinessStatus::Fail);
}

#[test]
fn readiness_fails_without_known_csam_provider() {
    let yaml = config_with_safety(
        "safety:\n  profile: public-node\n  indexing:\n    index_before_scan: false\n    on_scan_error: hold\n  storage:\n    permanent_blob_storage: false\n  events:\n    emit_signed_moderation_events: true\n",
    );
    let resolved = load_and_validate(&yaml).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert_check(
        &report,
        "known_csam_provider_configured",
        ReadinessStatus::Fail,
    );
    assert_check(
        &report,
        "known_csam_credential_secret_configured",
        ReadinessStatus::Fail,
    );
}

#[test]
fn readiness_fails_when_known_csam_is_not_required() {
    let yaml = config_with_safety(&complete_safety().replace("required: true", "required: false"));
    let resolved = load_and_validate(&yaml).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert_check(
        &report,
        "known_csam_provider_required",
        ReadinessStatus::Fail,
    );
}

#[test]
fn readiness_fails_when_permanent_blob_storage_is_enabled() {
    let yaml = config_with_safety(&complete_safety().replace(
        "permanent_blob_storage: false",
        "permanent_blob_storage: true",
    ));
    let resolved = load_and_validate(&yaml).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert_check(
        &report,
        "permanent_blob_storage_disabled",
        ReadinessStatus::Fail,
    );
}

#[test]
fn readiness_fails_when_signed_events_are_disabled() {
    let yaml = config_with_safety(&complete_safety().replace(
        "emit_signed_moderation_events: true",
        "emit_signed_moderation_events: false",
    ));
    let resolved = load_and_validate(&yaml).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert_check(
        &report,
        "signed_moderation_events_enabled",
        ReadinessStatus::Fail,
    );
}

#[test]
fn readiness_fails_without_signing_key_secret() {
    // signing_key_secret_id 行を取り除くと signing key check が fail する。
    let yaml = config_with_safety(&complete_safety().replace(
        "    signing_key_secret_id: kukuri-cn-safety-signing-key\n",
        "",
    ));
    let resolved = load_and_validate(&yaml).unwrap();
    let report = evaluate_public_node_readiness(&resolved, "public-node");
    assert_check(
        &report,
        "signing_key_secret_configured",
        ReadinessStatus::Fail,
    );
}

#[test]
fn safety_invalid_signing_key_secret_id_is_rejected() {
    let yaml =
        config_with_safety("safety:\n  events:\n    signing_key_secret_id: \"bad secret\"\n");
    let err = load_and_validate(&yaml).unwrap_err();
    assert!(
        err.to_string().contains("signing_key_secret_id"),
        "unexpected error: {err}"
    );
}

#[cfg(feature = "safety-mock")]
#[test]
fn mock_provider_route_matches_test_provider_path() {
    use kukuri_cn_safety::{
        MockSafetyProvider, ProviderScanRequest, SafetyPolicy, SafetyProvider, SubjectKind, route,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider =
        MockSafetyProvider::known_csam("mock-known-csam").with_known_hash_match("blob-1");
    let scan = runtime
        .block_on(async {
            provider
                .scan(&ProviderScanRequest::for_subject(
                    SubjectKind::Blob,
                    "blob-1",
                ))
                .await
        })
        .unwrap();
    let verdict = route(
        &[scan],
        &SafetyPolicy::public_node_default(),
        "mock-scanned-at",
    );
    assert!(!verdict.is_indexable());
}

fn assert_check(report: &kukuri_cn_operator::ReadinessReport, id: &str, status: ReadinessStatus) {
    let check = report
        .checks
        .iter()
        .find(|check| check.id == id)
        .unwrap_or_else(|| panic!("missing readiness check {id}"));
    assert_eq!(check.status, status, "{}: {}", check.id, check.detail);
}
