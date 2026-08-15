use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use kukuri_cn_protocol::{
    AUTH_CHALLENGE_PATH, AUTH_VERIFY_PATH, AcceptConsentsRequest, ApiErrorBody,
    AuthChallengeRequest, AuthChallengeResponse, AuthVerifyRequest, AuthVerifyResponse,
    BOOTSTRAP_HEARTBEAT_PATH, BOOTSTRAP_NODES_PATH, BootstrapHeartbeatRequest,
    BootstrapHeartbeatResponse, CONSENTS_PATH, CONSENTS_STATUS_PATH, CommunityNodeConsentStatus,
    CommunityNodeReportRequest, CommunityNodeReportResponse, CommunityNodeResolvedUrls,
    CommunityNodeSeedPeer, NODE_MANIFEST_PATH, TOPIC_RENDEZVOUS_HEARTBEAT_PATH,
    TopicRendezvousHeartbeat, build_auth_envelope_json, normalize_http_url,
};
use kukuri_core::{
    TopicId, public_topic_rendezvous_key,
    wire::{HINT_TOPIC_PREFIX, PRIVATE_CHANNEL_TOPIC_PREFIX},
};
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
mod index_query_support;
mod indexing_request_support;
mod invite_storage_support;
mod manifest_support;
mod reconnect_support;
mod report_routing_support;
mod requests_support;
mod scheduler_support;
mod session_runtime_support;
mod session_state_support;
mod token_storage_support;
mod trust_relation_support;

pub(crate) use config_support::*;
pub(crate) use http_client_support::*;
pub(crate) use index_query_support::IndexOperation;
pub use index_query_support::{CommunityNodeIndexQueryError, CommunityNodeIndexQueryRequest};
pub use indexing_request_support::{
    CommunityNodeIndexingRequest, CommunityNodeIndexingRequestError,
};
pub(crate) use invite_storage_support::*;
pub use kukuri_cn_protocol::{
    CommunityNodeReportAppeal, IndexEntryView, IndexQueryResponse, IndexScopeKind,
    RelationNeighborsResponse, RelationOptoutResponse, RelationReadResponse,
    SubmitIndexingRequestResponse, TrustUserReadResponse,
};
pub use manifest_support::{
    CommunityNodeAuthorityScope, CommunityNodeCapabilityScope, CommunityNodeManifest,
    CommunityNodeManifestFetch, CommunityNodeManifestFetchStatus, CommunityNodeP2pBoundary,
};
pub use report_routing_support::{
    CommunityNodeReportError, SubmitCommunityNodeReportRequest, SubmitCommunityNodeReportResult,
    SubmitCommunityNodeReportStatus,
};
pub(crate) use token_storage_support::*;
pub use trust_relation_support::{
    CommunityNodeRelationNeighborsRequest, CommunityNodeTrustRelationError,
    CommunityNodeUserAdvisoryRequest,
};

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
pub(crate) const COMMUNITY_NODE_INVITE_CODE_PURPOSE: &str = "community-node-invite-code";
pub(crate) const COMMUNITY_NODE_PREVIEW_BASE_URL: &str = "https://api.kukuri.app";
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_RETRY_SECONDS: i64 = 10;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_METADATA_RETRY_SECONDS: i64 = 5;
pub(crate) const COMMUNITY_NODE_SESSION_RETRY_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_AUTH_REFRESH_SKEW_SECONDS: i64 = 300;
pub(crate) const COMMUNITY_NODE_RECONNECT_UNHEALTHY_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_RECONNECT_BACKOFF_SECONDS: [i64; 3] = [30, 60, 120];
// heartbeat の次回期限は「サーバ TTL(90 秒)− 30 秒」で管理されるため、tick は 30 秒未満が必須。
// 15 秒は失効防止を満たしつつ、tick 毎の consent GET を可視時の現行ポーリング(3 秒 ×2 getter)
// より低頻度に抑える値(WP-C1 プランの決定事項 1)。
pub(crate) const COMMUNITY_NODE_SESSION_SCHEDULER_TICK_SECONDS: u64 = 15;
// topic rendezvous presence の次回 refresh 期限は「サーバ返却 expires_in_seconds(45 秒)−
// このマージン」で管理する(#572)。マージンは tick(15 秒)より大きいことが必須: tick 量子化で
// 実際の POST が deadline から最大 tick 分遅れても TTL に達しないため(deadline +25 秒、
// 最悪 POST +40 秒 < TTL 45 秒)。
pub(crate) const COMMUNITY_NODE_TOPIC_RENDEZVOUS_REFRESH_MARGIN_SECONDS: i64 = 20;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeNodeConfig {
    pub base_url: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>"))]
    pub auto_approve: bool,
    #[serde(default)]
    pub resolved_urls: Option<CommunityNodeResolvedUrls>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeConfig {
    #[serde(default)]
    pub nodes: Vec<CommunityNodeNodeConfig>,
}

// CN HTTP response 型は kukuri-cn-protocol の共有定義を使う(WP-B17)。
// 旧ローカルミラー(TopicRendezvousPeerCandidate = 共有側の TopicRendezvousCandidate)は
// フィールド一致の二重定義だったため撤去した。serde 名不変 = wire 互換。
pub(crate) use kukuri_cn_protocol::{BootstrapNodesResponse, TopicRendezvousHeartbeatResponse};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct SetCommunityNodeConfigNode {
    pub base_url: String,
    #[serde(default)]
    pub auto_approve: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct SetCommunityNodeConfigRequest {
    pub nodes: Vec<SetCommunityNodeConfigNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeTargetRequest {
    pub base_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct SetCommunityNodeInviteCodeRequest {
    pub base_url: String,
    pub invite_code: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommunityNodeAdmissionRejectionCode {
    InviteRequired,
    InviteInvalid,
    InviteExpired,
    InviteExhausted,
    InviteRevoked,
    NotAllowlisted,
    Banned,
}

impl CommunityNodeAdmissionRejectionCode {
    pub(crate) fn from_wire_code(code: &str) -> Option<Self> {
        match code {
            "INVITE_REQUIRED" => Some(Self::InviteRequired),
            "INVITE_INVALID" => Some(Self::InviteInvalid),
            "INVITE_EXPIRED" => Some(Self::InviteExpired),
            "INVITE_EXHAUSTED" => Some(Self::InviteExhausted),
            "INVITE_REVOKED" => Some(Self::InviteRevoked),
            "NOT_ALLOWLISTED" => Some(Self::NotAllowlisted),
            "BANNED" => Some(Self::Banned),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommunityNodeAdmissionRejection {
    pub code: CommunityNodeAdmissionRejectionCode,
    pub message: String,
}

impl std::fmt::Display for CommunityNodeAdmissionRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommunityNodeAdmissionRejection {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct AcceptCommunityNodeConsentsRequest {
    pub base_url: String,
    #[serde(default)]
    pub policy_slugs: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    AwaitingAdmission,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CommunityNodeSessionState {
    pub(crate) heartbeat_deadline: i64,
    // default 0 = 即時 due。refresh 成功時のみ bump する(#572)。
    pub(crate) rendezvous_refresh_deadline: i64,
    pub(crate) metadata_refresh_deadline: i64,
    pub(crate) session_retry_deadline: i64,
    pub(crate) session_phase: CommunityNodeSessionPhase,
    pub(crate) ready_refresh_pending: bool,
    pub(crate) last_error: Option<String>,
    pub(crate) admission_rejection: Option<CommunityNodeAdmissionRejection>,
    pub(crate) cached_consent: Option<CommunityNodeConsentStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct CommunityNodeNodeStatus {
    pub base_url: String,
    // 現行 types.ts と同じく任意(#[serde(default)])。
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>"))]
    pub auto_approve: bool,
    pub auth_state: CommunityNodeAuthState,
    pub consent_state: Option<CommunityNodeConsentStatus>,
    pub resolved_urls: Option<CommunityNodeResolvedUrls>,
    pub last_error: Option<String>,
    #[serde(default)]
    pub invite_code_saved: bool,
    pub admission_rejection: Option<CommunityNodeAdmissionRejection>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(as = "Option<CommunityNodeSessionPhase>"))]
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
    use kukuri_cn_protocol::CommunityNodeConsentItem;

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
