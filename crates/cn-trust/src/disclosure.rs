//! cross-node pull への開示境界（ADR 0026 §6.3 Decision, #415）。
//!
//! read は pull 型: 他 node がこの CN に問い合わせると、この CN の視点の signal が返る。
//! cross-node pull に対しては **confirmed（basis = known-hash / provider-verdict）な絶対成分
//! のみ**を根拠つき（issuer / basis / confidence / expiry）で応答する。
//!
//! - **相対成分は返さない**（文化依存の指標を node 外へ拡散しない）。
//! - **suspected（classifier / local-policy 由来）は返さない**（誤検知を拡散しない）。
//! - `visibility` は pull へのアクセス範囲: `Local` は決して返さず（既定）、
//!   `SubscribedNodes` は subscriber と認証できた要求者のみ、`Public` は誰でも。
//! - relation は本 module に開示口が **存在しない**（`Local` 固定、
//!   `relation_defaults_local_and_not_cross_node_pullable` の構造的保証）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use kukuri_cn_safety::{AppealStatus, Basis, Visibility};

use crate::inputs::{TrustComponentKind, TrustRiskInput, TrustRiskInputs};
use crate::params::TrustParams;
use crate::read::{TrustBasisEntry, build_trust_read};
use crate::score::UniformRelationWeight;

/// cross-node pull の要求者区分。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullAudience {
    /// 匿名 / 未認証の要求者（`Public` visibility のみ見える）。
    Public,
    /// この CN の subscriber として認証済みの要求者（`SubscribedNodes` 以上が見える）。
    SubscribedNodes,
}

/// cross-node pull への応答（confirmed 絶対成分のみ、根拠つき）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrossNodeTrustDisclosure {
    pub target_id: String,
    /// 開示可能な入力のみから再計算した絶対成分（`[-1, 1]`）。開示対象外の signal の
    /// 存在が値から推測できないよう、全量からではなく開示分から計算する。
    pub absolute: f64,
    /// 計算時刻（RFC3339）。
    pub computed_at: String,
    /// 開示した signal の根拠（issuer / basis / confidence / expiry を同伴）。
    pub basis: Vec<TrustBasisEntry>,
}

/// 入力 1 件が `audience` へ開示可能か（§6.3 の全条件）。
fn disclosable(input: &TrustRiskInput, audience: PullAudience) -> bool {
    if input.appeal_status == AppealStatus::Cleared {
        return false;
    }
    // 絶対成分のみ（相対成分・relation は node-local に閉じる）。
    if input.component != TrustComponentKind::Absolute {
        return false;
    }
    // confirmed のみ（classifier / local-policy 由来の suspected は拡散しない）。
    if !matches!(input.basis, Basis::KnownHashMatch | Basis::ProviderVerdict) {
        return false;
    }
    // visibility = pull へのアクセス範囲。
    match input.visibility {
        Visibility::Local => false,
        Visibility::SubscribedNodes => audience == PullAudience::SubscribedNodes,
        Visibility::Public => true,
    }
}

/// cross-node pull への開示を組み立てる純関数。
///
/// 開示可能な入力だけで絶対成分を再計算する（開示不可の signal が値に影響しない =
/// `Local` signal の存在を pull 応答から推測させない）。相対成分・合成 trust は含めない。
pub fn cross_node_trust_disclosure(
    target_id: &str,
    inputs: &TrustRiskInputs,
    audience: PullAudience,
    now: DateTime<Utc>,
) -> CrossNodeTrustDisclosure {
    let disclosed = TrustRiskInputs {
        absolute: inputs
            .absolute
            .iter()
            .filter(|input| disclosable(input, audience))
            .cloned()
            .collect(),
        relative: Vec::new(),
    };
    // 絶対成分は decay / relation 重み / params 非依存なので、view の absolute / basis を
    // そのまま開示に使える（trust / relative は使わない）。
    let view = build_trust_read(
        target_id,
        &disclosed,
        now,
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    CrossNodeTrustDisclosure {
        target_id: view.target_id,
        absolute: view.absolute,
        computed_at: view.computed_at,
        basis: view.basis,
    }
}
