//! policy router の verdict 分岐と fail-closed 挙動を固定する（#353）。
//!
//! 受け入れ条件:
//! - known CSAM match → exclude（critical / confirmed）
//! - unknown CSAM/CSE suspected を confirmed と分けて扱う
//! - general moderation と critical safety route が分離される
//! - scan failure / provider unavailable / unscanned で fail-closed（allow にしない）
//! - `NoKnownMatch` を safe / clean 扱いしない

use kukuri_cn_safety::provider::{
    ProviderScanRequest, ProviderScanResult, SafetyProvider, ScanError, ScanOutcome, SubjectKind,
};
use kukuri_cn_safety::verdict::{ReasonCode, SafetyAction, SafetyCategory};
use kukuri_cn_safety::{
    MockSafetyProvider, SafetyLabel, SafetyPolicy, SafetyProviderCapability, route,
};

const SCANNED_AT: &str = "2026-06-29T00:00:00Z";

fn known_hash_result() -> ProviderScanResult {
    ProviderScanResult {
        provider: "known-csam".to_string(),
        capability: SafetyProviderCapability::KnownCsamHashMatch,
        outcome: ScanOutcome::Completed,
        derived_tags: Vec::new(),
        known_hash_match: true,
        score: None,
        labels: vec![SafetyLabel::new(SafetyCategory::Csam)],
    }
}

fn no_known_match_result() -> ProviderScanResult {
    ProviderScanResult {
        provider: "known-csam".to_string(),
        capability: SafetyProviderCapability::KnownCsamHashMatch,
        outcome: ScanOutcome::NoKnownMatch,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    }
}

fn score_result(
    capability: SafetyProviderCapability,
    category: SafetyCategory,
    score: u8,
) -> ProviderScanResult {
    ProviderScanResult {
        provider: "classifier".to_string(),
        capability,
        outcome: ScanOutcome::Completed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: Some(score),
        labels: vec![SafetyLabel::new(category).with_confidence(score)],
    }
}

fn general_result(category: SafetyCategory) -> ProviderScanResult {
    ProviderScanResult {
        provider: "general".to_string(),
        capability: SafetyProviderCapability::GeneralMediaModeration,
        outcome: ScanOutcome::Completed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: Some(95),
        labels: vec![SafetyLabel::new(category).with_confidence(95)],
    }
}

#[test]
fn known_csam_match_is_excluded_and_confirmed() {
    let policy = SafetyPolicy::public_node_default();
    let verdict = route(&[known_hash_result()], &policy, SCANNED_AT);
    assert_eq!(verdict.action, SafetyAction::Exclude);
    assert!(verdict.critical);
    assert_eq!(verdict.reason_code, ReasonCode::CsamConfirmed);
    assert!(!verdict.is_indexable());
}

#[test]
fn suspected_unknown_csam_is_quarantined_not_confirmed() {
    let policy = SafetyPolicy::public_node_default();
    let outcomes = [score_result(
        SafetyProviderCapability::NovelCsamImageClassifier,
        SafetyCategory::Csam,
        policy.suspected_threshold,
    )];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    // suspected は confirmed と別扱い。
    assert_eq!(verdict.reason_code, ReasonCode::CsamSuspected);
    assert_ne!(verdict.reason_code, ReasonCode::CsamConfirmed);
    assert!(verdict.critical);
    assert_eq!(verdict.action, SafetyAction::Quarantine);
    assert!(!verdict.is_indexable());
}

#[test]
fn cse_suspected_uses_cse_reason() {
    let policy = SafetyPolicy::public_node_default();
    let outcomes = [score_result(
        SafetyProviderCapability::CseTextClassifier,
        SafetyCategory::Cse,
        90,
    )];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::CseSuspected);
    assert!(verdict.critical);
}

#[test]
fn score_below_threshold_critical_detection_fails_closed_not_allow() {
    let policy = SafetyPolicy::public_node_default();
    let outcomes = [score_result(
        SafetyProviderCapability::NovelCsamImageClassifier,
        SafetyCategory::Csam,
        policy.suspected_threshold - 1,
    )];
    let verdict = route(&outcomes, &policy, SCANNED_AT);
    // 閾値未満でも critical な検知は safe と断定せず fail-closed する（Allow にしない）。
    assert!(
        !verdict.is_indexable(),
        "below-threshold CSAM must not index"
    );
    assert!(verdict.critical);
    assert_ne!(verdict.reason_code, ReasonCode::Clean);
}

#[test]
fn critical_detection_with_no_score_fails_closed() {
    // score=None でも critical capability の Completed 検知は Allow に取りこぼさない。
    let policy = SafetyPolicy::public_node_default();
    let result = ProviderScanResult {
        provider: "classifier".to_string(),
        capability: SafetyProviderCapability::NovelCsamImageClassifier,
        outcome: ScanOutcome::Completed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: vec![SafetyLabel::new(SafetyCategory::Csam)],
    };
    let verdict = route(&[result], &policy, SCANNED_AT);
    assert!(!verdict.is_indexable());
    assert!(verdict.critical);
}

#[test]
fn clean_critical_classifier_capability_is_not_itself_a_detection() {
    let policy = SafetyPolicy::public_node_default();
    let clean_unknown_csam = ProviderScanResult::completed(
        "unknown-csam-classifier",
        SafetyProviderCapability::NovelCsamImageClassifier,
    );

    let verdict = route(
        &[no_known_match_result(), clean_unknown_csam],
        &policy,
        SCANNED_AT,
    );

    assert_eq!(verdict.action, SafetyAction::Allow);
    assert_eq!(verdict.reason_code, ReasonCode::NoKnownMatch);
    assert!(!verdict.critical);
}

#[test]
fn critical_label_confidence_drives_suspected_when_score_absent() {
    // result.score が無くても label.confidence>=threshold なら suspected として扱う。
    let policy = SafetyPolicy::public_node_default();
    let result = ProviderScanResult {
        provider: "classifier".to_string(),
        capability: SafetyProviderCapability::NovelCsamImageClassifier,
        outcome: ScanOutcome::Completed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: vec![
            SafetyLabel::new(SafetyCategory::Csam).with_confidence(policy.suspected_threshold),
        ],
    };
    let verdict = route(&[result], &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::CsamSuspected);
    assert!(verdict.critical);
    assert!(!verdict.is_indexable());
}

#[test]
fn cse_first_label_noncritical_still_reports_cse_not_csam() {
    // CSE capability で先頭ラベルが非 critical(Nsfw) でも、CSE として報告する（取り違えない）。
    let policy = SafetyPolicy::public_node_default();
    let result = ProviderScanResult {
        provider: "cse".to_string(),
        capability: SafetyProviderCapability::CseTextClassifier,
        outcome: ScanOutcome::Completed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: Some(95),
        labels: vec![
            SafetyLabel::new(SafetyCategory::Nsfw).with_confidence(95),
            SafetyLabel::new(SafetyCategory::Cse).with_confidence(95),
        ],
    };
    let verdict = route(&[result], &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::CseSuspected);
    assert_ne!(verdict.reason_code, ReasonCode::CsamSuspected);
    assert!(verdict.critical);
}

#[test]
fn general_moderation_is_separate_route_from_critical() {
    let policy = SafetyPolicy::public_node_default();
    let verdict = route(
        &[
            no_known_match_result(),
            general_result(SafetyCategory::Nsfw),
        ],
        &policy,
        SCANNED_AT,
    );
    assert_eq!(verdict.reason_code, ReasonCode::GeneralModeration);
    assert!(!verdict.critical);
    // 既定 policy では high-confidence nsfw は exclude だが critical ではない。
    assert_eq!(verdict.action, SafetyAction::Exclude);
}

#[test]
fn spam_uses_general_route() {
    let policy = SafetyPolicy::public_node_default();
    let verdict = route(
        &[
            no_known_match_result(),
            general_result(SafetyCategory::Spam),
        ],
        &policy,
        SCANNED_AT,
    );
    assert_eq!(verdict.reason_code, ReasonCode::GeneralModeration);
    assert!(!verdict.critical);
}

#[test]
fn missing_required_known_csam_provider_fails_closed() {
    let policy = SafetyPolicy::public_node_default();
    let clean_general = ProviderScanResult {
        provider: "general".to_string(),
        capability: SafetyProviderCapability::GeneralMediaModeration,
        outcome: ScanOutcome::Completed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    };
    let verdict = route(&[clean_general], &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::ProviderUnavailable);
    assert_ne!(verdict.action, SafetyAction::Allow);
    assert!(!verdict.is_indexable());
}

#[test]
fn scan_failure_fails_closed_not_allow() {
    let policy = SafetyPolicy::public_node_default();
    let failed = ProviderScanResult {
        provider: "known-csam".to_string(),
        capability: SafetyProviderCapability::KnownCsamHashMatch,
        outcome: ScanOutcome::Failed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    };
    let verdict = route(&[failed], &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::ScanFailed);
    assert_ne!(verdict.action, SafetyAction::Allow);
    assert!(!verdict.is_indexable());
}

#[test]
fn provider_unavailable_fails_closed_not_allow() {
    let policy = SafetyPolicy::public_node_default();
    let unavailable = ProviderScanResult {
        provider: "known-csam".to_string(),
        capability: SafetyProviderCapability::KnownCsamHashMatch,
        outcome: ScanOutcome::Unavailable,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    };
    let verdict = route(&[unavailable], &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::ProviderUnavailable);
    assert_ne!(verdict.action, SafetyAction::Allow);
    assert!(!verdict.is_indexable());
}

#[test]
fn empty_scan_outcomes_fail_closed_unscanned() {
    let policy = SafetyPolicy::public_node_default();
    let verdict = route(&[], &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::Unscanned);
    assert_ne!(verdict.action, SafetyAction::Allow);
    assert!(!verdict.is_indexable());
}

#[test]
fn no_known_match_is_not_treated_as_clean() {
    let policy = SafetyPolicy::public_node_default();
    let no_match = ProviderScanResult {
        provider: "known-csam".to_string(),
        capability: SafetyProviderCapability::KnownCsamHashMatch,
        outcome: ScanOutcome::NoKnownMatch,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    };
    let verdict = route(&[no_match], &policy, SCANNED_AT);
    // no match は safe の証明ではない: reason_code は NoKnownMatch（Clean ではない）。
    assert_eq!(verdict.reason_code, ReasonCode::NoKnownMatch);
    assert_ne!(verdict.reason_code, ReasonCode::Clean);
}

#[test]
fn known_match_takes_priority_over_other_failures() {
    let policy = SafetyPolicy::public_node_default();
    let failed = ProviderScanResult {
        provider: "classifier".to_string(),
        capability: SafetyProviderCapability::NovelCsamImageClassifier,
        outcome: ScanOutcome::Failed,
        derived_tags: Vec::new(),
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    };
    let verdict = route(&[failed, known_hash_result()], &policy, SCANNED_AT);
    // confirmed CSAM は他 provider の失敗より優先して exclude。
    assert_eq!(verdict.action, SafetyAction::Exclude);
    assert_eq!(verdict.reason_code, ReasonCode::CsamConfirmed);
}

#[test]
fn suspected_threshold_default_0_7_operator_tunable() {
    // ADR 0028 §2.2: 既定閾値は 0.7（score 70）。operator が可変。
    let policy = SafetyPolicy::public_node_default();
    assert_eq!(policy.suspected_threshold, 70);

    // score 70（閾値ちょうど）は suspected として route される。
    let verdict = route(
        &[score_result(
            SafetyProviderCapability::NovelCsamImageClassifier,
            SafetyCategory::Csam,
            70,
        )],
        &policy,
        SCANNED_AT,
    );
    assert_eq!(verdict.reason_code, ReasonCode::CsamSuspected);
    assert_eq!(verdict.action, SafetyAction::Quarantine);

    // operator が閾値を 90 に上げると score 80 は suspected（Quarantine）にならないが、
    // critical 検知は Allow にも落ちない（取りこぼし防止で Hold 側に fail-closed）。
    let mut strict = SafetyPolicy::public_node_default();
    strict.suspected_threshold = 90;
    let verdict = route(
        &[score_result(
            SafetyProviderCapability::NovelCsamImageClassifier,
            SafetyCategory::Csam,
            80,
        )],
        &strict,
        SCANNED_AT,
    );
    assert!(!verdict.is_indexable());
    assert!(verdict.critical);

    // 旧 config 名 `unknown_csam_score_threshold` も deserialization で受理する（operator 互換）。
    let legacy = serde_json::json!({
        "policy_version": "legacy",
        "index_before_scan": false,
        "on_scan_error": "hold",
        "unknown_csam_score_threshold": 85,
        "suspected_critical_action": "quarantine",
        "on_high_confidence_nsfw": "exclude",
        "on_spam": "exclude",
        "on_malware_phishing": "exclude",
        "require_known_csam": true
    });
    let parsed: SafetyPolicy = serde_json::from_value(legacy).unwrap();
    assert_eq!(parsed.suspected_threshold, 85);
}

#[test]
fn high_confidence_critical_is_fail_closed_indexing() {
    // ADR 0028 §2.4: critical risk タグが閾値以上の高 confidence なら allow にしない
    // （自 node の index / discovery / recommendation に出さない）。
    let policy = SafetyPolicy::public_node_default();
    for capability in [
        SafetyProviderCapability::NovelCsamImageClassifier,
        SafetyProviderCapability::NovelCsamVideoClassifier,
        SafetyProviderCapability::CseTextClassifier,
        SafetyProviderCapability::GroomingTextClassifier,
    ] {
        let category = match capability {
            SafetyProviderCapability::CseTextClassifier => SafetyCategory::Cse,
            SafetyProviderCapability::GroomingTextClassifier => SafetyCategory::Grooming,
            _ => SafetyCategory::Csam,
        };
        let verdict = route(
            &[score_result(capability, category, 95)],
            &policy,
            SCANNED_AT,
        );
        assert!(
            !verdict.is_indexable(),
            "high-confidence critical ({capability:?}) must be fail-closed for indexing"
        );
        assert!(verdict.critical);
    }

    // policy が誤って suspected_critical_action=Allow を設定しても index には入れない。
    let mut broken = SafetyPolicy::public_node_default();
    broken.suspected_critical_action = SafetyAction::Allow;
    let verdict = route(
        &[score_result(
            SafetyProviderCapability::NovelCsamImageClassifier,
            SafetyCategory::Csam,
            95,
        )],
        &broken,
        SCANNED_AT,
    );
    assert!(!verdict.is_indexable());
}

#[test]
fn general_detection_below_threshold_does_not_block_indexing() {
    // ADR 0028 §2.2: general route も suspected 閾値に従う。閾値未満の低スコア検知は
    // GeneralModeration として発火せず、known CSAM scan が揃っていれば allow に落ちる。
    let policy = SafetyPolicy::public_node_default();
    let mut low_scored = general_result(SafetyCategory::Nsfw);
    low_scored.score = Some(policy.suspected_threshold - 1);
    low_scored.labels = vec![
        SafetyLabel::new(SafetyCategory::Nsfw).with_confidence(policy.suspected_threshold - 1),
    ];
    let verdict = route(&[no_known_match_result(), low_scored], &policy, SCANNED_AT);
    assert_ne!(verdict.reason_code, ReasonCode::GeneralModeration);
    assert_eq!(verdict.action, SafetyAction::Allow);

    // score / confidence の無い categorical 検知は従来どおり発火する（取りこぼさない）。
    let mut categorical = general_result(SafetyCategory::Spam);
    categorical.score = None;
    categorical.labels = vec![SafetyLabel::new(SafetyCategory::Spam)];
    let verdict = route(&[no_known_match_result(), categorical], &policy, SCANNED_AT);
    assert_eq!(verdict.reason_code, ReasonCode::GeneralModeration);
    assert!(!verdict.is_indexable());
}

#[test]
fn policy_with_allow_on_scan_error_is_overridden_to_fail_closed() {
    // policy が誤って on_scan_error=Allow を設定しても fail-closed を保証する。
    let mut policy = SafetyPolicy::public_node_default();
    policy.on_scan_error = SafetyAction::Allow;
    let verdict = route(&[], &policy, SCANNED_AT);
    assert_ne!(verdict.action, SafetyAction::Allow);
}

// --- provider abstraction / mock ---

#[tokio::test]
async fn mock_provider_returns_configured_known_match() {
    let provider =
        MockSafetyProvider::known_csam("arachnid-mock").with_known_hash_match("blob-bad");
    let result = provider
        .scan(&ProviderScanRequest::for_subject(
            SubjectKind::Blob,
            "blob-bad",
        ))
        .await
        .unwrap();
    assert!(result.known_hash_match);
    assert_eq!(result.outcome, ScanOutcome::Completed);
    assert_eq!(provider.name(), "arachnid-mock");
    assert_eq!(
        provider.capabilities(),
        &[SafetyProviderCapability::KnownCsamHashMatch]
    );
}

#[tokio::test]
async fn mock_provider_default_is_no_known_match_not_clean() {
    let provider = MockSafetyProvider::known_csam("arachnid-mock");
    let result = provider
        .scan(&ProviderScanRequest::for_subject(
            SubjectKind::Blob,
            "unconfigured",
        ))
        .await
        .unwrap();
    // 既定は NoKnownMatch（safe ではない）。
    assert_eq!(result.outcome, ScanOutcome::NoKnownMatch);
    assert!(!result.known_hash_match);
}

#[tokio::test]
async fn mock_provider_can_be_unavailable_for_fail_closed_tests() {
    let provider = MockSafetyProvider::known_csam("arachnid-mock").default_unavailable();
    let result = provider
        .scan(&ProviderScanRequest::for_subject(SubjectKind::Blob, "x"))
        .await
        .unwrap();
    assert_eq!(result.outcome, ScanOutcome::Unavailable);
    assert!(result.outcome.is_fail_closed());
}

#[tokio::test]
async fn mock_provider_can_error() {
    let provider = MockSafetyProvider::known_csam("arachnid-mock")
        .default_error(ScanError::Timeout("deadline".to_string()));
    let err = provider
        .scan(&ProviderScanRequest::for_subject(SubjectKind::Blob, "x"))
        .await
        .unwrap_err();
    assert_eq!(err, ScanError::Timeout("deadline".to_string()));
}

#[tokio::test]
async fn end_to_end_known_match_to_exclude_verdict() {
    let policy = SafetyPolicy::public_node_default();
    let provider =
        MockSafetyProvider::known_csam("arachnid-mock").with_known_hash_match("blob-bad");
    let scan = provider
        .scan(&ProviderScanRequest::for_subject(
            SubjectKind::Blob,
            "blob-bad",
        ))
        .await
        .unwrap();
    let verdict = route(&[scan], &policy, SCANNED_AT);
    assert_eq!(verdict.action, SafetyAction::Exclude);
    assert!(verdict.critical);
    assert!(!verdict.is_indexable());
}
