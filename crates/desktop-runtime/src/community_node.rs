use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use kukuri_cn_core::{
    AuthChallengeResponse, AuthVerifyResponse, BootstrapHeartbeatResponse,
    CommunityNodeConsentStatus, CommunityNodeResolvedUrls, CommunityNodeSeedPeer,
    build_auth_envelope_json, normalize_http_url,
};
use kukuri_transport::{SeedPeer, Transport, TransportRelayConfig};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::discovery::{DiscoveryConfig, normalize_seed_peers};
use crate::identity::{IdentityStorageMode, load_optional_secret, persist_optional_secret};
use crate::paths::community_node_config_path;
use crate::runtime::DesktopRuntime;

pub(crate) const COMMUNITY_NODE_TOKEN_PURPOSE: &str = "community-node-token";
pub(crate) const COMMUNITY_NODE_PREVIEW_BASE_URL: &str = "https://api.kukuri.app";
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_RETRY_SECONDS: i64 = 10;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_METADATA_RETRY_SECONDS: i64 = 5;
pub(crate) const COMMUNITY_NODE_SESSION_RETRY_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_AUTH_REFRESH_SKEW_SECONDS: i64 = 300;

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

pub(crate) fn load_community_node_config_from_file(
    db_path: &Path,
) -> Result<Option<CommunityNodeConfig>> {
    let path = community_node_config_path(db_path);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read community-node config `{}`", path.display()))?;
    let config = serde_json::from_str::<CommunityNodeConfig>(&raw)
        .with_context(|| format!("failed to parse community-node config `{}`", path.display()))?;
    Ok(Some(normalize_community_node_config(config)?))
}

pub(crate) fn default_preview_community_node_config() -> CommunityNodeConfig {
    CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: COMMUNITY_NODE_PREVIEW_BASE_URL.to_string(),
            auto_approve: true,
            resolved_urls: None,
        }],
    }
}

pub(crate) fn save_community_node_config(
    db_path: &Path,
    config: &CommunityNodeConfig,
) -> Result<()> {
    let path = community_node_config_path(db_path);
    let normalized = normalize_community_node_config(config.clone())?;
    let json = serde_json::to_vec_pretty(&normalized).with_context(|| {
        format!(
            "failed to encode community-node config `{}`",
            path.display()
        )
    })?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write community-node config `{}`", path.display()))
}

pub(crate) fn normalize_community_node_config(
    config: CommunityNodeConfig,
) -> Result<CommunityNodeConfig> {
    let mut deduped = std::collections::BTreeMap::<String, CommunityNodeNodeConfig>::new();
    for node in config.nodes {
        let base_url = normalize_http_url(node.base_url.as_str())?;
        let incoming_auto_approve = node.auto_approve;
        let incoming_resolved_urls = match node.resolved_urls {
            Some(resolved) => Some(CommunityNodeResolvedUrls::new(
                resolved.public_base_url,
                resolved.connectivity_urls,
                resolved.seed_peers,
            )?),
            None => None,
        };
        let resolved_urls = if let Some(existing) = deduped.get(&base_url) {
            merge_community_node_resolved_urls(
                existing.resolved_urls.clone(),
                incoming_resolved_urls,
            )?
        } else {
            incoming_resolved_urls
        };
        let auto_approve = deduped
            .get(&base_url)
            .map(|existing| existing.auto_approve || incoming_auto_approve)
            .unwrap_or(incoming_auto_approve);
        deduped.insert(
            base_url.clone(),
            CommunityNodeNodeConfig {
                base_url,
                auto_approve,
                resolved_urls,
            },
        );
    }
    Ok(CommunityNodeConfig {
        nodes: deduped.into_values().collect(),
    })
}

pub(crate) fn merge_community_node_resolved_urls(
    current: Option<CommunityNodeResolvedUrls>,
    incoming: Option<CommunityNodeResolvedUrls>,
) -> Result<Option<CommunityNodeResolvedUrls>> {
    match (current, incoming) {
        (None, None) => Ok(None),
        (Some(resolved), None) | (None, Some(resolved)) => Ok(Some(resolved)),
        (Some(current), Some(incoming)) => {
            let public_base_url = incoming.public_base_url;
            let connectivity_urls = current
                .connectivity_urls
                .into_iter()
                .chain(incoming.connectivity_urls)
                .collect();
            let seed_peers = current
                .seed_peers
                .into_iter()
                .chain(incoming.seed_peers)
                .collect();
            Ok(Some(CommunityNodeResolvedUrls::new(
                public_base_url,
                connectivity_urls,
                seed_peers,
            )?))
        }
    }
}

pub(crate) fn community_node_seed_peers(
    config: &CommunityNodeConfig,
) -> impl Iterator<Item = SeedPeer> + '_ {
    config
        .nodes
        .iter()
        .filter_map(|node| node.resolved_urls.as_ref())
        .flat_map(|resolved| resolved.seed_peers.iter())
        .filter_map(seed_peer_from_community_node)
}

pub(crate) fn seed_peer_from_community_node(seed_peer: &CommunityNodeSeedPeer) -> Option<SeedPeer> {
    let endpoint_id = seed_peer.endpoint_id.trim();
    if endpoint_id.is_empty() {
        return None;
    }
    Some(SeedPeer {
        endpoint_id: endpoint_id.to_string(),
        addr_hint: seed_peer.addr_hint.clone(),
    })
}

pub(crate) fn relay_config_from_community_node_config(
    config: &CommunityNodeConfig,
) -> TransportRelayConfig {
    let mut iroh_relay_urls = std::collections::BTreeSet::new();
    for node in &config.nodes {
        if let Some(resolved) = node.resolved_urls.as_ref() {
            for relay_url in &resolved.connectivity_urls {
                iroh_relay_urls.insert(relay_url.clone());
            }
        }
    }
    TransportRelayConfig {
        iroh_relay_urls: iroh_relay_urls.into_iter().collect(),
    }
}

pub(crate) fn runtime_connectivity_assist_state(
    discovery_config: &DiscoveryConfig,
    community_node_config: &CommunityNodeConfig,
) -> RuntimeConnectivityAssistState {
    let relay_config = relay_config_from_community_node_config(community_node_config).normalized();
    let configured_seed_peers = normalize_seed_peers(discovery_config.seed_peers.clone());
    let bootstrap_seed_peers =
        normalize_seed_peers(community_node_seed_peers(community_node_config).collect());
    RuntimeConnectivityAssistState {
        discovery_mode: discovery_config.mode.clone(),
        discovery_env_locked: discovery_config.env_locked,
        configured_seed_peers,
        bootstrap_seed_peers,
        relay_urls: relay_config.iroh_relay_urls,
    }
}

pub(crate) fn effective_seed_peer_apply_state(
    discovery_config: &DiscoveryConfig,
    community_node_config: &CommunityNodeConfig,
) -> EffectiveSeedPeerApplyState {
    EffectiveSeedPeerApplyState {
        discovery_mode: discovery_config.mode.clone(),
        discovery_env_locked: discovery_config.env_locked,
        configured_seed_peers: normalize_seed_peers(discovery_config.seed_peers.clone()),
        bootstrap_seed_peers: normalize_seed_peers(
            community_node_seed_peers(community_node_config).collect(),
        ),
    }
}

pub(crate) fn load_community_node_token(
    db_path: &Path,
    mode: IdentityStorageMode,
    base_url: &str,
) -> Result<Option<StoredCommunityNodeToken>> {
    let Some(raw) = load_optional_secret(db_path, mode, COMMUNITY_NODE_TOKEN_PURPOSE, base_url)?
    else {
        return Ok(None);
    };
    let token = serde_json::from_str::<StoredCommunityNodeToken>(&raw)
        .context("failed to decode persisted community-node token")?;
    Ok(Some(token))
}

pub(crate) fn persist_community_node_token(
    db_path: &Path,
    mode: IdentityStorageMode,
    base_url: &str,
    token: &StoredCommunityNodeToken,
) -> Result<()> {
    let encoded = serde_json::to_string(token).context("failed to encode community-node token")?;
    persist_optional_secret(
        db_path,
        mode,
        COMMUNITY_NODE_TOKEN_PURPOSE,
        base_url,
        encoded.as_str(),
    )
}

pub(crate) fn community_node_http_client() -> Result<Client> {
    Client::builder()
        .build()
        .context("failed to build community-node http client")
}

#[derive(Debug)]
enum CommunityNodeRequestError {
    AuthRequired,
    ConsentRequired,
    Other(anyhow::Error),
}

impl CommunityNodeRequestError {
    fn into_anyhow(self) -> anyhow::Error {
        match self {
            Self::AuthRequired => anyhow!("community node authentication is required"),
            Self::ConsentRequired => anyhow!("community node consent is required"),
            Self::Other(error) => error,
        }
    }
}

impl DesktopRuntime {
    pub(crate) async fn set_community_node_session_phase(
        &self,
        base_url: &str,
        phase: CommunityNodeSessionPhase,
    ) {
        self.community_node_session_phases
            .lock()
            .await
            .insert(base_url.to_string(), phase);
        if phase != CommunityNodeSessionPhase::Ready
            && matches!(
                phase,
                CommunityNodeSessionPhase::Idle | CommunityNodeSessionPhase::Retrying
            )
        {
            self.community_node_ready_refresh_pending
                .lock()
                .await
                .remove(base_url);
        }
    }

    pub(crate) async fn set_community_node_session_ready(
        &self,
        base_url: &str,
        schedule_immediate_refresh: bool,
    ) {
        let previous = self
            .community_node_session_phases
            .lock()
            .await
            .insert(base_url.to_string(), CommunityNodeSessionPhase::Ready);
        if schedule_immediate_refresh {
            self.community_node_ready_refresh_pending
                .lock()
                .await
                .insert(base_url.to_string(), true);
            debug!(
                %base_url,
                previous_phase = ?previous,
                "scheduled immediate community-node metadata refresh after ready transition"
            );
        } else {
            self.community_node_ready_refresh_pending
                .lock()
                .await
                .remove(base_url);
            debug!(
                %base_url,
                previous_phase = ?previous,
                "keeping community-node metadata refresh pending state cleared for an already-ready session"
            );
        }
    }

    pub(crate) async fn community_node_session_was_ready(&self, base_url: &str) -> bool {
        self.community_node_session_phases
            .lock()
            .await
            .get(base_url)
            .copied()
            == Some(CommunityNodeSessionPhase::Ready)
    }

    pub(crate) async fn set_community_node_cached_consent(
        &self,
        base_url: &str,
        consent_state: Option<CommunityNodeConsentStatus>,
    ) {
        let mut cached = self.community_node_cached_consents.lock().await;
        if let Some(consent_state) = consent_state {
            cached.insert(base_url.to_string(), consent_state);
        } else {
            cached.remove(base_url);
        }
    }

    pub(crate) async fn clear_community_node_retry_state(&self, base_url: &str) {
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .remove(base_url);
        self.community_node_last_errors
            .lock()
            .await
            .remove(base_url);
    }

    pub(crate) async fn set_community_node_retry_state(
        &self,
        base_url: &str,
        error: anyhow::Error,
    ) {
        let now = Utc::now().timestamp();
        self.community_node_last_errors
            .lock()
            .await
            .insert(base_url.to_string(), error.to_string());
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .insert(
                base_url.to_string(),
                now.saturating_add(COMMUNITY_NODE_SESSION_RETRY_SECONDS),
            );
        self.set_community_node_session_phase(base_url, CommunityNodeSessionPhase::Retrying)
            .await;
    }

    fn community_node_token_requires_refresh(token: &StoredCommunityNodeToken, now: i64) -> bool {
        token.expires_at <= now.saturating_add(COMMUNITY_NODE_AUTH_REFRESH_SKEW_SECONDS)
    }

    fn map_community_node_send_error(
        action: &str,
        error: reqwest::Error,
    ) -> CommunityNodeRequestError {
        CommunityNodeRequestError::Other(anyhow!(error).context(action.to_string()))
    }

    fn map_community_node_status_error(
        action: &str,
        error: reqwest::Error,
    ) -> CommunityNodeRequestError {
        match error.status() {
            Some(StatusCode::UNAUTHORIZED) => CommunityNodeRequestError::AuthRequired,
            Some(StatusCode::FORBIDDEN) => CommunityNodeRequestError::ConsentRequired,
            _ => CommunityNodeRequestError::Other(anyhow!(error).context(action.to_string())),
        }
    }

    pub(crate) async fn request_community_node_authentication_token(
        &self,
        base_url: &str,
    ) -> Result<StoredCommunityNodeToken> {
        let base_url = normalize_http_url(base_url)?;
        let client = community_node_http_client()?;
        let challenge_url = format!("{}/v1/auth/challenge", base_url);
        let pubkey = self.author_keys.public_key_hex();
        let seed_peer = self.local_community_node_seed_peer("auth").await?;
        let challenge = client
            .post(challenge_url)
            .json(&serde_json::json!({ "pubkey": pubkey }))
            .send()
            .await
            .context("failed to request auth challenge")?
            .error_for_status()
            .context("auth challenge request failed")?
            .json::<AuthChallengeResponse>()
            .await
            .context("failed to decode auth challenge response")?;

        let public_base_url = self
            .community_node_config
            .lock()
            .await
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .and_then(|node| {
                node.resolved_urls
                    .as_ref()
                    .map(|resolved| resolved.public_base_url.clone())
            })
            .unwrap_or_else(|| base_url.clone());
        let auth_envelope_json = build_auth_envelope_json(
            self.author_keys.as_ref(),
            challenge.challenge.as_str(),
            public_base_url.as_str(),
        )?;
        let verify_url = format!("{}/v1/auth/verify", base_url);
        let verify = client
            .post(verify_url)
            .json(&serde_json::json!({
                "auth_envelope_json": auth_envelope_json,
                "endpoint_id": seed_peer.endpoint_id,
                "addr_hint": seed_peer.addr_hint,
            }))
            .send()
            .await
            .context("failed to verify auth envelope")?
            .error_for_status()
            .context("auth verify request failed")?
            .json::<AuthVerifyResponse>()
            .await
            .context("failed to decode auth verify response")?;
        let token = StoredCommunityNodeToken {
            access_token: verify.access_token,
            expires_at: verify.expires_at,
        };
        persist_community_node_token(&self.db_path, self.identity_mode, base_url.as_str(), &token)?;
        Ok(token)
    }

    async fn request_community_node_consent_status(
        &self,
        base_url: &str,
        access_token: &str,
    ) -> std::result::Result<CommunityNodeConsentStatus, CommunityNodeRequestError> {
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .get(format!("{}/v1/consents/status", base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to fetch community node consent status",
                    error,
                )
            })?;
        let response = response.error_for_status().map_err(|error| {
            Self::map_community_node_status_error(
                "community node consent status request failed",
                error,
            )
        })?;
        response
            .json::<CommunityNodeConsentStatus>()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to decode community node consent status",
                    error,
                )
            })
    }

    async fn request_accept_community_node_consents(
        &self,
        base_url: &str,
        access_token: &str,
        policy_slugs: &[String],
    ) -> std::result::Result<CommunityNodeConsentStatus, CommunityNodeRequestError> {
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .post(format!("{}/v1/consents", base_url))
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "policy_slugs": policy_slugs }))
            .send()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to accept community node consents",
                    error,
                )
            })?;
        let response = response.error_for_status().map_err(|error| {
            Self::map_community_node_status_error(
                "community node consent accept request failed",
                error,
            )
        })?;
        response
            .json::<CommunityNodeConsentStatus>()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to decode accepted community node consents",
                    error,
                )
            })
    }

    async fn sync_community_node_bootstrap_metadata(
        &self,
        base_url: &str,
        access_token: &str,
    ) -> std::result::Result<CommunityNodeNodeConfig, CommunityNodeRequestError> {
        let base_url = normalize_http_url(base_url).map_err(CommunityNodeRequestError::Other)?;
        let config = self.community_node_config.lock().await.clone();
        let Some(index) = config
            .nodes
            .iter()
            .position(|node| node.base_url == base_url)
        else {
            return Err(CommunityNodeRequestError::Other(anyhow!(
                "community node `{base_url}` is not configured"
            )));
        };
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .get(format!("{}/v1/bootstrap/nodes", base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to refresh community node metadata",
                    error,
                )
            })?;
        let bootstrap = response
            .error_for_status()
            .map_err(|error| {
                Self::map_community_node_status_error(
                    "community node bootstrap request failed",
                    error,
                )
            })?
            .json::<BootstrapNodesResponse>()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to decode community node bootstrap response",
                    error,
                )
            })?;
        let resolved_urls = bootstrap
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .map(|node| node.resolved_urls.clone())
            .ok_or_else(|| {
                CommunityNodeRequestError::Other(anyhow!(
                    "community node bootstrap response is missing self metadata"
                ))
            })?;
        debug!(
            %base_url,
            relay_url_count = resolved_urls.connectivity_urls.len(),
            seed_peer_count = resolved_urls.seed_peers.len(),
            "community-node metadata sync resolved bootstrap metadata"
        );
        let mut next_config = config;
        next_config.nodes[index].resolved_urls = Some(resolved_urls);
        let normalized = normalize_community_node_config(next_config)
            .map_err(CommunityNodeRequestError::Other)?;
        save_community_node_config(&self.db_path, &normalized)
            .map_err(CommunityNodeRequestError::Other)?;
        *self.community_node_config.lock().await = normalized.clone();
        self.apply_runtime_connectivity_assist()
            .await
            .map_err(CommunityNodeRequestError::Other)?;
        self.apply_effective_seed_peers()
            .await
            .map_err(CommunityNodeRequestError::Other)?;
        normalized
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .cloned()
            .ok_or_else(|| {
                CommunityNodeRequestError::Other(anyhow!(
                    "community node `{base_url}` disappeared after normalization"
                ))
            })
    }

    pub(crate) async fn community_node_bootstrap_metadata_retry_due(
        &self,
        base_url: &str,
        now: i64,
    ) -> bool {
        let seed_peers_empty = self
            .community_node_config
            .lock()
            .await
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .and_then(|node| node.resolved_urls.as_ref())
            .is_none_or(|resolved_urls| resolved_urls.seed_peers.is_empty());
        if !seed_peers_empty {
            self.community_node_metadata_refresh_deadlines
                .lock()
                .await
                .remove(base_url);
            return false;
        }
        let next_due_at = self
            .community_node_metadata_refresh_deadlines
            .lock()
            .await
            .get(base_url)
            .copied()
            .unwrap_or_default();
        next_due_at <= now
    }

    pub(crate) async fn record_community_node_bootstrap_metadata_refresh(
        &self,
        base_url: &str,
        seed_peers_empty: bool,
        now: i64,
    ) {
        let mut deadlines = self.community_node_metadata_refresh_deadlines.lock().await;
        if seed_peers_empty {
            deadlines.insert(
                base_url.to_string(),
                now.saturating_add(COMMUNITY_NODE_BOOTSTRAP_METADATA_RETRY_SECONDS),
            );
        } else {
            deadlines.remove(base_url);
        }
    }

    pub(crate) async fn local_community_node_seed_peer(
        &self,
        operation: &str,
    ) -> Result<CommunityNodeSeedPeer> {
        let endpoint_id = self
            .iroh_stack
            .transport
            .discovery()
            .await
            .with_context(|| {
                format!("failed to read local endpoint id for community node {operation}")
            })?
            .local_endpoint_id;
        let addr_hint = self
            .local_peer_ticket()
            .await
            .with_context(|| {
                format!("failed to read local peer ticket for community node {operation}")
            })?
            .and_then(|ticket| {
                ticket
                    .split_once('@')
                    .map(|(_, addr)| addr.trim().to_string())
                    .filter(|addr| !addr.is_empty())
            });
        CommunityNodeSeedPeer::new(endpoint_id, addr_hint)
    }

    pub(crate) async fn fetch_community_node_consent_status_with_retry(
        &self,
        base_url: &str,
        token: &mut StoredCommunityNodeToken,
        allow_reauthenticate: bool,
    ) -> Result<CommunityNodeConsentStatus> {
        match self
            .request_community_node_consent_status(base_url, token.access_token.as_str())
            .await
        {
            Ok(status) => Ok(status),
            Err(CommunityNodeRequestError::AuthRequired) if allow_reauthenticate => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Authenticating,
                )
                .await;
                *token = self
                    .request_community_node_authentication_token(base_url)
                    .await?;
                self.request_community_node_consent_status(base_url, token.access_token.as_str())
                    .await
                    .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(error) => Err(error.into_anyhow()),
        }
    }

    pub(crate) async fn accept_community_node_consents_with_retry(
        &self,
        base_url: &str,
        token: &mut StoredCommunityNodeToken,
        policy_slugs: &[String],
    ) -> Result<CommunityNodeConsentStatus> {
        match self
            .request_accept_community_node_consents(
                base_url,
                token.access_token.as_str(),
                policy_slugs,
            )
            .await
        {
            Ok(status) => Ok(status),
            Err(CommunityNodeRequestError::AuthRequired) => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Authenticating,
                )
                .await;
                *token = self
                    .request_community_node_authentication_token(base_url)
                    .await?;
                self.request_accept_community_node_consents(
                    base_url,
                    token.access_token.as_str(),
                    policy_slugs,
                )
                .await
                .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(error) => Err(error.into_anyhow()),
        }
    }

    pub(crate) async fn sync_community_node_bootstrap_metadata_with_retry(
        &self,
        base_url: &str,
        token: &mut StoredCommunityNodeToken,
        auto_approve: bool,
    ) -> Result<CommunityNodeNodeConfig> {
        match self
            .sync_community_node_bootstrap_metadata(base_url, token.access_token.as_str())
            .await
        {
            Ok(node) => Ok(node),
            Err(CommunityNodeRequestError::AuthRequired) => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Authenticating,
                )
                .await;
                *token = self
                    .request_community_node_authentication_token(base_url)
                    .await?;
                let consent_status = self
                    .fetch_community_node_consent_status_with_retry(base_url, token, false)
                    .await?;
                self.set_community_node_cached_consent(base_url, Some(consent_status.clone()))
                    .await;
                if !consent_status.all_required_accepted {
                    if !auto_approve {
                        bail!("community node consent is required");
                    }
                    self.set_community_node_session_phase(
                        base_url,
                        CommunityNodeSessionPhase::Accepting,
                    )
                    .await;
                    let accepted = self
                        .accept_community_node_consents_with_retry(base_url, token, &[])
                        .await?;
                    self.set_community_node_cached_consent(base_url, Some(accepted))
                        .await;
                }
                self.sync_community_node_bootstrap_metadata(base_url, token.access_token.as_str())
                    .await
                    .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(CommunityNodeRequestError::ConsentRequired) if auto_approve => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Accepting,
                )
                .await;
                let accepted = self
                    .accept_community_node_consents_with_retry(base_url, token, &[])
                    .await?;
                self.set_community_node_cached_consent(base_url, Some(accepted))
                    .await;
                self.sync_community_node_bootstrap_metadata(base_url, token.access_token.as_str())
                    .await
                    .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(CommunityNodeRequestError::ConsentRequired) => {
                bail!("community node consent is required")
            }
            Err(error) => Err(error.into_anyhow()),
        }
    }

    async fn refresh_community_node_registration_with_token_if_due_once(
        &self,
        base_url: &str,
        access_token: &str,
    ) -> std::result::Result<(), CommunityNodeRequestError> {
        let base_url = normalize_http_url(base_url).map_err(CommunityNodeRequestError::Other)?;
        let now = Utc::now().timestamp();
        let next_due_at = self
            .community_node_heartbeat_deadlines
            .lock()
            .await
            .get(base_url.as_str())
            .copied()
            .unwrap_or_default();
        if next_due_at > now {
            let ready_refresh_pending = self
                .community_node_ready_refresh_pending
                .lock()
                .await
                .remove(base_url.as_str())
                .unwrap_or(false);
            if !self
                .community_node_bootstrap_metadata_retry_due(base_url.as_str(), now)
                .await
                && !ready_refresh_pending
            {
                debug!(
                    %base_url,
                    next_due_at,
                    now,
                    "skipping community-node heartbeat because the next refresh is not due"
                );
                return Ok(());
            }
            info!(
                %base_url,
                next_due_at,
                now,
                ready_refresh_pending,
                "running community-node metadata refresh without waiting for the next heartbeat"
            );
            return match self
                .sync_community_node_bootstrap_metadata(base_url.as_str(), access_token)
                .await
            {
                Ok(node) => {
                    self.record_community_node_bootstrap_metadata_refresh(
                        base_url.as_str(),
                        node.resolved_urls
                            .as_ref()
                            .is_none_or(|resolved_urls| resolved_urls.seed_peers.is_empty()),
                        now,
                    )
                    .await;
                    Ok(())
                }
                Err(error) => {
                    self.record_community_node_bootstrap_metadata_refresh(
                        base_url.as_str(),
                        true,
                        now,
                    )
                    .await;
                    Err(error)
                }
            };
        }
        let seed_peer = self
            .local_community_node_seed_peer("heartbeat")
            .await
            .map_err(CommunityNodeRequestError::Other)?;
        info!(
            %base_url,
            next_due_at,
            now,
            "refreshing community-node bootstrap heartbeat"
        );
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .post(format!("{}/v1/bootstrap/heartbeat", base_url))
            .bearer_auth(access_token)
            .json(&serde_json::json!({
                "endpoint_id": seed_peer.endpoint_id,
                "addr_hint": seed_peer.addr_hint,
            }))
            .send()
            .await;
        match response {
            Ok(response) => {
                let heartbeat = response
                    .error_for_status()
                    .map_err(|error| {
                        Self::map_community_node_status_error(
                            "community node bootstrap heartbeat request failed",
                            error,
                        )
                    })?
                    .json::<BootstrapHeartbeatResponse>()
                    .await
                    .map_err(|error| {
                        Self::map_community_node_send_error(
                            "failed to decode community node bootstrap heartbeat response",
                            error,
                        )
                    })?;
                self.community_node_heartbeat_deadlines.lock().await.insert(
                    base_url.clone(),
                    heartbeat
                        .expires_at
                        .saturating_sub(COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS),
                );
                debug!(
                    %base_url,
                    expires_at = heartbeat.expires_at,
                    "community-node bootstrap heartbeat refreshed"
                );
                match self
                    .sync_community_node_bootstrap_metadata(base_url.as_str(), access_token)
                    .await
                {
                    Ok(node) => {
                        self.record_community_node_bootstrap_metadata_refresh(
                            base_url.as_str(),
                            node.resolved_urls
                                .as_ref()
                                .is_none_or(|resolved_urls| resolved_urls.seed_peers.is_empty()),
                            now,
                        )
                        .await;
                        Ok(())
                    }
                    Err(error) => {
                        self.record_community_node_bootstrap_metadata_refresh(
                            base_url.as_str(),
                            true,
                            now,
                        )
                        .await;
                        Err(error)
                    }
                }
            }
            Err(error) => {
                self.community_node_heartbeat_deadlines.lock().await.insert(
                    base_url,
                    now.saturating_add(COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_RETRY_SECONDS),
                );
                Err(Self::map_community_node_send_error(
                    "failed to refresh community node bootstrap registration",
                    error,
                ))
            }
        }
    }

    pub(crate) async fn refresh_community_node_registration_with_token_if_due(
        &self,
        base_url: &str,
        token: &mut StoredCommunityNodeToken,
        auto_approve: bool,
    ) -> Result<()> {
        match self
            .refresh_community_node_registration_with_token_if_due_once(
                base_url,
                token.access_token.as_str(),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(CommunityNodeRequestError::AuthRequired) => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Authenticating,
                )
                .await;
                *token = self
                    .request_community_node_authentication_token(base_url)
                    .await?;
                let consent_status = self
                    .fetch_community_node_consent_status_with_retry(base_url, token, false)
                    .await?;
                self.set_community_node_cached_consent(base_url, Some(consent_status.clone()))
                    .await;
                if !consent_status.all_required_accepted {
                    if !auto_approve {
                        self.set_community_node_session_phase(
                            base_url,
                            CommunityNodeSessionPhase::Idle,
                        )
                        .await;
                        return Ok(());
                    }
                    self.set_community_node_session_phase(
                        base_url,
                        CommunityNodeSessionPhase::Accepting,
                    )
                    .await;
                    let accepted = self
                        .accept_community_node_consents_with_retry(base_url, token, &[])
                        .await?;
                    self.set_community_node_cached_consent(base_url, Some(accepted))
                        .await;
                }
                self.refresh_community_node_registration_with_token_if_due_once(
                    base_url,
                    token.access_token.as_str(),
                )
                .await
                .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(CommunityNodeRequestError::ConsentRequired) if auto_approve => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Accepting,
                )
                .await;
                let accepted = self
                    .accept_community_node_consents_with_retry(base_url, token, &[])
                    .await?;
                self.set_community_node_cached_consent(base_url, Some(accepted))
                    .await;
                self.refresh_community_node_registration_with_token_if_due_once(
                    base_url,
                    token.access_token.as_str(),
                )
                .await
                .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(CommunityNodeRequestError::ConsentRequired) => Ok(()),
            Err(error) => Err(error.into_anyhow()),
        }
    }

    pub(crate) async fn ensure_community_node_session(&self, base_url: &str) -> Result<()> {
        let base_url = normalize_http_url(base_url)?;
        let now = Utc::now().timestamp();
        let retry_after = self
            .community_node_session_retry_deadlines
            .lock()
            .await
            .get(base_url.as_str())
            .copied();
        if retry_after.is_some_and(|retry_after| retry_after > now) {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Retrying,
            )
            .await;
            return Ok(());
        }

        let _guard = self.community_node_session_guard.lock().await;
        let now = Utc::now().timestamp();
        let retry_after = self
            .community_node_session_retry_deadlines
            .lock()
            .await
            .get(base_url.as_str())
            .copied();
        if retry_after.is_some_and(|retry_after| retry_after > now) {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Retrying,
            )
            .await;
            return Ok(());
        }

        let was_ready = self
            .community_node_session_was_ready(base_url.as_str())
            .await;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Connecting,
        )
        .await;
        let node = self.require_community_node(base_url.as_str()).await?;
        let auto_approve = node.auto_approve;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?;

        if token
            .as_ref()
            .is_some_and(|token| Self::community_node_token_requires_refresh(token, now))
            || (token.is_none() && auto_approve)
        {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Authenticating,
            )
            .await;
            token = Some(
                self.request_community_node_authentication_token(base_url.as_str())
                    .await?,
            );
        } else if token.is_none() {
            self.clear_community_node_retry_state(base_url.as_str())
                .await;
            self.set_community_node_cached_consent(base_url.as_str(), None)
                .await;
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Idle,
            )
            .await;
            return Ok(());
        }

        let mut token = token.expect("token must exist after authentication");
        let consent_status = self
            .fetch_community_node_consent_status_with_retry(base_url.as_str(), &mut token, true)
            .await?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(consent_status.clone()))
            .await;
        if !consent_status.all_required_accepted {
            if !auto_approve {
                self.clear_community_node_retry_state(base_url.as_str())
                    .await;
                self.set_community_node_session_phase(
                    base_url.as_str(),
                    CommunityNodeSessionPhase::Idle,
                )
                .await;
                return Ok(());
            }
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Accepting,
            )
            .await;
            let accepted = self
                .accept_community_node_consents_with_retry(base_url.as_str(), &mut token, &[])
                .await?;
            self.set_community_node_cached_consent(base_url.as_str(), Some(accepted))
                .await;
        }

        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Refreshing,
        )
        .await;
        self.refresh_community_node_registration_with_token_if_due(
            base_url.as_str(),
            &mut token,
            auto_approve,
        )
        .await?;
        self.clear_community_node_retry_state(base_url.as_str())
            .await;
        self.set_community_node_session_ready(base_url.as_str(), !was_ready)
            .await;
        Ok(())
    }

    pub(crate) async fn refresh_community_node_registration_if_due(
        &self,
        base_url: &str,
    ) -> Result<()> {
        let base_url = normalize_http_url(base_url)?;
        match self.ensure_community_node_session(base_url.as_str()).await {
            Ok(()) => Ok(()),
            Err(error) => {
                self.set_community_node_retry_state(base_url.as_str(), error)
                    .await;
                Ok(())
            }
        }
    }

    pub(crate) async fn require_community_node(
        &self,
        base_url: &str,
    ) -> Result<CommunityNodeNodeConfig> {
        self.community_node_config
            .lock()
            .await
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .cloned()
            .ok_or_else(|| anyhow!("community node `{base_url}` is not configured"))
    }

    pub(crate) async fn community_node_status(
        &self,
        node: CommunityNodeNodeConfig,
        consent_state: Option<CommunityNodeConsentStatus>,
        last_error: Option<String>,
    ) -> Result<CommunityNodeNodeStatus> {
        let now = Utc::now().timestamp();
        let token =
            load_community_node_token(&self.db_path, self.identity_mode, node.base_url.as_str())?;
        let auth_state = match token {
            Some(token) if token.expires_at > now => CommunityNodeAuthState {
                authenticated: true,
                expires_at: Some(token.expires_at),
            },
            Some(token) => CommunityNodeAuthState {
                authenticated: false,
                expires_at: Some(token.expires_at),
            },
            None => CommunityNodeAuthState::default(),
        };
        let consent_state = if let Some(consent_state) = consent_state {
            Some(consent_state)
        } else {
            self.community_node_cached_consents
                .lock()
                .await
                .get(node.base_url.as_str())
                .cloned()
        };
        let last_error = if let Some(last_error) = last_error {
            Some(last_error)
        } else {
            self.community_node_last_errors
                .lock()
                .await
                .get(node.base_url.as_str())
                .cloned()
        };
        let retry_after = self
            .community_node_session_retry_deadlines
            .lock()
            .await
            .get(node.base_url.as_str())
            .copied()
            .filter(|deadline| *deadline > now);
        let session_phase = self
            .community_node_session_phases
            .lock()
            .await
            .get(node.base_url.as_str())
            .copied()
            .unwrap_or_else(|| {
                if auth_state.authenticated
                    && consent_state
                        .as_ref()
                        .is_none_or(|consent| consent.all_required_accepted)
                    && node.resolved_urls.is_some()
                {
                    CommunityNodeSessionPhase::Ready
                } else {
                    CommunityNodeSessionPhase::Idle
                }
            });
        let current_connectivity_urls = relay_config_from_community_node_config(
            &self.community_node_config.lock().await.clone(),
        )
        .iroh_relay_urls;
        Ok(CommunityNodeNodeStatus {
            base_url: node.base_url,
            auto_approve: node.auto_approve,
            auth_state,
            consent_state,
            resolved_urls: node.resolved_urls,
            last_error,
            session_phase,
            retry_after,
            restart_required: current_connectivity_urls
                != *self.active_connectivity_urls.lock().await,
        })
    }

    pub(crate) async fn apply_runtime_connectivity_assist(&self) -> Result<()> {
        let discovery_config = self.discovery_config.lock().await.clone();
        let community_node_config = self.community_node_config.lock().await.clone();
        let next_state =
            runtime_connectivity_assist_state(&discovery_config, &community_node_config);
        {
            let current_state = self.last_runtime_connectivity_assist_state.lock().await;
            if current_state.as_ref() == Some(&next_state) {
                debug!(
                    relay_url_count = next_state.relay_urls.len(),
                    bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
                    "skipping runtime connectivity apply because relay and seed inputs are unchanged"
                );
                return Ok(());
            }
        }
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: next_state.relay_urls.clone(),
        };
        self.iroh_stack
            .apply_runtime_connectivity(
                &discovery_config,
                &next_state.bootstrap_seed_peers,
                relay_config.clone(),
            )
            .await?;
        debug!(
            relay_url_count = relay_config.iroh_relay_urls.len(),
            bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
            "applied runtime connectivity assist from community-node metadata"
        );
        *self.active_connectivity_urls.lock().await = relay_config.iroh_relay_urls;
        *self.last_runtime_connectivity_assist_state.lock().await = Some(next_state);
        self.runtime_connectivity_apply_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub(crate) async fn apply_effective_seed_peers(&self) -> Result<()> {
        let discovery_config = self.discovery_config.lock().await.clone();
        let community_node_config = self.community_node_config.lock().await.clone();
        let next_state = effective_seed_peer_apply_state(&discovery_config, &community_node_config);
        {
            let current_state = self.last_effective_seed_peer_apply_state.lock().await;
            if current_state.as_ref() == Some(&next_state) {
                debug!(
                    bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
                    configured_seed_peer_count = next_state.configured_seed_peers.len(),
                    "skipping discovery seed apply because the effective seed inputs are unchanged"
                );
                return Ok(());
            }
        }
        self.app_service
            .set_discovery_seeds(
                next_state.discovery_mode.clone(),
                next_state.discovery_env_locked,
                next_state.configured_seed_peers.clone(),
                next_state.bootstrap_seed_peers.clone(),
            )
            .await?;
        debug!(
            bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
            configured_seed_peer_count = next_state.configured_seed_peers.len(),
            "applied effective discovery seeds from community-node metadata"
        );
        *self.last_effective_seed_peer_apply_state.lock().await = Some(next_state);
        self.effective_seed_peer_apply_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}
