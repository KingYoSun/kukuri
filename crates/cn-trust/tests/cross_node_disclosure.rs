//! cross-node pull 開示境界（ADR 0026 §6.3, Issue #415）の contract テスト。DB 不要。

use chrono::{DateTime, Utc};

use kukuri_cn_safety::{
    AppealStatus, Basis, RiskSignalTarget, SafetyCategory, Severity, Visibility,
};
use kukuri_cn_trust::{
    PullAudience, TrustComponentKind, TrustRiskInput, TrustRiskInputs, cross_node_trust_disclosure,
};

#[allow(clippy::unwrap_used)] // test fixture helper
fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-07-02T09:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn input(
    id: &str,
    component: TrustComponentKind,
    category: SafetyCategory,
    basis: Basis,
    visibility: Visibility,
) -> TrustRiskInput {
    TrustRiskInput {
        signal_id: id.to_string(),
        issuer_node_id: "issuer-node".to_string(),
        target: RiskSignalTarget::UserPubkey,
        target_id: "pubkey-1".to_string(),
        component,
        category,
        severity: Severity::Critical,
        basis,
        confidence: Some(100),
        visibility,
        appeal_status: AppealStatus::None,
        expires_at: Some("2026-12-31T00:00:00Z".to_string()),
        persisted_at: now(),
    }
}

fn disclosed_ids(inputs: &TrustRiskInputs, audience: PullAudience) -> Vec<String> {
    cross_node_trust_disclosure("pubkey-1", inputs, audience, now())
        .basis
        .iter()
        .map(|e| e.signal_id.clone())
        .collect()
}

#[test]
fn cross_node_pull_discloses_only_confirmed_absolute_component() {
    let inputs = TrustRiskInputs {
        absolute: vec![
            // confirmed（known-hash）+ Public → 開示される唯一の signal。
            input(
                "abs-confirmed-public",
                TrustComponentKind::Absolute,
                SafetyCategory::Csam,
                Basis::KnownHashMatch,
                Visibility::Public,
            ),
            // suspected（classifier 由来）は visibility が Public でも返さない（誤検知を拡散しない）。
            input(
                "abs-suspected-public",
                TrustComponentKind::Absolute,
                SafetyCategory::Csam,
                Basis::ClassifierScore,
                Visibility::Public,
            ),
            // confirmed でも Local は返さない（既定 = 最も安全側）。
            input(
                "abs-confirmed-local",
                TrustComponentKind::Absolute,
                SafetyCategory::Csam,
                Basis::KnownHashMatch,
                Visibility::Local,
            ),
        ],
        relative: vec![
            // 相対成分は confirmed / Public でも決して返さない（文化依存指標を拡散しない）。
            input(
                "rel-confirmed-public",
                TrustComponentKind::Relative,
                SafetyCategory::Nsfw,
                Basis::ProviderVerdict,
                Visibility::Public,
            ),
        ],
    };

    let ids = disclosed_ids(&inputs, PullAudience::Public);
    assert_eq!(ids, vec!["abs-confirmed-public".to_string()]);

    // 開示値は開示分のみから計算する（Local signal の存在が absolute 値から漏れない）。
    let all_public = TrustRiskInputs {
        absolute: vec![input(
            "abs-confirmed-public",
            TrustComponentKind::Absolute,
            SafetyCategory::Csam,
            Basis::KnownHashMatch,
            Visibility::Public,
        )],
        relative: Vec::new(),
    };
    let full = cross_node_trust_disclosure("pubkey-1", &inputs, PullAudience::Public, now());
    let only = cross_node_trust_disclosure("pubkey-1", &all_public, PullAudience::Public, now());
    assert_eq!(full.absolute, only.absolute);

    // 根拠（issuer / basis / confidence / expiry）を同伴する。
    let entry = &full.basis[0];
    assert_eq!(entry.issuer_node_id, "issuer-node");
    assert_eq!(entry.basis, Basis::KnownHashMatch);
    assert_eq!(entry.confidence, Some(100));
    assert_eq!(entry.expires_at, Some("2026-12-31T00:00:00Z".to_string()));
}

#[test]
fn subscribed_nodes_visibility_requires_subscriber_audience() {
    let inputs = TrustRiskInputs {
        absolute: vec![input(
            "abs-confirmed-subscribed",
            TrustComponentKind::Absolute,
            SafetyCategory::Csam,
            Basis::ProviderVerdict,
            Visibility::SubscribedNodes,
        )],
        relative: Vec::new(),
    };
    // 匿名 pull には返らない。
    assert!(disclosed_ids(&inputs, PullAudience::Public).is_empty());
    // subscriber 認証済み pull には返る。
    assert_eq!(
        disclosed_ids(&inputs, PullAudience::SubscribedNodes),
        vec!["abs-confirmed-subscribed".to_string()]
    );
}

#[test]
fn relative_component_never_crosses_node_boundary() {
    // relation そのものに開示口が無いこと（`Local` 固定）は型レベルで保証される
    // （`cross_node_trust_disclosure` は relation を受け取らず、relative は常に落とす）。
    let inputs = TrustRiskInputs {
        absolute: Vec::new(),
        relative: vec![input(
            "rel-any",
            TrustComponentKind::Relative,
            SafetyCategory::Spam,
            Basis::KnownHashMatch,
            Visibility::Public,
        )],
    };
    for audience in [PullAudience::Public, PullAudience::SubscribedNodes] {
        let disclosure = cross_node_trust_disclosure("pubkey-1", &inputs, audience, now());
        assert!(disclosure.basis.is_empty());
        assert_eq!(disclosure.absolute, 0.0);
    }
}
