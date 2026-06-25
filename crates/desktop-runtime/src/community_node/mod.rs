use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use kukuri_cn_core::{
    AuthChallengeResponse, AuthVerifyResponse, BootstrapHeartbeatResponse,
    CommunityNodeConsentStatus, CommunityNodeResolvedUrls, CommunityNodeSeedPeer,
    build_auth_envelope_json, normalize_http_url,
};
use kukuri_core::{TopicId, public_topic_rendezvous_key};
use kukuri_transport::{SeedPeer, Transport, TransportRelayConfig, parse_seed_peer};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::discovery::{DiscoveryConfig, normalize_seed_peers};
use crate::identity::{IdentityStorageMode, load_optional_secret, persist_optional_secret};
use crate::paths::community_node_config_path;
use crate::runtime::DesktopRuntime;

mod config_support;
mod http_client_support;
mod manifest_support;
mod reconnect_support;
mod report_routing_support;
mod requests_support;
mod session_runtime_support;
mod session_state_support;
mod token_storage_support;

pub(crate) use config_support::*;
pub(crate) use http_client_support::*;
pub use manifest_support::{
    CommunityNodeAuthorityScope, CommunityNodeCapabilityScope, CommunityNodeManifest,
    CommunityNodeManifestFetch, CommunityNodeManifestFetchStatus, CommunityNodeP2pBoundary,
};
pub use report_routing_support::{
    SubmitCommunityNodeReportRequest, SubmitCommunityNodeReportResult,
    SubmitCommunityNodeReportStatus,
};
pub(crate) use token_storage_support::*;

/// 「版が上がって再同意が必要（更新）」な required ポリシーが存在するか。
///
/// `accepted_at` が None（現行版を未同意）かつ `previously_accepted_version` が Some
/// （過去に別版を同意済み）の required ポリシーがあれば true。auto_approve の node でも、
/// 更新時は黙って再受諾せずユーザーへ本文を再提示するための判定。
pub(crate) fn community_node_consent_has_pending_update(
    status: &CommunityNodeConsentStatus,
) -> bool {
    status.items.iter().any(|item| {
        item.required && item.accepted_at.is_none() && item.previously_accepted_version.is_some()
    })
}

pub(crate) const COMMUNITY_NODE_TOKEN_PURPOSE: &str = "community-node-token";
pub(crate) const COMMUNITY_NODE_PREVIEW_BASE_URL: &str = "https://api.kukuri.app";
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_RETRY_SECONDS: i64 = 10;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_METADATA_RETRY_SECONDS: i64 = 5;
pub(crate) const COMMUNITY_NODE_SESSION_RETRY_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_AUTH_REFRESH_SKEW_SECONDS: i64 = 300;
pub(crate) const COMMUNITY_NODE_RECONNECT_UNHEALTHY_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_RECONNECT_BACKOFF_SECONDS: [i64; 3] = [30, 60, 120];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeNodeConfig {
    pub base_url: String,
    #[serde(default)]
    pub auto_approve: bool,
    #[serde(default)]
    pub resolved_urls: Option<CommunityNodeResolvedUrls>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeConfig {
    #[serde(default)]
    pub nodes: Vec<CommunityNodeNodeConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BootstrapNodesResponse {
    pub(crate) nodes: Vec<kukuri_cn_core::CommunityNodeBootstrapNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TopicRendezvousHeartbeatResponse {
    pub(crate) expires_in_seconds: u64,
    pub(crate) topics: Vec<TopicRendezvousTopicResponse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TopicRendezvousTopicResponse {
    pub(crate) topic_key: String,
    pub(crate) peers: Vec<TopicRendezvousPeerCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TopicRendezvousPeerCandidate {
    pub(crate) endpoint_id: String,
    #[serde(default)]
    pub(crate) addr_hint: Option<String>,
    #[serde(default)]
    pub(crate) relay_urls: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCommunityNodeConfigNode {
    pub base_url: String,
    #[serde(default)]
    pub auto_approve: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCommunityNodeConfigRequest {
    pub nodes: Vec<SetCommunityNodeConfigNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeTargetRequest {
    pub base_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptCommunityNodeConsentsRequest {
    pub base_url: String,
    #[serde(default)]
    pub policy_slugs: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeAuthState {
    pub authenticated: bool,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeConnectivityAssistState {
    pub(crate) discovery_mode: kukuri_transport::DiscoveryMode,
    pub(crate) discovery_env_locked: bool,
    pub(crate) configured_seed_peers: Vec<SeedPeer>,
    pub(crate) bootstrap_seed_peers: Vec<SeedPeer>,
    pub(crate) relay_urls: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EffectiveSeedPeerApplyState {
    pub(crate) discovery_mode: kukuri_transport::DiscoveryMode,
    pub(crate) discovery_env_locked: bool,
    pub(crate) configured_seed_peers: Vec<SeedPeer>,
    pub(crate) bootstrap_seed_peers: Vec<SeedPeer>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CommunityNodeReconnectState {
    pub(crate) unhealthy_since: Option<i64>,
    pub(crate) next_retry_at: i64,
    pub(crate) backoff_step: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunityNodeSessionPhase {
    #[default]
    Idle,
    Connecting,
    Authenticating,
    Accepting,
    Refreshing,
    Ready,
    Retrying,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeNodeStatus {
    pub base_url: String,
    #[serde(default)]
    pub auto_approve: bool,
    pub auth_state: CommunityNodeAuthState,
    pub consent_state: Option<CommunityNodeConsentStatus>,
    pub resolved_urls: Option<CommunityNodeResolvedUrls>,
    pub last_error: Option<String>,
    #[serde(default)]
    pub session_phase: CommunityNodeSessionPhase,
    pub retry_after: Option<i64>,
    pub restart_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredCommunityNodeToken {
    pub(crate) access_token: String,
    pub(crate) expires_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kukuri_cn_core::CommunityNodeConsentItem;

    fn consent_item(
        accepted: bool,
        previously_accepted_version: Option<i32>,
        required: bool,
    ) -> CommunityNodeConsentItem {
        CommunityNodeConsentItem {
            policy_slug: "terms_of_service".to_string(),
            policy_version: 2,
            title: "Terms of Service".to_string(),
            body: "body".to_string(),
            required,
            accepted_at: accepted.then_some(1_700_000_000),
            previously_accepted_version,
        }
    }

    #[test]
    fn pending_update_false_when_first_time_not_accepted() {
        // 初回未同意（過去版の同意なし）は「更新」ではない。auto_approve の auto 受諾を許す。
        let status = CommunityNodeConsentStatus {
            all_required_accepted: false,
            items: vec![consent_item(false, None, true)],
        };
        assert!(!community_node_consent_has_pending_update(&status));
    }

    #[test]
    fn pending_update_true_when_previous_version_accepted_but_current_not() {
        // 旧版を同意済みだが現行版を未同意 = 版が上がった「更新」。auto_approve でも再提示。
        let status = CommunityNodeConsentStatus {
            all_required_accepted: false,
            items: vec![consent_item(false, Some(1), true)],
        };
        assert!(community_node_consent_has_pending_update(&status));
    }

    #[test]
    fn pending_update_false_when_current_version_accepted() {
        let status = CommunityNodeConsentStatus {
            all_required_accepted: true,
            items: vec![consent_item(true, Some(2), true)],
        };
        assert!(!community_node_consent_has_pending_update(&status));
    }

    #[test]
    fn pending_update_ignores_optional_policies() {
        // optional ポリシーの更新は接続 gate に影響しないため pending update としない。
        let status = CommunityNodeConsentStatus {
            all_required_accepted: true,
            items: vec![consent_item(false, Some(1), false)],
        };
        assert!(!community_node_consent_has_pending_update(&status));
    }
}
