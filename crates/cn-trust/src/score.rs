//! trust scoring の純関数群（ADR 0026 §6.2）。
//!
//! - 絶対成分: relation 非依存・**減衰しない**・viewer 非依存（`trust_absolute_component_does_not_decay`）。
//! - 相対成分: relation 重み付け + 半減期減衰（`trust_relative_component_decays_over_time`）。
//! - 合成: `w_abs = w_abs_negative if absolute < 0 else w_abs_positive`、
//!   `trust = clamp(-1, 1, (w_abs * absolute + relative) / 2)`（`trust_is_clamped_to_unit_interval`）。
//!
//! risk signal は負の evidence として寄与する（severity × confidence）。正の evidence source は
//! 現状存在しないため絶対 / 相対成分の実効値は `[-1, 0]` だが、値域契約は §6.2 どおり ±1 で扱う。

use chrono::{DateTime, Utc};

use kukuri_cn_safety::{AppealStatus, Severity};

use crate::inputs::TrustRiskInput;
use crate::params::TrustParams;

/// 値を `[-1, 1]` にクランプする（成分・最終値の共通契約, §6.2）。
pub fn clamp_unit(value: f64) -> f64 {
    value.clamp(-1.0, 1.0)
}

/// severity → 寄与の大きさ（初期決め打ち mapping。プラン Assumption 8）。
///
/// operator 可変化は `w_abs` / 半減期を優先し、この mapping の可変化は後続に残す。
pub fn severity_magnitude(severity: Severity) -> f64 {
    match severity {
        Severity::Critical => 1.0,
        Severity::High => 0.7,
        Severity::Medium => 0.4,
        Severity::Low => 0.2,
    }
}

/// confidence（0..=100）→ 係数。未申告（None）は 1.0（申告なしを理由に薄めない）。
pub fn confidence_factor(confidence: Option<u8>) -> f64 {
    match confidence {
        Some(value) => f64::from(value.min(100)) / 100.0,
        None => 1.0,
    }
}

/// risk signal 1 件の生寄与（decay / relation 重み前）。負の evidence なので常に `<= 0`。
pub fn signal_contribution(input: &TrustRiskInput) -> f64 {
    -(severity_magnitude(input.severity) * confidence_factor(input.confidence))
}

/// 半減期方式の時間減衰係数（§6.2。相対成分にのみ適用する）。
///
/// `factor = 0.5^(age_days / half_life_days)`。`persisted_at` が未来（クロックずれ）の場合は
/// age を 0 に切り上げ、減衰なし（1.0）とする。
pub fn decay_factor(persisted_at: DateTime<Utc>, now: DateTime<Utc>, half_life_days: f64) -> f64 {
    let age_seconds = (now - persisted_at).num_seconds().max(0) as f64;
    let age_days = age_seconds / 86_400.0;
    0.5_f64.powf(age_days / half_life_days)
}

/// 相対成分入力への relation 重み付け（ADR 0026 §2.3 / §2.7 の seam）。
///
/// 重みは `[0, 1]` に丸めて使う。foundation では相対成分入力は本 node の scan 由来
/// （observer なし）のみなので production は [`UniformRelationWeight`]（= 1.0）だが、
/// observer-attributed 観測（`ObservedSignal`）の producer が実装され次第、observer の
/// cluster 近接度から重みを導出する実装に差し替える（report-bombing 耐性の効き先）。
pub trait RelationWeighting {
    fn weight_for(&self, input: &TrustRiskInput) -> f64;
}

/// 一様な relation 重み。
#[derive(Clone, Copy, Debug)]
pub struct UniformRelationWeight(pub f64);

impl Default for UniformRelationWeight {
    fn default() -> Self {
        Self(1.0)
    }
}

impl RelationWeighting for UniformRelationWeight {
    fn weight_for(&self, _input: &TrustRiskInput) -> f64 {
        self.0
    }
}

/// 合成結果（適用された `w_abs` を説明のため同伴する）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComposedTrust {
    /// 適用された絶対成分の重み。
    pub w_abs_applied: f64,
    /// 最終 trust 値（`[-1, 1]` にクランプ済み）。
    pub trust: f64,
}

/// §6.2 の合成式。成分はそれぞれ ±1 にクランプしてから合成し、最終値も ±1 にクランプする。
///
/// 絶対成分がマイナスのとき `w_abs_negative`（初期 2.0）で distrust を支配的にする
/// （`trust_absolute_negative_is_weighted_double`）。クランプにより read 値域は対称に保たれる
/// （`trust_is_clamped_to_unit_interval`）。
pub fn compose_trust(params: &TrustParams, absolute: f64, relative: f64) -> ComposedTrust {
    let absolute = clamp_unit(absolute);
    let relative = clamp_unit(relative);
    let w_abs = if absolute < 0.0 {
        params.w_abs_negative
    } else {
        params.w_abs_positive
    };
    ComposedTrust {
        w_abs_applied: w_abs,
        trust: clamp_unit((w_abs * absolute + relative) / 2.0),
    }
}

/// scoring 層での appeal 防御（ADR 0026 §6.2）。
///
/// `Cleared`（= accepted）は供給層（`trust_risk_inputs_from`）で既に除外されるが、
/// 万一 scoring まで届いても寄与させない（defense in depth）。`Disputed`（= pending）は
/// 寄与据え置きなので含める。
pub(crate) fn contributes(input: &TrustRiskInput) -> bool {
    input.appeal_status != AppealStatus::Cleared
}
