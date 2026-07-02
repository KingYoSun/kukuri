//! operator 可変な trust scoring パラメータ（ADR 0026 §6.2）。
//!
//! 合成式の重み（`w_abs`）と相対成分の半減期は **operator が変更できる**ことが contract
//! （`trust_composition_weights_are_operator_tunable`）。env（`COMMUNITY_NODE_TRUST_*`）から
//! 供給し、未設定は ADR 0026 §6.2 の初期決め打ち値に倒す。cn-operator config への正式節追加は
//! capability 昇格判断時（プラン Assumption 9）。
//!
//! 断定閾値は置かない（§6.2 Decision）: read は連続値 advisory のまま返し、バケット化が
//! 必要になった場合も operator 可変パラメータとして本型に足す（本 foundation では持たない）。

use anyhow::{Result, bail};

/// `w_abs`（絶対成分がマイナスのとき）の env 変数名。
pub const ENV_W_ABS_NEGATIVE: &str = "COMMUNITY_NODE_TRUST_W_ABS_NEGATIVE";
/// `w_abs`（絶対成分が 0 以上のとき）の env 変数名。
pub const ENV_W_ABS_POSITIVE: &str = "COMMUNITY_NODE_TRUST_W_ABS_POSITIVE";
/// 相対成分の半減期（日）の env 変数名。
pub const ENV_RELATIVE_HALF_LIFE_DAYS: &str = "COMMUNITY_NODE_TRUST_RELATIVE_HALF_LIFE_DAYS";

/// trust 合成のパラメータ（operator 可変, ADR 0026 §6.2）。
#[derive(Clone, Debug, PartialEq)]
pub struct TrustParams {
    /// 絶対成分がマイナス（CSAM など確定的に排除すべき）のときの重み。distrust を支配的にし、
    /// 相対指標（文化依存）が良好でも薄まらないようにする。初期値 2.0。
    pub w_abs_negative: f64,
    /// 絶対成分が 0 以上のときの重み。初期値 1.0。
    pub w_abs_positive: f64,
    /// 相対成分・node-local 観測の半減期（日）。絶対成分は evidence / 検知ベースのため
    /// 減衰させない（§6.2）。初期値 30 日。
    pub relative_half_life_days: f64,
}

impl Default for TrustParams {
    fn default() -> Self {
        Self {
            w_abs_negative: 2.0,
            w_abs_positive: 1.0,
            relative_half_life_days: 30.0,
        }
    }
}

impl TrustParams {
    /// パラメータの妥当性検証。重みは正の有限値、半減期は正の有限値のみ許す
    /// （0 や負・NaN は合成式 / decay を壊すため拒否する）。
    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            (ENV_W_ABS_NEGATIVE, self.w_abs_negative),
            (ENV_W_ABS_POSITIVE, self.w_abs_positive),
            (ENV_RELATIVE_HALF_LIFE_DAYS, self.relative_half_life_days),
        ] {
            if !value.is_finite() || value <= 0.0 {
                bail!("trust param {name} must be a positive finite number, got {value}");
            }
        }
        Ok(())
    }

    /// 任意の lookup（env 相当）からパラメータを組み立てる。未設定キーは既定値。
    ///
    /// 不正値（数値でない / 非正 / 非有限）は黙って既定値に倒さず Err にする
    /// （operator の設定ミスを起動時に検出する。fail-closed な設定検証の既存流儀に合わせる）。
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let mut params = Self::default();
        for (name, slot) in [
            (ENV_W_ABS_NEGATIVE, &mut params.w_abs_negative),
            (ENV_W_ABS_POSITIVE, &mut params.w_abs_positive),
            (
                ENV_RELATIVE_HALF_LIFE_DAYS,
                &mut params.relative_half_life_days,
            ),
        ] {
            if let Some(raw) = lookup(name) {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let parsed: f64 = trimmed
                    .parse()
                    .map_err(|_| anyhow::anyhow!("trust param {name} is not a number: `{raw}`"))?;
                *slot = parsed;
            }
        }
        params.validate()?;
        Ok(params)
    }

    /// プロセス env（`COMMUNITY_NODE_TRUST_*`）からパラメータを組み立てる。
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }
}
