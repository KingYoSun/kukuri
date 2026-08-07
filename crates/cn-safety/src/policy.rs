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
use crate::verdict::{
    Basis, ReasonCode, SafetyAction, SafetyCategory, SafetyLabel, SafetyVerdict, Visibility,
};

/// router の挙動を決める policy。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SafetyPolicy {
    pub policy_version: String,
    /// scan 前 index を許すか（public-node では false 固定）。
    pub index_before_scan: bool,
    /// scan failure / provider unavailable / unscanned 時の action（fail-closed）。`Allow` にしない。
    pub on_scan_error: SafetyAction,
    /// suspected 判定の classifier スコア閾値（0-100。ADR 0028 §2.2、既定 70 = 0.7）。
    ///
    /// critical（未知 CSAM / CSE）と general の両 route で共通に使う（ADR 0028 §6 の未決事項は
    /// 「共通 1 本」で決定。critical は閾値未満でも規則 6 で fail-closed に取りこぼし防止される）。
    /// operator 可変。旧名 `unknown_csam_score_threshold` も deserialization で受理する。
    #[serde(alias = "unknown_csam_score_threshold")]
    pub suspected_threshold: u8,
    /// suspected（`Basis::ClassifierScore`）risk signal の配布 visibility（ADR 0028 §2.4 / §2.7）。
    ///
    /// 既定は `Local`（安全側）。hard cap ではなく operator が `SubscribedNodes` / `Public` へ
    /// 上げられる。content category を持たない operational fail-closed signal には適用されない
    /// （常に `Local`。適用点は cn-safety-runtime の artifact 生成側）。
    #[serde(default)]
    pub suspected_signal_visibility: Visibility,
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
            policy_version: "2026-07-public-node-v2".to_string(),
            index_before_scan: false,
            on_scan_error: SafetyAction::Hold,
            suspected_threshold: 70,
            suspected_signal_visibility: Visibility::Local,
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

    // 3. provider self-test データ一致（Project Arachnid Shield の `test` classification 等、
    //    #391）。実 CSAM の confirmed（規則2）とは専用 reason で区別しつつ、既知リスト一致で
    //    あるため policy に依らず index には決して入れない（Exclude / critical=false）。
    if let Some(result) = scan_outcomes.iter().find(|r| {
        r.outcome == ScanOutcome::Completed
            && r.labels
                .iter()
                .any(|l| l.category == SafetyCategory::ProviderTest)
    }) {
        let mut verdict = base(SafetyAction::Exclude, ReasonCode::ProviderTestMatch, false);
        verdict.provider = Some(result.provider.clone());
        verdict.provider_capability = Some(result.capability);
        verdict.confidence = result.score;
        verdict.labels = non_empty_labels(result, SafetyCategory::ProviderTest);
        return verdict;
    }

    // 4. 未知 CSAM / CSE 疑い（critical な検知 かつ effective score >= threshold）。
    if let Some(result) = scan_outcomes
        .iter()
        .filter(|r| r.outcome == ScanOutcome::Completed && is_critical_detection(r))
        .find(|r| effective_critical_score(r).is_some_and(|s| s >= policy.suspected_threshold))
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

    // 5. scan failure / provider unavailable → fail-closed（allow にしない）。
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

    // 6. critical な検知があるが suspected 閾値に達しなかった / score が無いものを
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

    // 7. public-node で必須の known CSAM provider 結果が無いなら fail-closed。
    //    general moderation が clean / allow を返せても、known CSAM scan 欠落時は index しない。
    if policy.require_known_csam && !has_known_csam_scan_result(scan_outcomes) {
        return base(
            policy.fail_closed_action(),
            ReasonCode::ProviderUnavailable,
            false,
        );
    }

    // 8. 一般 moderation（critical 以外のラベル）→ critical とは別 route（critical=false）。
    //    classifier が score / confidence を返す検知は suspected 閾値以上のときのみ発火する
    //    （ADR 0028 §2.2。VLM が全 media に低スコアのラベルを付けても index を塞がない）。
    //    score も confidence も無い categorical な検知は従来どおり発火する。
    if let Some((result, category)) = scan_outcomes.iter().find_map(|r| {
        general_category(r)
            .filter(|_| effective_general_score(r).is_none_or(|s| s >= policy.suspected_threshold))
            .map(|category| (r, category))
    }) {
        let action = general_action(policy, category);
        let mut verdict = base(action, ReasonCode::GeneralModeration, false);
        verdict.provider = Some(result.provider.clone());
        verdict.provider_capability = Some(result.capability);
        verdict.confidence = result.score;
        verdict.labels = non_empty_labels(result, category);
        return verdict;
    }

    // 9. 検知なし。既知一致なし（NoKnownMatch）は safe と断定しない。
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
/// critical label があれば categorical な検知として扱う。label を持たない provider との互換性の
/// ため、critical capability と score の組み合わせも検知として扱うが、capability だけでは検知と
/// みなさない。`Completed` + label/score なしは classifier の clean 応答だからである。
fn is_critical_detection(result: &ProviderScanResult) -> bool {
    (result.capability.is_critical_safety() && result.score.is_some())
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

/// general route の閾値判定に使う実効スコア。
///
/// `result.score` を優先し、無ければ non-critical category ラベルの最大 confidence を使う。
/// どちらも無い（categorical な検知）場合は `None` を返し、呼び出し側は閾値 gating を
/// 適用せずに発火させる（既存の categorical 検知を取りこぼさない）。
fn effective_general_score(result: &ProviderScanResult) -> Option<u8> {
    result.score.or_else(|| {
        result
            .labels
            .iter()
            .filter(|l| !l.category.is_critical_safety())
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
        // provider self-test 一致は route() 規則3で先に処理されるが、万一ここへ落ちても
        // policy に依らず exclude（index に入れない）に倒す（防御の重ね）。
        SafetyCategory::ProviderTest => SafetyAction::Exclude,
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
// moderation event / risk signal 側で使う。ここでは router が決めた verdict から
// 後段が basis を導出できるよう、対応関係を関数で提供する。
/// verdict から対応する判定根拠（basis）を導く補助（ADR 0027 §2.2）。
///
/// - confirmed（`CsamConfirmed`）は capability で分ける: 完全一致
///   （`KnownCsamHashMatch`）は `KnownHashMatch`、それ以外（perceptual near match 等の
///   provider 断定）は `ProviderVerdict`（§2.7「near match は exact ではない」との整合）。
/// - 一般判定（`GeneralModeration`）は provider の分類器由来なので `ClassifierScore`
///   （`ProviderVerdict` にしない — confirmed 相当の根拠を分類結果に付けない）。
pub fn basis_for_verdict(verdict: &SafetyVerdict) -> Basis {
    match verdict.reason_code {
        ReasonCode::CsamConfirmed => {
            if verdict.provider_capability == Some(SafetyProviderCapability::KnownCsamHashMatch) {
                Basis::KnownHashMatch
            } else {
                Basis::ProviderVerdict
            }
        }
        ReasonCode::CsamSuspected | ReasonCode::CseSuspected | ReasonCode::GeneralModeration => {
            Basis::ClassifierScore
        }
        _ => Basis::LocalPolicy,
    }
}
