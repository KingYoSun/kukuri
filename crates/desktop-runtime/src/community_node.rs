use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use kukuri_cn_core::{
    BootstrapHeartbeatResponse, CommunityNodeConsentStatus, CommunityNodeResolvedUrls,
    CommunityNodeSeedPeer, normalize_http_url,
};
use kukuri_transport::{SeedPeer, Transport, TransportRelayConfig};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::identity::{IdentityStorageMode, load_optional_secret, persist_optional_secret};
use crate::paths::community_node_config_path;
use crate::runtime::DesktopRuntime;

pub(crate) const COMMUNITY_NODE_TOKEN_PURPOSE: &str = "community-node-token";
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS: i64 = 30;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_RETRY_SECONDS: i64 = 10;
pub(crate) const COMMUNITY_NODE_BOOTSTRAP_METADATA_RETRY_SECONDS: i64 = 5;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeNodeConfig {
    pub base_url: String,
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
pub struct SetCommunityNodeConfigRequest {
    pub base_urls: Vec<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeNodeStatus {
    pub base_url: String,
    pub auth_state: CommunityNodeAuthState,
    pub consent_state: Option<CommunityNodeConsentStatus>,
    pub resolved_urls: Option<CommunityNodeResolvedUrls>,
    pub last_error: Option<String>,
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

pub(crate) fn remove_community_node_config(db_path: &Path) -> Result<()> {
    let path = community_node_config_path(db_path);
    if path.exists() {
        fs::remove_file(&path).with_context(|| {
            format!(
                "failed to remove community-node config `{}`",
                path.display()
            )
        })?;
    }
    Ok(())
}

pub(crate) fn normalize_community_node_config(
    config: CommunityNodeConfig,
) -> Result<CommunityNodeConfig> {
    let mut deduped = std::collections::BTreeMap::<String, CommunityNodeNodeConfig>::new();
    for node in config.nodes {
        let base_url = normalize_http_url(node.base_url.as_str())?;
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
        deduped.insert(
            base_url.clone(),
            CommunityNodeNodeConfig {
                base_url,
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

impl DesktopRuntime {
    pub(crate) async fn sync_community_node_bootstrap_metadata(
        &self,
        base_url: &str,
        access_token: &str,
    ) -> Result<CommunityNodeNodeConfig> {
        let base_url = normalize_http_url(base_url)?;
        let config = self.community_node_config.lock().await.clone();
        let Some(index) = config
            .nodes
            .iter()
            .position(|node| node.base_url == base_url)
        else {
            bail!("community node `{base_url}` is not configured");
        };
        let client = community_node_http_client()?;
        let response = client
            .get(format!("{}/v1/bootstrap/nodes", base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .context("failed to refresh community node metadata")?;
        let bootstrap = response
            .error_for_status()
            .context("community node bootstrap request failed")?
            .json::<BootstrapNodesResponse>()
            .await
            .context("failed to decode community node bootstrap response")?;
        let resolved_urls = bootstrap
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .map(|node| node.resolved_urls.clone())
            .ok_or_else(|| anyhow!("community node bootstrap response is missing self metadata"))?;
        let mut next_config = config;
        next_config.nodes[index].resolved_urls = Some(resolved_urls);
        let normalized = normalize_community_node_config(next_config)?;
        save_community_node_config(&self.db_path, &normalized)?;
        *self.community_node_config.lock().await = normalized.clone();
        self.apply_runtime_connectivity_assist().await?;
        self.apply_effective_seed_peers().await?;
        normalized
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .cloned()
            .ok_or_else(|| anyhow!("community node `{base_url}` disappeared after normalization"))
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

    pub(crate) async fn refresh_community_node_registration_if_due(
        &self,
        base_url: &str,
    ) -> Result<()> {
        let base_url = normalize_http_url(base_url)?;
        let token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?;
        let Some(token) = token else {
            return Ok(());
        };
        let now = Utc::now().timestamp();
        if token.expires_at <= now {
            return Ok(());
        }
        let next_due_at = self
            .community_node_heartbeat_deadlines
            .lock()
            .await
            .get(base_url.as_str())
            .copied()
            .unwrap_or_default();
        if next_due_at > now {
            if !self
                .community_node_bootstrap_metadata_retry_due(base_url.as_str(), now)
                .await
            {
                return Ok(());
            }
            return match self
                .sync_community_node_bootstrap_metadata(
                    base_url.as_str(),
                    token.access_token.as_str(),
                )
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
        let seed_peer = self.local_community_node_seed_peer("heartbeat").await?;
        let client = community_node_http_client()?;
        let response = client
            .post(format!("{}/v1/bootstrap/heartbeat", base_url))
            .bearer_auth(token.access_token.as_str())
            .json(&serde_json::json!({
                "endpoint_id": seed_peer.endpoint_id,
                "addr_hint": seed_peer.addr_hint,
            }))
            .send()
            .await
            .context("failed to refresh community node bootstrap registration");
        match response {
            Ok(response) => {
                let heartbeat = response
                    .error_for_status()
                    .context("community node bootstrap heartbeat request failed")?
                    .json::<BootstrapHeartbeatResponse>()
                    .await
                    .context("failed to decode community node bootstrap heartbeat response")?;
                self.community_node_heartbeat_deadlines.lock().await.insert(
                    base_url.clone(),
                    heartbeat
                        .expires_at
                        .saturating_sub(COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS),
                );
                match self
                    .sync_community_node_bootstrap_metadata(
                        base_url.as_str(),
                        token.access_token.as_str(),
                    )
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
                Err(error)
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
        let token =
            load_community_node_token(&self.db_path, self.identity_mode, node.base_url.as_str())?;
        let auth_state = match token {
            Some(token) if token.expires_at > Utc::now().timestamp() => CommunityNodeAuthState {
                authenticated: true,
                expires_at: Some(token.expires_at),
            },
            Some(token) => CommunityNodeAuthState {
                authenticated: false,
                expires_at: Some(token.expires_at),
            },
            None => CommunityNodeAuthState::default(),
        };
        let current_connectivity_urls = relay_config_from_community_node_config(
            &self.community_node_config.lock().await.clone(),
        )
        .iroh_relay_urls;
        Ok(CommunityNodeNodeStatus {
            base_url: node.base_url,
            auth_state,
            consent_state,
            resolved_urls: node.resolved_urls,
            last_error,
            restart_required: current_connectivity_urls
                != *self.active_connectivity_urls.lock().await,
        })
    }

    pub(crate) async fn apply_runtime_connectivity_assist(&self) -> Result<()> {
        let community_node_config = self.community_node_config.lock().await.clone();
        let relay_config = relay_config_from_community_node_config(&community_node_config);
        let discovery_config = self.discovery_config.lock().await.clone();
        let bootstrap_seed_peers =
            community_node_seed_peers(&community_node_config).collect::<Vec<_>>();
        self.iroh_stack
            .apply_runtime_connectivity(
                &discovery_config,
                &bootstrap_seed_peers,
                relay_config.clone(),
            )
            .await?;
        *self.active_connectivity_urls.lock().await = relay_config.iroh_relay_urls;
        Ok(())
    }

    pub(crate) async fn apply_effective_seed_peers(&self) -> Result<()> {
        let discovery_config = self.discovery_config.lock().await.clone();
        let community_node_config = self.community_node_config.lock().await.clone();
        let bootstrap_seed_peers =
            community_node_seed_peers(&community_node_config).collect::<Vec<_>>();
        self.app_service
            .set_discovery_seeds(
                discovery_config.mode.clone(),
                discovery_config.env_locked,
                discovery_config.seed_peers,
                bootstrap_seed_peers,
            )
            .await
    }
}
