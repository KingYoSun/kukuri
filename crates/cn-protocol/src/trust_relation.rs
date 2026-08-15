//! trust / relation の共有 wire 契約。

pub use kukuri_cn_safety::{
    AppealStatus, Basis, RiskSignalTarget, SafetyCategory, Severity, Visibility,
};
use serde::{Deserialize, Serialize};

/// risk signal category の trust 成分振り分け先。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TrustComponentKind {
    Absolute,
    Relative,
}

/// 寄与 signal 1 件の説明（basis entry）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct TrustBasisEntry {
    pub signal_id: String,
    pub issuer_node_id: String,
    pub target: RiskSignalTarget,
    pub target_id: String,
    pub component: TrustComponentKind,
    pub category: SafetyCategory,
    pub severity: Severity,
    pub basis: Basis,
    pub confidence: Option<u8>,
    pub visibility: Visibility,
    pub appeal_status: AppealStatus,
    pub expires_at: Option<String>,
    pub raw_contribution: f64,
    pub decay_factor: f64,
    pub relation_weight: f64,
    pub contribution: f64,
}

/// trust read の応答形（node-local advisory）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrustReadView {
    pub target_id: String,
    pub absolute: f64,
    pub relative: f64,
    pub trust: f64,
    pub w_abs_applied: f64,
    pub computed_at: String,
    pub basis: Vec<TrustBasisEntry>,
}

/// viewer を含む per-user trust read wire 応答。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TrustUserReadResponse {
    pub viewer_pubkey: String,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub view: TrustReadView,
}

/// proximity の feature ごとの根拠。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProximityBasisEntry {
    pub feature: String,
    pub value: f64,
    pub weight: f64,
    pub contribution: f64,
}

/// pairwise cluster proximity（根拠つき）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Proximity {
    pub score: f64,
    pub basis: Vec<ProximityBasisEntry>,
}

/// viewer と target を含む pairwise relation read wire 応答。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RelationReadResponse {
    pub viewer_pubkey: String,
    pub target_pubkey: String,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub proximity: Proximity,
}

/// viewer の近接近傍一覧。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RelationNeighborsResponse {
    pub viewer_pubkey: String,
    pub neighbors: Vec<String>,
}

/// 本人向け distance opt-out 状態。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct RelationOptoutResponse {
    pub pubkey: String,
    pub opted_out: bool,
    pub opted_out_at: Option<String>,
    pub min_proximity: f64,
}
