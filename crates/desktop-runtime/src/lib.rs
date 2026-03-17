mod identity;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use kukuri_app_api::{
    AppService, BlobMediaPayload, CreateGameRoomInput, CreateLiveSessionInput, GameRoomView,
    GameScoreView, LiveSessionView, PendingAttachment, SyncStatus, TimelineView,
    UpdateGameRoomInput,
};
use kukuri_blob_service::{BlobService, IrohBlobService};
use kukuri_cn_core::{
    AuthChallengeResponse, AuthVerifyResponse, CommunityNodeConsentStatus,
    CommunityNodeResolvedUrls, CommunityNodeSeedPeer, build_auth_envelope_json,
    normalize_http_url,
};
use kukuri_core::{AssetRole, GameRoomStatus, KukuriKeys};
use kukuri_docs_sync::{DocsSync, IrohDocsNode, IrohDocsSync};
use kukuri_store::{SqliteStore, TimelineCursor};
use kukuri_transport::{
    ConnectMode, DhtDiscoveryOptions, DiscoveryMode, IrohGossipTransport, SeedPeer, Transport,
    TransportNetworkConfig, TransportRelayConfig, parse_seed_peer,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::identity::{
    IdentityStorageMode, delete_optional_secret, load_optional_secret, load_or_create_keys,
    persist_optional_secret,
};

const DB_FILE_NAME: &str = "kukuri.db";
const DISCOVERY_CONFIG_FILE_EXTENSION: &str = "discovery.json";
const COMMUNITY_NODE_CONFIG_FILE_EXTENSION: &str = "community-node.json";
const DISCOVERY_MODE_ENV: &str = "KUKURI_DISCOVERY_MODE";
const DISCOVERY_SEEDS_ENV: &str = "KUKURI_DISCOVERY_SEEDS";
const COMMUNITY_NODE_TOKEN_PURPOSE: &str = "community-node-token";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub topic: String,
    pub content: String,
    pub reply_to: Option<String>,
    #[serde(default)]
    pub attachments: Vec<CreateAttachmentRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateAttachmentRequest {
    pub file_name: Option<String>,
    pub mime: String,
    pub byte_size: u64,
    pub data_base64: String,
    pub role: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTimelineRequest {
    pub topic: String,
    pub cursor: Option<TimelineCursor>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListThreadRequest {
    pub topic: String,
    pub thread_id: String,
    pub cursor: Option<TimelineCursor>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPeerTicketRequest {
    pub ticket: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeTopicRequest {
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBlobPreviewRequest {
    pub hash: String,
    pub mime: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBlobMediaRequest {
    pub hash: String,
    pub mime: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListLiveSessionsRequest {
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateLiveSessionRequest {
    pub topic: String,
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionCommandRequest {
    pub topic: String,
    pub session_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListGameRoomsRequest {
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGameRoomRequest {
    pub topic: String,
    pub title: String,
    pub description: String,
    pub participants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateGameRoomRequest {
    pub topic: String,
    pub room_id: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub mode: DiscoveryMode,
    pub connect_mode: ConnectMode,
    pub env_locked: bool,
    pub seed_peers: Vec<SeedPeer>,
}

impl DiscoveryConfig {
    fn static_peer_default() -> Self {
        Self {
            mode: DiscoveryMode::StaticPeer,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: false,
            seed_peers: Vec::new(),
        }
    }

    fn seeded_dht_default() -> Self {
        Self {
            mode: DiscoveryMode::SeededDht,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: false,
            seed_peers: Vec::new(),
        }
    }

    fn from_stored(stored: StoredDiscoveryConfig, env_locked: bool) -> Self {
        Self {
            mode: stored.mode,
            connect_mode: ConnectMode::DirectOnly,
            env_locked,
            seed_peers: normalize_seed_peers(stored.seed_peers),
        }
    }

    fn stored(&self) -> StoredDiscoveryConfig {
        StoredDiscoveryConfig {
            mode: self.mode.clone(),
            seed_peers: normalize_seed_peers(self.seed_peers.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDiscoverySeedsRequest {
    pub seed_entries: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredDiscoveryConfig {
    #[serde(default)]
    mode: DiscoveryMode,
    #[serde(default)]
    seed_peers: Vec<SeedPeer>,
}

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
struct StoredCommunityNodeToken {
    access_token: String,
    expires_at: i64,
}

pub struct DesktopRuntime {
    app_service: AppService,
    author_keys: Arc<KukuriKeys>,
    db_path: PathBuf,
    store: Arc<SqliteStore>,
    iroh_stack: SharedIrohStack,
    discovery_config: Arc<Mutex<DiscoveryConfig>>,
    community_node_config: Arc<Mutex<CommunityNodeConfig>>,
    startup_connectivity_urls: Vec<String>,
}

struct SharedIrohStack {
    _node: Arc<IrohDocsNode>,
    transport: Arc<IrohGossipTransport>,
    docs_sync: Arc<IrohDocsSync>,
    blob_service: Arc<IrohBlobService>,
}

impl DesktopRuntime {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config_and_identity_and_discovery(
            db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::from_env(),
            DiscoveryConfig::static_peer_default(),
            DhtDiscoveryOptions::disabled(),
        )
        .await
    }

    pub async fn new_with_config(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
    ) -> Result<Self> {
        Self::new_with_config_and_identity_and_discovery(
            db_path,
            network_config,
            IdentityStorageMode::from_env(),
            DiscoveryConfig::static_peer_default(),
            DhtDiscoveryOptions::disabled(),
        )
        .await
    }

    #[cfg(test)]
    async fn new_with_config_and_identity(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
        identity_mode: IdentityStorageMode,
    ) -> Result<Self> {
        Self::new_with_config_and_identity_and_discovery(
            db_path,
            network_config,
            identity_mode,
            DiscoveryConfig::static_peer_default(),
            DhtDiscoveryOptions::disabled(),
        )
        .await
    }

    async fn new_with_config_and_identity_and_discovery(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
        identity_mode: IdentityStorageMode,
        discovery_config: DiscoveryConfig,
        dht_options: DhtDiscoveryOptions,
    ) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let community_node_config =
            load_community_node_config_from_file(&db_path)?.unwrap_or_default();
        let relay_config = relay_config_from_community_node_config(&community_node_config);
        let effective_discovery_config = DiscoveryConfig {
            seed_peers: effective_seed_peers(&discovery_config, &community_node_config),
            ..discovery_config.clone()
        };
        let docs_root = db_path.with_extension("iroh-data");
        let store = Arc::new(SqliteStore::connect_file(&db_path).await?);
        let iroh_stack = SharedIrohStack::new(
            &docs_root,
            network_config.clone(),
            &effective_discovery_config,
            dht_options,
            relay_config.clone(),
        )
        .await?;
        let keys = load_or_create_keys(&db_path, identity_mode)?;
        let author_keys = Arc::new(keys.clone());
        let app_service = AppService::new_with_services(
            store.clone(),
            store.clone(),
            iroh_stack.transport.clone(),
            iroh_stack.transport.clone(),
            iroh_stack.docs_sync.clone(),
            iroh_stack.blob_service.clone(),
            keys,
        );

        Ok(Self {
            app_service,
            author_keys,
            db_path,
            store,
            iroh_stack,
            discovery_config: Arc::new(Mutex::new(discovery_config)),
            community_node_config: Arc::new(Mutex::new(community_node_config)),
            startup_connectivity_urls: relay_config.iroh_relay_urls,
        })
    }

    pub async fn from_env(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let discovery_config = resolve_discovery_config_from_env(&db_path)?;
        let dht_options = match discovery_config.mode {
            DiscoveryMode::SeededDht => DhtDiscoveryOptions::seeded_dht(),
            DiscoveryMode::StaticPeer => DhtDiscoveryOptions::disabled(),
        };
        Self::new_with_config_and_identity_and_discovery(
            &db_path,
            TransportNetworkConfig::from_env()?,
            IdentityStorageMode::from_env(),
            discovery_config,
            dht_options,
        )
        .await
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub async fn create_post(&self, request: CreatePostRequest) -> Result<String> {
        let attachments = request
            .attachments
            .into_iter()
            .map(pending_attachment_from_request)
            .collect::<Result<Vec<_>>>()?;
        self.app_service
            .create_post_with_attachments(
                request.topic.as_str(),
                request.content.as_str(),
                request.reply_to.as_deref(),
                attachments,
            )
            .await
    }

    pub async fn list_timeline(&self, request: ListTimelineRequest) -> Result<TimelineView> {
        self.app_service
            .list_timeline(
                request.topic.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn list_thread(&self, request: ListThreadRequest) -> Result<TimelineView> {
        self.app_service
            .list_thread(
                request.topic.as_str(),
                request.thread_id.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        self.app_service.get_sync_status().await
    }

    pub async fn get_discovery_config(&self) -> Result<DiscoveryConfig> {
        Ok(self.discovery_config.lock().await.clone())
    }

    pub async fn list_live_sessions(
        &self,
        request: ListLiveSessionsRequest,
    ) -> Result<Vec<LiveSessionView>> {
        self.app_service
            .list_live_sessions(request.topic.as_str())
            .await
    }

    pub async fn create_live_session(&self, request: CreateLiveSessionRequest) -> Result<String> {
        self.app_service
            .create_live_session(
                request.topic.as_str(),
                CreateLiveSessionInput {
                    title: request.title,
                    description: request.description,
                },
            )
            .await
    }

    pub async fn end_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .end_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn join_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .join_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn leave_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .leave_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn list_game_rooms(
        &self,
        request: ListGameRoomsRequest,
    ) -> Result<Vec<GameRoomView>> {
        self.app_service
            .list_game_rooms(request.topic.as_str())
            .await
    }

    pub async fn create_game_room(&self, request: CreateGameRoomRequest) -> Result<String> {
        self.app_service
            .create_game_room(
                request.topic.as_str(),
                CreateGameRoomInput {
                    title: request.title,
                    description: request.description,
                    participants: request.participants,
                },
            )
            .await
    }

    pub async fn update_game_room(&self, request: UpdateGameRoomRequest) -> Result<()> {
        self.app_service
            .update_game_room(
                request.topic.as_str(),
                request.room_id.as_str(),
                UpdateGameRoomInput {
                    status: request.status,
                    phase_label: request.phase_label,
                    scores: request.scores,
                },
            )
            .await
    }

    pub async fn import_peer_ticket(&self, request: ImportPeerTicketRequest) -> Result<()> {
        self.app_service
            .import_peer_ticket(request.ticket.as_str())
            .await
    }

    pub async fn set_discovery_seeds(
        &self,
        request: SetDiscoverySeedsRequest,
    ) -> Result<DiscoveryConfig> {
        let mut next_config = self.discovery_config.lock().await.clone();
        if next_config.env_locked {
            bail!("discovery configuration is locked by environment variables");
        }
        next_config.seed_peers = parse_seed_entries(&request.seed_entries)?;
        save_discovery_config(&self.db_path, &next_config.stored())?;
        *self.discovery_config.lock().await = next_config.clone();
        self.apply_effective_seed_peers().await?;
        Ok(next_config)
    }

    pub async fn unsubscribe_topic(&self, request: UnsubscribeTopicRequest) -> Result<()> {
        self.app_service
            .unsubscribe_topic(request.topic.as_str())
            .await
    }

    pub async fn local_peer_ticket(&self) -> Result<Option<String>> {
        self.app_service.peer_ticket().await
    }

    pub async fn get_blob_preview_url(
        &self,
        request: GetBlobPreviewRequest,
    ) -> Result<Option<String>> {
        self.app_service
            .blob_preview_data_url(request.hash.as_str(), request.mime.as_str())
            .await
    }

    pub async fn get_blob_media_payload(
        &self,
        request: GetBlobMediaRequest,
    ) -> Result<Option<BlobMediaPayload>> {
        self.app_service
            .blob_media_payload(request.hash.as_str(), request.mime.as_str())
            .await
    }

    pub async fn get_community_node_config(&self) -> Result<CommunityNodeConfig> {
        Ok(self.community_node_config.lock().await.clone())
    }

    pub async fn get_community_node_statuses(&self) -> Result<Vec<CommunityNodeNodeStatus>> {
        let config = self.community_node_config.lock().await.clone();
        let mut statuses = Vec::with_capacity(config.nodes.len());
        for node in config.nodes {
            statuses.push(self.community_node_status(node, None, None).await?);
        }
        Ok(statuses)
    }

    pub async fn set_community_node_config(
        &self,
        request: SetCommunityNodeConfigRequest,
    ) -> Result<CommunityNodeConfig> {
        let nodes = request
            .base_urls
            .into_iter()
            .map(|base_url| -> Result<CommunityNodeNodeConfig> {
                Ok(CommunityNodeNodeConfig {
                    base_url: normalize_http_url(base_url.as_str())?,
                    resolved_urls: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let next_config = normalize_community_node_config(CommunityNodeConfig { nodes })?;
        save_community_node_config(&self.db_path, &next_config)?;
        *self.community_node_config.lock().await = next_config.clone();
        self.apply_effective_seed_peers().await?;
        Ok(next_config)
    }

    pub async fn clear_community_node_config(&self) -> Result<()> {
        let existing = self.community_node_config.lock().await.clone();
        for node in existing.nodes {
            self.clear_community_node_token(CommunityNodeTargetRequest {
                base_url: node.base_url,
            })
            .await?;
        }
        remove_community_node_config(&self.db_path)?;
        *self.community_node_config.lock().await = CommunityNodeConfig::default();
        self.apply_effective_seed_peers().await?;
        Ok(())
    }

    pub async fn authenticate_community_node(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let client = community_node_http_client()?;
        let challenge_url = format!("{}/v1/auth/challenge", base_url);
        let pubkey = self.author_keys.public_key_hex();
        let endpoint_id = self
            .iroh_stack
            .transport
            .discovery()
            .await
            .context("failed to read local endpoint id for community node auth")?
            .local_endpoint_id;
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
                "endpoint_id": endpoint_id,
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
        persist_community_node_token(
            &self.db_path,
            IdentityStorageMode::from_env(),
            base_url.as_str(),
            &token,
        )?;
        let node = self.require_community_node(base_url.as_str()).await?;
        let consent_state = client
            .get(format!("{}/v1/consents/status", base_url))
            .bearer_auth(token.access_token.as_str())
            .send()
            .await
            .context("failed to fetch community node consent status")?
            .error_for_status()
            .context("community node consent status request failed")?
            .json::<CommunityNodeConsentStatus>()
            .await
            .context("failed to decode community node consent status")?;
        if consent_state.all_required_accepted {
            self.refresh_community_node_metadata(CommunityNodeTargetRequest {
                base_url: base_url.clone(),
            })
            .await?;
            let refreshed = self.require_community_node(base_url.as_str()).await?;
            return self
                .community_node_status(refreshed, Some(consent_state), None)
                .await;
        }
        self.community_node_status(node, Some(consent_state), None)
            .await
    }

    pub async fn clear_community_node_token(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        delete_optional_secret(
            &self.db_path,
            COMMUNITY_NODE_TOKEN_PURPOSE,
            base_url.as_str(),
        )?;
        let node = self
            .community_node_config
            .lock()
            .await
            .nodes
            .clone()
            .into_iter()
            .find(|node| node.base_url == base_url)
            .ok_or_else(|| anyhow!("community node `{base_url}` is not configured"))?;
        self.community_node_status(node, None, None).await
    }

    pub async fn get_community_node_consent_status(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        let client = community_node_http_client()?;
        let token = load_community_node_token(
            &self.db_path,
            IdentityStorageMode::from_env(),
            base_url.as_str(),
        )?
        .ok_or_else(|| anyhow!("community node authentication is required"))?;
        let consent_url = format!("{}/v1/consents/status", base_url);
        let response = client
            .get(consent_url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await
            .context("failed to fetch community node consent status")?;
        let status = response
            .error_for_status()
            .context("community node consent status request failed")?
            .json::<CommunityNodeConsentStatus>()
            .await
            .context("failed to decode community node consent status")?;
        self.community_node_status(node, Some(status), None).await
    }

    pub async fn accept_community_node_consents(
        &self,
        request: AcceptCommunityNodeConsentsRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        let client = community_node_http_client()?;
        let token = load_community_node_token(
            &self.db_path,
            IdentityStorageMode::from_env(),
            base_url.as_str(),
        )?
        .ok_or_else(|| anyhow!("community node authentication is required"))?;
        let consent_url = format!("{}/v1/consents", base_url);
        let response = client
            .post(consent_url)
            .bearer_auth(token.access_token.as_str())
            .json(&serde_json::json!({ "policy_slugs": request.policy_slugs }))
            .send()
            .await
            .context("failed to accept community node consents")?;
        let status = response
            .error_for_status()
            .context("community node consent accept request failed")?
            .json::<CommunityNodeConsentStatus>()
            .await
            .context("failed to decode accepted community node consents")?;
        if status.all_required_accepted {
            self.refresh_community_node_metadata(CommunityNodeTargetRequest {
                base_url: base_url.clone(),
            })
            .await?;
            let refreshed = self.require_community_node(base_url.as_str()).await?;
            return self
                .community_node_status(refreshed, Some(status), None)
                .await;
        }
        self.community_node_status(node, Some(status), None).await
    }

    pub async fn refresh_community_node_metadata(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let mut config = self.community_node_config.lock().await.clone();
        let Some(index) = config
            .nodes
            .iter()
            .position(|node| node.base_url == base_url)
        else {
            bail!("community node `{base_url}` is not configured");
        };
        let client = community_node_http_client()?;
        let token = load_community_node_token(
            &self.db_path,
            IdentityStorageMode::from_env(),
            base_url.as_str(),
        )?
        .ok_or_else(|| anyhow!("community node authentication is required"))?;
        let bootstrap_url = format!("{}/v1/bootstrap/nodes", base_url);
        let response = client
            .get(bootstrap_url)
            .bearer_auth(token.access_token.as_str())
            .send()
            .await
            .context("failed to refresh community node metadata")?;
        let bootstrap = response
            .error_for_status()
            .context("community node bootstrap request failed")?
            .json::<serde_json::Value>()
            .await
            .context("failed to decode community node bootstrap response")?;
        let nodes = bootstrap
            .get("nodes")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| anyhow!("community node bootstrap response is missing nodes"))?;
        let resolved_urls = nodes
            .iter()
            .filter_map(|node| {
                let candidate_base_url = node.get("base_url")?.as_str()?;
                if candidate_base_url != base_url {
                    return None;
                }
                serde_json::from_value::<CommunityNodeNodeConfig>(node.clone())
                    .ok()
                    .and_then(|node| node.resolved_urls)
            })
            .next()
            .ok_or_else(|| anyhow!("community node bootstrap response is missing self metadata"))?;
        config.nodes[index].resolved_urls = Some(resolved_urls);
        let normalized = normalize_community_node_config(config)?;
        save_community_node_config(&self.db_path, &normalized)?;
        *self.community_node_config.lock().await = normalized.clone();
        self.apply_effective_seed_peers().await?;
        let node = normalized.nodes[index].clone();
        self.community_node_status(node, None, None).await
    }

    pub async fn shutdown(&self) {
        self.app_service.shutdown().await;
        self.iroh_stack.shutdown().await;
        self.store.close().await;
    }
}

impl DesktopRuntime {
    async fn require_community_node(&self, base_url: &str) -> Result<CommunityNodeNodeConfig> {
        self.community_node_config
            .lock()
            .await
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .cloned()
            .ok_or_else(|| anyhow!("community node `{base_url}` is not configured"))
    }

    async fn community_node_status(
        &self,
        node: CommunityNodeNodeConfig,
        consent_state: Option<CommunityNodeConsentStatus>,
        last_error: Option<String>,
    ) -> Result<CommunityNodeNodeStatus> {
        let token = load_community_node_token(
            &self.db_path,
            IdentityStorageMode::from_env(),
            node.base_url.as_str(),
        )?;
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
            restart_required: current_connectivity_urls != self.startup_connectivity_urls,
        })
    }

    async fn apply_effective_seed_peers(&self) -> Result<()> {
        let discovery_config = self.discovery_config.lock().await.clone();
        let community_node_config = self.community_node_config.lock().await.clone();
        let seed_peers = effective_seed_peers(&discovery_config, &community_node_config);
        self.app_service
            .set_discovery_seeds(
                discovery_config.mode.clone(),
                discovery_config.env_locked,
                seed_peers,
            )
            .await
    }
}

pub fn resolve_db_path_from_env(base_app_data_dir: &Path) -> Result<PathBuf> {
    let mut app_data_dir = std::env::var("KUKURI_APP_DATA_DIR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| base_app_data_dir.to_path_buf());

    if app_data_dir == base_app_data_dir
        && let Some(instance) = std::env::var("KUKURI_INSTANCE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    {
        app_data_dir = app_data_dir.join(instance);
    }

    fs::create_dir_all(&app_data_dir)
        .with_context(|| format!("failed to create app data dir `{}`", app_data_dir.display()))?;

    let db_path = app_data_dir.join(DB_FILE_NAME);
    Ok(db_path)
}

fn pending_attachment_from_request(request: CreateAttachmentRequest) -> Result<PendingAttachment> {
    let bytes = BASE64_STANDARD
        .decode(request.data_base64.as_bytes())
        .context("failed to decode attachment data")?;
    let role = match request.role.as_deref() {
        Some("image_preview") => AssetRole::ImagePreview,
        Some("video_poster") => AssetRole::VideoPoster,
        Some("video_manifest") => AssetRole::VideoManifest,
        Some("attachment") => AssetRole::Attachment,
        _ => AssetRole::ImageOriginal,
    };
    Ok(PendingAttachment {
        mime: request.mime,
        bytes,
        role,
    })
}

fn discovery_config_path(db_path: &Path) -> PathBuf {
    db_path.with_extension(DISCOVERY_CONFIG_FILE_EXTENSION)
}

fn load_discovery_config_from_file(db_path: &Path) -> Result<Option<StoredDiscoveryConfig>> {
    let path = discovery_config_path(db_path);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read discovery config `{}`", path.display()))?;
    let config = serde_json::from_str::<StoredDiscoveryConfig>(&raw)
        .with_context(|| format!("failed to parse discovery config `{}`", path.display()))?;
    Ok(Some(config))
}

fn save_discovery_config(db_path: &Path, config: &StoredDiscoveryConfig) -> Result<()> {
    let path = discovery_config_path(db_path);
    let json = serde_json::to_vec_pretty(config)
        .with_context(|| format!("failed to encode discovery config `{}`", path.display()))?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write discovery config `{}`", path.display()))
}

fn community_node_config_path(db_path: &Path) -> PathBuf {
    db_path.with_extension(COMMUNITY_NODE_CONFIG_FILE_EXTENSION)
}

fn load_community_node_config_from_file(db_path: &Path) -> Result<Option<CommunityNodeConfig>> {
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

fn save_community_node_config(db_path: &Path, config: &CommunityNodeConfig) -> Result<()> {
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

fn remove_community_node_config(db_path: &Path) -> Result<()> {
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

fn normalize_community_node_config(config: CommunityNodeConfig) -> Result<CommunityNodeConfig> {
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

fn merge_community_node_resolved_urls(
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

fn effective_seed_peers(
    discovery_config: &DiscoveryConfig,
    community_node_config: &CommunityNodeConfig,
) -> Vec<SeedPeer> {
    normalize_seed_peers(
        discovery_config
            .seed_peers
            .iter()
            .cloned()
            .chain(community_node_seed_peers(community_node_config))
            .collect(),
    )
}

fn community_node_seed_peers(config: &CommunityNodeConfig) -> impl Iterator<Item = SeedPeer> + '_ {
    config
        .nodes
        .iter()
        .filter_map(|node| node.resolved_urls.as_ref())
        .flat_map(|resolved| resolved.seed_peers.iter())
        .filter_map(seed_peer_from_community_node)
}

fn seed_peer_from_community_node(seed_peer: &CommunityNodeSeedPeer) -> Option<SeedPeer> {
    let endpoint_id = seed_peer.endpoint_id.trim();
    if endpoint_id.is_empty() {
        return None;
    }
    Some(SeedPeer {
        endpoint_id: endpoint_id.to_string(),
        addr_hint: seed_peer.addr_hint.clone(),
    })
}

fn relay_config_from_community_node_config(config: &CommunityNodeConfig) -> TransportRelayConfig {
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

fn load_community_node_token(
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

fn persist_community_node_token(
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

fn community_node_http_client() -> Result<Client> {
    Client::builder()
        .build()
        .context("failed to build community-node http client")
}

fn resolve_discovery_config_from_env(db_path: &Path) -> Result<DiscoveryConfig> {
    let env_mode = std::env::var(DISCOVERY_MODE_ENV).ok();
    let env_seeds = std::env::var(DISCOVERY_SEEDS_ENV).ok();
    let env_locked = env_mode.is_some() || env_seeds.is_some();

    if env_locked {
        let mode = match env_mode.as_deref() {
            Some(value) => parse_discovery_mode(value)?,
            None => DiscoveryMode::SeededDht,
        };
        let seed_peers = parse_seed_entries_from_csv(env_seeds.as_deref().unwrap_or(""))?;
        return Ok(DiscoveryConfig {
            mode,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: true,
            seed_peers,
        });
    }

    if let Some(stored) = load_discovery_config_from_file(db_path)? {
        return Ok(DiscoveryConfig::from_stored(stored, false));
    }

    Ok(DiscoveryConfig::seeded_dht_default())
}

fn parse_discovery_mode(value: &str) -> Result<DiscoveryMode> {
    match value.trim() {
        "static_peer" => Ok(DiscoveryMode::StaticPeer),
        "seeded_dht" => Ok(DiscoveryMode::SeededDht),
        other => Err(anyhow!(
            "invalid {} value `{}` (expected static_peer or seeded_dht)",
            DISCOVERY_MODE_ENV,
            other
        )),
    }
}

fn parse_seed_entries(entries: &[String]) -> Result<Vec<SeedPeer>> {
    parse_seed_entries_from_iter(entries.iter().map(String::as_str))
}

fn parse_seed_entries_from_csv(value: &str) -> Result<Vec<SeedPeer>> {
    parse_seed_entries_from_iter(value.split(','))
}

fn parse_seed_entries_from_iter<'a>(
    entries: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<SeedPeer>> {
    let mut parsed = Vec::new();
    for entry in entries {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        parsed.push(parse_seed_peer(trimmed)?);
    }
    Ok(normalize_seed_peers(parsed))
}

fn normalize_seed_peers(peers: Vec<SeedPeer>) -> Vec<SeedPeer> {
    let mut deduped = std::collections::BTreeMap::new();
    for peer in peers {
        deduped.insert(peer.display(), peer);
    }
    deduped.into_values().collect()
}

impl SharedIrohStack {
    async fn new(
        root: &Path,
        network_config: TransportNetworkConfig,
        discovery_config: &DiscoveryConfig,
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let node = IrohDocsNode::persistent_with_discovery_config(
            root,
            network_config.clone(),
            dht_options,
            relay_config.clone(),
        )
        .await?;
        let transport = Arc::new(IrohGossipTransport::from_shared_parts(
            node.endpoint().clone(),
            node.gossip().clone(),
            node.discovery(),
            network_config,
            relay_config.clone(),
        )?);
        let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
        let blob_service = Arc::new(IrohBlobService::new(node.clone()));
        transport
            .configure_discovery(
                discovery_config.mode.clone(),
                discovery_config.env_locked,
                discovery_config.seed_peers.clone(),
            )
            .await?;
        docs_sync
            .set_seed_peers(discovery_config.seed_peers.clone())
            .await?;
        blob_service
            .set_seed_peers(discovery_config.seed_peers.clone())
            .await?;
        Ok(Self {
            _node: node,
            transport,
            docs_sync,
            blob_service,
        })
    }

    async fn shutdown(&self) {
        let _ = self._node.clone().shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use iroh::address_lookup::EndpointInfo;
    use pkarr::errors::{ConcurrencyError, PublishError};
    use pkarr::{Client as PkarrClient, SignedPacket, Timestamp, mainline::Testnet};
    use tempfile::tempdir;
    use tokio::time::{Duration, sleep, timeout};

    fn image_attachment_request(name: &str, mime: &str, bytes: &[u8]) -> CreateAttachmentRequest {
        CreateAttachmentRequest {
            file_name: Some(name.to_string()),
            mime: mime.to_string(),
            byte_size: bytes.len() as u64,
            data_base64: BASE64_STANDARD.encode(bytes),
            role: Some("image_original".to_string()),
        }
    }

    fn video_attachment_request(
        name: &str,
        mime: &str,
        bytes: &[u8],
        role: &str,
    ) -> CreateAttachmentRequest {
        CreateAttachmentRequest {
            file_name: Some(name.to_string()),
            mime: mime.to_string(),
            byte_size: bytes.len() as u64,
            data_base64: BASE64_STANDARD.encode(bytes),
            role: Some(role.to_string()),
        }
    }

    fn dht_test_client(testnet: &Testnet) -> PkarrClient {
        let mut builder = PkarrClient::builder();
        builder.no_default_network().bootstrap(&testnet.bootstrap);
        builder.build().expect("pkarr client")
    }

    fn build_endpoint_signed_packet_with_timestamp(
        endpoint_info: &EndpointInfo,
        secret_key: &iroh::SecretKey,
        ttl: u32,
        timestamp: Timestamp,
    ) -> SignedPacket {
        use pkarr::dns::{self, rdata};

        let keypair = pkarr::Keypair::from_secret_key(&secret_key.to_bytes());
        let mut builder = SignedPacket::builder().timestamp(timestamp);
        let name = dns::Name::new("_iroh").expect("iroh txt name");
        for entry in endpoint_info.to_txt_strings() {
            let mut txt = rdata::TXT::new();
            txt.add_string(&entry)
                .expect("valid endpoint info txt entry");
            builder = builder.txt(name.clone(), txt.into_owned(), ttl);
        }
        builder.sign(&keypair).expect("sign endpoint info packet")
    }

    async fn publish_runtime_endpoint_to_testnet(runtime: &DesktopRuntime, testnet: &Testnet) {
        let endpoint = runtime.iroh_stack._node.endpoint();
        let client = dht_test_client(testnet);
        let public_key =
            pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
        let expected_info = EndpointInfo::from(endpoint.addr());
        for _ in 0..20 {
            let previous_timestamp = client
                .resolve_most_recent(&public_key)
                .await
                .map(|packet| packet.timestamp());
            let now = Timestamp::now();
            let timestamp = match previous_timestamp {
                Some(previous) if previous >= now => previous + 1,
                _ => now,
            };
            let signed_packet = build_endpoint_signed_packet_with_timestamp(
                &expected_info,
                endpoint.secret_key(),
                1,
                timestamp,
            );
            match client.publish(&signed_packet, previous_timestamp).await {
                Ok(()) => break,
                Err(PublishError::Concurrency(
                    ConcurrencyError::ConflictRisk
                    | ConcurrencyError::NotMostRecent
                    | ConcurrencyError::CasFailed,
                )) => sleep(Duration::from_millis(50)).await,
                Err(error) => panic!("publish endpoint info: {error}"),
            }
        }
        timeout(Duration::from_secs(5), async {
            loop {
                if client
                    .resolve_most_recent(&public_key)
                    .await
                    .as_ref()
                    .and_then(|packet| EndpointInfo::from_pkarr_signed_packet(packet).ok())
                    .is_some_and(|packet_info| {
                        packet_info.to_txt_strings() == expected_info.to_txt_strings()
                    })
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("resolve published endpoint info");
    }

    fn seeded_dht_config(seed_peers: Vec<SeedPeer>) -> DiscoveryConfig {
        DiscoveryConfig {
            mode: DiscoveryMode::SeededDht,
            connect_mode: ConnectMode::DirectOnly,
            env_locked: false,
            seed_peers,
        }
    }

    async fn new_seeded_dht_runtime_with_config(
        db_path: &Path,
        testnet: &Testnet,
        discovery_config: DiscoveryConfig,
    ) -> DesktopRuntime {
        let runtime = DesktopRuntime::new_with_config_and_identity_and_discovery(
            db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
            discovery_config,
            DhtDiscoveryOptions::with_client(dht_test_client(testnet)),
        )
        .await
        .expect("seeded dht runtime");
        publish_runtime_endpoint_to_testnet(&runtime, testnet).await;
        runtime
    }

    async fn new_seeded_dht_runtime(db_path: &Path, testnet: &Testnet) -> DesktopRuntime {
        new_seeded_dht_runtime_with_config(db_path, testnet, seeded_dht_config(Vec::new())).await
    }

    #[test]
    fn resolve_db_path_ignores_legacy_runtime_artifacts() {
        let dir = tempdir().expect("tempdir");
        let legacy_db_path = dir.path().join("kukuri-next.db");
        let legacy_data_dir = dir.path().join("kukuri-next.iroh-data");
        fs::write(&legacy_db_path, b"sqlite").expect("legacy db");
        fs::create_dir_all(&legacy_data_dir).expect("legacy data dir");
        fs::write(legacy_data_dir.join("blob.bin"), b"blob").expect("legacy blob");

        let resolved = resolve_db_path_from_env(dir.path()).expect("resolved db path");

        assert_eq!(resolved, dir.path().join("kukuri.db"));
        assert!(!resolved.exists());
        assert!(!resolved.with_extension("iroh-data").exists());
        assert!(legacy_db_path.exists());
        assert!(legacy_data_dir.join("blob.bin").exists());
    }

    #[tokio::test]
    async fn desktop_runtime_persists_posts_and_author_identity_after_restart() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let runtime = timeout(
            Duration::from_secs(15),
            DesktopRuntime::new_with_config_and_identity(
                &db_path,
                TransportNetworkConfig::loopback(),
                IdentityStorageMode::FileOnly,
            ),
        )
        .await
        .expect("runtime creation timeout")
        .expect("runtime");
        let object_id = runtime
            .create_post(CreatePostRequest {
                topic: "kukuri:topic:runtime".into(),
                content: "persist me".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");
        timeout(Duration::from_secs(15), runtime.shutdown())
            .await
            .expect("runtime shutdown timeout");
        drop(runtime);

        let restarted = timeout(
            Duration::from_secs(15),
            DesktopRuntime::new_with_config_and_identity(
                &db_path,
                TransportNetworkConfig::loopback(),
                IdentityStorageMode::FileOnly,
            ),
        )
        .await
        .expect("runtime restart timeout")
        .expect("runtime restart");
        let restarted_object_id = restarted
            .create_post(CreatePostRequest {
                topic: "kukuri:topic:runtime".into(),
                content: "persist me again".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post after restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: "kukuri:topic:runtime".into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");

        assert!(
            timeline
                .items
                .iter()
                .any(|post| post.object_id == object_id)
        );
        assert!(
            timeline
                .items
                .iter()
                .any(|post| post.object_id == restarted_object_id)
        );
        let original_post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("original post");
        let restarted_post = timeline
            .items
            .iter()
            .find(|post| post.object_id == restarted_object_id)
            .expect("restarted post");
        assert_eq!(original_post.author_pubkey, restarted_post.author_pubkey);
        assert_eq!(restarted.db_path(), db_path.as_path());
        timeout(Duration::from_secs(15), restarted.shutdown())
            .await
            .expect("restarted shutdown timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn desktop_runtime_syncs_between_instances() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("a.db");
        let db_b = dir.path().join("b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let runtime_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime b");
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = runtime_b
            .local_peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");

        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_b })
            .await
            .expect("import b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .expect("import a");

        let topic = "kukuri:topic:desktop-runtime";
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");
        let object_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello desktop runtime".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("runtime sync timeout");

        assert_eq!(received.content, "hello desktop runtime");
        let status = runtime_b.get_sync_status().await.expect("sync status");
        assert!(status.last_sync_ts.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn set_discovery_seeds_reapplies_runtime_without_restart() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("seeded-a.db");
        let db_b = dir.path().join("seeded-b.db");
        let testnet = Testnet::new(5).expect("testnet");
        let runtime_a = new_seeded_dht_runtime(&db_a, &testnet).await;
        let runtime_b = new_seeded_dht_runtime(&db_b, &testnet).await;
        let endpoint_a = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;

        runtime_a
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_b.clone()],
            })
            .await
            .expect("set seeds a");
        runtime_b
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_a.clone()],
            })
            .await
            .expect("set seeds b");

        let config_a = runtime_a
            .get_discovery_config()
            .await
            .expect("discovery config a");
        let config_b = runtime_b
            .get_discovery_config()
            .await
            .expect("discovery config b");
        assert_eq!(config_a.seed_peers[0].endpoint_id, endpoint_b);
        assert_eq!(config_b.seed_peers[0].endpoint_id, endpoint_a);
        let topic = "kukuri:topic:runtime-seeded-dht";
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe a");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");
        timeout(Duration::from_secs(20), async {
            loop {
                let status_a = runtime_a.get_sync_status().await.expect("status a");
                let status_b = runtime_b.get_sync_status().await.expect("status b");
                let ready_a = status_a
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                let ready_b = status_b
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                if ready_a && ready_b {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded runtime ready timeout");

        let object_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello seeded runtime".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded runtime sync timeout");

        assert_eq!(received.content, "hello seeded runtime");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn restart_restores_seeded_dht_config_and_reconnects() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("restart-seeded-a.db");
        let db_b = dir.path().join("restart-seeded-b.db");
        let testnet = Testnet::new(5).expect("testnet");
        let runtime_a = new_seeded_dht_runtime(&db_a, &testnet).await;
        let runtime_b = new_seeded_dht_runtime(&db_b, &testnet).await;
        let endpoint_a = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;

        runtime_a
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_b.clone()],
            })
            .await
            .expect("set seeds a");
        runtime_b
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec![endpoint_a.clone()],
            })
            .await
            .expect("set seeds b");

        timeout(Duration::from_secs(15), runtime_a.shutdown())
            .await
            .expect("shutdown a");
        timeout(Duration::from_secs(15), runtime_b.shutdown())
            .await
            .expect("shutdown b");
        drop(runtime_a);
        drop(runtime_b);

        let restored_a =
            resolve_discovery_config_from_env(&db_a).expect("restored discovery config a");
        let restored_b =
            resolve_discovery_config_from_env(&db_b).expect("restored discovery config b");
        let restarted_a =
            new_seeded_dht_runtime_with_config(&db_a, &testnet, restored_a.clone()).await;
        let restarted_b =
            new_seeded_dht_runtime_with_config(&db_b, &testnet, restored_b.clone()).await;
        let restarted_endpoint_a = restarted_a
            .get_sync_status()
            .await
            .expect("restarted status a")
            .discovery
            .local_endpoint_id;
        let restarted_endpoint_b = restarted_b
            .get_sync_status()
            .await
            .expect("restarted status b")
            .discovery
            .local_endpoint_id;

        assert_eq!(restored_a.mode, DiscoveryMode::SeededDht);
        assert_eq!(restored_b.mode, DiscoveryMode::SeededDht);
        assert_eq!(restored_a.seed_peers[0].endpoint_id, endpoint_b);
        assert_eq!(restored_b.seed_peers[0].endpoint_id, endpoint_a);
        assert_eq!(restarted_endpoint_a, endpoint_a);
        assert_eq!(restarted_endpoint_b, endpoint_b);
        let topic = "kukuri:topic:runtime-seeded-restart";
        let _ = restarted_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted a");
        let _ = restarted_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted b");

        let object_id = restarted_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello after restart".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = restarted_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("restart seeded dht sync timeout");

        assert_eq!(received.content, "hello after restart");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_seed_entry_rejected_without_mutating_runtime() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("invalid-seed.db");
        let testnet = Testnet::new(5).expect("testnet");
        let runtime = new_seeded_dht_runtime(&db_path, &testnet).await;

        let error = runtime
            .set_discovery_seeds(SetDiscoverySeedsRequest {
                seed_entries: vec!["not-a-node-id".into()],
            })
            .await
            .expect_err("invalid seed should fail");
        assert!(error.to_string().contains("invalid seed endpoint id"));

        let config = runtime
            .get_discovery_config()
            .await
            .expect("discovery config");
        assert!(config.seed_peers.is_empty());
        assert!(!discovery_config_path(&db_path).exists());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_timeline_from_docs() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("late-a.db");
        let db_b = dir.path().join("late-b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let topic = "kukuri:topic:late-join";
        let object_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello from before join".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post before join");
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        let runtime_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late join timeout");

        assert_eq!(received.content, "hello from before join");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_image_post_from_docs() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("late-image-a.db");
        let db_b = dir.path().join("late-image-b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let topic = "kukuri:topic:late-image-runtime";
        let object_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "late image".into(),
                reply_to: None,
                attachments: vec![image_attachment_request(
                    "late.png",
                    "image/png",
                    b"late-image-runtime",
                )],
            })
            .await
            .expect("create image post before join");
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        let runtime_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late image timeout");

        assert_eq!(received.attachments.len(), 1);
        let preview = runtime_b
            .get_blob_preview_url(GetBlobPreviewRequest {
                hash: received.attachments[0].hash.clone(),
                mime: received.attachments[0].mime.clone(),
            })
            .await
            .expect("blob preview");
        assert!(preview.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_video_media_payload() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("late-video-a.db");
        let db_b = dir.path().join("late-video-b.db");
        let runtime_a = DesktopRuntime::new_with_config_and_identity(
            &db_a,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime a");
        let topic = "kukuri:topic:late-video-runtime";
        let object_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "late video".into(),
                reply_to: None,
                attachments: vec![
                    video_attachment_request(
                        "late-video.mp4",
                        "video/mp4",
                        b"late-video-runtime",
                        "video_manifest",
                    ),
                    video_attachment_request(
                        "late-poster.jpg",
                        "image/jpeg",
                        b"late-video-poster",
                        "video_poster",
                    ),
                ],
            })
            .await
            .expect("create video post before join");
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        let runtime_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late video timeout");

        let poster = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("video poster");
        let preview = runtime_b
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: poster.hash.clone(),
                mime: poster.mime.clone(),
            })
            .await
            .expect("video poster payload");
        assert!(preview.is_some());
        let manifest = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("video manifest");
        let playback = runtime_b
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: manifest.hash.clone(),
                mime: manifest.mime.clone(),
            })
            .await
            .expect("video playback payload");
        assert!(playback.is_some());
    }

    #[tokio::test]
    async fn blob_media_payload_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("blob-media-roundtrip.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:blob-media-roundtrip";
        let object_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "roundtrip".into(),
                reply_to: None,
                attachments: vec![image_attachment_request(
                    "roundtrip.png",
                    "image/png",
                    b"blob-media-roundtrip",
                )],
            })
            .await
            .expect("create image post");
        let timeline = runtime
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");
        let created = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("created post");

        let payload = runtime
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: created.attachments[0].hash.clone(),
                mime: created.attachments[0].mime.clone(),
            })
            .await
            .expect("blob media payload")
            .expect("blob media payload present");

        assert_eq!(payload.mime, "image/png");
        assert_eq!(
            payload.bytes_base64,
            BASE64_STANDARD.encode(b"blob-media-roundtrip")
        );
    }

    #[tokio::test]
    async fn sqlite_deletion_does_not_lose_shared_state() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("delete-sqlite.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:sqlite-delete";
        let root_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "root".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("root post");
        let reply_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "reply".into(),
                reply_to: Some(root_id.clone()),
                attachments: vec![],
            })
            .await
            .expect("reply post");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");
        let thread = restarted
            .list_thread(ListThreadRequest {
                topic: topic.into(),
                thread_id: root_id.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("thread");

        assert!(timeline.items.iter().any(|post| post.object_id == root_id));
        assert!(timeline.items.iter().any(|post| post.object_id == reply_id));
        assert!(thread.items.iter().any(|post| post.object_id == root_id));
        assert!(thread.items.iter().any(|post| post.object_id == reply_id));
    }

    #[tokio::test]
    async fn restart_restores_from_docs_blobs_without_sqlite_seed() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-no-seed.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-no-seed";
        let object_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "restored from docs".into(),
                reply_to: None,
                attachments: vec![],
            })
            .await
            .expect("create post");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");

        let restored = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("restored post");
        assert_eq!(restored.content, "restored from docs");
    }

    #[tokio::test]
    async fn restart_restores_image_post_preview() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-image.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-image";
        let object_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "restored image".into(),
                reply_to: None,
                attachments: vec![image_attachment_request(
                    "restored.png",
                    "image/png",
                    b"restart-image-preview",
                )],
            })
            .await
            .expect("create image post");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");
        let restored = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("restored image post");

        assert_eq!(restored.attachments.len(), 1);
        let preview = restarted
            .get_blob_preview_url(GetBlobPreviewRequest {
                hash: restored.attachments[0].hash.clone(),
                mime: restored.attachments[0].mime.clone(),
            })
            .await
            .expect("preview after restart");
        assert!(preview.is_some());
    }

    #[tokio::test]
    async fn restart_restores_video_media_payload() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-video.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-video";
        let object_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "restored video".into(),
                reply_to: None,
                attachments: vec![
                    video_attachment_request(
                        "clip.mp4",
                        "video/mp4",
                        b"restart-video-manifest",
                        "video_manifest",
                    ),
                    video_attachment_request(
                        "clip-poster.jpg",
                        "image/jpeg",
                        b"restart-video-poster",
                        "video_poster",
                    ),
                ],
            })
            .await
            .expect("create video post");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline");
        let restored = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("restored video post");

        let poster = restored
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("restored poster");
        let preview = restarted
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: poster.hash.clone(),
                mime: poster.mime.clone(),
            })
            .await
            .expect("video payload after restart");
        assert!(preview.is_some());
        let manifest = restored
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("restored video manifest");
        let playback = restarted
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: manifest.hash.clone(),
                mime: manifest.mime.clone(),
            })
            .await
            .expect("video playback payload after restart");
        assert!(playback.is_some());
    }

    #[tokio::test]
    async fn restart_restores_live_session_manifest() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-live.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-live";
        let session_id = runtime
            .create_live_session(CreateLiveSessionRequest {
                topic: topic.into(),
                title: "restart live".into(),
                description: "session".into(),
            })
            .await
            .expect("create live session");
        runtime
            .join_live_session(LiveSessionCommandRequest {
                topic: topic.into(),
                session_id: session_id.clone(),
            })
            .await
            .expect("join live session");
        runtime
            .end_live_session(LiveSessionCommandRequest {
                topic: topic.into(),
                session_id: session_id.clone(),
            })
            .await
            .expect("end live session");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let sessions = restarted
            .list_live_sessions(ListLiveSessionsRequest {
                topic: topic.into(),
            })
            .await
            .expect("list live sessions");
        let restored = sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .expect("restored live session");
        assert_eq!(restored.status, kukuri_core::LiveSessionStatus::Ended);
        assert_eq!(restored.viewer_count, 0);
        assert!(!restored.joined_by_me);
    }

    #[tokio::test]
    async fn restart_restores_game_room_manifest() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("restart-game.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let topic = "kukuri:topic:restart-game";
        let room_id = runtime
            .create_game_room(CreateGameRoomRequest {
                topic: topic.into(),
                title: "restart finals".into(),
                description: "set".into(),
                participants: vec!["Alice".into(), "Bob".into()],
            })
            .await
            .expect("create game room");
        runtime
            .update_game_room(UpdateGameRoomRequest {
                topic: topic.into(),
                room_id: room_id.clone(),
                status: GameRoomStatus::Running,
                phase_label: Some("Round 3".into()),
                scores: vec![
                    GameScoreView {
                        participant_id: "participant-1".into(),
                        label: "Alice".into(),
                        score: 2,
                    },
                    GameScoreView {
                        participant_id: "participant-2".into(),
                        label: "Bob".into(),
                        score: 1,
                    },
                ],
            })
            .await
            .expect("update game room");
        runtime.shutdown().await;
        drop(runtime);
        std::fs::remove_file(&db_path).expect("delete sqlite");

        let restarted = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart");
        let rooms = restarted
            .list_game_rooms(ListGameRoomsRequest {
                topic: topic.into(),
            })
            .await
            .expect("list game rooms");
        let restored = rooms
            .iter()
            .find(|room| room.room_id == room_id)
            .expect("restored game room");
        assert_eq!(restored.status, GameRoomStatus::Running);
        assert_eq!(restored.phase_label.as_deref(), Some("Round 3"));
        assert_eq!(
            restored
                .scores
                .iter()
                .find(|score| score.label == "Alice")
                .map(|score| score.score),
            Some(2)
        );
    }

    #[test]
    fn community_node_config_normalizes_base_urls_and_connectivity_urls() {
        let config = normalize_community_node_config(CommunityNodeConfig {
            nodes: vec![
                CommunityNodeNodeConfig {
                    base_url: "https://community.example.com/".into(),
                    resolved_urls: Some(
                        CommunityNodeResolvedUrls::new(
                            "https://public.example.com/",
                            vec![
                                "https://relay-b.example.com/".into(),
                                "https://relay-a.example.com/".into(),
                                "https://relay-a.example.com/".into(),
                            ],
                            vec![CommunityNodeSeedPeer::new("peer-b", None).expect("seed peer")],
                        )
                        .expect("resolved urls"),
                    ),
                },
                CommunityNodeNodeConfig {
                    base_url: "https://community.example.com".into(),
                    resolved_urls: None,
                },
            ],
        })
        .expect("normalized config");

        assert_eq!(config.nodes.len(), 1);
        assert_eq!(config.nodes[0].base_url, "https://community.example.com");
        assert_eq!(
            config.nodes[0]
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .connectivity_urls,
            vec![
                "https://relay-a.example.com".to_string(),
                "https://relay-b.example.com".to_string(),
            ]
        );
        assert_eq!(
            config.nodes[0]
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .seed_peers,
            vec![CommunityNodeSeedPeer::new("peer-b", None).expect("seed peer")]
        );
    }

    #[test]
    fn community_node_config_preserves_public_kukuri_urls() {
        let config = normalize_community_node_config(CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: "https://api.kukuri.app/".into(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://api.kukuri.app/",
                        vec!["https://iroh-relay.kukuri.app/".into()],
                        Vec::new(),
                    )
                    .expect("resolved urls"),
                ),
            }],
        })
        .expect("normalized config");

        let resolved = config.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls");

        assert_eq!(config.nodes[0].base_url, "https://api.kukuri.app");
        assert_eq!(resolved.public_base_url, "https://api.kukuri.app");
        assert_eq!(
            resolved.connectivity_urls,
            vec!["https://iroh-relay.kukuri.app".to_string()]
        );
        assert!(
            resolved
                .connectivity_urls
                .iter()
                .all(|url| !url.contains("api.kukuri.app/relay"))
        );
    }

    #[test]
    fn stored_community_node_config_restores_cached_connectivity_union() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("community-relay.db");
        save_community_node_config(
            &db_path,
            &CommunityNodeConfig {
                nodes: vec![CommunityNodeNodeConfig {
                    base_url: "https://community.example.com".into(),
                    resolved_urls: Some(
                        CommunityNodeResolvedUrls::new(
                            "https://public.example.com",
                            vec!["https://relay.example.com".into()],
                            vec![CommunityNodeSeedPeer::new("peer-a", None).expect("seed peer")],
                        )
                        .expect("resolved urls"),
                    ),
                }],
            },
        )
        .expect("save community node config");
        let restored = load_community_node_config_from_file(&db_path)
            .expect("load community node config")
            .expect("community node config");
        let relay_config = relay_config_from_community_node_config(&restored);

        assert_eq!(relay_config.connect_mode(), ConnectMode::DirectOrRelay);
        assert_eq!(
            relay_config.iroh_relay_urls,
            vec!["https://relay.example.com".to_string()]
        );
        assert_eq!(
            restored.nodes[0]
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .seed_peers,
            vec![CommunityNodeSeedPeer::new("peer-a", None).expect("seed peer")]
        );
    }
}
