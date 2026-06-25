use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use kukuri_app_api::{
    AppService, AuthorSocialView, BlobMediaPayload, BookmarkedCustomReactionView,
    BookmarkedPostView, ChannelAccessTokenExport, ChannelAccessTokenPreview,
    CreateCustomReactionAssetInput, CreateGameRoomInput, CreateLiveSessionInput,
    CreateMetaverseRoomInput, CustomReactionAssetView, DirectMessageConversationView,
    DirectMessageStatusView, DirectMessageTimelineView, DirectMessageTopicStatusView, GameRoomView,
    ImportMetaverseRoomAssetInput, JoinedPrivateChannelView, LiveSessionView,
    MetaverseAssetRefView, MetaverseRoomEventView, NotificationStatusView, NotificationView,
    PrivateChannelCapability, ProfileInput, PublishMetaverseRoomEventInput, ReactionStateView,
    RecentReactionView, SyncStatus, TimelineView, UpdateGameRoomInput, UpdateMetaverseRoomInput,
};
use kukuri_cn_core::{CommunityNodeConsentStatus, normalize_http_url};
use kukuri_core::{
    BlobHash, CreatePrivateChannelInput, CustomReactionAssetSnapshotV1, FriendOnlyGrantPreview,
    FriendPlusSharePreview, KukuriKeys, PrivateChannelInvitePreview, Profile, TopicId,
};
use kukuri_docs_sync::{DocQuery, DocsSync};
use kukuri_store::SqliteStore;
use kukuri_transport::{DhtDiscoveryOptions, DiscoveryMode, TransportNetworkConfig};
use tokio::sync::Mutex;

use crate::attachments::{
    normalize_custom_reaction_upload, pending_attachment_from_request, reaction_key_from_request,
};
use crate::community_node::{
    AcceptCommunityNodeConsentsRequest, COMMUNITY_NODE_TOKEN_PURPOSE, CommunityNodeConfig,
    CommunityNodeManifestFetch, CommunityNodeNodeConfig, CommunityNodeNodeStatus,
    CommunityNodeReconnectState, CommunityNodeSessionPhase, CommunityNodeTargetRequest,
    SetCommunityNodeConfigRequest, SubmitCommunityNodeReportRequest,
    SubmitCommunityNodeReportResult, community_node_consent_has_pending_update,
    community_node_seed_peers, default_preview_community_node_config,
    effective_seed_peer_apply_state, load_community_node_config_from_file,
    load_community_node_token, normalize_community_node_config,
    relay_config_from_community_node_config, runtime_connectivity_assist_state,
    save_community_node_config,
};
use crate::discovery::{
    DiscoveryConfig, SetDiscoverySeedsRequest, parse_seed_entries,
    resolve_discovery_config_from_env, save_discovery_config,
};
use crate::identity::{
    IdentityStorageMode, delete_optional_secret, load_optional_secret, load_or_create_keys,
    persist_optional_secret,
};
use crate::requests::*;
use crate::stack::SharedIrohStack;

mod community_node_api;
mod content_profile_api;
mod notifications_messages_api;
mod private_channels_game_api;
mod sync_live_api;

pub(crate) const PRIVATE_CHANNEL_CAPABILITIES_PURPOSE: &str = "private-channel-capabilities";
pub(crate) const PRIVATE_CHANNEL_CAPABILITIES_KEY: &str = "registry";
pub(crate) const GOSSIP_SUBSCRIPTION_STATE_PURPOSE: &str = "gossip-subscription-state";
pub(crate) const GOSSIP_SUBSCRIPTION_STATE_KEY: &str = "registry";

pub struct DesktopRuntime {
    pub(crate) app_service: AppService,
    pub(crate) author_keys: Arc<KukuriKeys>,
    pub(crate) db_path: PathBuf,
    pub(crate) identity_mode: IdentityStorageMode,
    pub(crate) store: Arc<SqliteStore>,
    pub(crate) iroh_stack: SharedIrohStack,
    pub(crate) discovery_config: Arc<Mutex<DiscoveryConfig>>,
    pub(crate) community_node_config: Arc<Mutex<CommunityNodeConfig>>,
    pub(crate) community_node_heartbeat_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    pub(crate) community_node_metadata_refresh_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    pub(crate) community_node_session_retry_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    pub(crate) community_node_session_phases:
        Arc<Mutex<HashMap<String, CommunityNodeSessionPhase>>>,
    pub(crate) community_node_ready_refresh_pending: Arc<Mutex<HashMap<String, bool>>>,
    pub(crate) community_node_last_errors: Arc<Mutex<HashMap<String, String>>>,
    pub(crate) community_node_cached_consents:
        Arc<Mutex<HashMap<String, CommunityNodeConsentStatus>>>,
    pub(crate) community_node_rendezvous_seed_peers: Arc<Mutex<Vec<kukuri_transport::SeedPeer>>>,
    pub(crate) community_node_session_guard: Arc<Mutex<()>>,
    pub(crate) community_node_reconnect_state: Arc<Mutex<CommunityNodeReconnectState>>,
    pub(crate) community_node_reconnect_guard: Arc<Mutex<()>>,
    pub(crate) active_connectivity_urls: Arc<Mutex<Vec<String>>>,
    pub(crate) last_runtime_connectivity_assist_state:
        Arc<Mutex<Option<crate::community_node::RuntimeConnectivityAssistState>>>,
    pub(crate) last_effective_seed_peer_apply_state:
        Arc<Mutex<Option<crate::community_node::EffectiveSeedPeerApplyState>>>,
    pub(crate) runtime_connectivity_apply_version: Arc<AtomicU64>,
    pub(crate) effective_seed_peer_apply_version: Arc<AtomicU64>,
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

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct GossipSubscriptionState {
    #[serde(default)]
    disabled_topics: Vec<String>,
    #[serde(default)]
    disabled_channels: Vec<String>,
}

fn load_gossip_subscription_state(
    db_path: &Path,
    mode: IdentityStorageMode,
) -> Result<GossipSubscriptionState> {
    let Some(raw) = load_optional_secret(
        db_path,
        mode,
        GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
        GOSSIP_SUBSCRIPTION_STATE_KEY,
    )?
    else {
        return Ok(GossipSubscriptionState::default());
    };
    serde_json::from_str(&raw).context("failed to decode gossip subscription state")
}

fn persist_gossip_subscription_state(
    db_path: &Path,
    mode: IdentityStorageMode,
    state: &GossipSubscriptionState,
) -> Result<()> {
    let encoded =
        serde_json::to_string(state).context("failed to encode gossip subscription state")?;
    persist_optional_secret(
        db_path,
        mode,
        GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
        GOSSIP_SUBSCRIPTION_STATE_KEY,
        encoded.as_str(),
    )
}

impl DesktopRuntime {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_config_and_identity_and_discovery(
            db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::from_env(),
            DiscoveryConfig::static_peer_default(),
            DhtDiscoveryOptions::disabled(),
            false,
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
            false,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn new_with_config_and_identity(
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
            false,
        )
        .await
    }

    pub(crate) async fn new_with_config_and_identity_and_discovery(
        db_path: impl AsRef<Path>,
        network_config: TransportNetworkConfig,
        identity_mode: IdentityStorageMode,
        discovery_config: DiscoveryConfig,
        dht_options: DhtDiscoveryOptions,
        preload_preview_community_node: bool,
    ) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let community_node_config = match load_community_node_config_from_file(&db_path)? {
            Some(config) => config,
            None if preload_preview_community_node => {
                let config = default_preview_community_node_config();
                save_community_node_config(&db_path, &config)?;
                config
            }
            None => CommunityNodeConfig::default(),
        };
        let relay_config = relay_config_from_community_node_config(&community_node_config);
        let community_node_seed_peers =
            community_node_seed_peers(&community_node_config).collect::<Vec<_>>();
        let initial_runtime_connectivity_state =
            runtime_connectivity_assist_state(&discovery_config, &community_node_config);
        let initial_effective_seed_peer_state =
            effective_seed_peer_apply_state(&discovery_config, &community_node_config);
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
        let gossip_subscription_state = load_gossip_subscription_state(&db_path, identity_mode)?;
        app_service
            .restore_gossip_disabled_state(
                gossip_subscription_state.disabled_topics,
                gossip_subscription_state.disabled_channels,
            )
            .await;
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
            community_node_session_retry_deadlines: Arc::new(Mutex::new(HashMap::new())),
            community_node_session_phases: Arc::new(Mutex::new(HashMap::new())),
            community_node_ready_refresh_pending: Arc::new(Mutex::new(HashMap::new())),
            community_node_last_errors: Arc::new(Mutex::new(HashMap::new())),
            community_node_cached_consents: Arc::new(Mutex::new(HashMap::new())),
            community_node_rendezvous_seed_peers: Arc::new(Mutex::new(Vec::new())),
            community_node_session_guard: Arc::new(Mutex::new(())),
            community_node_reconnect_state: Arc::new(Mutex::new(
                CommunityNodeReconnectState::default(),
            )),
            community_node_reconnect_guard: Arc::new(Mutex::new(())),
            active_connectivity_urls: Arc::new(Mutex::new(relay_config.iroh_relay_urls.clone())),
            last_runtime_connectivity_assist_state: Arc::new(Mutex::new(Some(
                initial_runtime_connectivity_state,
            ))),
            last_effective_seed_peer_apply_state: Arc::new(Mutex::new(Some(
                initial_effective_seed_peer_state,
            ))),
            runtime_connectivity_apply_version: Arc::new(AtomicU64::new(0)),
            effective_seed_peer_apply_version: Arc::new(AtomicU64::new(0)),
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
            true,
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

    pub(crate) async fn persist_gossip_subscription_state_from_app(&self) -> Result<()> {
        persist_gossip_subscription_state(
            &self.db_path,
            self.identity_mode,
            &GossipSubscriptionState {
                disabled_topics: self.app_service.list_gossip_disabled_topics().await,
                disabled_channels: self.app_service.list_gossip_disabled_channels().await,
            },
        )
    }
}
