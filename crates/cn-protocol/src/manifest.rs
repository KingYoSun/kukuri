//! community node manifest(#355/#356)の client 側 slim 表現。
//!
//! dependency 表示に必要なフィールドのみを保持する。未知フィールドは無視し、欠落は
//! default で補う(manifest schema が拡張されても client が壊れない)。
//! サーバ側の生成型(cn-operator)とは別型のままとし、互換は desktop-runtime の
//! round-trip 契約テスト(サーバ実出力 → 本型)で固定する(WP-S3 / WP-H3 Decision 2)。

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeManifest {
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub node_name: String,
    #[serde(default)]
    pub node_role: String,
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub manifest_version: String,
    #[serde(default)]
    pub capability_scope: CommunityNodeCapabilityScope,
    #[serde(default)]
    pub authority_scope: CommunityNodeAuthorityScope,
    #[serde(default)]
    pub p2p_boundary: CommunityNodeP2pBoundary,
    #[serde(default)]
    pub abuse_contact: String,
    /// node が公開する通報受付 endpoint(#310)。未公開の場合は空文字。
    /// client は空なら `abuse_contact` を mailto / copyable contact として案内する。
    #[serde(default)]
    pub report_endpoint: String,
    /// node が公開する権利侵害申出画面。未公開の場合は空文字。
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>"))]
    pub rights_request_url: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>"))]
    pub rights_request_policy_url: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>"))]
    pub rights_request_initial_response_target_days: u32,
    #[serde(default)]
    pub terms_url: String,
    #[serde(default)]
    pub privacy_url: String,
    #[serde(default)]
    pub moderation_policy_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeCapabilityScope {
    #[serde(default)]
    pub available_enabled: Vec<String>,
    #[serde(default)]
    pub planned_enabled: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeAuthorityScope {
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub does_not_apply_to: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeP2pBoundary {
    #[serde(default)]
    pub identity_authority: bool,
    #[serde(default)]
    pub profile_canonical_store: bool,
    #[serde(default)]
    pub social_graph_canonical_store: bool,
    #[serde(default)]
    pub content_truth_source: bool,
    #[serde(default)]
    pub network_wide_authority: bool,
}
