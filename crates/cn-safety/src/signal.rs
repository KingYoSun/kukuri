//! trustness / relation risk signal（#353 / ADR 0026 trust-relation, ADR 0027 §2.6 visibility）。
//!
//! trustness / relation には断定ラベルではなく、根拠つき risk signal として反映する。
//! 受け手はこれを使って重み付けを決める（advisory であり command ではない）。
//!
//! 既定の visibility 規則:
//! - suspected unknown CSAM / CSE → `Visibility::Local`（誤検知を public に拡散しない）。
//! - known hash match / provider confirmed の場合のみ `SubscribedNodes` 以上を検討する。

use serde::{Deserialize, Serialize};

use crate::verdict::{Basis, SafetyCategory, Severity, Visibility};

/// risk signal の対象種別。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskSignalTarget {
    UserPubkey,
    PeerNode,
    PostId,
    BlobCid,
}

/// 異議申し立ての状態。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppealStatus {
    #[default]
    None,
    Disputed,
    Cleared,
}

/// 根拠つき risk signal。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SafetyRiskSignal {
    pub target: RiskSignalTarget,
    pub target_id: String,
    pub category: SafetyCategory,
    pub severity: Severity,
    pub basis: Basis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    pub visibility: Visibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appeal_status: Option<AppealStatus>,
}

impl SafetyRiskSignal {
    /// この signal が安全側の既定で issuer node の外へ配布してよいか。
    ///
    /// suspected（`Basis::ClassifierScore`）の critical safety は `Local` 既定。
    /// known hash match / provider confirmed のみ `SubscribedNodes` 以上を許す。
    pub fn default_visibility_for(category: SafetyCategory, basis: Basis) -> Visibility {
        let confirmed = matches!(basis, Basis::KnownHashMatch | Basis::ProviderVerdict);
        if category.is_critical_safety() && !confirmed {
            Visibility::Local
        } else if confirmed {
            Visibility::SubscribedNodes
        } else {
            Visibility::Local
        }
    }
}
