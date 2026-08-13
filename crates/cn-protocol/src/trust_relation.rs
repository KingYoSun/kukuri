//! trust / relation の共有 wire 契約。

use serde::{Deserialize, Serialize};

/// 本人向け distance opt-out 状態。
///
/// 既存の `pubkey` / `opted_out` / `opted_out_at` を維持しつつ、判定に使う
/// node-local policy を説明可能にする。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelationOptoutResponse {
    pub pubkey: String,
    pub opted_out: bool,
    pub opted_out_at: Option<String>,
    pub min_proximity: f64,
}
