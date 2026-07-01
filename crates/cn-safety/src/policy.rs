//! policy router（#353）。provider の scan 結果群から最終 verdict を決める純関数。
//!
//! ADR 0027 `docs/adr/0027-deterministic-moderation-critical-safety.md` §2.2 / §2.3 / §2.4 に従い:
//! - 既知 CSAM hash match → `exclude`（critical / confirmed）
//! - 未知 CSAM / CSE 疑い（classifier score >= threshold）→ `hold` / `quarantine`（critical / suspected）
//! - 一般 moderation（nsfw / spam / malware / phishing）→ critical とは別 route
//! - scan failure / provider unavailable / unscanned → fail-closed（`allow` にしない）
//! - 既知一致なし（`NoKnownMatch`）→ safe と断定しない（`reason_code = NoKnownMatch`）
//!
//! この関数は時計・I/O・乱数を持たない。`scanned_at` は呼び出し側が与える。

use serde::{Deserialize, Serialize};

use crate::capability::SafetyProviderCapability;
use crate::provider::{ProviderScanResult, ScanOutcome};
use crate::verdict::{Basis, ReasonCode, SafetyAction, SafetyCategory, SafetyLabel, SafetyVerdict};

/// router の挙動を決める policy。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SafetyPolicy {
    pub policy_version: String,
    /// scan 前 index を許すか（public-node では false 固定）。
    pub index_before_scan: bool,
    /// scan failure / provider unavailable / unscanned 時の action（fail-closed）。`Allow` にしない。
    pub on_scan_error: SafetyAction,
    /// 未知 CSAM / CSE 疑いの classifier スコア閾値（0-100）。
    pub unknown_csam_score_threshold: u8,
    /// 未知 CSAM / CSE 疑いに対する action（`Hold` または `Quarantine`）。
    pub suspected_critical_action: SafetyAction,
    /// 一般 nsfw（高信頼）に対する action。
    pub on_high_confidence_nsfw: SafetyAction,
    /// spam に対する action。
    pub on_spam: SafetyAction,
    /// malware / phishing に対する action。
    pub on_malware_phishing: SafetyAction,
    /// public-node が known CSAM provider を必須とするか（readiness で使う。router では参照のみ）。
    pub require_known_csam: bool,
}

impl SafetyPolicy {
    /// public community node の最小既定（fail-closed 寄り）。
    pub fn public_node_default() -> Self {
        Self {
            policy_version: "2026-06-public-node-v1".to_string(),
            index_before_scan: false,
            on_scan_error: SafetyAction::Hold,
            unknown_csam_score_threshold: 80,
            suspected_critical_action: SafetyAction::Quarantine,
            on_high_confidence_nsfw: SafetyAction::Exclude,
            on_spam: SafetyAction::Exclude,
            on_malware_phishing: SafetyAction::Exclude,
            require_known_csam: true,
        }
    }

    fn fail_closed_action(&self) -> SafetyAction {
        // 万一 policy が `Allow`（indexable）を設定していても fail-closed を保証する。
        ensure_non_indexing(self.on_scan_error, SafetyAction::Hold)
    }
}

/// 設定された action が index を許す（`allows_indexing()`）場合に、必ず非 index の fallback へ倒す。
///
/// fail-closed が要求される全経路（scan error / suspected critical / critical 検知の取りこぼし）で
/// 共通して使い、「configured action が indexable なら採用しない」という不変条件を 1 箇所に集約する。
fn ensure_non_indexing(action: SafetyAction, fallback: SafetyAction) -> SafetyAction {
    debug_assert!(
        !fallback.allows_indexing(),
        "fail-closed fallback must not allow indexing"
    );
    if action.allows_indexing() {
        fallback
    } else {
        action
    }
}

/// scan 結果群から最終 verdict を決める。
///
/// `scanned_at` は RFC3339 等の時刻文字列を呼び出し側が与える（router は時計を持たない）。
pub fn route(
    scan_outcomes: &[ProviderScanResult],
    policy: &SafetyPolicy,
    scanned_at: impl Into<String>,
) -> SafetyVerdict {
    let scanned_at = scanned_at.into();
    let base = |action: SafetyAction, reason: ReasonCode, critical: bool| SafetyVerdict {
        action,
        labels: Vec::new(),
        critical,
        reason_code: reason,
        confidence: None,
        provider: None,
        provider_capability: None,
        policy_version: policy.policy_version.clone(),
        scanned_at: scanned_at.clone(),
    };

    // 1. unscanned: scan 結果が無い → fail-closed（allow にしない）。
    if scan_outcomes.is_empty() {
        return base(policy.fail_closed_action(), ReasonCode::Unscanned, false);
    }

    // 2. known CSAM hash match → exclude（critical / confirmed）。
    //    他 provider の失敗があっても confirmed を優先する。
    if let Some(result) = scan_outcomes.iter().find(|r| r.known_hash_match) {
        let mut verdict = base(SafetyAction::Exclude, ReasonCode::CsamConfirmed, true);
        verdict.provider = Some(result.provider.clone());
        verdict.provider_capability = Some(result.capability);
        verdict.confidence = result.score;
        verdict.labels = non_empty_labels(result, SafetyCategory::Csam);
        return verdict;
    }

    // 3. 未知 CSAM / CSE 疑い（critical な検知 かつ effective score >= threshold）。
    if let Some(result) = scan_outcomes
        .iter()
        .filter(|r| r.outcome == ScanOutcome::Completed && is_critical_detection(r))
        .find(|r| {
            effective_critical_score(r).is_some_and(|s| s >= policy.unknown_csam_score_threshold)
        })
    {
        let category = critical_category(result).unwrap_or(SafetyCategory::Csam);
        let reason = if category == SafetyCategory::Cse {
            ReasonCode::CseSuspected
        } else {
            ReasonCode::CsamSuspected
        };
        let mut verdict = base(suspected_action(policy), reason, true);
        verdict.provider = Some(result.provider.clone());
        verdict.provider_capability = Some(result.capability);
        verdict.confidence = effective_critical_score(result);
        verdict.labels = non_empty_labels(result, category);
        return verdict;
    }

    // 4. scan failure / provider unavailable → fail-closed（allow にしない）。
    if let Some(result) = scan_outcomes.iter().find(|r| r.outcome.is_fail_closed()) {
        let reason = match result.outcome {
            ScanOutcome::Unavailable => ReasonCode::ProviderUnavailable,
            _ => ReasonCode::ScanFailed,
        };
        let mut verdict = base(policy.fail_closed_action(), reason, false);
        verdict.provider = Some(result.provider.clone());
        verdict.provider_capability = Some(result.capability);
        return verdict;
    }

    // 5. critical な検知があるが suspected 閾値に達しなかった / score が無いものを
    //    Allow に取りこぼさない（fail-closed）。critical safety を safe と断定しない。
    if let Some(result) = scan_outcomes
        .iter()
        .find(|r| r.outcome == ScanOutcome::Completed && is_critical_detection(r))
    {
        let category = critical_category(result).unwrap_or(SafetyCategory::Csam);
        let reason = if category == SafetyCategory::Cse {
            ReasonCode::CseSuspected
        } else {
            ReasonCode::CsamSuspected
        };
        let action = ensure_non_indexing(policy.suspected_critical_action, SafetyAction::Hold);
        let mut verdict = base(action, reason, true);
        verdict.provider = Some(result.provider.clone());
        verdict.provider_capability = Some(result.capability);
        verdict.confidence = effective_critical_score(result);
        verdict.labels = non_empty_labels(result, category);
        return verdict;
    }

    // 6. public-node で必須の known CSAM provider 結果が無いなら fail-closed。
    //    general moderation が clean / allow を返せても、known CSAM scan 欠落時は index しない。
    if policy.require_known_csam && !has_known_csam_scan_result(scan_outcomes) {
        return base(
            policy.fail_closed_action(),
            ReasonCode::ProviderUnavailable,
            false,
        );
    }

    // 7. 一般 moderation（critical 以外のラベル）→ critical とは別 route（critical=false）。
    if let Some((result, category)) = scan_outcomes
        .iter()
        .find_map(|r| general_category(r).map(|category| (r, category)))
    {
        let action = general_action(policy, category);
        let mut verdict = base(action, ReasonCode::GeneralModeration, false);
        verdict.provider = Some(result.provider.clone());
        verdict.provider_capability = Some(result.capability);
        verdict.confidence = result.score;
        verdict.labels = non_empty_labels(result, category);
        return verdict;
    }

    // 8. 検知なし。既知一致なし（NoKnownMatch）は safe と断定しない。
    //    すべて Completed かつラベル無しのときのみ Clean とする。
    let has_no_known_match = scan_outcomes
        .iter()
        .any(|r| r.outcome == ScanOutcome::NoKnownMatch);
    let reason = if has_no_known_match {
        ReasonCode::NoKnownMatch
    } else {
        ReasonCode::Clean
    };
    base(SafetyAction::Allow, reason, false)
}

/// suspected critical の action（`Allow`（indexable）は採用せず、必ず非 index に倒す）。
fn suspected_action(policy: &SafetyPolicy) -> SafetyAction {
    ensure_non_indexing(policy.suspected_critical_action, SafetyAction::Quarantine)
}

/// result が critical safety（CSAM / CSE / grooming）の検知か。
///
/// capability か、いずれかのラベル category のどちらかが critical safety なら true。
/// score の有無に依存しない（categorical な検知でも取りこぼさないため）。
fn is_critical_detection(result: &ProviderScanResult) -> bool {
    result.capability.is_critical_safety()
        || result
            .labels
            .iter()
            .any(|l| l.category.is_critical_safety())
}

/// suspected 判定に使う実効スコア。
///
/// `result.score` を優先し、無ければ critical category ラベルの最大 confidence を使う。
/// `score` と label `confidence` が独立フィールドであることによる取りこぼしを防ぐ。
fn effective_critical_score(result: &ProviderScanResult) -> Option<u8> {
    result.score.or_else(|| {
        result
            .labels
            .iter()
            .filter(|l| l.category.is_critical_safety())
            .filter_map(|l| l.confidence)
            .max()
    })
}

/// reason / category 判定に使う critical category。
///
/// まず critical なラベル category を優先し、無ければ capability から導く
/// （`labels.first()` に依存せず、CSE を CSAM と取り違えない）。
fn critical_category(result: &ProviderScanResult) -> Option<SafetyCategory> {
    result
        .labels
        .iter()
        .map(|l| l.category)
        .find(|c| c.is_critical_safety())
        .or_else(|| critical_category_for_capability(result.capability))
}

/// critical capability から代表 category を導く。
fn critical_category_for_capability(
    capability: SafetyProviderCapability,
) -> Option<SafetyCategory> {
    match capability {
        SafetyProviderCapability::KnownCsamHashMatch
        | SafetyProviderCapability::PerceptualHashMatch
        | SafetyProviderCapability::NovelCsamImageClassifier
        | SafetyProviderCapability::NovelCsamVideoClassifier => Some(SafetyCategory::Csam),
        SafetyProviderCapability::CseTextClassifier => Some(SafetyCategory::Cse),
        SafetyProviderCapability::GroomingTextClassifier => Some(SafetyCategory::Grooming),
        _ => None,
    }
}

/// mandatory known CSAM provider の scan 結果が含まれているか。
fn has_known_csam_scan_result(scan_outcomes: &[ProviderScanResult]) -> bool {
    scan_outcomes
        .iter()
        .any(|r| r.capability == SafetyProviderCapability::KnownCsamHashMatch)
}

/// result が一般 moderation（critical 以外）のラベルを持つなら、その代表カテゴリを返す。
fn general_category(result: &ProviderScanResult) -> Option<SafetyCategory> {
    if result.outcome != ScanOutcome::Completed {
        return None;
    }
    result
        .labels
        .iter()
        .map(|l| l.category)
        .find(|c| !c.is_critical_safety())
}

/// 一般カテゴリに対する action を policy から選ぶ。
fn general_action(policy: &SafetyPolicy, category: SafetyCategory) -> SafetyAction {
    match category {
        SafetyCategory::Spam => policy.on_spam,
        SafetyCategory::Malware | SafetyCategory::Phishing => policy.on_malware_phishing,
        // nsfw / その他一般。
        _ => policy.on_high_confidence_nsfw,
    }
}

/// result のラベルを返す。空なら category から最小ラベルを補う。
fn non_empty_labels(result: &ProviderScanResult, category: SafetyCategory) -> Vec<SafetyLabel> {
    if result.labels.is_empty() {
        let mut label = SafetyLabel::new(category).with_provider_capability(result.capability);
        if let Some(score) = result.score {
            label = label.with_confidence(score);
        }
        vec![label]
    } else {
        result.labels.clone()
    }
}

// Basis を verdict に直接は載せていない（verdict は reason_code を持つ）。Basis は
// moderation event / risk signal 側で使う。ここでは router が決めた reason_code から
// 後段が basis を導出できるよう、対応関係を関数で提供する。
/// reason_code から対応する基準（basis）を導く補助。
pub fn basis_for_reason(reason: ReasonCode) -> Basis {
    match reason {
        ReasonCode::CsamConfirmed => Basis::KnownHashMatch,
        ReasonCode::CsamSuspected | ReasonCode::CseSuspected => Basis::ClassifierScore,
        ReasonCode::GeneralModeration => Basis::ProviderVerdict,
        _ => Basis::LocalPolicy,
    }
}
