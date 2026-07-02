//! trust / relation reads への risk signal 供給（#406 → #415）。
//!
//! 永続化された risk signal（`cn_safety.risk_signals`、#405）を、trust / relation の read が
//! 消費できる **根拠つき入力** として返す。ADR 0026 §2.7 に従い、signal の category で
//! trust の **絶対成分**（CSAM 等 critical safety。relation 非依存・report-bomb 不動）と
//! **相対成分**（nsfw / spam 等。relation 重み付け・viewer 相対）へ振り分ける。
//!
//! ここは供給契約のみを担う。trust の合成式（ADR 0026 §6.2）・relation の cluster proximity・
//! read surface（`CommunityLocalTrust`）本体は #415 が実装し、本モジュールの
//! [`list_trust_risk_inputs`] を入力として消費する。
//!
//! 断定ラベルにしない（ADR 0026 §2.5 / trust-semantics）: 入力は必ず basis / confidence /
//! visibility / expiry / appeal を同伴し、read surface が説明可能性を落とせない形にする。
//! cross-node 開示（confirmed 絶対成分のみ、§6.3）は本 API の責務外（node-local read）で、
//! visibility を同伴して返すことで #415 側が開示判定できる。
//!
//! appeal の反映（ADR 0026 §6.2。code の `AppealStatus` は ADR の語彙と次の対応）:
//! - `Disputed`（= ADR の `pending`）: 寄与を**据え置き**。入力に含めたまま返す
//!   （申し立て中に勝手に緩めない）。
//! - `Cleared`（= ADR の `accepted`）: 寄与を**除外**。入力に含めない。
//! - `None`: 確定寄与（ADR の `rejected` 相当を含む、異議が立っていない状態）。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use kukuri_cn_safety::{
    AppealStatus, Basis, RiskSignalTarget, SafetyCategory, Severity, Visibility,
};

use crate::safety_events::{StoredRiskSignal, list_risk_signals_for_target};

/// risk signal category の trust 成分振り分け先（ADR 0026 §2.7）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustComponentKind {
    /// 絶対成分: critical safety（CSAM / CSE / grooming）。relation で重み付けせず、
    /// 通報数で動かない（report-bomb 不動）。時間減衰もしない（§6.2、scoring は #415）。
    Absolute,
    /// 相対成分: nsfw / spam 等の community / 文化圏依存の指標。relation で重み付けし、
    /// viewer / cluster 相対で扱う。半減期減衰する（§6.2、scoring は #415）。
    Relative,
}

/// category → trust 成分の振り分け（初期規則。operator 可変化は #415）。
///
/// critical safety（`SafetyCategory::is_critical_safety()` = Csam / Cse / Grooming）は絶対成分、
/// それ以外（nsfw / spam / malware / phishing 等）は相対成分（ADR 0026 §2.7）。
pub fn trust_component_for(category: SafetyCategory) -> TrustComponentKind {
    if category.is_critical_safety() {
        TrustComponentKind::Absolute
    } else {
        TrustComponentKind::Relative
    }
}

/// trust / relation read への入力 1 件（根拠つき advisory）。
///
/// 断定ラベルではない。basis / confidence / visibility / expiry / appeal を必ず同伴し、
/// 消費側（#415）が issuer / 根拠 / 有効期限を説明できる状態を保つ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustRiskInput {
    /// 永続化側が採番した risk signal id。
    pub signal_id: String,
    /// この signal を発行した issuer node。
    pub issuer_node_id: String,
    /// 振り分け先の trust 成分。
    pub component: TrustComponentKind,
    pub category: SafetyCategory,
    pub severity: Severity,
    pub basis: Basis,
    pub confidence: Option<u8>,
    /// cross-node 開示の判定材料（§6.3。開示判定は消費側の責務）。
    pub visibility: Visibility,
    /// `Disputed`（= pending）は寄与据え置きのまま含まれる。消費側が状態を説明できるよう同伴する。
    pub appeal_status: AppealStatus,
    pub expires_at: Option<String>,
    pub persisted_at: DateTime<Utc>,
}

/// 対象 1 つ分の trust 入力（絶対 / 相対に振り分け済み）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrustRiskInputs {
    /// 絶対成分への入力（critical safety）。
    pub absolute: Vec<TrustRiskInput>,
    /// 相対成分への入力（relation 重み付けの対象）。
    pub relative: Vec<TrustRiskInput>,
}

impl TrustRiskInputs {
    /// 入力が 1 件も無いか。
    pub fn is_empty(&self) -> bool {
        self.absolute.is_empty() && self.relative.is_empty()
    }
}

/// 永続化済み risk signal 列から trust 入力を組み立てる純関数。
///
/// - `expires_at` が `now_rfc3339` 時点で失効した signal は除外する（`expires_at` が無い signal は
///   無期限で残る。相対成分の半減期 decay 本体は #415）。
/// - `AppealStatus::Cleared`（= accepted）は寄与除外。`Disputed`（= pending）は据え置きで含む。
/// - category は [`trust_component_for`] で絶対 / 相対へ振り分ける。
///
/// 不正な RFC3339（`now_rfc3339` / `expires_at`）は Err（黙って含めたり落としたりしない）。
pub fn trust_risk_inputs_from(
    signals: &[StoredRiskSignal],
    now_rfc3339: &str,
) -> Result<TrustRiskInputs> {
    let now = DateTime::parse_from_rfc3339(now_rfc3339)
        .with_context(|| format!("invalid now timestamp `{now_rfc3339}` (expected RFC3339)"))?;

    let mut inputs = TrustRiskInputs::default();
    for stored in signals {
        let signal = &stored.signal;

        if let Some(expires_at) = signal.expires_at.as_deref() {
            let expires = DateTime::parse_from_rfc3339(expires_at).with_context(|| {
                format!(
                    "risk signal `{}` has invalid expires_at `{expires_at}` (expected RFC3339)",
                    stored.id
                )
            })?;
            if expires <= now {
                continue;
            }
        }

        let appeal_status = signal.appeal_status.unwrap_or_default();
        if appeal_status == AppealStatus::Cleared {
            // accepted された異議は寄与から除外する（ADR 0026 §6.2）。
            continue;
        }

        let component = trust_component_for(signal.category);
        let input = TrustRiskInput {
            signal_id: stored.id.clone(),
            issuer_node_id: stored.issuer_node_id.clone(),
            component,
            category: signal.category,
            severity: signal.severity,
            basis: signal.basis,
            confidence: signal.confidence,
            visibility: signal.visibility,
            appeal_status,
            expires_at: signal.expires_at.clone(),
            persisted_at: stored.persisted_at,
        };
        match component {
            TrustComponentKind::Absolute => inputs.absolute.push(input),
            TrustComponentKind::Relative => inputs.relative.push(input),
        }
    }
    Ok(inputs)
}

/// 対象ごとの trust 入力 read（node-local）。
///
/// `list_risk_signals_for_target`（visibility を問わない node-local 参照）と
/// [`trust_risk_inputs_from`] の合成。#415 の trust 絶対成分 / relation 相対重み付けが
/// これを消費する。
pub async fn list_trust_risk_inputs(
    pool: &PgPool,
    target: RiskSignalTarget,
    target_id: &str,
    now_rfc3339: &str,
) -> Result<TrustRiskInputs> {
    let signals = list_risk_signals_for_target(pool, target, target_id).await?;
    trust_risk_inputs_from(&signals, now_rfc3339)
}
