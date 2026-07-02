//! 根拠つき trust read view（ADR 0026 §2.5 / trust-semantics §4）。
//!
//! read は断定ラベルではなく **根拠つき advisory**: 絶対 / 相対成分を分離して返し
//! （`trust_is_not_single_absolute_scalar` / `trust_separates_absolute_and_relative_indicators`）、
//! 寄与 signal ごとに issuer / basis / confidence / visibility / expiry / appeal と
//! 実効寄与（decay / relation 重み込み）を説明できる形にする
//! （`trust_read_is_explainable_with_basis`）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use kukuri_cn_safety::{AppealStatus, Basis, SafetyCategory, Severity, Visibility};

use crate::inputs::{TrustComponentKind, TrustRiskInput, TrustRiskInputs};
use crate::params::TrustParams;
use crate::score::{
    RelationWeighting, clamp_unit, compose_trust, contributes, decay_factor, signal_contribution,
};

/// 寄与 signal 1 件の説明（basis entry）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrustBasisEntry {
    pub signal_id: String,
    pub issuer_node_id: String,
    pub component: TrustComponentKind,
    pub category: SafetyCategory,
    pub severity: Severity,
    pub basis: Basis,
    pub confidence: Option<u8>,
    pub visibility: Visibility,
    pub appeal_status: AppealStatus,
    pub expires_at: Option<String>,
    /// 生寄与（severity × confidence。decay / relation 重み前）。
    pub raw_contribution: f64,
    /// 適用された時間減衰係数（絶対成分は常に 1.0 = 減衰しない）。
    pub decay_factor: f64,
    /// 適用された relation 重み（絶対成分は常に 1.0 = relation 非依存）。
    pub relation_weight: f64,
    /// 実効寄与（raw × decay × relation 重み）。
    pub contribution: f64,
}

/// trust read の応答形（node-local advisory）。
///
/// 単一の絶対スカラーではなく、絶対成分（viewer 非依存）+ 相対成分（viewer / cluster 依存）+
/// 合成値を根拠つきで返す。「このユーザーは troll」等の断定ラベルは持たない（§6.2 Decision:
/// 断定閾値を置かない）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrustReadView {
    /// 対象 pubkey。
    pub target_id: String,
    /// 絶対成分（`[-1, 1]`。relation 非依存・減衰なし・viewer 非依存）。
    pub absolute: f64,
    /// 相対成分（`[-1, 1]`。relation 重み付け・半減期減衰・viewer / cluster 相対）。
    pub relative: f64,
    /// 合成 trust（`[-1, 1]` にクランプ済み）。
    pub trust: f64,
    /// 合成に適用された絶対成分の重み（operator 可変, §6.2）。
    pub w_abs_applied: f64,
    /// 計算時刻（RFC3339。相対成分の decay 基準時刻）。
    pub computed_at: String,
    /// 寄与 signal ごとの根拠。
    pub basis: Vec<TrustBasisEntry>,
}

fn basis_entry(input: &TrustRiskInput, decay: f64, relation_weight: f64) -> TrustBasisEntry {
    let raw = signal_contribution(input);
    TrustBasisEntry {
        signal_id: input.signal_id.clone(),
        issuer_node_id: input.issuer_node_id.clone(),
        component: input.component,
        category: input.category,
        severity: input.severity,
        basis: input.basis,
        confidence: input.confidence,
        visibility: input.visibility,
        appeal_status: input.appeal_status,
        expires_at: input.expires_at.clone(),
        raw_contribution: raw,
        decay_factor: decay,
        relation_weight,
        contribution: raw * decay * relation_weight,
    }
}

/// 入力（#406 供給契約）から根拠つき trust read view を組み立てる。
///
/// - 絶対成分: `inputs.absolute` の生寄与の総和（decay なし・relation 重みなし）を ±1 にクランプ。
/// - 相対成分: `inputs.relative` の寄与 × 半減期減衰 × relation 重み（`[0,1]` に丸め）の総和を
///   ±1 にクランプ。
/// - 合成: [`compose_trust`]（§6.2 の式 + 最終クランプ）。
/// - `AppealStatus::Cleared` は届いても寄与させない（供給層と二重の防御）。
pub fn build_trust_read(
    target_id: &str,
    inputs: &TrustRiskInputs,
    now: DateTime<Utc>,
    params: &TrustParams,
    relation_weighting: &dyn RelationWeighting,
) -> TrustReadView {
    let mut basis = Vec::new();

    let mut absolute_sum = 0.0;
    for input in &inputs.absolute {
        if !contributes(input) || input.component != TrustComponentKind::Absolute {
            continue;
        }
        // 絶対成分は relation 非依存（重み 1.0 固定）かつ減衰しない（decay 1.0 固定）。
        let entry = basis_entry(input, 1.0, 1.0);
        absolute_sum += entry.contribution;
        basis.push(entry);
    }

    let mut relative_sum = 0.0;
    for input in &inputs.relative {
        if !contributes(input) || input.component != TrustComponentKind::Relative {
            continue;
        }
        let decay = decay_factor(input.persisted_at, now, params.relative_half_life_days);
        let weight = relation_weighting.weight_for(input).clamp(0.0, 1.0);
        let entry = basis_entry(input, decay, weight);
        relative_sum += entry.contribution;
        basis.push(entry);
    }

    let absolute = clamp_unit(absolute_sum);
    let relative = clamp_unit(relative_sum);
    let composed = compose_trust(params, absolute, relative);

    TrustReadView {
        target_id: target_id.to_string(),
        absolute,
        relative,
        trust: composed.trust,
        w_abs_applied: composed.w_abs_applied,
        computed_at: now.to_rfc3339(),
        basis,
    }
}
