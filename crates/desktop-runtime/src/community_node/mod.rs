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
pub(crate) use token_storage_support::*;

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
