//! verdict から未署名 moderation artifact を生成する規則（#353 段階3b）。
//!
//! `cn-safety` の `route()` が返した `SafetyVerdict` を、未署名・未永続化の
//! `ModerationEventBody` / `SafetyRiskSignal` に写像する純粋ロジック。署名（secp256k1）と
//! 永続化は後続段階で行うため、ここでは body / signal を組み立てるだけで `SignedModerationEvent`
//! は作らない。
//!
//! 設計の真実源:
//! - `docs/safety/community-node-critical-safety.md` §9（advisory / visibility 規則）
//! - `docs/architecture/moderation-event-trust-semantics.md`
//!
//! 生成ガードレール:
//! - indexable（`allow`）な verdict では artifact を生成しない。
//! - target（subject_kind / subject_id）が欠けている場合は artifact を生成しない
//!   （空 target_id / 不明 target_type の moderation event は監査上危険なため）。
//! - operational fail-closed（scan_failed / provider_unavailable / unscanned）は content の
//!   safety category を示さないため、risk signal を生成しない（虚偽の risk label を作らない）。
//! - suspected unknown CSAM / CSE の visibility は既定 `Local`（誤検知を public に拡散しない）。

use kukuri_cn_safety::policy::basis_for_reason;
use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety::{
    AppealStatus, Basis, ModerationAction, ModerationEventBody, ReasonCode, RiskSignalTarget,
    SafetyAction, SafetyCategory, SafetyRiskSignal, SafetyVerdict, Severity, Visibility,
};

use crate::id::EventIdGenerator;

/// verdict から未署名の moderation event / risk signal を生成する。
///
/// `issuer_node_id` と `scanned_at` は orchestrator が供給する。`ids` は event id 生成器。
pub(crate) fn build_artifacts(
    verdict: &SafetyVerdict,
    request: &ProviderScanRequest,
    issuer_node_id: &str,
    ids: &dyn EventIdGenerator,
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

    let basis = basis_for_reason(verdict.reason_code);
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
    );
    let risk_signal =
        build_risk_signal(verdict, subject_kind, subject_id, basis, severity, category);

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
/// content category がある場合は `SafetyRiskSignal::default_visibility_for` の規則に従い、
/// suspected unknown CSAM / CSE は `Local`、confirmed のみ `SubscribedNodes` 以上。
/// operational fail-closed（category 無し）は `Local`。
fn visibility_for(category: Option<SafetyCategory>, basis: Basis) -> Visibility {
    match category {
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
        visibility: visibility_for(category, basis),
        policy_version: verdict.policy_version.clone(),
        created_at: verdict.scanned_at.clone(),
    })
}

fn build_risk_signal(
    verdict: &SafetyVerdict,
    subject_kind: SubjectKind,
    subject_id: &str,
    basis: Basis,
    severity: Severity,
    category: Option<SafetyCategory>,
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
        visibility: visibility_for(Some(category), basis),
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
