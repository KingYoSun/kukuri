mod identity;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use image::imageops::FilterType;
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat};
use kukuri_app_api::{
    AppService, AuthorSocialView, BlobMediaPayload, BookmarkedCustomReactionView,
    BookmarkedPostView, ChannelAccessTokenExport, ChannelAccessTokenPreview,
    CreateCustomReactionAssetInput, CreateGameRoomInput, CreateLiveSessionInput,
    CustomReactionAssetView, DirectMessageConversationView, DirectMessageStatusView,
    DirectMessageTimelineView, GameRoomView, GameScoreView, JoinedPrivateChannelView,
    LiveSessionView, PendingAttachment, PrivateChannelCapability, ProfileInput, ReactionStateView,
    RecentReactionView, SyncStatus, TimelineView, UpdateGameRoomInput,
};
use kukuri_blob_service::{BlobService, BlobStatus, IrohBlobService, StoredBlob};
use kukuri_cn_core::{
    AuthChallengeResponse, AuthVerifyResponse, BootstrapHeartbeatResponse,
    CommunityNodeConsentStatus, CommunityNodeResolvedUrls, CommunityNodeSeedPeer,
    build_auth_envelope_json, normalize_http_url,
};
use kukuri_core::{
    AssetRole, BlobHash, ChannelAudienceKind, ChannelRef, CreatePrivateChannelInput,
    CustomReactionAssetSnapshotV1, FriendOnlyGrantPreview, FriendPlusSharePreview,
    GameRoomStatus, GossipHint, KukuriKeys, PrivateChannelInvitePreview, Profile, ReactionKeyV1,
    ReplicaId, TimelineScope, TopicId,
};
use kukuri_docs_sync::{
    DocEventStream, DocOp, DocQuery, DocRecord, DocsSync, IrohDocsNode, IrohDocsSync,
};
use kukuri_store::{SqliteStore, TimelineCursor};
use kukuri_transport::{
    ConnectMode, DhtDiscoveryOptions, DiscoveryMode, DiscoverySnapshot, HintStream, HintTransport,
    IrohGossipTransport, PeerSnapshot, SeedPeer, Transport, TransportNetworkConfig,
    TransportRelayConfig, parse_seed_peer,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

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
const PRIVATE_CHANNEL_CAPABILITIES_PURPOSE: &str = "private-channel-capabilities";
const PRIVATE_CHANNEL_CAPABILITIES_KEY: &str = "registry";
const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS: i64 = 30;
const COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_RETRY_SECONDS: i64 = 10;
const COMMUNITY_NODE_BOOTSTRAP_METADATA_RETRY_SECONDS: i64 = 5;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub topic: String,
    pub content: String,
    pub reply_to: Option<String>,
    #[serde(default)]
    pub channel_ref: ChannelRef,
    #[serde(default)]
    pub attachments: Vec<CreateAttachmentRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRepostRequest {
    pub topic: String,
    pub source_topic: String,
    pub source_object_id: String,
    #[serde(default)]
    pub commentary: Option<String>,
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReactionKeyRequest {
    Emoji {
        emoji: String,
    },
    CustomAsset {
        asset_id: String,
        owner_pubkey: String,
        blob_hash: String,
        search_key: String,
        mime: String,
        bytes: u64,
        width: u32,
        height: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleReactionRequest {
    pub target_topic_id: String,
    pub target_object_id: String,
    pub reaction_key: ReactionKeyRequest,
    pub channel_ref: Option<ChannelRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomReactionCropRect {
    pub x: u32,
    pub y: u32,
    pub size: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCustomReactionAssetRequest {
    pub upload: CreateAttachmentRequest,
    pub crop_rect: CustomReactionCropRect,
    pub search_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkCustomReactionRequest {
    pub asset_id: String,
    pub owner_pubkey: String,
    pub blob_hash: String,
    pub search_key: String,
    pub mime: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveBookmarkedCustomReactionRequest {
    pub asset_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkPostRequest {
    pub topic: String,
    pub object_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveBookmarkedPostRequest {
    pub object_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRecentReactionsRequest {
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTimelineRequest {
    pub topic: String,
    #[serde(default)]
    pub scope: TimelineScope,
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
pub struct ListProfileTimelineRequest {
    pub pubkey: String,
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
pub struct AuthorRequest {
    pub pubkey: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageRequest {
    pub pubkey: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDirectMessageMessagesRequest {
    pub pubkey: String,
    pub cursor: Option<TimelineCursor>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendDirectMessageRequest {
    pub pubkey: String,
    pub text: Option<String>,
    pub reply_to_message_id: Option<String>,
    #[serde(default)]
    pub attachments: Vec<CreateAttachmentRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteDirectMessageMessageRequest {
    pub pubkey: String,
    pub message_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetMyProfileRequest {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub picture_upload: Option<CreateAttachmentRequest>,
    #[serde(default)]
    pub clear_picture: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListLiveSessionsRequest {
    pub topic: String,
    #[serde(default)]
    pub scope: TimelineScope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateLiveSessionRequest {
    pub topic: String,
    #[serde(default)]
    pub channel_ref: ChannelRef,
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
    #[serde(default)]
    pub scope: TimelineScope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGameRoomRequest {
    pub topic: String,
    #[serde(default)]
    pub channel_ref: ChannelRef,
    pub title: String,
    pub description: String,
    pub participants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePrivateChannelRequest {
    pub topic: String,
    pub label: String,
    #[serde(default)]
    pub audience_kind: ChannelAudienceKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPrivateChannelInviteRequest {
    pub topic: String,
    pub channel_id: String,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPrivateChannelInviteRequest {
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportChannelAccessTokenRequest {
    pub topic: String,
    pub channel_id: String,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportChannelAccessTokenRequest {
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFriendOnlyGrantRequest {
    pub topic: String,
    pub channel_id: String,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFriendOnlyGrantRequest {
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFriendPlusShareRequest {
    pub topic: String,
    pub channel_id: String,
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFriendPlusShareRequest {
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezePrivateChannelRequest {
    pub topic: String,
    pub channel_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotatePrivateChannelRequest {
    pub topic: String,
    pub channel_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListJoinedPrivateChannelsRequest {
    pub topic: String,
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
struct BootstrapNodesResponse {
    nodes: Vec<kukuri_cn_core::CommunityNodeBootstrapNode>,
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

struct BoundIrohStack {
    node: Arc<IrohDocsNode>,
    transport: Arc<IrohGossipTransport>,
    docs_sync: Arc<IrohDocsSync>,
    blob_service: Arc<IrohBlobService>,
}

#[derive(Clone)]
struct ReloadableTransport {
    inner: Arc<RwLock<Arc<IrohGossipTransport>>>,
}

#[derive(Clone)]
struct ReloadableDocsSync {
    inner: Arc<RwLock<Arc<IrohDocsSync>>>,
}

#[derive(Clone)]
struct ReloadableBlobService {
    inner: Arc<RwLock<Arc<IrohBlobService>>>,
}

pub struct DesktopRuntime {
    app_service: AppService,
    author_keys: Arc<KukuriKeys>,
    db_path: PathBuf,
    identity_mode: IdentityStorageMode,
    store: Arc<SqliteStore>,
    iroh_stack: SharedIrohStack,
    discovery_config: Arc<Mutex<DiscoveryConfig>>,
    community_node_config: Arc<Mutex<CommunityNodeConfig>>,
    community_node_heartbeat_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    community_node_metadata_refresh_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    active_connectivity_urls: Arc<Mutex<Vec<String>>>,
}

struct SharedIrohStack {
    current: Mutex<Option<BoundIrohStack>>,
    transport: Arc<ReloadableTransport>,
    docs_sync: Arc<ReloadableDocsSync>,
    blob_service: Arc<ReloadableBlobService>,
    root: PathBuf,
    network_config: TransportNetworkConfig,
    dht_options: DhtDiscoveryOptions,
}

impl ReloadableTransport {
    fn new(inner: Arc<IrohGossipTransport>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    async fn current(&self) -> Arc<IrohGossipTransport> {
        self.inner.read().await.clone()
    }

    async fn replace(&self, inner: Arc<IrohGossipTransport>) {
        *self.inner.write().await = inner;
    }
}

#[async_trait]
impl Transport for ReloadableTransport {
    async fn peers(&self) -> Result<PeerSnapshot> {
        self.current().await.peers().await
    }

    async fn export_ticket(&self) -> Result<Option<String>> {
        self.current().await.export_ticket().await
    }

    async fn import_ticket(&self, ticket: &str) -> Result<()> {
        self.current().await.import_ticket(ticket).await
    }

    async fn configure_discovery(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        self.current()
            .await
            .configure_discovery(
                mode,
                env_locked,
                configured_seed_peers,
                bootstrap_seed_peers,
            )
            .await
    }

    async fn discovery(&self) -> Result<DiscoverySnapshot> {
        self.current().await.discovery().await
    }
}

#[async_trait]
impl HintTransport for ReloadableTransport {
    async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
        self.current().await.subscribe_hints(topic).await
    }

    async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
        self.current().await.unsubscribe_hints(topic).await
    }

    async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
        self.current().await.publish_hint(topic, hint).await
    }
}

impl ReloadableDocsSync {
    fn new(inner: Arc<IrohDocsSync>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    async fn current(&self) -> Arc<IrohDocsSync> {
        self.inner.read().await.clone()
    }

    async fn replace(&self, inner: Arc<IrohDocsSync>) {
        *self.inner.write().await = inner;
    }
}

#[async_trait]
impl DocsSync for ReloadableDocsSync {
    async fn open_replica(&self, replica_id: &ReplicaId) -> Result<()> {
        self.current().await.open_replica(replica_id).await
    }

    async fn register_private_replica_secret(
        &self,
        replica_id: &ReplicaId,
        namespace_secret_hex: &str,
    ) -> Result<()> {
        self.current()
            .await
            .register_private_replica_secret(replica_id, namespace_secret_hex)
            .await
    }

    async fn remove_private_replica_secret(&self, replica_id: &ReplicaId) -> Result<()> {
        self.current()
            .await
            .remove_private_replica_secret(replica_id)
            .await
    }

    async fn apply_doc_op(&self, replica_id: &ReplicaId, op: DocOp) -> Result<()> {
        self.current().await.apply_doc_op(replica_id, op).await
    }

    async fn query_replica(
        &self,
        replica_id: &ReplicaId,
        query: DocQuery,
    ) -> Result<Vec<DocRecord>> {
        self.current().await.query_replica(replica_id, query).await
    }

    async fn subscribe_replica(&self, replica_id: &ReplicaId) -> Result<DocEventStream> {
        self.current().await.subscribe_replica(replica_id).await
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.current().await.import_peer_ticket(ticket).await
    }

    async fn set_seed_peers(&self, peers: Vec<SeedPeer>) -> Result<()> {
        self.current().await.set_seed_peers(peers).await
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        self.current().await.assist_peer_ids().await
    }
}

impl ReloadableBlobService {
    fn new(inner: Arc<IrohBlobService>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    async fn current(&self) -> Arc<IrohBlobService> {
        self.inner.read().await.clone()
    }

    async fn replace(&self, inner: Arc<IrohBlobService>) {
        *self.inner.write().await = inner;
    }
}

#[async_trait]
impl BlobService for ReloadableBlobService {
    async fn put_blob(&self, data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
        self.current().await.put_blob(data, mime).await
    }

    async fn fetch_blob(&self, hash: &BlobHash) -> Result<Option<Vec<u8>>> {
        self.current().await.fetch_blob(hash).await
    }

    async fn pin_blob(&self, hash: &BlobHash) -> Result<()> {
        self.current().await.pin_blob(hash).await
    }

    async fn blob_status(&self, hash: &BlobHash) -> Result<BlobStatus> {
        self.current().await.blob_status(hash).await
    }

    async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.current().await.import_peer_ticket(ticket).await
    }

    async fn learn_peer(&self, endpoint_id: &str) -> Result<()> {
        self.current().await.learn_peer(endpoint_id).await
    }

    async fn set_seed_peers(&self, peers: Vec<SeedPeer>) -> Result<()> {
        self.current().await.set_seed_peers(peers).await
    }

    async fn assist_peer_ids(&self) -> Result<Vec<String>> {
        self.current().await.assist_peer_ids().await
    }
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
        let community_node_seed_peers =
            community_node_seed_peers(&community_node_config).collect::<Vec<_>>();
        let docs_root = db_path.with_extension("iroh-data");
        let store = Arc::new(SqliteStore::connect_file(&db_path).await?);
        let iroh_stack = SharedIrohStack::new(
            &docs_root,
            network_config.clone(),
            &discovery_config,
            &community_node_seed_peers,
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
        for capability in load_private_channel_capabilities(&db_path, identity_mode)? {
            app_service
                .restore_private_channel_capability(capability)
                .await?;
        }
        app_service.warm_social_graph().await?;
        app_service.resume_direct_message_state().await?;

        Ok(Self {
            app_service,
            author_keys,
            db_path,
            identity_mode,
            store,
            iroh_stack,
            discovery_config: Arc::new(Mutex::new(discovery_config)),
            community_node_config: Arc::new(Mutex::new(community_node_config)),
            community_node_heartbeat_deadlines: Arc::new(Mutex::new(HashMap::new())),
            community_node_metadata_refresh_deadlines: Arc::new(Mutex::new(HashMap::new())),
            active_connectivity_urls: Arc::new(Mutex::new(relay_config.iroh_relay_urls.clone())),
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

    async fn persist_private_channel_capabilities_from_app(&self) -> Result<()> {
        persist_private_channel_capabilities(
            &self.db_path,
            self.identity_mode,
            &self.app_service.list_private_channel_capabilities().await?,
        )
    }

    pub async fn create_post(&self, request: CreatePostRequest) -> Result<String> {
        let attachments = request
            .attachments
            .into_iter()
            .map(pending_attachment_from_request)
            .collect::<Result<Vec<_>>>()?;
        self.app_service
            .create_post_with_attachments_in_channel(
                request.topic.as_str(),
                request.channel_ref,
                request.content.as_str(),
                request.reply_to.as_deref(),
                attachments,
            )
            .await
    }

    pub async fn create_repost(&self, request: CreateRepostRequest) -> Result<String> {
        self.app_service
            .create_repost(
                request.topic.as_str(),
                request.source_topic.as_str(),
                request.source_object_id.as_str(),
                request.commentary.as_deref(),
            )
            .await
    }

    pub async fn toggle_reaction(
        &self,
        request: ToggleReactionRequest,
    ) -> Result<ReactionStateView> {
        self.app_service
            .toggle_reaction(
                request.target_topic_id.as_str(),
                request.target_object_id.as_str(),
                reaction_key_from_request(request.reaction_key)?,
                request.channel_ref,
            )
            .await
    }

    pub async fn list_my_custom_reaction_assets(&self) -> Result<Vec<CustomReactionAssetView>> {
        self.app_service.list_my_custom_reaction_assets().await
    }

    pub async fn list_recent_reactions(
        &self,
        request: ListRecentReactionsRequest,
    ) -> Result<Vec<RecentReactionView>> {
        self.app_service
            .list_recent_reactions(request.limit.unwrap_or(8))
            .await
    }

    pub async fn create_custom_reaction_asset(
        &self,
        request: CreateCustomReactionAssetRequest,
    ) -> Result<CustomReactionAssetView> {
        let upload = request.upload;
        let raw = BASE64_STANDARD
            .decode(upload.data_base64.as_bytes())
            .context("failed to decode custom reaction upload")?;
        let normalized =
            normalize_custom_reaction_upload(raw, upload.mime.as_str(), &request.crop_rect)?;
        self.app_service
            .create_custom_reaction_asset(CreateCustomReactionAssetInput {
                search_key: request.search_key,
                mime: normalized.mime,
                bytes: normalized.bytes,
                width: 128,
                height: 128,
            })
            .await
    }

    pub async fn list_bookmarked_custom_reactions(
        &self,
    ) -> Result<Vec<BookmarkedCustomReactionView>> {
        self.app_service.list_bookmarked_custom_reactions().await
    }

    pub async fn bookmark_custom_reaction(
        &self,
        request: BookmarkCustomReactionRequest,
    ) -> Result<BookmarkedCustomReactionView> {
        self.app_service
            .bookmark_custom_reaction(CustomReactionAssetSnapshotV1 {
                asset_id: request.asset_id,
                owner_pubkey: request.owner_pubkey.into(),
                blob_hash: BlobHash::new(request.blob_hash),
                search_key: request.search_key,
                mime: request.mime,
                bytes: request.bytes,
                width: request.width,
                height: request.height,
            })
            .await
    }

    pub async fn remove_bookmarked_custom_reaction(
        &self,
        request: RemoveBookmarkedCustomReactionRequest,
    ) -> Result<()> {
        self.app_service
            .remove_bookmarked_custom_reaction(request.asset_id.as_str())
            .await
    }

    pub async fn list_bookmarked_posts(&self) -> Result<Vec<BookmarkedPostView>> {
        self.app_service.list_bookmarked_posts().await
    }

    pub async fn bookmark_post(&self, request: BookmarkPostRequest) -> Result<BookmarkedPostView> {
        self.app_service
            .bookmark_post(request.topic.as_str(), request.object_id.as_str())
            .await
    }

    pub async fn remove_bookmarked_post(&self, request: RemoveBookmarkedPostRequest) -> Result<()> {
        self.app_service
            .remove_bookmarked_post(request.object_id.as_str())
            .await
    }

    pub async fn list_timeline(&self, request: ListTimelineRequest) -> Result<TimelineView> {
        self.app_service
            .list_timeline_scoped(
                request.topic.as_str(),
                request.scope,
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

    pub async fn list_profile_timeline(
        &self,
        request: ListProfileTimelineRequest,
    ) -> Result<TimelineView> {
        self.app_service
            .list_profile_timeline(
                request.pubkey.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn get_my_profile(&self) -> Result<Profile> {
        self.app_service.get_my_profile().await
    }

    pub async fn set_my_profile(&self, request: SetMyProfileRequest) -> Result<Profile> {
        self.app_service
            .set_my_profile(ProfileInput {
                name: request.name,
                display_name: request.display_name,
                about: request.about,
                picture: request.picture,
                picture_upload: request
                    .picture_upload
                    .map(pending_attachment_from_request)
                    .transpose()?,
                clear_picture: request.clear_picture,
            })
            .await
    }

    pub async fn follow_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .follow_author(request.pubkey.as_str())
            .await
    }

    pub async fn unfollow_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .unfollow_author(request.pubkey.as_str())
            .await
    }

    pub async fn get_author_social_view(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .get_author_social_view(request.pubkey.as_str())
            .await
    }

    pub async fn open_direct_message(
        &self,
        request: DirectMessageRequest,
    ) -> Result<DirectMessageConversationView> {
        self.app_service
            .open_direct_message(request.pubkey.as_str())
            .await
    }

    pub async fn list_direct_messages(&self) -> Result<Vec<DirectMessageConversationView>> {
        self.app_service.list_direct_messages().await
    }

    pub async fn list_direct_message_messages(
        &self,
        request: ListDirectMessageMessagesRequest,
    ) -> Result<DirectMessageTimelineView> {
        self.app_service
            .list_direct_message_messages(
                request.pubkey.as_str(),
                request.cursor,
                request.limit.unwrap_or(50),
            )
            .await
    }

    pub async fn send_direct_message(&self, request: SendDirectMessageRequest) -> Result<String> {
        let attachments = request
            .attachments
            .into_iter()
            .map(pending_attachment_from_request)
            .collect::<Result<Vec<_>>>()?;
        self.app_service
            .send_direct_message(
                request.pubkey.as_str(),
                request.text.as_deref(),
                request.reply_to_message_id.as_deref(),
                attachments,
            )
            .await
    }

    pub async fn delete_direct_message_message(
        &self,
        request: DeleteDirectMessageMessageRequest,
    ) -> Result<()> {
        self.app_service
            .delete_direct_message_message(request.pubkey.as_str(), request.message_id.as_str())
            .await
    }

    pub async fn clear_direct_message(&self, request: DirectMessageRequest) -> Result<()> {
        self.app_service
            .clear_direct_message(request.pubkey.as_str())
            .await
    }

    pub async fn get_direct_message_status(
        &self,
        request: DirectMessageRequest,
    ) -> Result<DirectMessageStatusView> {
        self.app_service
            .get_direct_message_status(request.pubkey.as_str())
            .await
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        self.app_service.get_sync_status().await
    }

    pub async fn has_topic_timeline_doc_index_entry(
        &self,
        topic: &str,
        object_id: &str,
    ) -> Result<bool> {
        let replica = kukuri_docs_sync::topic_replica_id(topic);
        let current = self.iroh_stack.current.lock().await;
        let docs_sync = current
            .as_ref()
            .context("desktop runtime stack is not initialized")?
            .docs_sync
            .clone();
        drop(current);
        let rows = docs_sync
            .query_replica(&replica, DocQuery::Prefix("indexes/timeline/".into()))
            .await?;
        Ok(rows.iter().any(|row| row.key.ends_with(object_id)))
    }

    pub async fn get_discovery_config(&self) -> Result<DiscoveryConfig> {
        Ok(self.discovery_config.lock().await.clone())
    }

    pub async fn list_live_sessions(
        &self,
        request: ListLiveSessionsRequest,
    ) -> Result<Vec<LiveSessionView>> {
        self.app_service
            .list_live_sessions_scoped(request.topic.as_str(), request.scope)
            .await
    }

    pub async fn create_live_session(&self, request: CreateLiveSessionRequest) -> Result<String> {
        self.app_service
            .create_live_session_in_channel(
                request.topic.as_str(),
                request.channel_ref,
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
            .list_game_rooms_scoped(request.topic.as_str(), request.scope)
            .await
    }

    pub async fn create_game_room(&self, request: CreateGameRoomRequest) -> Result<String> {
        self.app_service
            .create_game_room_in_channel(
                request.topic.as_str(),
                request.channel_ref,
                CreateGameRoomInput {
                    title: request.title,
                    description: request.description,
                    participants: request.participants,
                },
            )
            .await
    }

    pub async fn create_private_channel(
        &self,
        request: CreatePrivateChannelRequest,
    ) -> Result<JoinedPrivateChannelView> {
        let channel = self
            .app_service
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(request.topic),
                label: request.label,
                audience_kind: request.audience_kind,
            })
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(channel)
    }

    pub async fn export_private_channel_invite(
        &self,
        request: ExportPrivateChannelInviteRequest,
    ) -> Result<String> {
        self.app_service
            .export_private_channel_invite(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_private_channel_invite(
        &self,
        request: ImportPrivateChannelInviteRequest,
    ) -> Result<PrivateChannelInvitePreview> {
        let preview = self
            .app_service
            .import_private_channel_invite(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn export_channel_access_token(
        &self,
        request: ExportChannelAccessTokenRequest,
    ) -> Result<ChannelAccessTokenExport> {
        self.app_service
            .export_channel_access_token(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_channel_access_token(
        &self,
        request: ImportChannelAccessTokenRequest,
    ) -> Result<ChannelAccessTokenPreview> {
        let preview = self
            .app_service
            .import_channel_access_token(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn export_friend_only_grant(
        &self,
        request: ExportFriendOnlyGrantRequest,
    ) -> Result<String> {
        self.app_service
            .export_friend_only_grant(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_friend_only_grant(
        &self,
        request: ImportFriendOnlyGrantRequest,
    ) -> Result<FriendOnlyGrantPreview> {
        let preview = self
            .app_service
            .import_friend_only_grant(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn export_friend_plus_share(
        &self,
        request: ExportFriendPlusShareRequest,
    ) -> Result<String> {
        self.app_service
            .export_friend_plus_share(
                request.topic.as_str(),
                request.channel_id.as_str(),
                request.expires_at,
            )
            .await
    }

    pub async fn import_friend_plus_share(
        &self,
        request: ImportFriendPlusShareRequest,
    ) -> Result<FriendPlusSharePreview> {
        let preview = self
            .app_service
            .import_friend_plus_share(request.token.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(preview)
    }

    pub async fn freeze_private_channel(
        &self,
        request: FreezePrivateChannelRequest,
    ) -> Result<JoinedPrivateChannelView> {
        let view = self
            .app_service
            .freeze_private_channel(request.topic.as_str(), request.channel_id.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(view)
    }

    pub async fn rotate_private_channel(
        &self,
        request: RotatePrivateChannelRequest,
    ) -> Result<JoinedPrivateChannelView> {
        let view = self
            .app_service
            .rotate_private_channel(request.topic.as_str(), request.channel_id.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(view)
    }

    pub async fn list_joined_private_channels(
        &self,
        request: ListJoinedPrivateChannelsRequest,
    ) -> Result<Vec<JoinedPrivateChannelView>> {
        let items = self
            .app_service
            .list_joined_private_channels(request.topic.as_str())
            .await?;
        self.persist_private_channel_capabilities_from_app().await?;
        Ok(items)
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
        if request.hash.trim().is_empty() {
            tracing::warn!(mime = %request.mime, "blob media payload request skipped because hash was blank");
            return Ok(None);
        }
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
            let base_url = node.base_url.clone();
            let heartbeat_error = self
                .refresh_community_node_registration_if_due(base_url.as_str())
                .await
                .err()
                .map(|error| error.to_string());
            let current_node = self
                .community_node_config
                .lock()
                .await
                .nodes
                .iter()
                .find(|candidate| candidate.base_url == base_url)
                .cloned()
                .unwrap_or(node);
            statuses.push(
                self.community_node_status(current_node, None, heartbeat_error)
                    .await?,
            );
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
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .clear();
        self.apply_runtime_connectivity_assist().await?;
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
        self.community_node_heartbeat_deadlines.lock().await.clear();
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .clear();
        self.apply_runtime_connectivity_assist().await?;
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
            self.identity_mode,
            COMMUNITY_NODE_TOKEN_PURPOSE,
            base_url.as_str(),
        )?;
        self.community_node_heartbeat_deadlines
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .remove(base_url.as_str());
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
        let token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
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
        let token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
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
        let token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        let node = self
            .sync_community_node_bootstrap_metadata(base_url.as_str(), token.access_token.as_str())
            .await?;
        self.community_node_status(node, None, None).await
    }

    pub async fn shutdown(&self) {
        self.app_service.shutdown().await;
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            self.iroh_stack.shutdown(),
        )
        .await;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), self.store.close()).await;
    }
}

impl DesktopRuntime {
    async fn sync_community_node_bootstrap_metadata(
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

    async fn community_node_bootstrap_metadata_retry_due(&self, base_url: &str, now: i64) -> bool {
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

    async fn record_community_node_bootstrap_metadata_refresh(
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

    async fn local_community_node_seed_peer(
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

    async fn refresh_community_node_registration_if_due(&self, base_url: &str) -> Result<()> {
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

    async fn apply_runtime_connectivity_assist(&self) -> Result<()> {
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

    async fn apply_effective_seed_peers(&self) -> Result<()> {
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
        Some("profile_avatar") => AssetRole::ProfileAvatar,
        Some("attachment") => AssetRole::Attachment,
        _ => AssetRole::ImageOriginal,
    };
    Ok(PendingAttachment {
        mime: request.mime,
        bytes,
        role,
    })
}

struct NormalizedCustomReactionUpload {
    mime: String,
    bytes: Vec<u8>,
}

fn reaction_key_from_request(request: ReactionKeyRequest) -> Result<ReactionKeyV1> {
    Ok(match request {
        ReactionKeyRequest::Emoji { emoji } => ReactionKeyV1::Emoji { emoji },
        ReactionKeyRequest::CustomAsset {
            asset_id,
            owner_pubkey,
            blob_hash,
            search_key,
            mime,
            bytes,
            width,
            height,
        } => ReactionKeyV1::CustomAsset {
            asset_id: asset_id.clone(),
            snapshot: CustomReactionAssetSnapshotV1 {
                asset_id,
                owner_pubkey: owner_pubkey.into(),
                blob_hash: BlobHash::new(blob_hash),
                search_key,
                mime,
                bytes,
                width,
                height,
            },
        },
    })
}

fn normalize_custom_reaction_upload(
    bytes: Vec<u8>,
    mime: &str,
    crop_rect: &CustomReactionCropRect,
) -> Result<NormalizedCustomReactionUpload> {
    if crop_rect.size == 0 {
        bail!("custom reaction crop size must be greater than zero");
    }
    if mime.trim() == "image/gif" {
        return normalize_custom_reaction_gif(bytes, crop_rect);
    }
    normalize_custom_reaction_static(bytes, crop_rect)
}

fn normalize_custom_reaction_static(
    bytes: Vec<u8>,
    crop_rect: &CustomReactionCropRect,
) -> Result<NormalizedCustomReactionUpload> {
    let image = image::load_from_memory(bytes.as_slice()).context("failed to decode image")?;
    validate_crop_rect(image.width(), image.height(), crop_rect)?;
    let cropped = crop_static_image(image, crop_rect);
    let mut out = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut out, ImageFormat::Png)
        .context("failed to encode normalized PNG")?;
    Ok(NormalizedCustomReactionUpload {
        mime: "image/png".into(),
        bytes: out.into_inner(),
    })
}

fn normalize_custom_reaction_gif(
    bytes: Vec<u8>,
    crop_rect: &CustomReactionCropRect,
) -> Result<NormalizedCustomReactionUpload> {
    let decoder = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))
        .context("failed to decode GIF")?;
    let (width, height) = decoder.dimensions();
    validate_crop_rect(width, height, crop_rect)?;
    let frames = decoder
        .into_frames()
        .collect_frames()
        .context("failed to collect GIF frames")?;
    let normalized_frames = frames.into_iter().map(|frame| {
        let delay = frame.delay();
        let buffer = frame.into_buffer();
        let image = DynamicImage::ImageRgba8(buffer);
        let resized = crop_static_image(image, crop_rect).into_rgba8();
        image::Frame::from_parts(resized, 0, 0, delay)
    });
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut out);
        encoder
            .encode_frames(normalized_frames)
            .context("failed to encode normalized GIF")?;
    }
    Ok(NormalizedCustomReactionUpload {
        mime: "image/gif".into(),
        bytes: out.into_inner(),
    })
}

fn crop_static_image(image: DynamicImage, crop_rect: &CustomReactionCropRect) -> DynamicImage {
    image
        .crop_imm(crop_rect.x, crop_rect.y, crop_rect.size, crop_rect.size)
        .resize_exact(128, 128, FilterType::Lanczos3)
}

fn validate_crop_rect(width: u32, height: u32, crop_rect: &CustomReactionCropRect) -> Result<()> {
    if crop_rect.x.saturating_add(crop_rect.size) > width
        || crop_rect.y.saturating_add(crop_rect.size) > height
    {
        bail!("custom reaction crop rectangle exceeds the source image bounds");
    }
    Ok(())
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
    bootstrap_seed_peers: &[SeedPeer],
) -> Vec<SeedPeer> {
    normalize_seed_peers(
        discovery_config
            .seed_peers
            .iter()
            .cloned()
            .chain(bootstrap_seed_peers.iter().cloned())
            .collect(),
    )
}

fn effective_dht_options(
    dht_options: &DhtDiscoveryOptions,
    bootstrap_seed_peers: &[SeedPeer],
    relay_config: &TransportRelayConfig,
) -> DhtDiscoveryOptions {
    if relay_config.connect_mode() == ConnectMode::DirectOrRelay && !bootstrap_seed_peers.is_empty()
    {
        DhtDiscoveryOptions::disabled()
    } else {
        dht_options.clone()
    }
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

fn load_private_channel_capabilities(
    db_path: &Path,
    mode: IdentityStorageMode,
) -> Result<Vec<PrivateChannelCapability>> {
    let Some(raw) = load_optional_secret(
        db_path,
        mode,
        PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
        PRIVATE_CHANNEL_CAPABILITIES_KEY,
    )?
    else {
        return Ok(Vec::new());
    };
    serde_json::from_str(&raw).context("failed to decode private channel capabilities")
}

fn persist_private_channel_capabilities(
    db_path: &Path,
    mode: IdentityStorageMode,
    capabilities: &[PrivateChannelCapability],
) -> Result<()> {
    let encoded = serde_json::to_string(capabilities)
        .context("failed to encode private channel capabilities")?;
    persist_optional_secret(
        db_path,
        mode,
        PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
        PRIVATE_CHANNEL_CAPABILITIES_KEY,
        encoded.as_str(),
    )
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
        bootstrap_seed_peers: &[SeedPeer],
        dht_options: DhtDiscoveryOptions,
        relay_config: TransportRelayConfig,
    ) -> Result<Self> {
        let dht_options = effective_dht_options(&dht_options, bootstrap_seed_peers, &relay_config);
        let current = BoundIrohStack::new(
            root,
            network_config.clone(),
            discovery_config,
            bootstrap_seed_peers,
            dht_options.clone(),
            relay_config,
        )
        .await?;
        let transport = Arc::new(ReloadableTransport::new(current.transport.clone()));
        let docs_sync = Arc::new(ReloadableDocsSync::new(current.docs_sync.clone()));
        let blob_service = Arc::new(ReloadableBlobService::new(current.blob_service.clone()));
        Ok(Self {
            current: Mutex::new(Some(current)),
            transport,
            docs_sync,
            blob_service,
            root: root.to_path_buf(),
            network_config,
            dht_options,
        })
    }

    async fn rebuild(
        &self,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
        relay_config: TransportRelayConfig,
    ) -> Result<()> {
        let dht_options =
            effective_dht_options(&self.dht_options, bootstrap_seed_peers, &relay_config);
        let previous = self
            .current
            .lock()
            .await
            .take()
            .context("missing active iroh stack during rebuild")?;
        previous.shutdown().await;
        let next = BoundIrohStack::new(
            &self.root,
            self.network_config.clone(),
            discovery_config,
            bootstrap_seed_peers,
            dht_options,
            relay_config,
        )
        .await?;
        self.transport.replace(next.transport.clone()).await;
        self.docs_sync.replace(next.docs_sync.clone()).await;
        self.blob_service.replace(next.blob_service.clone()).await;
        *self.current.lock().await = Some(next);
        Ok(())
    }

    async fn apply_runtime_connectivity(
        &self,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
        relay_config: TransportRelayConfig,
    ) -> Result<()> {
        let relay_config = relay_config.normalized();
        let next_relay_urls = relay_config.parsed_relay_urls()?;
        let current_relay_urls = {
            let current = self.current.lock().await;
            current
                .as_ref()
                .context("missing active iroh stack while reading relay urls")?
                .node
                .relay_urls()
                .await
        };
        if current_relay_urls != next_relay_urls
            && discovery_config.mode != DiscoveryMode::StaticPeer
        {
            return self
                .rebuild(discovery_config, bootstrap_seed_peers, relay_config)
                .await;
        }
        let current = self.current.lock().await;
        let current = current
            .as_ref()
            .context("missing active iroh stack while applying runtime connectivity")?;
        current
            .node
            .apply_relay_config(relay_config.clone())
            .await?;
        current.transport.update_relay_config(relay_config).await?;
        current
            .transport
            .configure_discovery(
                discovery_config.mode.clone(),
                discovery_config.env_locked,
                discovery_config.seed_peers.clone(),
                bootstrap_seed_peers.to_vec(),
            )
            .await?;
        let effective_seed_peers = effective_seed_peers(discovery_config, bootstrap_seed_peers);
        current
            .docs_sync
            .set_seed_peers(effective_seed_peers.clone())
            .await?;
        current
            .blob_service
            .set_seed_peers(effective_seed_peers)
            .await?;
        Ok(())
    }

    async fn shutdown(&self) {
        if let Some(current) = self.current.lock().await.take() {
            let _ =
                tokio::time::timeout(std::time::Duration::from_secs(15), current.shutdown()).await;
        }
    }

    #[cfg(test)]
    async fn endpoint(&self) -> iroh::Endpoint {
        self.current
            .lock()
            .await
            .as_ref()
            .expect("missing active iroh stack")
            .node
            .endpoint()
            .clone()
    }
}

impl BoundIrohStack {
    async fn new(
        root: &Path,
        network_config: TransportNetworkConfig,
        discovery_config: &DiscoveryConfig,
        bootstrap_seed_peers: &[SeedPeer],
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
                bootstrap_seed_peers.to_vec(),
            )
            .await?;
        let effective_seed_peers = effective_seed_peers(discovery_config, bootstrap_seed_peers);
        docs_sync
            .set_seed_peers(effective_seed_peers.clone())
            .await?;
        blob_service.set_seed_peers(effective_seed_peers).await?;
        Ok(Self {
            node,
            transport,
            docs_sync,
            blob_service,
        })
    }

    async fn shutdown(&self) {
        self.transport.shutdown().await;
        self.docs_sync.shutdown().await;
        let _ = self.node.clone().shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::State,
        routing::{get, post},
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use image::{AnimationDecoder, Delay, Frame, GenericImageView, ImageFormat, Rgba, RgbaImage};
    use iroh::address_lookup::EndpointInfo;
    use kukuri_core::AuthorProfilePostDocV1;
    use kukuri_docs_sync::author_replica_id;
    use pkarr::errors::{ConcurrencyError, PublishError};
    use pkarr::{Client as PkarrClient, SignedPacket, Timestamp, mainline::Testnet};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;
    use tokio::net::TcpListener;
    use tokio::time::{Duration, sleep, timeout};

    fn social_graph_propagation_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(300)
        } else {
            Duration::from_secs(30)
        }
    }

    fn seeded_dht_runtime_ready_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(120)
        } else {
            Duration::from_secs(20)
        }
    }

    fn runtime_replication_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(180)
        } else {
            Duration::from_secs(30)
        }
    }

    fn runtime_shutdown_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(60)
        } else {
            Duration::from_secs(15)
        }
    }

    fn png_source_bytes() -> Vec<u8> {
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(320, 180, Rgba([0, 179, 164, 255])));
        let mut out = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut out, ImageFormat::Png)
            .expect("encode png");
        out.into_inner()
    }

    fn animated_gif_source_bytes() -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut out);
            let frames = vec![
                Frame::from_parts(
                    RgbaImage::from_pixel(4, 2, Rgba([255, 0, 0, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(40, 1),
                ),
                Frame::from_parts(
                    RgbaImage::from_pixel(4, 2, Rgba([0, 0, 255, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(40, 1),
                ),
            ];
            encoder.encode_frames(frames).expect("encode gif");
        }
        out.into_inner()
    }

    #[test]
    fn normalize_custom_reaction_static_resizes_png_to_square() {
        let normalized = normalize_custom_reaction_static(
            png_source_bytes(),
            &CustomReactionCropRect {
                x: 70,
                y: 0,
                size: 180,
            },
        )
        .expect("normalize png");
        let image = image::load_from_memory(normalized.bytes.as_slice()).expect("decode png");

        assert_eq!(normalized.mime, "image/png");
        assert_eq!(image.dimensions(), (128, 128));
    }

    #[test]
    fn animated_gif_custom_reaction_preserves_gif_mime_after_normalization() {
        let normalized = normalize_custom_reaction_gif(
            animated_gif_source_bytes(),
            &CustomReactionCropRect {
                x: 1,
                y: 0,
                size: 2,
            },
        )
        .expect("normalize gif");
        let decoder =
            image::codecs::gif::GifDecoder::new(std::io::Cursor::new(normalized.bytes.clone()))
                .expect("decode normalized gif");
        let dimensions = decoder.dimensions();
        let frame_count = decoder
            .into_frames()
            .collect_frames()
            .expect("collect normalized gif frames")
            .len();

        assert_eq!(normalized.mime, "image/gif");
        assert_eq!(dimensions, (128, 128));
        assert_eq!(frame_count, 2);
    }

    fn format_sync_snapshot(status: &SyncStatus, topic: &str) -> String {
        let topic_status = status
            .topic_diagnostics
            .iter()
            .find(|entry| entry.topic == topic)
            .map(|entry| {
                format!(
                    "topic_peers={}, connected_peers={:?}, assist_peer_ids={:?}, configured_peer_ids={:?}, status_detail={}",
                    entry.peer_count,
                    entry.connected_peers,
                    entry.assist_peer_ids,
                    entry.configured_peer_ids,
                    entry.status_detail
                )
            })
            .unwrap_or_else(|| "topic_status=missing".to_string());
        format!(
            "connected={}, peer_count={}, status_detail={}, last_error={:?}, discovery_connected_peers={:?}, {}",
            status.connected,
            status.peer_count,
            status.status_detail,
            status.last_error,
            status.discovery.connected_peer_ids,
            topic_status
        )
    }

    async fn wait_for_connected_topic_peer_count(
        runtime: &DesktopRuntime,
        topic: &str,
        expected: usize,
        timeout_label: &str,
    ) {
        match timeout(runtime_replication_timeout(), async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status = runtime.get_sync_status().await.expect("sync status");
                let ready = status.connected
                    && status.peer_count >= expected
                    && status.topic_diagnostics.iter().any(|topic_status| {
                        topic_status.topic == topic
                            && topic_status.joined
                            && (topic_status.connected_peers.len() >= expected.min(1)
                                || topic_status.assist_peer_ids.len() >= expected.min(1))
                            && topic_status.peer_count >= expected
                    });
                if ready {
                    stable_ready_polls += 1;
                    if stable_ready_polls >= 3 {
                        return;
                    }
                } else {
                    stable_ready_polls = 0;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let status = runtime.get_sync_status().await.expect("sync status");
                panic!("{timeout_label}: {}", format_sync_snapshot(&status, topic));
            }
        }
    }

    async fn wait_for_connected_topic_peer_count_result(
        runtime: &DesktopRuntime,
        topic: &str,
        expected: usize,
        step_timeout: Duration,
    ) -> Result<()> {
        match timeout(step_timeout, async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status = runtime.get_sync_status().await.context("sync status")?;
                let ready = status.connected
                    && status.peer_count >= expected
                    && status.topic_diagnostics.iter().any(|topic_status| {
                        topic_status.topic == topic
                            && topic_status.joined
                            && (topic_status.connected_peers.len() >= expected.min(1)
                                || topic_status.assist_peer_ids.len() >= expected.min(1))
                            && topic_status.peer_count >= expected
                    });
                if ready {
                    stable_ready_polls += 1;
                    if stable_ready_polls >= 3 {
                        return Ok::<(), anyhow::Error>(());
                    }
                } else {
                    stable_ready_polls = 0;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let status = runtime
                    .get_sync_status()
                    .await
                    .ok()
                    .map(|value| format_sync_snapshot(&value, topic))
                    .unwrap_or_else(|| "failed to read sync status".to_string());
                bail!("topic readiness timeout; {status}");
            }
        }
    }

    fn topic_has_direct_peer(status: &SyncStatus, topic: &str, expected: usize) -> bool {
        status.connected
            && status.peer_count >= expected
            && status.topic_diagnostics.iter().any(|topic_status| {
                topic_status.topic == topic
                    && topic_status.joined
                    && topic_status.connected_peers.len() >= expected.min(1)
                    && topic_status.peer_count >= expected
            })
    }

    fn should_swap_shared_identity_public_replication_direction(
        publisher_status: &SyncStatus,
        subscriber_status: &SyncStatus,
        topic: &str,
        expected: usize,
    ) -> bool {
        !topic_has_direct_peer(publisher_status, topic, expected)
            && topic_has_direct_peer(subscriber_status, topic, expected)
    }

    async fn wait_for_direct_topic_peer_count_result(
        runtime: &DesktopRuntime,
        topic: &str,
        expected: usize,
        step_timeout: Duration,
    ) -> Result<()> {
        match timeout(step_timeout, async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status = runtime.get_sync_status().await.context("sync status")?;
                let ready = topic_has_direct_peer(&status, topic, expected);
                if ready {
                    stable_ready_polls += 1;
                    if stable_ready_polls >= 3 {
                        return Ok::<(), anyhow::Error>(());
                    }
                } else {
                    stable_ready_polls = 0;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let status = runtime
                    .get_sync_status()
                    .await
                    .ok()
                    .map(|value| format_sync_snapshot(&value, topic))
                    .unwrap_or_else(|| "failed to read sync status".to_string());
                bail!("direct topic readiness timeout; {status}");
            }
        }
    }

    async fn wait_for_connected_peer_count(
        runtime: &DesktopRuntime,
        expected: usize,
        timeout_label: &str,
    ) {
        match timeout(social_graph_propagation_timeout(), async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status = runtime.get_sync_status().await.expect("sync status");
                let ready = status.connected && status.peer_count >= expected;
                if ready {
                    stable_ready_polls += 1;
                    if stable_ready_polls >= 3 {
                        return;
                    }
                } else {
                    stable_ready_polls = 0;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let status = runtime.get_sync_status().await.expect("sync status");
                panic!("{timeout_label}: {}", format_sync_snapshot(&status, ""));
            }
        }
    }

    async fn wait_for_mutual_author_view(
        runtime: &DesktopRuntime,
        author_pubkey: &str,
        topic: &str,
    ) {
        match timeout(social_graph_propagation_timeout(), async {
            loop {
                let view = runtime
                    .get_author_social_view(AuthorRequest {
                        pubkey: author_pubkey.to_string(),
                    })
                    .await
                    .expect("author social view");
                if view.mutual {
                    return;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let social_view = runtime
                    .get_author_social_view(AuthorRequest {
                        pubkey: author_pubkey.to_string(),
                    })
                    .await
                    .ok()
                    .map(|value| {
                        format!(
                            "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                            value.following,
                            value.followed_by,
                            value.mutual,
                            value.friend_of_friend,
                            value.friend_of_friend_via_pubkeys
                        )
                    })
                    .unwrap_or_else(|| "social_view=unavailable".to_string());
                let status = runtime.get_sync_status().await.expect("sync status");
                panic!(
                    "mutual author view timeout for {author_pubkey}; {social_view}; {}",
                    format_sync_snapshot(&status, topic)
                );
            }
        }
    }

    async fn warm_author_social_view(
        runtime: &DesktopRuntime,
        author_pubkey: &str,
        timeout_label: &str,
    ) {
        match timeout(social_graph_propagation_timeout(), async {
            loop {
                if runtime
                    .get_author_social_view(AuthorRequest {
                        pubkey: author_pubkey.to_string(),
                    })
                    .await
                    .is_ok()
                {
                    return;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let status = runtime.get_sync_status().await.expect("sync status");
                panic!("{timeout_label}: {}", format_sync_snapshot(&status, ""));
            }
        }
    }

    async fn wait_for_topic_doc_index_entry_result(
        runtime: &DesktopRuntime,
        topic: &str,
        object_id: &str,
        step_timeout: Duration,
    ) -> Result<()> {
        match timeout(step_timeout, async {
            loop {
                if runtime
                    .has_topic_timeline_doc_index_entry(topic, object_id)
                    .await
                    .context("failed to query topic docs index")?
                {
                    return Ok::<(), anyhow::Error>(());
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let status = runtime
                    .get_sync_status()
                    .await
                    .ok()
                    .map(|value| format_sync_snapshot(&value, topic))
                    .unwrap_or_else(|| "failed to read sync status".to_string());
                bail!("topic docs index timeout; {status}");
            }
        }
    }

    async fn wait_for_timeline_post(
        runtime: &DesktopRuntime,
        topic: &str,
        scope: &TimelineScope,
        object_id: &str,
        timeout_label: &str,
    ) {
        match timeout(runtime_replication_timeout(), async {
            loop {
                let timeline = runtime
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("timeline");
                if timeline
                    .items
                    .iter()
                    .any(|post| post.object_id == object_id)
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let status = runtime.get_sync_status().await.expect("sync status");
                let private_items = runtime
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .ok()
                    .map(|timeline| {
                        timeline
                            .items
                            .into_iter()
                            .map(|post| post.object_id)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                panic!(
                    "{timeout_label}: {}; private_items={private_items:?}",
                    format_sync_snapshot(&status, topic)
                );
            }
        }
    }

    async fn wait_for_timeline_post_result(
        runtime: &DesktopRuntime,
        topic: &str,
        scope: &TimelineScope,
        object_id: &str,
        step_timeout: Duration,
    ) -> Result<()> {
        match timeout(step_timeout, async {
            loop {
                let timeline = runtime
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .context("timeline query failed")?;
                if timeline
                    .items
                    .iter()
                    .any(|post| post.object_id == object_id)
                {
                    return Ok::<(), anyhow::Error>(());
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let status = runtime
                    .get_sync_status()
                    .await
                    .ok()
                    .map(|value| format_sync_snapshot(&value, topic))
                    .unwrap_or_else(|| "failed to read sync status".to_string());
                bail!("timeline visibility timeout; {status}");
            }
        }
    }

    async fn wait_for_profile_timeline_posts(
        runtime: &DesktopRuntime,
        author_pubkey: &str,
        object_ids: &[String],
        timeout_label: &str,
    ) -> TimelineView {
        match timeout(runtime_replication_timeout(), async {
            loop {
                let timeline = runtime
                    .list_profile_timeline(ListProfileTimelineRequest {
                        pubkey: author_pubkey.to_string(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("profile timeline");
                if object_ids.iter().all(|object_id| {
                    timeline
                        .items
                        .iter()
                        .any(|post| post.object_id == *object_id)
                }) {
                    return timeline;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(timeline) => timeline,
            Err(_) => {
                let status = runtime.get_sync_status().await.expect("sync status");
                let visible_items = runtime
                    .list_profile_timeline(ListProfileTimelineRequest {
                        pubkey: author_pubkey.to_string(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .ok()
                    .map(|timeline| {
                        timeline
                            .items
                            .into_iter()
                            .map(|post| format!("{}@{:?}", post.object_id, post.origin_topic_id))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                panic!(
                    "{timeout_label}: {}; visible_items={visible_items:?}",
                    format_sync_snapshot(&status, "")
                );
            }
        }
    }

    async fn wait_for_profile_post_doc(
        runtime: &DesktopRuntime,
        author_pubkey: &str,
        object_id: &str,
        timeout_label: &str,
    ) {
        let author_replica = author_replica_id(author_pubkey);
        match timeout(runtime_replication_timeout(), async {
            loop {
                let _ = runtime
                    .list_profile_timeline(ListProfileTimelineRequest {
                        pubkey: author_pubkey.to_string(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("profile timeline");
                let current = runtime.iroh_stack.current.lock().await;
                let docs_sync = current.as_ref().expect("current stack").docs_sync.clone();
                drop(current);
                let docs = docs_sync
                    .query_replica(&author_replica, DocQuery::Prefix("profile/posts/".into()))
                    .await
                    .expect("profile post docs");
                if docs.into_iter().any(|record| {
                    serde_json::from_slice::<AuthorProfilePostDocV1>(record.value.as_slice())
                        .map(|doc| doc.object_id.as_str() == object_id)
                        .unwrap_or(false)
                }) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let status = runtime.get_sync_status().await.expect("sync status");
                let current = runtime.iroh_stack.current.lock().await;
                let docs_sync = current.as_ref().expect("current stack").docs_sync.clone();
                drop(current);
                let visible_doc_ids = docs_sync
                    .query_replica(&author_replica, DocQuery::Prefix("profile/posts/".into()))
                    .await
                    .ok()
                    .map(|docs| {
                        docs.into_iter()
                            .filter_map(|record| {
                                serde_json::from_slice::<AuthorProfilePostDocV1>(
                                    record.value.as_slice(),
                                )
                                .ok()
                                .map(|doc| doc.object_id.as_str().to_string())
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                panic!(
                    "{timeout_label}: {}; visible_doc_ids={visible_doc_ids:?}",
                    format_sync_snapshot(&status, "")
                );
            }
        }
    }

    fn public_replication_retry_schedule(
        step_timeout: Duration,
        same_author_shared_identity: bool,
    ) -> (usize, Duration) {
        let attempts = if cfg!(target_os = "windows")
            || std::env::var_os("GITHUB_ACTIONS").is_some()
            || same_author_shared_identity
        {
            3
        } else {
            1
        };
        let per_attempt_timeout = if attempts > 1 {
            Duration::from_millis(
                (step_timeout.as_millis() / attempts as u128)
                    .max(1)
                    .try_into()
                    .expect("public replication timeout fits in u64"),
            )
        } else {
            step_timeout
        };
        (attempts, per_attempt_timeout)
    }

    async fn topic_timeline_doc_index_rows(runtime: &DesktopRuntime, topic: &str) -> Vec<String> {
        let replica = kukuri_docs_sync::topic_replica_id(topic);
        let current = runtime.iroh_stack.current.lock().await;
        let docs_sync = current.as_ref().expect("current stack").docs_sync.clone();
        drop(current);
        docs_sync
            .query_replica(&replica, DocQuery::Prefix("indexes/timeline/".into()))
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| row.key)
            .collect()
    }

    async fn replicate_public_post_with_retry(
        publisher: &DesktopRuntime,
        subscriber: &DesktopRuntime,
        topic: &str,
        content_prefix: &str,
        timeout_label: &str,
    ) -> String {
        let same_author_shared_identity = publisher
            .get_sync_status()
            .await
            .ok()
            .zip(subscriber.get_sync_status().await.ok())
            .is_some_and(|(publisher_status, subscriber_status)| {
                publisher_status.local_author_pubkey == subscriber_status.local_author_pubkey
            });
        let (attempts, attempt_timeout) = public_replication_retry_schedule(
            runtime_replication_timeout(),
            same_author_shared_identity,
        );
        let scope = TimelineScope::Public;
        let mut last_error = None;

        for attempt in 1..=attempts {
            let attempt_result = async {
                let _ = publisher
                    .list_timeline(ListTimelineRequest {
                        topic: topic.to_string(),
                        scope: scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .context("failed to resubscribe publisher to public topic")?;
                let _ = subscriber
                    .list_timeline(ListTimelineRequest {
                        topic: topic.to_string(),
                        scope: scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .context("failed to resubscribe subscriber to public topic")?;
                wait_for_connected_topic_peer_count_result(publisher, topic, 1, attempt_timeout)
                    .await
                    .context("publisher did not observe public topic connectivity")?;
                wait_for_connected_topic_peer_count_result(subscriber, topic, 1, attempt_timeout)
                    .await
                    .context("subscriber did not observe public topic connectivity")?;
                let publisher_status = publisher
                    .get_sync_status()
                    .await
                    .context("publisher sync status")?;
                let subscriber_status = subscriber
                    .get_sync_status()
                    .await
                    .context("subscriber sync status")?;
                let publish_from_subscriber = same_author_shared_identity
                    && should_swap_shared_identity_public_replication_direction(
                        &publisher_status,
                        &subscriber_status,
                        topic,
                        1,
                    );
                let (active_publisher, active_subscriber) = if publish_from_subscriber {
                    (subscriber, publisher)
                } else {
                    (publisher, subscriber)
                };
                if publish_from_subscriber {
                    wait_for_direct_topic_peer_count_result(
                        active_publisher,
                        topic,
                        1,
                        attempt_timeout,
                    )
                    .await
                    .context(
                        "publishing runtime did not observe direct public topic connectivity",
                    )?;
                }
                let object_id = active_publisher
                    .create_post(CreatePostRequest {
                        topic: topic.to_string(),
                        content: format!("{content_prefix} #{attempt}"),
                        reply_to: None,
                        channel_ref: ChannelRef::Public,
                        attachments: Vec::new(),
                    })
                    .await
                    .context("failed to create public post")?;
                wait_for_topic_doc_index_entry_result(
                    active_publisher,
                    topic,
                    object_id.as_str(),
                    attempt_timeout,
                )
                .await
                .context("publisher did not persist public post into docs index")?;
                wait_for_timeline_post_result(
                    active_subscriber,
                    topic,
                    &scope,
                    object_id.as_str(),
                    attempt_timeout,
                )
                .await
                .context("subscriber did not observe replicated public post")?;
                Ok::<String, anyhow::Error>(object_id)
            }
            .await;

            match attempt_result {
                Ok(object_id) => return object_id,
                Err(error) if attempt < attempts => {
                    last_error = Some(format!("{error:#}"));
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    last_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }

        let publisher_status = publisher
            .get_sync_status()
            .await
            .expect("publisher sync status");
        let subscriber_status = subscriber
            .get_sync_status()
            .await
            .expect("subscriber sync status");
        let publisher_docs_rows = topic_timeline_doc_index_rows(publisher, topic).await;
        let subscriber_docs_rows = topic_timeline_doc_index_rows(subscriber, topic).await;
        panic!(
            "{timeout_label}; last_error={last_error:?}; publisher=({}); subscriber=({}); publisher_docs_rows={publisher_docs_rows:?}; subscriber_docs_rows={subscriber_docs_rows:?}",
            format_sync_snapshot(&publisher_status, topic),
            format_sync_snapshot(&subscriber_status, topic),
        );
    }

    fn sync_status_with_topic(
        topic: &str,
        connected_peers: &[&str],
        assist_peer_ids: &[&str],
    ) -> SyncStatus {
        SyncStatus {
            connected: true,
            last_sync_ts: None,
            peer_count: connected_peers.len().max(assist_peer_ids.len()),
            pending_events: 0,
            status_detail: "test".to_string(),
            last_error: None,
            configured_peers: Vec::new(),
            subscribed_topics: vec![topic.to_string()],
            topic_diagnostics: vec![kukuri_app_api::TopicSyncStatus {
                topic: topic.to_string(),
                joined: true,
                peer_count: connected_peers.len().max(assist_peer_ids.len()),
                connected_peers: connected_peers
                    .iter()
                    .map(|peer| peer.to_string())
                    .collect(),
                assist_peer_ids: assist_peer_ids
                    .iter()
                    .map(|peer| peer.to_string())
                    .collect(),
                configured_peer_ids: Vec::new(),
                missing_peer_ids: Vec::new(),
                last_received_at: None,
                status_detail: "test".to_string(),
                last_error: None,
            }],
            local_author_pubkey: "author".to_string(),
            discovery: Default::default(),
        }
    }

    #[test]
    fn shared_identity_public_replication_prefers_direct_connected_runtime() {
        let topic = "kukuri:topic:test";
        let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer"]);
        let subscriber_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);

        assert!(should_swap_shared_identity_public_replication_direction(
            &publisher_status,
            &subscriber_status,
            topic,
            1,
        ));
    }

    #[test]
    fn shared_identity_public_replication_keeps_original_publisher_when_it_is_direct() {
        let topic = "kukuri:topic:test";
        let publisher_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);
        let subscriber_status = sync_status_with_topic(topic, &[], &["assist-peer"]);

        assert!(!should_swap_shared_identity_public_replication_direction(
            &publisher_status,
            &subscriber_status,
            topic,
            1,
        ));
    }

    async fn wait_for_joined_private_channel_epoch(
        runtime: &DesktopRuntime,
        topic: &str,
        channel_id: &str,
        expected_epoch_id: &str,
        min_participant_count: usize,
        timeout_label: &str,
    ) -> JoinedPrivateChannelView {
        match timeout(runtime_replication_timeout(), async {
            let private_scope = TimelineScope::Channel {
                channel_id: kukuri_core::ChannelId::new(channel_id.to_string()),
            };
            loop {
                let _ = runtime
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: TimelineScope::Public,
                        cursor: None,
                        limit: Some(20),
                    })
                    .await;
                let joined = runtime
                    .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                        topic: topic.into(),
                    })
                    .await
                    .expect("joined channels");
                let Some(entry) = joined.iter().find(|item| item.channel_id == channel_id) else {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                };
                let _ = runtime
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await;
                if entry.current_epoch_id == expected_epoch_id
                    && entry.participant_count >= min_participant_count
                {
                    break entry.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(entry) => entry,
            Err(_) => {
                let status = runtime.get_sync_status().await.expect("sync status");
                let joined = runtime
                    .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                        topic: topic.into(),
                    })
                    .await
                    .unwrap_or_default();
                panic!(
                    "{timeout_label}: {} joined={joined:?}",
                    format_sync_snapshot(&status, topic)
                );
            }
        }
    }

    async fn wait_for_seeded_dht_topic_ready(
        runtime_a: &DesktopRuntime,
        runtime_b: &DesktopRuntime,
        topic: &str,
    ) {
        match timeout(seeded_dht_runtime_ready_timeout(), async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status_a = runtime_a.get_sync_status().await.expect("status a");
                let status_b = runtime_b.get_sync_status().await.expect("status b");
                let ready_a = status_a.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && (!topic_status.connected_peers.is_empty()
                            || !topic_status.assist_peer_ids.is_empty())
                        && topic_status.peer_count > 0
                });
                let ready_b = status_b.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && (!topic_status.connected_peers.is_empty()
                            || !topic_status.assist_peer_ids.is_empty())
                        && topic_status.peer_count > 0
                });
                if ready_a && ready_b {
                    stable_ready_polls += 1;
                    if stable_ready_polls >= 3 {
                        return;
                    }
                } else {
                    stable_ready_polls = 0;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let status_a = runtime_a.get_sync_status().await.expect("status a");
                let status_b = runtime_b.get_sync_status().await.expect("status b");
                panic!(
                    "seeded dht topic readiness timeout for `{topic}`: status_a={status_a:?} status_b={status_b:?}"
                );
            }
        }
    }

    fn image_attachment_request(name: &str, mime: &str, bytes: &[u8]) -> CreateAttachmentRequest {
        CreateAttachmentRequest {
            file_name: Some(name.to_string()),
            mime: mime.to_string(),
            byte_size: bytes.len() as u64,
            data_base64: BASE64_STANDARD.encode(bytes),
            role: Some("image_original".to_string()),
        }
    }

    fn profile_avatar_attachment_request(
        name: &str,
        mime: &str,
        bytes: &[u8],
    ) -> CreateAttachmentRequest {
        CreateAttachmentRequest {
            file_name: Some(name.to_string()),
            mime: mime.to_string(),
            byte_size: bytes.len() as u64,
            data_base64: BASE64_STANDARD.encode(bytes),
            role: Some("profile_avatar".to_string()),
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

    fn delete_sqlite_artifacts(db_path: &Path) {
        for path in [
            db_path.to_path_buf(),
            db_path.with_extension("db-shm"),
            db_path.with_extension("db-wal"),
        ] {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("delete sqlite artifact {}: {error}", path.display()),
            }
        }
    }

    async fn publish_runtime_endpoint_to_testnet(runtime: &DesktopRuntime, testnet: &Testnet) {
        let endpoint = runtime.iroh_stack.endpoint().await;
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

    #[derive(Clone)]
    struct MockCommunityNodeState {
        base_url: String,
        seed_peers: Arc<Mutex<Vec<CommunityNodeSeedPeer>>>,
        heartbeat_hits: Arc<AtomicUsize>,
        bootstrap_hits: Arc<AtomicUsize>,
    }

    async fn mock_bootstrap_heartbeat(
        State(state): State<Arc<MockCommunityNodeState>>,
        Json(_request): Json<serde_json::Value>,
    ) -> Json<BootstrapHeartbeatResponse> {
        state.heartbeat_hits.fetch_add(1, Ordering::SeqCst);
        Json(BootstrapHeartbeatResponse {
            expires_at: Utc::now().timestamp() + 300,
        })
    }

    async fn mock_bootstrap_nodes(
        State(state): State<Arc<MockCommunityNodeState>>,
    ) -> Json<BootstrapNodesResponse> {
        state.bootstrap_hits.fetch_add(1, Ordering::SeqCst);
        let seed_peers = state.seed_peers.lock().await.clone();
        Json(BootstrapNodesResponse {
            nodes: vec![kukuri_cn_core::CommunityNodeBootstrapNode {
                base_url: state.base_url.clone(),
                resolved_urls: CommunityNodeResolvedUrls::new(
                    state.base_url.clone(),
                    Vec::new(),
                    seed_peers,
                )
                .expect("resolved urls"),
            }],
        })
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
                channel_ref: ChannelRef::Public,
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
                channel_ref: ChannelRef::Public,
                attachments: vec![],
            })
            .await
            .expect("create post after restart");
        let timeline = restarted
            .list_timeline(ListTimelineRequest {
                topic: "kukuri:topic:runtime".into(),
                scope: TimelineScope::Public,
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

    #[tokio::test]
    async fn desktop_runtime_restores_profile_avatar_blob_after_restart() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("profile-avatar-restart.db");
        let avatar_bytes = b"runtime-profile-avatar".to_vec();
        let expected_payload = BASE64_STANDARD.encode(&avatar_bytes);
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

        let updated = runtime
            .set_my_profile(SetMyProfileRequest {
                name: Some("runtime-avatar-owner".into()),
                display_name: Some("Runtime Avatar Owner".into()),
                about: Some("profile avatar restart".into()),
                picture: None,
                picture_upload: Some(profile_avatar_attachment_request(
                    "avatar.png",
                    "image/png",
                    &avatar_bytes,
                )),
                clear_picture: false,
            })
            .await
            .expect("set profile");
        let asset = updated.picture_asset.clone().expect("profile avatar");
        let author_pubkey = updated.pubkey.as_str().to_string();
        let payload_before_restart = runtime
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: asset.hash.as_str().to_string(),
                mime: asset.mime.clone(),
            })
            .await
            .expect("avatar payload before restart")
            .expect("avatar payload before restart value");
        let author_before_restart = runtime
            .get_author_social_view(AuthorRequest {
                pubkey: author_pubkey.clone(),
            })
            .await
            .expect("author social view before restart");

        assert_eq!(payload_before_restart.mime, "image/png");
        assert_eq!(payload_before_restart.bytes_base64, expected_payload);
        assert_eq!(
            author_before_restart
                .picture_asset
                .as_ref()
                .map(|value| value.hash.as_str()),
            Some(asset.hash.as_str())
        );
        assert_eq!(
            author_before_restart
                .picture_asset
                .as_ref()
                .map(|value| value.role.as_str()),
            Some("profile_avatar")
        );

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
        let my_profile = restarted.get_my_profile().await.expect("my profile");
        let author_after_restart = restarted
            .get_author_social_view(AuthorRequest {
                pubkey: author_pubkey,
            })
            .await
            .expect("author social view after restart");
        let payload_after_restart = restarted
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: asset.hash.as_str().to_string(),
                mime: asset.mime.clone(),
            })
            .await
            .expect("avatar payload after restart")
            .expect("avatar payload after restart value");

        assert_eq!(
            my_profile
                .picture_asset
                .as_ref()
                .map(|value| value.hash.as_str()),
            Some(asset.hash.as_str())
        );
        assert_eq!(
            my_profile
                .picture_asset
                .as_ref()
                .map(|value| value.role.clone()),
            Some(AssetRole::ProfileAvatar)
        );
        assert_eq!(
            author_after_restart
                .picture_asset
                .as_ref()
                .map(|value| value.hash.as_str()),
            Some(asset.hash.as_str())
        );
        assert_eq!(
            author_after_restart
                .picture_asset
                .as_ref()
                .map(|value| value.role.as_str()),
            Some("profile_avatar")
        );
        assert_eq!(payload_after_restart.mime, "image/png");
        assert_eq!(payload_after_restart.bytes_base64, expected_payload);

        timeout(Duration::from_secs(15), restarted.shutdown())
            .await
            .expect("restarted shutdown timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn desktop_runtime_imports_peer_ticket_and_tracks_local_posts() {
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
        let endpoint_a = runtime_a
            .get_sync_status()
            .await
            .expect("status a before import")
            .discovery
            .local_endpoint_id;
        let endpoint_b = runtime_b
            .get_sync_status()
            .await
            .expect("status b before import")
            .discovery
            .local_endpoint_id;

        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("import b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("import a");

        let status_a = runtime_a
            .get_sync_status()
            .await
            .expect("status a after import");
        let status_b = runtime_b
            .get_sync_status()
            .await
            .expect("status b after import");
        assert_eq!(status_a.discovery.manual_ticket_peer_ids, vec![endpoint_b]);
        assert_eq!(status_b.discovery.manual_ticket_peer_ids, vec![endpoint_a]);

        let topic = "kukuri:topic:desktop-runtime";
        let object_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "hello desktop runtime".into(),
                reply_to: None,
                channel_ref: ChannelRef::Public,
                attachments: vec![],
            })
            .await
            .expect("create post");

        let timeline = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("timeline a");
        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("local post");
        assert_eq!(post.content, "hello desktop runtime");
        let status = runtime_a.get_sync_status().await.expect("sync status");
        assert!(status.last_sync_ts.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn profile_timeline_reads_author_public_posts_across_untracked_topics() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("profile-runtime-a.db");
        let db_b = dir.path().join("profile-runtime-b.db");
        let shared_keys = KukuriKeys::generate();
        let shared_secret = shared_keys.export_secret_hex();
        fs::write(
            db_a.with_extension("identity-key"),
            shared_secret.as_bytes(),
        )
        .expect("persist shared identity key a");
        fs::write(db_a.with_extension("identity-store"), b"file")
            .expect("persist shared identity backend a");
        fs::write(
            db_b.with_extension("identity-key"),
            shared_secret.as_bytes(),
        )
        .expect("persist shared identity key b");
        fs::write(db_b.with_extension("identity-store"), b"file")
            .expect("persist shared identity backend b");
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
        wait_for_connected_peer_count(&runtime_a, 1, "profile topic owner peer readiness timeout")
            .await;
        wait_for_connected_peer_count(&runtime_b, 1, "profile topic viewer peer readiness timeout")
            .await;

        let author_pubkey = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .local_author_pubkey;
        assert_eq!(
            author_pubkey,
            runtime_b
                .get_sync_status()
                .await
                .expect("status b")
                .local_author_pubkey
        );
        let tracked_topic = "kukuri:topic:desktop-profile-demo";
        let untracked_topic = "kukuri:topic:desktop-profile-relay";
        let public_scope = TimelineScope::Public;

        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: tracked_topic.into(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe a tracked topic");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: tracked_topic.into(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b tracked topic");
        wait_for_connected_topic_peer_count(
            &runtime_a,
            tracked_topic,
            1,
            "profile tracked topic readiness timeout a",
        )
        .await;
        wait_for_connected_topic_peer_count(
            &runtime_b,
            tracked_topic,
            1,
            "profile tracked topic readiness timeout b",
        )
        .await;

        let tracked_object_id = replicate_public_post_with_retry(
            &runtime_a,
            &runtime_b,
            tracked_topic,
            "tracked profile post",
            "tracked topic visibility timeout",
        )
        .await;
        let untracked_object_id = runtime_a
            .create_post(CreatePostRequest {
                topic: untracked_topic.into(),
                content: "untracked profile post".into(),
                reply_to: None,
                channel_ref: ChannelRef::Public,
                attachments: vec![],
            })
            .await
            .expect("untracked public post");
        wait_for_profile_post_doc(
            &runtime_b,
            author_pubkey.as_str(),
            untracked_object_id.as_str(),
            "profile post doc visibility timeout",
        )
        .await;

        let before_profile = runtime_b
            .get_sync_status()
            .await
            .expect("status before profile");
        assert!(
            before_profile
                .subscribed_topics
                .iter()
                .any(|topic| topic == tracked_topic)
        );
        assert!(
            before_profile
                .subscribed_topics
                .iter()
                .all(|topic| topic != untracked_topic)
        );

        let profile_timeline = wait_for_profile_timeline_posts(
            &runtime_b,
            author_pubkey.as_str(),
            &[tracked_object_id.clone(), untracked_object_id.clone()],
            "profile timeline visibility timeout",
        )
        .await;
        assert!(
            profile_timeline
                .items
                .iter()
                .any(|post| post.object_id == tracked_object_id
                    && post.origin_topic_id.as_deref() == Some(tracked_topic))
        );
        assert!(
            profile_timeline
                .items
                .iter()
                .any(|post| post.object_id == untracked_object_id
                    && post.origin_topic_id.as_deref() == Some(untracked_topic))
        );

        let after_profile = runtime_b
            .get_sync_status()
            .await
            .expect("status after profile");
        assert!(
            after_profile
                .subscribed_topics
                .iter()
                .all(|topic| topic != untracked_topic)
        );

        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: untracked_topic.into(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("open original topic");
        wait_for_timeline_post(
            &runtime_b,
            untracked_topic,
            &public_scope,
            untracked_object_id.as_str(),
            "origin topic visibility timeout",
        )
        .await;

        let after_origin = runtime_b
            .get_sync_status()
            .await
            .expect("status after origin");
        assert!(
            after_origin
                .subscribed_topics
                .iter()
                .any(|topic| topic == untracked_topic)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn private_channel_invite_restores_after_restart_without_reimport() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("private-runtime-a.db");
        let db_b = dir.path().join("private-runtime-b.db");
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
        wait_for_connected_peer_count(&runtime_a, 1, "friend-only owner peer readiness timeout")
            .await;
        wait_for_connected_peer_count(&runtime_b, 1, "friend-only invitee peer readiness timeout")
            .await;

        let topic = "kukuri:topic:desktop-private-channel";
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe a");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");
        let channel = runtime_a
            .create_private_channel(CreatePrivateChannelRequest {
                topic: topic.into(),
                label: "core".into(),
                audience_kind: ChannelAudienceKind::InviteOnly,
            })
            .await
            .expect("create private channel");
        let invite = runtime_a
            .export_private_channel_invite(ExportPrivateChannelInviteRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export invite");
        let preview = runtime_b
            .import_private_channel_invite(ImportPrivateChannelInviteRequest { token: invite })
            .await
            .expect("import invite");
        assert_eq!(preview.topic_id.as_str(), topic);
        assert_eq!(preview.channel_id.as_str(), channel.channel_id);

        let private_channel_id = kukuri_core::ChannelId::new(channel.channel_id.clone());
        let private_channel_ref = ChannelRef::PrivateChannel {
            channel_id: private_channel_id.clone(),
        };
        let private_scope = TimelineScope::Channel {
            channel_id: private_channel_id.clone(),
        };
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe private a");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe private b");

        let private_post_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "private hello from b".into(),
                reply_to: None,
                channel_ref: private_channel_ref.clone(),
                attachments: vec![],
            })
            .await
            .expect("create private post");

        let private_post = timeout(Duration::from_secs(10), async {
            loop {
                let public_timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: TimelineScope::Public,
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("public timeline");
                assert!(
                    public_timeline
                        .items
                        .iter()
                        .all(|post| post.object_id != private_post_id),
                    "private post leaked into public timeline"
                );
                let private_timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("private timeline");
                if let Some(post) = private_timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == private_post_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("private post timeout");
        assert_eq!(
            private_post.channel_id.as_deref(),
            Some(channel.channel_id.as_str())
        );
        assert_eq!(private_post.audience_label, "core");
        let _ = runtime_b
            .list_thread(ListThreadRequest {
                topic: topic.into(),
                thread_id: private_post_id.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe private thread");

        let private_reply_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "private reply".into(),
                reply_to: Some(private_post_id.clone()),
                channel_ref: ChannelRef::Public,
                attachments: vec![],
            })
            .await
            .expect("create private reply");
        let private_thread = timeout(Duration::from_secs(10), async {
            loop {
                let thread = runtime_b
                    .list_thread(ListThreadRequest {
                        topic: topic.into(),
                        thread_id: private_post_id.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("thread");
                if thread
                    .items
                    .iter()
                    .any(|post| post.object_id == private_reply_id)
                {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("private thread timeout");
        let reply = private_thread
            .items
            .iter()
            .find(|post| post.object_id == private_reply_id)
            .expect("reply");
        assert_eq!(
            reply.channel_id.as_deref(),
            Some(channel.channel_id.as_str())
        );

        let session_id = runtime_b
            .create_live_session(CreateLiveSessionRequest {
                topic: topic.into(),
                channel_ref: private_channel_ref.clone(),
                title: "core live".into(),
                description: "private stream".into(),
            })
            .await
            .expect("create private live session");
        let _private_session = timeout(Duration::from_secs(10), async {
            loop {
                let sessions = runtime_b
                    .list_live_sessions(ListLiveSessionsRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                    })
                    .await
                    .expect("list private live sessions");
                if let Some(session) = sessions
                    .iter()
                    .find(|session| session.session_id == session_id)
                {
                    return session.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("private live timeout");
        runtime_b
            .end_live_session(LiveSessionCommandRequest {
                topic: topic.into(),
                session_id: session_id.clone(),
            })
            .await
            .expect("end live session");
        timeout(Duration::from_secs(10), async {
            loop {
                let sessions = runtime_b
                    .list_live_sessions(ListLiveSessionsRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                    })
                    .await
                    .expect("list live sessions b");
                if sessions.iter().any(|session| {
                    session.session_id == session_id
                        && session.status == kukuri_core::LiveSessionStatus::Ended
                }) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("live end timeout");

        let room_id = runtime_b
            .create_game_room(CreateGameRoomRequest {
                topic: topic.into(),
                channel_ref: private_channel_ref.clone(),
                title: "core room".into(),
                description: "private set".into(),
                participants: vec!["Alice".into(), "Bob".into()],
            })
            .await
            .expect("create private game room");
        let room_before_update = timeout(Duration::from_secs(10), async {
            loop {
                let rooms = runtime_b
                    .list_game_rooms(ListGameRoomsRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                    })
                    .await
                    .expect("list private game rooms");
                if let Some(room) = rooms.iter().find(|room| room.room_id == room_id) {
                    return room.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("private game timeout");
        runtime_b
            .update_game_room(UpdateGameRoomRequest {
                topic: topic.into(),
                room_id: room_id.clone(),
                status: GameRoomStatus::Running,
                phase_label: Some("Round 2".into()),
                scores: room_before_update
                    .scores
                    .iter()
                    .map(|score| GameScoreView {
                        participant_id: score.participant_id.clone(),
                        label: score.label.clone(),
                        score: if score.label == "Alice" { 2 } else { 1 },
                    })
                    .collect(),
            })
            .await
            .expect("update private game room");
        timeout(Duration::from_secs(10), async {
            loop {
                let rooms = runtime_b
                    .list_game_rooms(ListGameRoomsRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                    })
                    .await
                    .expect("list updated game rooms");
                if rooms.iter().any(|room| {
                    room.room_id == room_id
                        && room.phase_label.as_deref() == Some("Round 2")
                        && room
                            .scores
                            .iter()
                            .any(|score| score.label == "Alice" && score.score == 2)
                }) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("game update timeout");

        let joined_before_restart = runtime_b
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.into(),
            })
            .await
            .expect("list joined channels before restart");
        assert_eq!(joined_before_restart.len(), 1);
        assert_eq!(joined_before_restart[0].channel_id, channel.channel_id);

        timeout(Duration::from_secs(30), runtime_a.shutdown())
            .await
            .expect("runtime a shutdown timeout");
        timeout(Duration::from_secs(30), runtime_b.shutdown())
            .await
            .expect("runtime b shutdown timeout");
        drop(runtime_a);
        drop(runtime_b);
        delete_sqlite_artifacts(&db_b);

        let restarted_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart runtime b");

        let joined_after_restart = restarted_b
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.into(),
            })
            .await
            .expect("list joined channels after restart");
        assert_eq!(joined_after_restart.len(), 1);
        assert_eq!(joined_after_restart[0].channel_id, channel.channel_id);
        assert_eq!(joined_after_restart[0].label, "core");

        let public_timeline_after_restart = restarted_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("public timeline after restart");
        assert!(
            public_timeline_after_restart
                .items
                .iter()
                .all(|post| post.object_id != private_post_id)
        );
        let private_timeline_after_restart = restarted_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("private timeline after restart");
        assert!(
            private_timeline_after_restart
                .items
                .iter()
                .any(|post| post.object_id == private_post_id)
        );
        assert!(
            private_timeline_after_restart
                .items
                .iter()
                .any(|post| post.object_id == private_reply_id)
        );

        let private_thread_after_restart = restarted_b
            .list_thread(ListThreadRequest {
                topic: topic.into(),
                thread_id: private_post_id.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("private thread after restart");
        assert!(
            private_thread_after_restart
                .items
                .iter()
                .any(|post| post.object_id == private_reply_id)
        );

        let sessions_after_restart = restarted_b
            .list_live_sessions(ListLiveSessionsRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
            })
            .await
            .expect("live sessions after restart");
        assert!(sessions_after_restart.iter().any(|session| {
            session.session_id == session_id
                && session.status == kukuri_core::LiveSessionStatus::Ended
        }));

        let rooms_after_restart = restarted_b
            .list_game_rooms(ListGameRoomsRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
            })
            .await
            .expect("game rooms after restart");
        assert!(rooms_after_restart.iter().any(|room| {
            room.room_id == room_id
                && room.phase_label.as_deref() == Some("Round 2")
                && room
                    .scores
                    .iter()
                    .any(|score| score.label == "Alice" && score.score == 2)
        }));

        let fresh_invite = restarted_b
            .export_private_channel_invite(ExportPrivateChannelInviteRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export fresh invite");
        assert!(fresh_invite.contains(topic));
        assert!(fresh_invite.contains(channel.channel_id.as_str()));

        timeout(Duration::from_secs(30), restarted_b.shutdown())
            .await
            .expect("restarted runtime shutdown timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn friend_only_channel_restore_keeps_archived_epoch_history() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("friend-only-runtime-a.db");
        let db_b = dir.path().join("friend-only-runtime-b.db");
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
        wait_for_connected_peer_count(&runtime_a, 1, "friend-only owner peer readiness timeout")
            .await;
        wait_for_connected_peer_count(
            &runtime_b,
            1,
            "friend-only recipient peer readiness timeout",
        )
        .await;

        let a_pubkey = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .local_author_pubkey;
        let b_pubkey = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .local_author_pubkey;
        warm_author_social_view(
            &runtime_a,
            b_pubkey.as_str(),
            "friend-only owner author warm timeout",
        )
        .await;
        warm_author_social_view(
            &runtime_b,
            a_pubkey.as_str(),
            "friend-only recipient author warm timeout",
        )
        .await;
        runtime_a
            .follow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("a follows b");
        runtime_b
            .follow_author(AuthorRequest {
                pubkey: a_pubkey.clone(),
            })
            .await
            .expect("b follows a");

        timeout(social_graph_propagation_timeout(), async {
            loop {
                let a_view = runtime_a
                    .get_author_social_view(AuthorRequest {
                        pubkey: b_pubkey.clone(),
                    })
                    .await
                    .expect("a loads b");
                let b_view = runtime_b
                    .get_author_social_view(AuthorRequest {
                        pubkey: a_pubkey.clone(),
                    })
                    .await
                    .expect("b loads a");
                if a_view.mutual && b_view.mutual {
                    return;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("mutual propagation timeout");

        let topic = "kukuri:topic:desktop-friend-only-restart";
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe a");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");

        let channel = runtime_a
            .create_private_channel(CreatePrivateChannelRequest {
                topic: topic.into(),
                label: "friends".into(),
                audience_kind: ChannelAudienceKind::FriendOnly,
            })
            .await
            .expect("create friend-only channel");
        let grant = runtime_a
            .export_friend_only_grant(ExportFriendOnlyGrantRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export friend-only grant");
        let preview = runtime_b
            .import_friend_only_grant(ImportFriendOnlyGrantRequest { token: grant })
            .await
            .expect("import friend-only grant");
        let original_epoch_id = preview.epoch_id.clone();
        assert_eq!(preview.topic_id.as_str(), topic);
        assert_eq!(preview.channel_id.as_str(), channel.channel_id);

        let private_channel_id = kukuri_core::ChannelId::new(channel.channel_id.clone());
        let private_channel_ref = ChannelRef::PrivateChannel {
            channel_id: private_channel_id.clone(),
        };
        let private_scope = TimelineScope::Channel {
            channel_id: private_channel_id.clone(),
        };
        let private_post_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "friends hello from b".into(),
                reply_to: None,
                channel_ref: private_channel_ref,
                attachments: vec![],
            })
            .await
            .expect("create friend-only post");

        timeout(runtime_replication_timeout(), async {
            loop {
                let public_timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: TimelineScope::Public,
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("public timeline");
                assert!(
                    public_timeline
                        .items
                        .iter()
                        .all(|post| post.object_id != private_post_id),
                    "friend-only post leaked into public timeline"
                );
                let private_timeline = runtime_b
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("private timeline");
                if private_timeline
                    .items
                    .iter()
                    .any(|post| post.object_id == private_post_id)
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("friend-only post timeout");

        let rotated = runtime_a
            .rotate_private_channel(RotatePrivateChannelRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
            })
            .await
            .expect("rotate friend-only channel");
        assert_ne!(rotated.current_epoch_id, original_epoch_id);
        assert_eq!(rotated.archived_epoch_ids, vec![original_epoch_id.clone()]);

        let fresh_grant = runtime_a
            .export_friend_only_grant(ExportFriendOnlyGrantRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export fresh friend-only grant");
        let fresh_preview = runtime_b
            .import_friend_only_grant(ImportFriendOnlyGrantRequest { token: fresh_grant })
            .await
            .expect("import fresh friend-only grant");
        assert_eq!(fresh_preview.epoch_id, rotated.current_epoch_id);

        let joined_before_restart = vec![
            wait_for_joined_private_channel_epoch(
                &runtime_b,
                topic,
                channel.channel_id.as_str(),
                rotated.current_epoch_id.as_str(),
                2,
                "joined channel update timeout",
            )
            .await,
        ];
        assert_eq!(joined_before_restart.len(), 1);
        assert_eq!(
            joined_before_restart[0].archived_epoch_ids,
            vec![original_epoch_id.clone()]
        );

        timeout(Duration::from_secs(30), runtime_a.shutdown())
            .await
            .expect("runtime a shutdown timeout");
        timeout(Duration::from_secs(30), runtime_b.shutdown())
            .await
            .expect("runtime b shutdown timeout");
        drop(runtime_a);
        drop(runtime_b);
        delete_sqlite_artifacts(&db_b);

        let restarted_b = DesktopRuntime::new_with_config_and_identity(
            &db_b,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart runtime b");

        let joined_after_restart = restarted_b
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.into(),
            })
            .await
            .expect("list joined channels after restart");
        assert_eq!(joined_after_restart.len(), 1);
        assert_eq!(joined_after_restart[0].channel_id, channel.channel_id);
        assert_eq!(
            joined_after_restart[0].audience_kind,
            ChannelAudienceKind::FriendOnly
        );
        assert_eq!(
            joined_after_restart[0].current_epoch_id,
            rotated.current_epoch_id
        );
        assert_eq!(
            joined_after_restart[0].archived_epoch_ids,
            vec![original_epoch_id.clone()]
        );

        let private_timeline_after_restart = restarted_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("private timeline after restart");
        assert!(
            private_timeline_after_restart
                .items
                .iter()
                .any(|post| post.object_id == private_post_id)
        );

        timeout(Duration::from_secs(30), restarted_b.shutdown())
            .await
            .expect("restarted runtime shutdown timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn friend_plus_channel_restore_accepts_fresh_share_after_restart() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("friend-plus-runtime-a.db");
        let db_b = dir.path().join("friend-plus-runtime-b.db");
        let db_c = dir.path().join("friend-plus-runtime-c.db");
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
        let runtime_c = DesktopRuntime::new_with_config_and_identity(
            &db_c,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime c");

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
        let ticket_c = runtime_c
            .local_peer_ticket()
            .await
            .expect("ticket c")
            .expect("ticket c value");

        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("a imports b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("b imports a");
        wait_for_connected_peer_count(&runtime_a, 1, "friend-plus owner peer readiness timeout")
            .await;
        wait_for_connected_peer_count(&runtime_b, 1, "friend-plus sponsor peer readiness timeout")
            .await;

        let a_pubkey = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .local_author_pubkey;
        let b_pubkey = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .local_author_pubkey;
        let c_pubkey = runtime_c
            .get_sync_status()
            .await
            .expect("status c")
            .local_author_pubkey;
        let topic = "kukuri:topic:desktop-friend-plus-restart";
        for runtime in [&runtime_a, &runtime_b] {
            let _ = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("subscribe runtime");
        }
        wait_for_connected_topic_peer_count(
            &runtime_a,
            topic,
            1,
            "friend-plus owner topic readiness timeout",
        )
        .await;
        wait_for_connected_topic_peer_count(
            &runtime_b,
            topic,
            1,
            "friend-plus sponsor topic readiness timeout",
        )
        .await;
        warm_author_social_view(
            &runtime_a,
            b_pubkey.as_str(),
            "friend-plus owner author warm timeout",
        )
        .await;
        warm_author_social_view(
            &runtime_b,
            a_pubkey.as_str(),
            "friend-plus sponsor owner author warm timeout",
        )
        .await;
        runtime_a
            .follow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("a follows b");
        runtime_b
            .follow_author(AuthorRequest {
                pubkey: a_pubkey.clone(),
            })
            .await
            .expect("b follows a");
        wait_for_mutual_author_view(&runtime_a, b_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&runtime_b, a_pubkey.as_str(), topic).await;
        let channel = runtime_a
            .create_private_channel(CreatePrivateChannelRequest {
                topic: topic.into(),
                label: "friends+".into(),
                audience_kind: ChannelAudienceKind::FriendPlus,
            })
            .await
            .expect("create friend-plus channel");
        let share_ab = runtime_a
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export a->b share");
        runtime_b
            .import_friend_plus_share(ImportFriendPlusShareRequest { token: share_ab })
            .await
            .expect("b imports friend-plus share");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_c.clone(),
            })
            .await
            .expect("a imports c");
        runtime_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("c imports a");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_c.clone(),
            })
            .await
            .expect("b imports c");
        runtime_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("c imports b");
        wait_for_connected_peer_count(&runtime_a, 2, "friend-plus owner full-mesh timeout").await;
        wait_for_connected_peer_count(&runtime_b, 2, "friend-plus sponsor full-mesh timeout").await;
        wait_for_connected_peer_count(&runtime_c, 2, "friend-plus recipient full-mesh timeout")
            .await;
        let _ = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe runtime c");
        wait_for_connected_topic_peer_count(
            &runtime_a,
            topic,
            2,
            "friend-plus owner topic mesh timeout",
        )
        .await;
        wait_for_connected_topic_peer_count(
            &runtime_b,
            topic,
            2,
            "friend-plus sponsor topic mesh timeout",
        )
        .await;
        wait_for_connected_topic_peer_count(
            &runtime_c,
            topic,
            2,
            "friend-plus recipient topic mesh timeout",
        )
        .await;
        warm_author_social_view(
            &runtime_b,
            c_pubkey.as_str(),
            "friend-plus sponsor recipient author warm timeout",
        )
        .await;
        warm_author_social_view(
            &runtime_c,
            b_pubkey.as_str(),
            "friend-plus recipient sponsor author warm timeout",
        )
        .await;
        runtime_b
            .follow_author(AuthorRequest {
                pubkey: c_pubkey.clone(),
            })
            .await
            .expect("b follows c");
        runtime_c
            .follow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("c follows b");
        wait_for_mutual_author_view(&runtime_b, c_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&runtime_c, b_pubkey.as_str(), topic).await;
        let share_bc = runtime_b
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export b->c share");
        let preview_c = runtime_c
            .import_friend_plus_share(ImportFriendPlusShareRequest { token: share_bc })
            .await
            .expect("c imports friend-plus share");
        let original_epoch_id = preview_c.epoch_id.clone();
        assert_eq!(preview_c.sponsor_pubkey.as_str(), b_pubkey.as_str());

        let private_scope = TimelineScope::Channel {
            channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
        };
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
        };
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe friend-plus private b");
        let _ = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe friend-plus private c");
        let joined_a_before_history = wait_for_joined_private_channel_epoch(
            &runtime_a,
            topic,
            channel.channel_id.as_str(),
            original_epoch_id.as_str(),
            3,
            "friend-plus owner private readiness timeout",
        )
        .await;
        assert_eq!(joined_a_before_history.participant_count, 3);
        let joined_b_before_history = wait_for_joined_private_channel_epoch(
            &runtime_b,
            topic,
            channel.channel_id.as_str(),
            original_epoch_id.as_str(),
            3,
            "friend-plus sponsor private readiness timeout",
        )
        .await;
        assert_eq!(
            joined_b_before_history.joined_via_pubkey.as_deref(),
            Some(a_pubkey.as_str())
        );
        assert_eq!(joined_b_before_history.participant_count, 3);
        let joined_c_before_history = wait_for_joined_private_channel_epoch(
            &runtime_c,
            topic,
            channel.channel_id.as_str(),
            original_epoch_id.as_str(),
            3,
            "friend-plus recipient private readiness timeout",
        )
        .await;
        assert_eq!(
            joined_c_before_history.joined_via_pubkey.as_deref(),
            Some(b_pubkey.as_str())
        );
        assert_eq!(joined_c_before_history.participant_count, 3);
        let old_post_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "friend-plus history".into(),
                reply_to: None,
                channel_ref: private_ref.clone(),
                attachments: vec![],
            })
            .await
            .expect("create friend-plus history post");
        wait_for_timeline_post(
            &runtime_b,
            topic,
            &private_scope,
            old_post_id.as_str(),
            "friend-plus sponsor history propagation timeout",
        )
        .await;

        let public_timeline_c = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("public timeline c");
        assert!(
            public_timeline_c
                .items
                .iter()
                .all(|post| post.object_id != old_post_id),
            "friend-plus post leaked into public timeline"
        );
        wait_for_timeline_post(
            &runtime_c,
            topic,
            &private_scope,
            old_post_id.as_str(),
            "friend-plus history propagation timeout",
        )
        .await;

        let joined_before_restart = runtime_c
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.into(),
            })
            .await
            .expect("joined channels before restart");
        assert_eq!(joined_before_restart.len(), 1);
        assert_eq!(joined_before_restart[0].channel_id, channel.channel_id);
        let restored_epoch_id = joined_before_restart[0].current_epoch_id.clone();
        assert_ne!(restored_epoch_id, original_epoch_id);
        assert_eq!(
            joined_before_restart[0].joined_via_pubkey.as_deref(),
            Some(b_pubkey.as_str())
        );

        timeout(Duration::from_secs(30), runtime_c.shutdown())
            .await
            .expect("runtime c shutdown timeout");
        drop(runtime_c);
        delete_sqlite_artifacts(&db_c);

        let restarted_c = DesktopRuntime::new_with_config_and_identity(
            &db_c,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("restart runtime c");
        restarted_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("restarted c imports a");
        restarted_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("restarted c imports b");
        let restarted_ticket_c = restarted_c
            .local_peer_ticket()
            .await
            .expect("restarted ticket c")
            .expect("restarted ticket c value");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: restarted_ticket_c.clone(),
            })
            .await
            .expect("a imports restarted c");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: restarted_ticket_c.clone(),
            })
            .await
            .expect("b imports restarted c");
        let _ = restarted_c
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted c public");
        let _ = restarted_c
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted c private");
        // Re-importing tickets forces existing topic subscriptions to rebuild against C's new endpoint.
        restarted_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("restarted c refreshes a");
        restarted_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("restarted c refreshes b");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: restarted_ticket_c.clone(),
            })
            .await
            .expect("a refreshes restarted c");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: restarted_ticket_c.clone(),
            })
            .await
            .expect("b refreshes restarted c");
        let joined_after_restart = restarted_c
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.into(),
            })
            .await
            .expect("joined channels after restart");
        assert_eq!(joined_after_restart.len(), 1);
        assert_eq!(joined_after_restart[0].channel_id, channel.channel_id);
        assert_eq!(joined_after_restart[0].current_epoch_id, restored_epoch_id);
        assert_eq!(
            joined_after_restart[0].joined_via_pubkey.as_deref(),
            Some(b_pubkey.as_str())
        );

        let private_timeline_after_restart = restarted_c
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("private timeline after restart");
        assert!(
            private_timeline_after_restart
                .items
                .iter()
                .any(|post| post.object_id == old_post_id)
        );
        let joined_restarted_before_rotate = wait_for_joined_private_channel_epoch(
            &restarted_c,
            topic,
            channel.channel_id.as_str(),
            restored_epoch_id.as_str(),
            3,
            "friend-plus restarted private readiness timeout",
        )
        .await;
        assert_eq!(
            joined_restarted_before_rotate.joined_via_pubkey.as_deref(),
            Some(b_pubkey.as_str())
        );
        assert_eq!(joined_restarted_before_rotate.participant_count, 3);

        wait_for_connected_topic_peer_count(
            &runtime_a,
            topic,
            1,
            "friend-plus owner topic readiness timeout",
        )
        .await;
        wait_for_connected_topic_peer_count(
            &runtime_b,
            topic,
            1,
            "friend-plus sponsor topic readiness timeout",
        )
        .await;

        let rotated = runtime_a
            .rotate_private_channel(RotatePrivateChannelRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
            })
            .await
            .expect("rotate friend-plus channel");
        assert_ne!(rotated.current_epoch_id, restored_epoch_id);

        let refreshed_share_ab = runtime_a
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export refreshed a->b share after rotate");
        let preview_b_after_rotate = runtime_b
            .import_friend_plus_share(ImportFriendPlusShareRequest {
                token: refreshed_share_ab,
            })
            .await
            .expect("b imports refreshed friend-plus share");
        let shared_epoch_id = preview_b_after_rotate.epoch_id.clone();
        assert_ne!(shared_epoch_id, restored_epoch_id);
        assert_eq!(
            preview_b_after_rotate.sponsor_pubkey.as_str(),
            a_pubkey.as_str()
        );
        let joined_b_after_rotate = wait_for_joined_private_channel_epoch(
            &runtime_b,
            topic,
            channel.channel_id.as_str(),
            shared_epoch_id.as_str(),
            2,
            "friend-plus sponsor refresh share redeem timeout",
        )
        .await;
        assert_eq!(
            joined_b_after_rotate.joined_via_pubkey.as_deref(),
            Some(a_pubkey.as_str())
        );
        assert!(
            joined_b_after_rotate
                .archived_epoch_ids
                .iter()
                .any(|epoch_id| epoch_id == &restored_epoch_id)
        );

        let fresh_share = runtime_b
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.into(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export fresh friend-plus share after restart");
        let preview_after_restart = restarted_c
            .import_friend_plus_share(ImportFriendPlusShareRequest { token: fresh_share })
            .await
            .expect("restarted c imports fresh friend-plus share");
        assert_eq!(preview_after_restart.epoch_id, shared_epoch_id);
        assert_eq!(
            preview_after_restart.sponsor_pubkey.as_str(),
            b_pubkey.as_str()
        );
        let joined_after_rotate = wait_for_joined_private_channel_epoch(
            &restarted_c,
            topic,
            channel.channel_id.as_str(),
            shared_epoch_id.as_str(),
            3,
            "friend-plus restarted share redeem timeout",
        )
        .await;
        assert_eq!(
            joined_after_rotate.joined_via_pubkey.as_deref(),
            Some(b_pubkey.as_str())
        );
        assert_eq!(joined_after_rotate.participant_count, 3);
        assert!(
            joined_after_rotate
                .archived_epoch_ids
                .iter()
                .any(|epoch_id| epoch_id == &restored_epoch_id)
        );
        wait_for_connected_topic_peer_count(
            &restarted_c,
            topic,
            1,
            "friend-plus restarted topic reconnect timeout",
        )
        .await;
        restarted_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("restarted c refreshes a after rotate");
        restarted_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("restarted c refreshes b after rotate");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: restarted_ticket_c.clone(),
            })
            .await
            .expect("a refreshes restarted c after rotate");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: restarted_ticket_c.clone(),
            })
            .await
            .expect("b refreshes restarted c after rotate");
        let _ = restarted_c
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("resubscribe restarted c private after fresh share");

        let restarted_post_id = restarted_c
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "friend-plus restarted after rotate".into(),
                reply_to: None,
                channel_ref: private_ref.clone(),
                attachments: vec![],
            })
            .await
            .expect("restarted c creates friend-plus rotated post");
        match timeout(runtime_replication_timeout(), async {
            loop {
                let public_timeline = restarted_c
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: TimelineScope::Public,
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("public timeline after rotate");
                assert!(
                    public_timeline
                        .items
                        .iter()
                        .all(|post| post.object_id != restarted_post_id),
                    "friend-plus rotated post leaked into public timeline"
                );
                let private_timeline = restarted_c
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .expect("private timeline after rotate");
                if private_timeline
                    .items
                    .iter()
                    .any(|post| post.object_id == restarted_post_id)
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let restarted_status = restarted_c.get_sync_status().await.expect("status c");
                let joined = restarted_c
                    .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                        topic: topic.into(),
                    })
                    .await
                    .unwrap_or_default();
                let private_timeline = restarted_c
                    .list_timeline(ListTimelineRequest {
                        topic: topic.into(),
                        scope: private_scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .unwrap_or_else(|_| TimelineView {
                        items: vec![],
                        next_cursor: None,
                    });
                panic!(
                    "friend-plus restarted rotated post visibility timeout: restarted={} joined={joined:?} private_items={:?}",
                    format_sync_snapshot(&restarted_status, topic),
                    private_timeline
                        .items
                        .iter()
                        .map(|item| item.object_id.clone())
                        .collect::<Vec<_>>()
                );
            }
        }

        timeout(Duration::from_secs(30), runtime_a.shutdown())
            .await
            .expect("runtime a shutdown timeout");
        timeout(Duration::from_secs(30), runtime_b.shutdown())
            .await
            .expect("runtime b shutdown timeout");
        timeout(Duration::from_secs(30), restarted_c.shutdown())
            .await
            .expect("restarted runtime c shutdown timeout");
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
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe a");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");
        wait_for_seeded_dht_topic_ready(&runtime_a, &runtime_b, topic).await;
        let status_a = runtime_a
            .get_sync_status()
            .await
            .expect("status a after seeds");
        let status_b = runtime_b
            .get_sync_status()
            .await
            .expect("status b after seeds");
        assert!(
            status_a
                .subscribed_topics
                .iter()
                .any(|entry| entry == topic)
        );
        assert!(
            status_b
                .subscribed_topics
                .iter()
                .any(|entry| entry == topic)
        );
        assert!(status_a.topic_diagnostics.iter().any(|entry| {
            entry.topic == topic
                && entry.joined
                && entry.peer_count > 0
                && (!entry.connected_peers.is_empty() || !entry.assist_peer_ids.is_empty())
        }));
        assert!(status_b.topic_diagnostics.iter().any(|entry| {
            entry.topic == topic
                && entry.joined
                && entry.peer_count > 0
                && (!entry.connected_peers.is_empty() || !entry.assist_peer_ids.is_empty())
        }));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn restart_restores_seeded_dht_config_and_endpoint_identity() {
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
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted a");
        let _ = restarted_b
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe restarted b");
        let status_a = restarted_a.get_sync_status().await.expect("sync status a");
        let status_b = restarted_b.get_sync_status().await.expect("sync status b");
        assert_eq!(status_a.discovery.mode, DiscoveryMode::SeededDht);
        assert_eq!(status_b.discovery.mode, DiscoveryMode::SeededDht);
        assert_eq!(status_a.discovery.local_endpoint_id, endpoint_a);
        assert_eq!(status_b.discovery.local_endpoint_id, endpoint_b);
        assert_eq!(
            status_a.discovery.configured_seed_peer_ids,
            vec![endpoint_b]
        );
        assert_eq!(
            status_b.discovery.configured_seed_peer_ids,
            vec![endpoint_a]
        );
        assert!(status_a.subscribed_topics.iter().any(|item| item == topic));
        assert!(status_b.subscribed_topics.iter().any(|item| item == topic));
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
                channel_ref: ChannelRef::Public,
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
                        scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
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
                        scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
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
                        scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
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
                scope: TimelineScope::Public,
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
    async fn blank_blob_media_hash_returns_none_without_panicking() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("blank-blob-media-hash.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");

        let payload = runtime
            .get_blob_media_payload(GetBlobMediaRequest {
                hash: "   ".into(),
                mime: "image/png".into(),
            })
            .await
            .expect("blank hash payload");

        assert!(payload.is_none());
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
                channel_ref: ChannelRef::Public,
                attachments: vec![],
            })
            .await
            .expect("root post");
        let reply_id = runtime
            .create_post(CreatePostRequest {
                topic: topic.into(),
                content: "reply".into(),
                reply_to: Some(root_id.clone()),
                channel_ref: ChannelRef::Public,
                attachments: vec![],
            })
            .await
            .expect("reply post");
        runtime.shutdown().await;
        drop(runtime);
        delete_sqlite_artifacts(&db_path);

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
                scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
                attachments: vec![],
            })
            .await
            .expect("create post");
        runtime.shutdown().await;
        drop(runtime);
        delete_sqlite_artifacts(&db_path);

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
                scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
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
        delete_sqlite_artifacts(&db_path);

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
                scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
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
        delete_sqlite_artifacts(&db_path);

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
                scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
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
        delete_sqlite_artifacts(&db_path);

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
                scope: TimelineScope::Public,
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
                channel_ref: ChannelRef::Public,
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
        delete_sqlite_artifacts(&db_path);

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
                scope: TimelineScope::Public,
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

    #[tokio::test]
    async fn community_node_status_does_not_require_restart_when_connectivity_is_active() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("community-status.db");
        let test_timeout = Duration::from_secs(15);
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");
        let base_url = "https://community.example.com".to_string();
        let connectivity_url = "http://127.0.0.1:9".to_string();
        let resolved_urls = CommunityNodeResolvedUrls::new(
            base_url.clone(),
            vec![connectivity_url.clone()],
            Vec::new(),
        )
        .expect("resolved urls");
        let node = CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            resolved_urls: Some(resolved_urls.clone()),
        };
        persist_community_node_token(
            &db_path,
            IdentityStorageMode::FileOnly,
            base_url.as_str(),
            &StoredCommunityNodeToken {
                access_token: "fake-token".to_string(),
                expires_at: Utc::now().timestamp() + 3600,
            },
        )
        .expect("persist community-node token");
        *runtime.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![node.clone()],
        };
        *runtime.active_connectivity_urls.lock().await = vec![connectivity_url.clone()];

        let status = timeout(
            test_timeout,
            runtime.community_node_status(
                node,
                Some(CommunityNodeConsentStatus {
                    all_required_accepted: true,
                    items: vec![kukuri_cn_core::CommunityNodeConsentItem {
                        policy_slug: "community-basic".to_string(),
                        policy_version: 1,
                        title: "Community Basic".to_string(),
                        required: true,
                        accepted_at: Some(Utc::now().timestamp()),
                    }],
                }),
                None,
            ),
        )
        .await
        .expect("community-node status timeout")
        .expect("community-node status");
        assert!(status.auth_state.authenticated);
        assert!(
            status
                .consent_state
                .as_ref()
                .expect("consent state")
                .all_required_accepted
        );
        assert_eq!(
            status
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .connectivity_urls,
            vec![connectivity_url]
        );
        assert!(!status.restart_required);

        timeout(test_timeout, runtime.shutdown())
            .await
            .expect("runtime shutdown timeout");
    }

    #[tokio::test]
    async fn community_node_connectivity_assist_preserves_manual_ticket_peers() {
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("assist-a.db");
        let db_b = dir.path().join("assist-b.db");
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

        let ticket_b = runtime_b
            .local_peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_b })
            .await
            .expect("import b");

        let manual_ticket_peer_ids = runtime_a
            .get_sync_status()
            .await
            .expect("sync status before assist")
            .discovery
            .manual_ticket_peer_ids;
        assert!(!manual_ticket_peer_ids.is_empty());

        let seed_peer = CommunityNodeSeedPeer::new(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            None,
        )
        .expect("seed peer");
        *runtime_a.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: "https://community.example.com".into(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://community.example.com",
                        Vec::new(),
                        vec![seed_peer],
                    )
                    .expect("resolved urls"),
                ),
            }],
        };

        runtime_a
            .apply_runtime_connectivity_assist()
            .await
            .expect("apply runtime connectivity assist");

        assert_eq!(
            runtime_a
                .get_sync_status()
                .await
                .expect("sync status after assist")
                .discovery
                .manual_ticket_peer_ids,
            manual_ticket_peer_ids
        );

        runtime_a.shutdown().await;
        runtime_b.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn community_node_connectivity_assist_syncs_public_timeline_without_manual_tickets() {
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("community-relay-a.db");
        let db_b = dir.path().join("community-relay-b.db");
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
        let addr_hint_a = ticket_a
            .split_once('@')
            .map(|(_, addr)| addr.to_string())
            .expect("addr hint a");
        let addr_hint_b = ticket_b
            .split_once('@')
            .map(|(_, addr)| addr.to_string())
            .expect("addr hint b");
        let base_url = "https://community.example.com";

        *runtime_a.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: base_url.to_string(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        base_url,
                        vec![relay_url.to_string()],
                        vec![
                            CommunityNodeSeedPeer::new(
                                endpoint_b.as_str(),
                                Some(addr_hint_b.clone()),
                            )
                            .expect("seed peer b"),
                        ],
                    )
                    .expect("resolved urls a"),
                ),
            }],
        };
        *runtime_b.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: base_url.to_string(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        base_url,
                        vec![relay_url.to_string()],
                        vec![
                            CommunityNodeSeedPeer::new(
                                endpoint_a.as_str(),
                                Some(addr_hint_a.clone()),
                            )
                            .expect("seed peer a"),
                        ],
                    )
                    .expect("resolved urls b"),
                ),
            }],
        };

        timeout(
            Duration::from_secs(15),
            runtime_a.apply_runtime_connectivity_assist(),
        )
        .await
        .expect("apply assist a timeout")
        .expect("apply assist a");
        timeout(
            Duration::from_secs(15),
            runtime_a.apply_effective_seed_peers(),
        )
        .await
        .expect("apply seed peers a timeout")
        .expect("apply seed peers a");
        timeout(
            Duration::from_secs(15),
            runtime_b.apply_runtime_connectivity_assist(),
        )
        .await
        .expect("apply assist b timeout")
        .expect("apply assist b");
        timeout(
            Duration::from_secs(15),
            runtime_b.apply_effective_seed_peers(),
        )
        .await
        .expect("apply seed peers b timeout")
        .expect("apply seed peers b");

        let topic = "kukuri:topic:community-node-relay-assist";
        let scope = TimelineScope::Public;
        let _ = timeout(
            Duration::from_secs(15),
            runtime_a.list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: scope.clone(),
                cursor: None,
                limit: Some(20),
            }),
        )
        .await
        .expect("subscribe a timeout")
        .expect("subscribe a");
        let _ = timeout(
            Duration::from_secs(15),
            runtime_b.list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: scope.clone(),
                cursor: None,
                limit: Some(20),
            }),
        )
        .await
        .expect("subscribe b timeout")
        .expect("subscribe b");

        wait_for_connected_topic_peer_count(
            &runtime_a,
            topic,
            1,
            "community-node assist topic readiness timeout a",
        )
        .await;
        wait_for_connected_topic_peer_count(
            &runtime_b,
            topic,
            1,
            "community-node assist topic readiness timeout b",
        )
        .await;

        let _object_id = replicate_public_post_with_retry(
            &runtime_b,
            &runtime_a,
            topic,
            "community relay hello",
            "community-node assist post sync timeout",
        )
        .await;

        timeout(runtime_shutdown_timeout(), runtime_a.shutdown())
            .await
            .expect("runtime a shutdown timeout");
        timeout(runtime_shutdown_timeout(), runtime_b.shutdown())
            .await
            .expect("runtime b shutdown timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn community_node_connectivity_assist_syncs_public_timeline_with_shared_identity() {
        let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
            .await
            .expect("relay server");
        let dir = tempdir().expect("tempdir");
        let db_a = dir.path().join("community-relay-shared-a.db");
        let db_b = dir.path().join("community-relay-shared-b.db");
        let shared_keys = KukuriKeys::generate();
        let shared_secret = shared_keys.export_secret_hex();
        fs::write(
            db_a.with_extension("identity-key"),
            shared_secret.as_bytes(),
        )
        .expect("persist shared identity key a");
        fs::write(db_a.with_extension("identity-store"), b"file")
            .expect("persist shared identity backend a");
        fs::write(
            db_b.with_extension("identity-key"),
            shared_secret.as_bytes(),
        )
        .expect("persist shared identity key b");
        fs::write(db_b.with_extension("identity-store"), b"file")
            .expect("persist shared identity backend b");

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

        let status_a = runtime_a.get_sync_status().await.expect("status a");
        let status_b = runtime_b.get_sync_status().await.expect("status b");
        assert_eq!(status_a.local_author_pubkey, status_b.local_author_pubkey);

        let endpoint_a = status_a.discovery.local_endpoint_id;
        let endpoint_b = status_b.discovery.local_endpoint_id;
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
        let addr_hint_a = ticket_a
            .split_once('@')
            .map(|(_, addr)| addr.to_string())
            .expect("addr hint a");
        let addr_hint_b = ticket_b
            .split_once('@')
            .map(|(_, addr)| addr.to_string())
            .expect("addr hint b");
        let base_url = "https://community.example.com";

        *runtime_a.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: base_url.to_string(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        base_url,
                        vec![relay_url.to_string()],
                        vec![
                            CommunityNodeSeedPeer::new(
                                endpoint_b.as_str(),
                                Some(addr_hint_b.clone()),
                            )
                            .expect("seed peer b"),
                        ],
                    )
                    .expect("resolved urls a"),
                ),
            }],
        };
        *runtime_b.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: base_url.to_string(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        base_url,
                        vec![relay_url.to_string()],
                        vec![
                            CommunityNodeSeedPeer::new(
                                endpoint_a.as_str(),
                                Some(addr_hint_a.clone()),
                            )
                            .expect("seed peer a"),
                        ],
                    )
                    .expect("resolved urls b"),
                ),
            }],
        };

        timeout(
            Duration::from_secs(15),
            runtime_a.apply_runtime_connectivity_assist(),
        )
        .await
        .expect("apply assist a timeout")
        .expect("apply assist a");
        timeout(
            Duration::from_secs(15),
            runtime_a.apply_effective_seed_peers(),
        )
        .await
        .expect("apply seed peers a timeout")
        .expect("apply seed peers a");
        timeout(
            Duration::from_secs(15),
            runtime_b.apply_runtime_connectivity_assist(),
        )
        .await
        .expect("apply assist b timeout")
        .expect("apply assist b");
        timeout(
            Duration::from_secs(15),
            runtime_b.apply_effective_seed_peers(),
        )
        .await
        .expect("apply seed peers b timeout")
        .expect("apply seed peers b");

        let topic = "kukuri:topic:community-node-relay-assist-shared";
        let scope = TimelineScope::Public;
        let _ = timeout(
            Duration::from_secs(15),
            runtime_a.list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: scope.clone(),
                cursor: None,
                limit: Some(20),
            }),
        )
        .await
        .expect("subscribe a timeout")
        .expect("subscribe a");
        let _ = timeout(
            Duration::from_secs(15),
            runtime_b.list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: scope.clone(),
                cursor: None,
                limit: Some(20),
            }),
        )
        .await
        .expect("subscribe b timeout")
        .expect("subscribe b");

        wait_for_connected_topic_peer_count(
            &runtime_a,
            topic,
            1,
            "community-node assist shared topic readiness timeout a",
        )
        .await;
        wait_for_connected_topic_peer_count(
            &runtime_b,
            topic,
            1,
            "community-node assist shared topic readiness timeout b",
        )
        .await;

        let _object_id = replicate_public_post_with_retry(
            &runtime_a,
            &runtime_b,
            topic,
            "community relay shared hello",
            "community-node assist shared identity post sync timeout",
        )
        .await;

        timeout(runtime_shutdown_timeout(), runtime_a.shutdown())
            .await
            .expect("runtime a shutdown timeout");
        timeout(runtime_shutdown_timeout(), runtime_b.shutdown())
            .await
            .expect("runtime b shutdown timeout");
    }

    #[tokio::test]
    async fn community_node_status_refresh_updates_bootstrap_seed_peers() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("community-heartbeat-refresh.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
        let state = Arc::new(MockCommunityNodeState {
            base_url: base_url.clone(),
            seed_peers: Arc::new(Mutex::new(vec![
                CommunityNodeSeedPeer::new(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                    None,
                )
                .expect("seed peer"),
            ])),
            heartbeat_hits: Arc::new(AtomicUsize::new(0)),
            bootstrap_hits: Arc::new(AtomicUsize::new(0)),
        });
        let app = Router::new()
            .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
            .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
            .with_state(state.clone());
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        persist_community_node_token(
            &db_path,
            IdentityStorageMode::FileOnly,
            base_url.as_str(),
            &StoredCommunityNodeToken {
                access_token: "fake-token".to_string(),
                expires_at: Utc::now().timestamp() + 3600,
            },
        )
        .expect("persist community-node token");
        *runtime.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: base_url.clone(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                        .expect("resolved urls"),
                ),
            }],
        };

        let statuses = runtime
            .get_community_node_statuses()
            .await
            .expect("community node statuses");
        assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
        assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
        assert_eq!(statuses.len(), 1);
        assert_eq!(
            statuses[0]
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .seed_peers,
            state.seed_peers.lock().await.clone()
        );
        assert_eq!(
            runtime.community_node_config.lock().await.nodes[0]
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .seed_peers,
            state.seed_peers.lock().await.clone()
        );

        runtime.shutdown().await;
        server.abort();
    }

    #[tokio::test]
    async fn community_node_status_retries_bootstrap_metadata_when_seed_peers_are_empty() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("community-metadata-retry.db");
        let runtime = DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
        let seed_peer = CommunityNodeSeedPeer::new(
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
            None,
        )
        .expect("seed peer");
        let state = Arc::new(MockCommunityNodeState {
            base_url: base_url.clone(),
            seed_peers: Arc::new(Mutex::new(Vec::new())),
            heartbeat_hits: Arc::new(AtomicUsize::new(0)),
            bootstrap_hits: Arc::new(AtomicUsize::new(0)),
        });
        let app = Router::new()
            .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
            .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
            .with_state(state.clone());
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        persist_community_node_token(
            &db_path,
            IdentityStorageMode::FileOnly,
            base_url.as_str(),
            &StoredCommunityNodeToken {
                access_token: "fake-token".to_string(),
                expires_at: Utc::now().timestamp() + 3600,
            },
        )
        .expect("persist community-node token");
        *runtime.community_node_config.lock().await = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: base_url.clone(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                        .expect("resolved urls"),
                ),
            }],
        };

        let initial_statuses = runtime
            .get_community_node_statuses()
            .await
            .expect("initial community node statuses");
        assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
        assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
        assert_eq!(
            initial_statuses[0]
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .seed_peers,
            Vec::<CommunityNodeSeedPeer>::new()
        );
        assert!(
            runtime
                .community_node_metadata_refresh_deadlines
                .lock()
                .await
                .contains_key(base_url.as_str()),
            "empty bootstrap metadata should schedule a retry"
        );

        *state.seed_peers.lock().await = vec![seed_peer.clone()];
        runtime
            .community_node_metadata_refresh_deadlines
            .lock()
            .await
            .insert(base_url.clone(), Utc::now().timestamp() - 1);

        let refreshed_statuses = runtime
            .get_community_node_statuses()
            .await
            .expect("refreshed community node statuses");
        assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
        assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 2);
        assert_eq!(
            refreshed_statuses[0]
                .resolved_urls
                .as_ref()
                .expect("resolved urls")
                .seed_peers,
            vec![seed_peer]
        );

        runtime.shutdown().await;
        server.abort();
    }
}
