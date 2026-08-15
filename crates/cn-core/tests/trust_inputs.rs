//! trust / relation reads への risk signal 供給契約の contract テスト（#406 → #415）。DB 不要。
//!
//! ADR 0026 §2.7（category による絶対 / 相対振り分け）・§6.2（appeal 反映）を純関数
//! `trust_risk_inputs_from` に対して固定する。

use chrono::Utc;

use kukuri_cn_core::{StoredRiskSignal, trust_risk_inputs_from};
use kukuri_cn_safety::{
    AppealStatus, Basis, RiskSignalTarget, SafetyCategory, SafetyRiskSignal, Severity, Visibility,
};
use kukuri_cn_trust::{TrustComponentKind, trust_component_for};

const NOW: &str = "2026-07-02T09:00:00Z";

fn stored_signal(
    id: &str,
    category: SafetyCategory,
    expires_at: Option<&str>,
    appeal_status: Option<AppealStatus>,
) -> StoredRiskSignal {
    StoredRiskSignal {
        id: id.to_string(),
        issuer_node_id: "issuer-node".to_string(),
        signal: SafetyRiskSignal {
            target: RiskSignalTarget::UserPubkey,
            target_id: "pubkey-1".to_string(),
            category,
            severity: Severity::High,
            basis: Basis::KnownHashMatch,
            confidence: Some(97),
            visibility: Visibility::Local,
            expires_at: expires_at.map(str::to_string),
            appeal_status,
        },
        persisted_at: Utc::now(),
    }
}

// --- 絶対 / 相対振り分け（ADR 0026 §2.7） ---

#[test]
fn csam_risk_signal_feeds_absolute_component() {
    // critical safety（CSAM / CSE / grooming）は絶対成分。relation 非依存・report-bomb 不動。
    for category in [
        SafetyCategory::Csam,
        SafetyCategory::Cse,
        SafetyCategory::Grooming,
    ] {
        assert_eq!(trust_component_for(category), TrustComponentKind::Absolute);
    }

    let signals = vec![stored_signal("sig-1", SafetyCategory::Csam, None, None)];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    assert_eq!(inputs.absolute.len(), 1);
    assert!(inputs.relative.is_empty());
    assert_eq!(inputs.absolute[0].component, TrustComponentKind::Absolute);
}

#[test]
fn general_moderation_signal_feeds_relative_component() {
    // nsfw / spam 等の文化圏依存の指標は相対成分（relation で重み付け）。
    for category in [
        SafetyCategory::Nsfw,
        SafetyCategory::Spam,
        SafetyCategory::Malware,
        SafetyCategory::Phishing,
    ] {
        assert_eq!(trust_component_for(category), TrustComponentKind::Relative);
    }

    let signals = vec![
        stored_signal("sig-1", SafetyCategory::Nsfw, None, None),
        stored_signal("sig-2", SafetyCategory::Csam, None, None),
    ];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    assert_eq!(inputs.absolute.len(), 1);
    assert_eq!(inputs.relative.len(), 1);
    assert_eq!(inputs.relative[0].signal_id, "sig-1");
    assert_eq!(inputs.absolute[0].signal_id, "sig-2");
}

// --- ADR 0028 contract: 非決定論的（VLM）suspected の trust 振り分け（#420） ---

/// VLM scan 由来の suspected signal（basis = ClassifierScore）を模す。
fn classifier_signal(id: &str, category: SafetyCategory) -> StoredRiskSignal {
    let mut stored = stored_signal(id, category, None, None);
    stored.signal.basis = Basis::ClassifierScore;
    stored.signal.severity = if category.is_critical_safety() {
        Severity::Critical
    } else {
        Severity::High
    };
    stored
}

#[test]
fn critical_suspected_feeds_trust_absolute_component() {
    // ADR 0028 §2.5 / ADR 0026 §2.3: critical（CSAM / CSE / grooming）の suspected
    // （厳格非決定論）は relation で薄まらない絶対成分に入る。
    let signals = vec![
        classifier_signal("sig-csam", SafetyCategory::Csam),
        classifier_signal("sig-cse", SafetyCategory::Cse),
        classifier_signal("sig-grooming", SafetyCategory::Grooming),
    ];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    assert_eq!(inputs.absolute.len(), 3);
    assert!(inputs.relative.is_empty());
    for input in &inputs.absolute {
        assert_eq!(input.component, TrustComponentKind::Absolute);
        // 断定ラベルではなく suspected（ClassifierScore）の根拠を同伴する。
        assert_eq!(input.basis, Basis::ClassifierScore);
    }
}

#[test]
fn general_moderation_feeds_trust_relative_component() {
    // ADR 0028 §2.5 / ADR 0026 §2.3: general（nsfw / spam 等の文化圏依存）の suspected は
    // relation で重み付けされる相対成分に入る。
    let signals = vec![
        classifier_signal("sig-nsfw", SafetyCategory::Nsfw),
        classifier_signal("sig-spam", SafetyCategory::Spam),
        classifier_signal("sig-phishing", SafetyCategory::Phishing),
    ];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    assert_eq!(inputs.relative.len(), 3);
    assert!(inputs.absolute.is_empty());
    for input in &inputs.relative {
        assert_eq!(input.component, TrustComponentKind::Relative);
        assert_eq!(input.basis, Basis::ClassifierScore);
    }
}

// --- expiry（失効 signal は寄与しない） ---

#[test]
fn expired_signal_is_excluded_from_trust_inputs() {
    let signals = vec![
        // NOW より過去に失効 → 除外。
        stored_signal(
            "sig-expired",
            SafetyCategory::Csam,
            Some("2026-07-01T00:00:00Z"),
            None,
        ),
        // 失効前 → 含む。
        stored_signal(
            "sig-live",
            SafetyCategory::Csam,
            Some("2026-08-01T00:00:00Z"),
            None,
        ),
        // expires_at なし → 無期限で含む。
        stored_signal("sig-forever", SafetyCategory::Csam, None, None),
    ];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    let ids: Vec<&str> = inputs
        .absolute
        .iter()
        .map(|input| input.signal_id.as_str())
        .collect();
    assert_eq!(ids, vec!["sig-live", "sig-forever"]);
}

#[test]
fn invalid_expires_at_is_an_error() {
    // 不正な RFC3339 は黙って含めたり落としたりせず Err にする。
    let signals = vec![stored_signal(
        "sig-broken",
        SafetyCategory::Csam,
        Some("not-a-timestamp"),
        None,
    )];
    let error = trust_risk_inputs_from(&signals, NOW).unwrap_err();
    assert!(error.to_string().contains("invalid expires_at"), "{error}");
}

// --- appeal 反映（ADR 0026 §6.2。Disputed = pending / Cleared = accepted） ---

#[test]
fn cleared_appeal_excludes_contribution() {
    let signals = vec![
        stored_signal(
            "sig-cleared",
            SafetyCategory::Csam,
            None,
            Some(AppealStatus::Cleared),
        ),
        stored_signal("sig-kept", SafetyCategory::Csam, None, None),
    ];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    assert_eq!(inputs.absolute.len(), 2);
    assert_eq!(inputs.absolute[0].signal_id, "sig-cleared");
    assert_eq!(inputs.absolute[0].appeal_status, AppealStatus::Cleared);
    assert_eq!(inputs.absolute[1].signal_id, "sig-kept");
}

#[test]
fn disputed_appeal_holds_contribution() {
    // 申し立て中（pending）は寄与据え置き: 含まれたまま、状態を同伴して返る。
    let signals = vec![stored_signal(
        "sig-disputed",
        SafetyCategory::Nsfw,
        None,
        Some(AppealStatus::Disputed),
    )];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    assert_eq!(inputs.relative.len(), 1);
    assert_eq!(inputs.relative[0].appeal_status, AppealStatus::Disputed);
}

// --- 根拠つき advisory（断定ラベル化しない） ---

#[test]
fn trust_inputs_carry_basis_confidence_visibility() {
    let signals = vec![stored_signal("sig-1", SafetyCategory::Csam, None, None)];
    let inputs = trust_risk_inputs_from(&signals, NOW).unwrap();
    let input = &inputs.absolute[0];
    // 消費側（#415）が issuer / 根拠 / 有効期限を説明できるよう、根拠フィールドを欠落なく同伴する。
    assert_eq!(input.issuer_node_id, "issuer-node");
    assert_eq!(input.category, SafetyCategory::Csam);
    assert_eq!(input.severity, Severity::High);
    assert_eq!(input.basis, Basis::KnownHashMatch);
    assert_eq!(input.confidence, Some(97));
    assert_eq!(input.visibility, Visibility::Local);
    assert_eq!(input.appeal_status, AppealStatus::None);
    assert_eq!(input.expires_at, None);
}
