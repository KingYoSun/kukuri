//! 根拠つき trust read view（ADR 0026 §2.5 / trust-semantics §4）。
//!
//! read は断定ラベルではなく **根拠つき advisory**: 絶対 / 相対成分を分離して返し
//! （`trust_is_not_single_absolute_scalar` / `trust_separates_absolute_and_relative_indicators`）、
//! 寄与 signal ごとに issuer / basis / confidence / visibility / expiry / appeal と
//! 実効寄与（decay / relation 重み込み）を説明できる形にする
//! （`trust_read_is_explainable_with_basis`）。

use chrono::{DateTime, Utc};
pub use kukuri_cn_protocol::{TrustBasisEntry, TrustReadView};

use crate::inputs::{TrustComponentKind, TrustRiskInput, TrustRiskInputs};
use crate::params::TrustParams;
use crate::score::{
    RelationWeighting, clamp_unit, compose_trust, contributes, decay_factor, signal_contribution,
};

fn basis_entry(
    input: &TrustRiskInput,
    decay: f64,
    relation_weight: f64,
    included: bool,
) -> TrustBasisEntry {
    let raw = signal_contribution(input);
    TrustBasisEntry {
        signal_id: input.signal_id.clone(),
        issuer_node_id: input.issuer_node_id.clone(),
        target: input.target,
        target_id: input.target_id.clone(),
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
        contribution: if included {
            raw * decay * relation_weight
        } else {
            0.0
        },
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
        if input.component != TrustComponentKind::Absolute {
            continue;
        }
        // 絶対成分は relation 非依存（重み 1.0 固定）かつ減衰しない（decay 1.0 固定）。
        let entry = basis_entry(input, 1.0, 1.0, contributes(input));
        if contributes(input) {
            absolute_sum += entry.contribution;
        }
        basis.push(entry);
    }

    let mut relative_sum = 0.0;
    for input in &inputs.relative {
        if input.component != TrustComponentKind::Relative {
            continue;
        }
        let decay = decay_factor(input.persisted_at, now, params.relative_half_life_days);
        let weight = relation_weighting.weight_for(input).clamp(0.0, 1.0);
        let entry = basis_entry(input, decay, weight, contributes(input));
        if contributes(input) {
            relative_sum += entry.contribution;
        }
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
