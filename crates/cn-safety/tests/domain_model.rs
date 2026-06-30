//! safety domain model の serde 表現と型の決定論的な振る舞いを固定する（#353）。

use kukuri_cn_safety::event::{ModerationEventBody, issue_signed_event};
use kukuri_cn_safety::provider::{ProviderScanResult, ScanOutcome, SubjectKind};
use kukuri_cn_safety::verdict::SafetyVerdict;
use kukuri_cn_safety::{
    AppealStatus, Basis, MockSigner, ModerationAction, ModerationEventSigner, ReasonCode,
    RiskSignalTarget, SafetyAction, SafetyCategory, SafetyLabel, SafetyProviderCapability,
    SafetyRiskSignal, Severity, Visibility,
};
use serde_json::json;

#[test]
fn safety_action_serializes_snake_case() {
    assert_eq!(
        serde_json::to_value(SafetyAction::Allow).unwrap(),
        json!("allow")
    );
    assert_eq!(
        serde_json::to_value(SafetyAction::Quarantine).unwrap(),
        json!("quarantine")
    );
    assert_eq!(
        serde_json::to_value(SafetyAction::Exclude).unwrap(),
        json!("exclude")
    );
}

#[test]
fn basis_and_visibility_serialize_snake_case() {
    assert_eq!(
        serde_json::to_value(Basis::KnownHashMatch).unwrap(),
        json!("known_hash_match")
    );
    assert_eq!(
        serde_json::to_value(Basis::ClassifierScore).unwrap(),
        json!("classifier_score")
    );
    assert_eq!(
        serde_json::to_value(Visibility::SubscribedNodes).unwrap(),
        json!("subscribed_nodes")
    );
}

#[test]
fn reason_code_distinguishes_confirmed_and_suspected() {
    assert_eq!(
        serde_json::to_value(ReasonCode::CsamConfirmed).unwrap(),
        json!("csam_confirmed")
    );
    assert_eq!(
        serde_json::to_value(ReasonCode::CsamSuspected).unwrap(),
        json!("csam_suspected")
    );
    // confirmed と suspected は別 variant であり、型レベルで混同しない。
    assert_ne!(ReasonCode::CsamConfirmed, ReasonCode::CsamSuspected);
    // NoKnownMatch と Clean も別物（no match を safe と同一視しない）。
    assert_ne!(ReasonCode::NoKnownMatch, ReasonCode::Clean);
}

#[test]
fn provider_capability_serializes_snake_case() {
    assert_eq!(
        serde_json::to_value(SafetyProviderCapability::KnownCsamHashMatch).unwrap(),
        json!("known_csam_hash_match")
    );
    assert_eq!(
        serde_json::to_value(SafetyProviderCapability::NovelCsamImageClassifier).unwrap(),
        json!("novel_csam_image_classifier")
    );
}

#[test]
fn capability_critical_vs_general_separation() {
    assert!(SafetyProviderCapability::KnownCsamHashMatch.is_critical_safety());
    assert!(SafetyProviderCapability::CseTextClassifier.is_critical_safety());
    assert!(!SafetyProviderCapability::GeneralMediaModeration.is_critical_safety());
    assert!(!SafetyProviderCapability::SpamAbuseModeration.is_critical_safety());

    // confirmed を生み得るのは known hash match のみ。
    assert!(SafetyProviderCapability::KnownCsamHashMatch.can_confirm_known_csam());
    assert!(!SafetyProviderCapability::NovelCsamImageClassifier.can_confirm_known_csam());
}

#[test]
fn category_critical_classification() {
    assert!(SafetyCategory::Csam.is_critical_safety());
    assert!(SafetyCategory::Cse.is_critical_safety());
    assert!(SafetyCategory::Grooming.is_critical_safety());
    assert!(!SafetyCategory::Nsfw.is_critical_safety());
    assert!(!SafetyCategory::Spam.is_critical_safety());
}

#[test]
fn visibility_defaults_to_local() {
    assert_eq!(Visibility::default(), Visibility::Local);
}

#[test]
fn safety_verdict_round_trips_snake_case() {
    let verdict = SafetyVerdict {
        action: SafetyAction::Hold,
        labels: vec![
            SafetyLabel::new(SafetyCategory::Csam)
                .with_confidence(91)
                .with_provider_capability(SafetyProviderCapability::NovelCsamImageClassifier),
        ],
        critical: true,
        reason_code: ReasonCode::CsamSuspected,
        confidence: Some(91),
        provider: Some("mock-classifier".to_string()),
        provider_capability: Some(SafetyProviderCapability::NovelCsamImageClassifier),
        policy_version: "2026-06-public-node-v1".to_string(),
        scanned_at: "2026-06-29T00:00:00Z".to_string(),
    };
    let value = serde_json::to_value(&verdict).unwrap();
    assert_eq!(value["action"], "hold");
    assert_eq!(value["reason_code"], "csam_suspected");
    assert_eq!(value["provider_capability"], "novel_csam_image_classifier");

    let back: SafetyVerdict = serde_json::from_value(value).unwrap();
    assert_eq!(back, verdict);
}

#[test]
fn verdict_is_indexable_only_when_allow() {
    let allow = make_verdict(SafetyAction::Allow);
    assert!(allow.is_indexable());
    for action in [
        SafetyAction::Hold,
        SafetyAction::Quarantine,
        SafetyAction::Exclude,
    ] {
        assert!(
            !make_verdict(action).is_indexable(),
            "{action:?} must not index"
        );
    }
}

fn make_verdict(action: SafetyAction) -> SafetyVerdict {
    SafetyVerdict {
        action,
        labels: Vec::new(),
        critical: false,
        reason_code: ReasonCode::Clean,
        confidence: None,
        provider: None,
        provider_capability: None,
        policy_version: "v1".to_string(),
        scanned_at: "2026-06-29T00:00:00Z".to_string(),
    }
}

#[test]
fn risk_signal_round_trips_snake_case() {
    let signal = SafetyRiskSignal {
        target: RiskSignalTarget::BlobCid,
        target_id: "bafy...".to_string(),
        category: SafetyCategory::Csam,
        severity: Severity::Critical,
        basis: Basis::KnownHashMatch,
        confidence: None,
        visibility: Visibility::SubscribedNodes,
        expires_at: None,
        appeal_status: Some(AppealStatus::None),
    };
    let value = serde_json::to_value(&signal).unwrap();
    assert_eq!(value["target"], "blob_cid");
    assert_eq!(value["basis"], "known_hash_match");
    assert_eq!(value["visibility"], "subscribed_nodes");
    assert_eq!(value["appeal_status"], "none");

    let back: SafetyRiskSignal = serde_json::from_value(value).unwrap();
    assert_eq!(back, signal);
}

#[test]
fn risk_signal_default_visibility_keeps_suspected_local() {
    // suspected unknown CSAM/CSE（classifier score）は local 既定。
    assert_eq!(
        SafetyRiskSignal::default_visibility_for(SafetyCategory::Csam, Basis::ClassifierScore),
        Visibility::Local
    );
    assert_eq!(
        SafetyRiskSignal::default_visibility_for(SafetyCategory::Cse, Basis::ClassifierScore),
        Visibility::Local
    );
    // known hash match / provider confirmed のみ subscribed 以上を許す。
    assert_eq!(
        SafetyRiskSignal::default_visibility_for(SafetyCategory::Csam, Basis::KnownHashMatch),
        Visibility::SubscribedNodes
    );
}

#[test]
fn moderation_event_body_canonical_is_deterministic() {
    let body = sample_body();
    // 同一内容なら canonical bytes は安定。
    assert_eq!(body.canonical_bytes(), body.clone().canonical_bytes());
    // 内容が変われば canonical bytes も変わる。
    let mut other = body.clone();
    other.target_id = "different".to_string();
    assert_ne!(body.canonical_bytes(), other.canonical_bytes());
}

#[test]
fn moderation_event_body_canonical_matches_golden_vector() {
    // canonical form をクロス実装・クロスバージョンで固定する golden vector。
    // object キーは辞書順、`confidence`（None）は省略される。
    let expected = "{\
\"action\":\"exclude\",\
\"basis\":\"known_hash_match\",\
\"created_at\":\"2026-06-29T00:00:00Z\",\
\"id\":\"evt-1\",\
\"issuer_node_id\":\"node-1\",\
\"labels\":[{\"category\":\"csam\"}],\
\"policy_version\":\"2026-06-public-node-v1\",\
\"reason_code\":\"csam_confirmed\",\
\"severity\":\"critical\",\
\"target_id\":\"bafy-target\",\
\"target_type\":\"blob\",\
\"visibility\":\"subscribed_nodes\"\
}";
    assert_eq!(sample_body().canonical_json(), expected);
}

#[test]
fn moderation_event_body_canonical_is_field_order_independent() {
    // 論理的に同じ body は、デシリアライズ元の JSON のキー順序に依存せず同一 canonical を生む。
    let ordered = r#"{
        "id":"evt-1","issuer_node_id":"node-1","target_type":"blob","target_id":"bafy-target",
        "action":"exclude","labels":[{"category":"csam"}],"reason_code":"csam_confirmed",
        "severity":"critical","basis":"known_hash_match","visibility":"subscribed_nodes",
        "policy_version":"2026-06-public-node-v1","created_at":"2026-06-29T00:00:00Z"
    }"#;
    let shuffled = r#"{
        "created_at":"2026-06-29T00:00:00Z","visibility":"subscribed_nodes","severity":"critical",
        "policy_version":"2026-06-public-node-v1","basis":"known_hash_match","reason_code":"csam_confirmed",
        "labels":[{"category":"csam"}],"action":"exclude","target_id":"bafy-target",
        "target_type":"blob","issuer_node_id":"node-1","id":"evt-1"
    }"#;
    let a: ModerationEventBody = serde_json::from_str(ordered).unwrap();
    let b: ModerationEventBody = serde_json::from_str(shuffled).unwrap();
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
}

#[test]
fn moderation_event_body_round_trips_snake_case() {
    let body = sample_body();
    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(value["issuer_node_id"], "node-1");
    assert_eq!(value["target_type"], "blob");
    assert_eq!(value["action"], "exclude");
    assert_eq!(value["reason_code"], "csam_confirmed");
    assert_eq!(value["basis"], "known_hash_match");

    let back: ModerationEventBody = serde_json::from_value(value).unwrap();
    assert_eq!(back, body);
}

#[test]
fn moderation_action_serializes_snake_case() {
    assert_eq!(
        serde_json::to_value(ModerationAction::RiskLabel).unwrap(),
        json!("risk_label")
    );
}

#[test]
fn provider_scan_result_round_trips() {
    let result = ProviderScanResult {
        provider: "mock".to_string(),
        capability: SafetyProviderCapability::KnownCsamHashMatch,
        outcome: ScanOutcome::NoKnownMatch,
        known_hash_match: false,
        score: None,
        labels: Vec::new(),
    };
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["outcome"], "no_known_match");
    assert_eq!(value["capability"], "known_csam_hash_match");
    let back: ProviderScanResult = serde_json::from_value(value).unwrap();
    assert_eq!(back, result);
}

#[test]
fn mock_signer_is_deterministic_and_separates_body_from_signature() {
    let signer = MockSigner::new("node-1");
    assert_eq!(signer.issuer_node_id(), "node-1");

    let body = sample_body();
    let signed_a = issue_signed_event(body.clone(), &signer);
    let signed_b = issue_signed_event(body.clone(), &signer);
    // 同一 body・同一 signer なら署名は決定論的。
    assert_eq!(signed_a.signature, signed_b.signature);
    // body は signature と分離して保持される。
    assert_eq!(signed_a.body, body);

    // body が変われば署名も変わる。
    let mut other = body.clone();
    other.target_id = "other-target".to_string();
    let signed_other = issue_signed_event(other, &signer);
    assert_ne!(signed_a.signature, signed_other.signature);
}

#[test]
fn signed_event_round_trips_snake_case() {
    let signer = MockSigner::new("node-1");
    let signed = issue_signed_event(sample_body(), &signer);
    let value = serde_json::to_value(&signed).unwrap();
    assert!(value["body"].is_object());
    assert!(value["signature"].is_string());
    let back: kukuri_cn_safety::SignedModerationEvent = serde_json::from_value(value).unwrap();
    assert_eq!(back, signed);
}

fn sample_body() -> ModerationEventBody {
    ModerationEventBody {
        id: "evt-1".to_string(),
        issuer_node_id: "node-1".to_string(),
        target_type: SubjectKind::Blob,
        target_id: "bafy-target".to_string(),
        action: ModerationAction::Exclude,
        labels: vec![SafetyLabel::new(SafetyCategory::Csam)],
        reason_code: ReasonCode::CsamConfirmed,
        severity: Severity::Critical,
        confidence: None,
        basis: Basis::KnownHashMatch,
        visibility: Visibility::SubscribedNodes,
        policy_version: "2026-06-public-node-v1".to_string(),
        created_at: "2026-06-29T00:00:00Z".to_string(),
    }
}
