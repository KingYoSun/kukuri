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
//! - `Cleared`（= ADR の `accepted`）: 評価への寄与を**除外**し、端末向け説明用に状態を同伴する。
//! - `None`: 確定寄与（ADR の `rejected` 相当を含む、異議が立っていない状態）。

use anyhow::{Context, Result};
use chrono::DateTime;
use sqlx::PgPool;

use kukuri_cn_safety::RiskSignalTarget;
// 型（TrustComponentKind / TrustRiskInput / TrustRiskInputs / trust_component_for）は
// pure domain の cn-trust（#415）へ移した。本 module は永続化 risk signal からの
// 組み立て（供給契約）のみを担う。
use kukuri_cn_trust::{TrustComponentKind, TrustRiskInput, TrustRiskInputs, trust_component_for};

use crate::safety_events::{
    StoredRiskSignal, list_risk_signals_for_target, list_risk_signals_for_user,
};

/// 永続化済み risk signal 列から trust 入力を組み立てる純関数。
///
/// - `expires_at` が `now_rfc3339` 時点で失効した signal は除外する（`expires_at` が無い signal は
///   無期限で残る。相対成分の半減期 decay 本体は #415）。
/// - `AppealStatus::Cleared`（= accepted）は状態を保持し、評価層で寄与ゼロにする。
///   `Disputed`（= pending）は寄与据え置きで含む。
/// - category は [`trust_component_for`] で絶対 / 相対へ振り分ける。
///
/// 不正な RFC3339 の扱い（#700）: `now_rfc3339` はサーバ生成値なので不正なら Err。
/// signal 側の `expires_at` は保存前検証で新規流入を防いだうえで、既存の不正値は
/// 警告ログを残して該当 signal だけを除外する（不正値 1 件で read 全体を失敗させない）。
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
            let Ok(expires) = DateTime::parse_from_rfc3339(expires_at) else {
                tracing::warn!(
                    signal_id = %stored.id,
                    expires_at,
                    "risk signal の expires_at が RFC3339 として解析できないため trust 入力から除外します"
                );
                continue;
            };
            if expires <= now {
                continue;
            }
        }

        let appeal_status = signal.appeal_status.unwrap_or_default();
        let component = trust_component_for(signal.category);
        let input = TrustRiskInput {
            signal_id: stored.id.clone(),
            issuer_node_id: stored.issuer_node_id.clone(),
            target: signal.target,
            target_id: signal.target_id.clone(),
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
    let signals = if target == RiskSignalTarget::UserPubkey {
        list_risk_signals_for_user(pool, target_id).await?
    } else {
        list_risk_signals_for_target(pool, target, target_id).await?
    };
    trust_risk_inputs_from(&signals, now_rfc3339)
}
