//! verdict から未署名 moderation artifact を生成する規則（#353 段階3b）。
//!
//! `cn-safety` の `route()` が返した `SafetyVerdict` を、未署名・未永続化の
//! `ModerationEventBody` / `SafetyRiskSignal` に写像する純粋ロジック。署名（secp256k1）と
//! 永続化は後続段階で行うため、ここでは body / signal を組み立てるだけで `SignedModerationEvent`
//! は作らない。
//!
//! 設計の真実源:
//! - `docs/adr/0027-deterministic-moderation-critical-safety.md` §2.5 / §2.6（signed events / advisory / visibility 規則）
//!
//! 生成ガードレール:
//! - indexable（`allow`）な verdict では artifact を生成しない。
//! - target（subject_kind / subject_id）が欠けている場合は artifact を生成しない
//!   （空 target_id / 不明 target_type の moderation event は監査上危険なため）。
//! - operational fail-closed（scan_failed / provider_unavailable / unscanned）は content の
//!   safety category を示さないため、risk signal を生成しない（虚偽の risk label を作らない）。
//! - suspected（`ClassifierScore`）advisory の visibility は既定 `Local`（誤検知を public に
//!   拡散しない）。ただし hard cap ではなく、`SafetyPolicy::suspected_signal_visibility` で
//!   operator が配布範囲を上げられる（ADR 0028 §2.4 / §2.7。誤検知は §2.8 の appeal で是正）。

use kukuri_cn_safety::policy::basis_for_verdict;
use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety::{
    AppealStatus, Basis, ModerationAction, ModerationEventBody, ReasonCode, RiskSignalTarget,
    SafetyAction, SafetyCategory, SafetyPolicy, SafetyRiskSignal, SafetyVerdict, Severity,
    Visibility,
};

use crate::id::EventIdGenerator;

/// verdict から未署名の moderation event / risk signal を生成する。
///
/// `issuer_node_id` と `scanned_at` は orchestrator が供給する。`ids` は event id 生成器。
/// `policy` は suspected（`ClassifierScore`）advisory の visibility override に使う。
pub(crate) fn build_artifacts(
    verdict: &SafetyVerdict,
    request: &ProviderScanRequest,
    issuer_node_id: &str,
    ids: &dyn EventIdGenerator,
    policy: &SafetyPolicy,
) -> (Option<ModerationEventBody>, Option<SafetyRiskSignal>) {
    // indexable（allow）な verdict では moderation artifact を作らない。
    if verdict.is_indexable() {
        return (None, None);
    }

    // target が揃っていない場合は artifact を作らない（空 target_id を作らない）。
    let (Some(subject_kind), Some(subject_id)) =
        (request.subject_kind, request.subject_id.as_deref())
    else {
        return (None, None);
    };
    let subject_id = subject_id.trim();
    if subject_id.is_empty() {
        return (None, None);
    }

    let basis = basis_for_verdict(verdict);
    let category = primary_category(verdict);
    let severity = severity_for(verdict);

    let moderation_event = build_event(
        verdict,
        subject_kind,
        subject_id,
        issuer_node_id,
        ids,
        basis,
        severity,
        category,
        policy,
    );
    let risk_signal = build_risk_signal(
        verdict,
        subject_kind,
        subject_id,
        basis,
        severity,
        category,
        policy,
    );

    (moderation_event, risk_signal)
}

/// verdict の主要 content category を導く。
///
/// critical reason は reason_code を優先する。provider が複数 label を返し、先頭が一般 label
/// （例: `nsfw`）でも、`cse_suspected` を一般 moderation として扱わないため。
/// general moderation は最初の non-critical label を使う。operational fail-closed
/// （scan_failed / provider_unavailable / unscanned）や NoKnownMatch / Clean は content の
/// category を持たないため `None`。
fn primary_category(verdict: &SafetyVerdict) -> Option<SafetyCategory> {
    match verdict.reason_code {
        ReasonCode::CsamConfirmed | ReasonCode::CsamSuspected => Some(SafetyCategory::Csam),
        ReasonCode::CseSuspected => Some(SafetyCategory::Cse),
        ReasonCode::GeneralModeration => verdict
            .labels
            .iter()
            .map(|label| label.category)
            .find(|category| !category.is_critical_safety()),
        // provider self-test 一致（#391）。非 critical だが exclude されるため、event / signal の
        // category は専用の ProviderTest を使う（csam と混同させない）。
        ReasonCode::ProviderTestMatch => Some(SafetyCategory::ProviderTest),
        ReasonCode::ScanFailed
        | ReasonCode::ProviderUnavailable
        | ReasonCode::Unscanned
        | ReasonCode::NoKnownMatch
        | ReasonCode::Clean => None,
    }
}

/// verdict から severity を導く。
fn severity_for(verdict: &SafetyVerdict) -> Severity {
    if verdict.critical {
        return Severity::Critical;
    }
    match verdict.action {
        SafetyAction::Exclude | SafetyAction::Quarantine => Severity::High,
        SafetyAction::Hold => Severity::Medium,
        // allow は呼び出し前に弾かれている。安全側に倒す。
        SafetyAction::Allow => Severity::Low,
    }
}

/// visibility を導く。
///
/// - suspected（`Basis::ClassifierScore`）かつ content category がある advisory は
///   `policy.suspected_signal_visibility` に従う（ADR 0028 §2.4 / §2.7: 既定は `Local` で
///   安全側だが hard cap ではなく、operator が `SubscribedNodes` / `Public` へ上げられる）。
/// - それ以外の content category ありは `SafetyRiskSignal::default_visibility_for` の規則
///   （confirmed のみ `SubscribedNodes` 以上）。
/// - operational fail-closed（category 無し）は常に `Local`（policy でも上書きできない）。
fn visibility_for(
    category: Option<SafetyCategory>,
    basis: Basis,
    policy: &SafetyPolicy,
) -> Visibility {
    match category {
        Some(_) if basis == Basis::ClassifierScore => policy.suspected_signal_visibility,
        Some(category) => SafetyRiskSignal::default_visibility_for(category, basis),
        None => Visibility::Local,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_event(
    verdict: &SafetyVerdict,
    subject_kind: SubjectKind,
    subject_id: &str,
    issuer_node_id: &str,
    ids: &dyn EventIdGenerator,
    basis: Basis,
    severity: Severity,
    category: Option<SafetyCategory>,
    policy: &SafetyPolicy,
) -> Option<ModerationEventBody> {
    let action = moderation_action_for(verdict.action)?;
    Some(ModerationEventBody {
        id: ids.next_id(),
        issuer_node_id: issuer_node_id.to_string(),
        target_type: subject_kind,
        target_id: subject_id.to_string(),
        action,
        labels: verdict.labels.clone(),
        reason_code: verdict.reason_code,
        severity,
        confidence: verdict.confidence,
        basis,
        visibility: visibility_for(category, basis, policy),
        policy_version: verdict.policy_version.clone(),
        created_at: verdict.scanned_at.clone(),
    })
}

#[allow(clippy::too_many_arguments)]
fn build_risk_signal(
    verdict: &SafetyVerdict,
    subject_kind: SubjectKind,
    subject_id: &str,
    basis: Basis,
    severity: Severity,
    category: Option<SafetyCategory>,
    policy: &SafetyPolicy,
) -> Option<SafetyRiskSignal> {
    // content category が無い operational fail-closed では risk signal を作らない
    // （scan failure は対象 content の safety category を示さない）。
    let category = category?;
    Some(SafetyRiskSignal {
        target: risk_target_for(subject_kind),
        target_id: subject_id.to_string(),
        category,
        severity,
        basis,
        confidence: verdict.confidence,
        visibility: visibility_for(Some(category), basis, policy),
        expires_at: None,
        appeal_status: Some(AppealStatus::None),
    })
}

/// `SafetyAction` を moderation event の `ModerationAction` に写像する。
///
/// `allow` は event を作らない（呼び出し前に弾かれているが安全側に `None`）。
fn moderation_action_for(action: SafetyAction) -> Option<ModerationAction> {
    match action {
        SafetyAction::Hold => Some(ModerationAction::Hold),
        SafetyAction::Quarantine => Some(ModerationAction::Quarantine),
        SafetyAction::Exclude => Some(ModerationAction::Exclude),
        SafetyAction::Allow => None,
    }
}

/// subject kind を risk signal の target 種別に写像する。
fn risk_target_for(kind: SubjectKind) -> RiskSignalTarget {
    match kind {
        SubjectKind::Post => RiskSignalTarget::PostId,
        SubjectKind::Blob => RiskSignalTarget::BlobCid,
        SubjectKind::User => RiskSignalTarget::UserPubkey,
        SubjectKind::Peer => RiskSignalTarget::PeerNode,
    }
}
