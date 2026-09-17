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
/// relation 値の重みの下限（これ未満の proximity は 0 とする）の env 変数名（ADR 0026 §8.2）。
pub const ENV_RELATION_MIN_WEIGHT: &str = "COMMUNITY_NODE_TRUST_RELATION_MIN_WEIGHT";
/// relation 値の集約に使う観測の上位件数の env 変数名。
pub const ENV_RELATION_TOP_K: &str = "COMMUNITY_NODE_TRUST_RELATION_TOP_K";
/// relation 値の減点の倍率の env 変数名。
pub const ENV_RELATION_PENALTY_SCALE: &str = "COMMUNITY_NODE_TRUST_RELATION_PENALTY_SCALE";
/// block 観測の強度の env 変数名。
pub const ENV_BLOCK_STRENGTH: &str = "COMMUNITY_NODE_TRUST_BLOCK_STRENGTH";
/// mute 観測の強度の env 変数名。
pub const ENV_MUTE_STRENGTH: &str = "COMMUNITY_NODE_TRUST_MUTE_STRENGTH";
/// 非表示推奨の閾値（node-local 表示 policy）の env 変数名（ADR 0026 §8.4）。
pub const ENV_HIDE_THRESHOLD: &str = "COMMUNITY_NODE_TRUST_HIDE_THRESHOLD";
/// 評価結果の再利用期限（秒）の env 変数名。
pub const ENV_EVALUATION_TTL_SECONDS: &str = "COMMUNITY_NODE_TRUST_EVALUATION_TTL_SECONDS";

/// trust 合成のパラメータ（operator 可変, ADR 0026 §6.2）。
#[derive(Clone, Debug, PartialEq)]
pub struct TrustParams {
    /// 絶対成分がマイナス（CSAM など確定的に排除すべき）のときの重み。distrust を支配的にし、
    /// 相対指標（文化依存）が良好でも薄まらないようにする。初期値 2.0。
    pub w_abs_negative: f64,
    /// 絶対成分が 0 以上のときの重み。初期値 1.0。
    pub w_abs_positive: f64,
    /// 相対成分・node-local 観測の半減期（日）。絶対成分は evidence / 検知ベースのため
    /// 減衰させない（§6.2）。初期値 30 日。ブロック / ミュート観測の減衰にも使う（§8.2）。
    pub relative_half_life_days: f64,
    /// relation 値の重みの下限。`[0, 1]`。初期値 0.1。
    pub relation_min_weight: f64,
    /// relation 値の集約に使う観測の上位件数。1 以上。初期値 5。
    pub relation_top_k: usize,
    /// relation 値の減点の倍率。`(0, 1]`。初期値 1.0。
    pub relation_penalty_scale: f64,
    /// block 観測の強度。`(0, 1]`。初期値 1.0。
    pub block_strength: f64,
    /// mute 観測の強度。`(0, 1]`。初期値 0.5。
    pub mute_strength: f64,
    /// 非表示推奨の閾値（`trust <= hide_threshold`）。`[-1, 0)`。初期値 -0.5。
    pub hide_threshold: f64,
    /// 評価結果の再利用期限（秒）。1 以上。初期値 600。
    pub evaluation_ttl_seconds: u32,
}

impl Default for TrustParams {
    fn default() -> Self {
        Self {
            w_abs_negative: 2.0,
            w_abs_positive: 1.0,
            relative_half_life_days: 30.0,
            relation_min_weight: 0.1,
            relation_top_k: 5,
            relation_penalty_scale: 1.0,
            block_strength: 1.0,
            mute_strength: 0.5,
            hide_threshold: -0.5,
            evaluation_ttl_seconds: 600,
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
        if !self.relation_min_weight.is_finite() || !(0.0..=1.0).contains(&self.relation_min_weight)
        {
            bail!(
                "trust param {ENV_RELATION_MIN_WEIGHT} must be within [0, 1], got {}",
                self.relation_min_weight
            );
        }
        for (name, value) in [
            (ENV_RELATION_PENALTY_SCALE, self.relation_penalty_scale),
            (ENV_BLOCK_STRENGTH, self.block_strength),
            (ENV_MUTE_STRENGTH, self.mute_strength),
        ] {
            if !value.is_finite() || value <= 0.0 || value > 1.0 {
                bail!("trust param {name} must be within (0, 1], got {value}");
            }
        }
        if !self.hide_threshold.is_finite() || !(-1.0..0.0).contains(&self.hide_threshold) {
            bail!(
                "trust param {ENV_HIDE_THRESHOLD} must be within [-1, 0), got {}",
                self.hide_threshold
            );
        }
        if self.relation_top_k == 0 {
            bail!("trust param {ENV_RELATION_TOP_K} must be at least 1");
        }
        if self.evaluation_ttl_seconds == 0 {
            bail!("trust param {ENV_EVALUATION_TTL_SECONDS} must be at least 1");
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
            (ENV_RELATION_MIN_WEIGHT, &mut params.relation_min_weight),
            (
                ENV_RELATION_PENALTY_SCALE,
                &mut params.relation_penalty_scale,
            ),
            (ENV_BLOCK_STRENGTH, &mut params.block_strength),
            (ENV_MUTE_STRENGTH, &mut params.mute_strength),
            (ENV_HIDE_THRESHOLD, &mut params.hide_threshold),
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
        if let Some(value) = parse_integer(&lookup, ENV_RELATION_TOP_K)? {
            params.relation_top_k = value;
        }
        if let Some(value) = parse_integer(&lookup, ENV_EVALUATION_TTL_SECONDS)? {
            params.evaluation_ttl_seconds = u32::try_from(value).map_err(|_| {
                anyhow::anyhow!("trust param {ENV_EVALUATION_TTL_SECONDS} is too large: {value}")
            })?;
        }
        params.validate()?;
        Ok(params)
    }

    /// 合算・表示 policy の識別子（ADR 0026 §8.4 `policy_version`）。parameter が同じなら同じ値になる。
    pub fn policy_version(&self) -> String {
        let canonical = format!(
            "v1;{};{};{};{};{};{};{};{};{};{}",
            self.w_abs_negative,
            self.w_abs_positive,
            self.relative_half_life_days,
            self.relation_min_weight,
            self.relation_top_k,
            self.relation_penalty_scale,
            self.block_strength,
            self.mute_strength,
            self.hide_threshold,
            self.evaluation_ttl_seconds,
        );
        let digest = blake3::hash(canonical.as_bytes()).to_hex();
        format!("v1-{}", &digest.as_str()[..16])
    }

    /// プロセス env（`COMMUNITY_NODE_TRUST_*`）からパラメータを組み立てる。
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }
}

fn parse_integer(lookup: &impl Fn(&str) -> Option<String>, name: &str) -> Result<Option<usize>> {
    let Some(raw) = lookup(name) else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<usize>()
        .map(Some)
        .map_err(|_| anyhow::anyhow!("trust param {name} is not a non-negative integer: `{raw}`"))
}
