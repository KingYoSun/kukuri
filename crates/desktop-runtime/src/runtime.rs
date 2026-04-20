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
    CustomReactionAssetView, DirectMessageConversationView, DirectMessageStatusView,
    DirectMessageTimelineView, DirectMessageTopicStatusView, GameRoomView,
    JoinedPrivateChannelView, LiveSessionView, NotificationStatusView, NotificationView,
    PrivateChannelCapability, ProfileInput, ReactionStateView, RecentReactionView, SyncStatus,
    TimelineView, UpdateGameRoomInput,
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
    CommunityNodeNodeConfig, CommunityNodeNodeStatus, CommunityNodeSessionPhase,
    CommunityNodeTargetRequest, SetCommunityNodeConfigRequest, community_node_seed_peers,
    default_preview_community_node_config, effective_seed_peer_apply_state,
    load_community_node_config_from_file, load_community_node_token,
    normalize_community_node_config, relay_config_from_community_node_config,
    runtime_connectivity_assist_state, save_community_node_config,
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

pub(crate) const PRIVATE_CHANNEL_CAPABILITIES_PURPOSE: &str = "private-channel-capabilities";
pub(crate) const PRIVATE_CHANNEL_CAPABILITIES_KEY: &str = "registry";

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
    pub(crate) community_node_session_guard: Arc<Mutex<()>>,
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
            community_node_session_guard: Arc::new(Mutex::new(())),
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

    pub async fn mute_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service.mute_author(request.pubkey.as_str()).await
    }

    pub async fn unmute_author(&self, request: AuthorRequest) -> Result<AuthorSocialView> {
        self.app_service
            .unmute_author(request.pubkey.as_str())
            .await
    }

    pub async fn list_social_connections(
        &self,
        request: ListSocialConnectionsRequest,
    ) -> Result<Vec<AuthorSocialView>> {
        self.app_service.list_social_connections(request.kind).await
    }

    pub async fn list_notifications(&self) -> Result<Vec<NotificationView>> {
        self.app_service.list_notifications().await
    }

    pub async fn mark_notification_read(
        &self,
        request: NotificationIdRequest,
    ) -> Result<NotificationStatusView> {
        self.app_service
            .mark_notification_read(request.notification_id.as_str())
            .await
    }

    pub async fn mark_all_notifications_read(&self) -> Result<NotificationStatusView> {
        self.app_service.mark_all_notifications_read().await
    }

    pub async fn get_notification_status(&self) -> Result<NotificationStatusView> {
        self.app_service.get_notification_status().await
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

    pub async fn get_direct_message_topic_status(
        &self,
        request: DirectMessageRequest,
    ) -> Result<Option<DirectMessageTopicStatusView>> {
        self.app_service
            .get_direct_message_topic_status(request.pubkey.as_str())
            .await
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let community_node_config = self.community_node_config.lock().await.clone();
        for node in community_node_config.nodes {
            if let Err(error) = self
                .refresh_community_node_registration_if_due(node.base_url.as_str())
                .await
            {
                tracing::warn!(
                    base_url = %node.base_url,
                    error = %error,
                    "failed to refresh community-node registration while loading sync status"
                );
            }
        }
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

    pub async fn preview_channel_access_token(
        &self,
        request: PreviewChannelAccessTokenRequest,
    ) -> Result<ChannelAccessTokenPreview> {
        self.app_service
            .preview_channel_access_token(request.token.as_str())
            .await
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
            let _ = self
                .refresh_community_node_registration_if_due(base_url.as_str())
                .await;
            let current_node = self
                .community_node_config
                .lock()
                .await
                .nodes
                .iter()
                .find(|candidate| candidate.base_url == base_url)
                .cloned()
                .unwrap_or(node);
            statuses.push(self.community_node_status(current_node, None, None).await?);
        }
        Ok(statuses)
    }

    pub async fn set_community_node_config(
        &self,
        request: SetCommunityNodeConfigRequest,
    ) -> Result<CommunityNodeConfig> {
        let current_config = self.community_node_config.lock().await.clone();
        let nodes = request
            .nodes
            .into_iter()
            .map(|base_url| -> Result<CommunityNodeNodeConfig> {
                let normalized_base_url = normalize_http_url(base_url.base_url.as_str())?;
                let resolved_urls = current_config
                    .nodes
                    .iter()
                    .find(|node| node.base_url == normalized_base_url)
                    .and_then(|node| node.resolved_urls.clone());
                Ok(CommunityNodeNodeConfig {
                    base_url: normalized_base_url,
                    auto_approve: base_url.auto_approve,
                    resolved_urls,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let next_config = normalize_community_node_config(CommunityNodeConfig { nodes })?;
        save_community_node_config(&self.db_path, &next_config)?;
        *self.community_node_config.lock().await = next_config.clone();
        self.community_node_heartbeat_deadlines.lock().await.clear();
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_phases.lock().await.clear();
        self.community_node_ready_refresh_pending
            .lock()
            .await
            .clear();
        self.community_node_last_errors.lock().await.clear();
        self.community_node_cached_consents.lock().await.clear();
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
        save_community_node_config(&self.db_path, &CommunityNodeConfig::default())?;
        *self.community_node_config.lock().await = CommunityNodeConfig::default();
        self.community_node_heartbeat_deadlines.lock().await.clear();
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_phases.lock().await.clear();
        self.community_node_ready_refresh_pending
            .lock()
            .await
            .clear();
        self.community_node_last_errors.lock().await.clear();
        self.community_node_cached_consents.lock().await.clear();
        self.apply_runtime_connectivity_assist().await?;
        self.apply_effective_seed_peers().await?;
        Ok(())
    }

    pub async fn authenticate_community_node(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Authenticating,
        )
        .await;
        let mut token = self
            .request_community_node_authentication_token(base_url.as_str())
            .await?;
        let mut consent_state = self
            .fetch_community_node_consent_status_with_retry(base_url.as_str(), &mut token, false)
            .await?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(consent_state.clone()))
            .await;
        if !consent_state.all_required_accepted && node.auto_approve {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Accepting,
            )
            .await;
            consent_state = self
                .accept_community_node_consents_with_retry(base_url.as_str(), &mut token, &[])
                .await?;
            self.set_community_node_cached_consent(base_url.as_str(), Some(consent_state.clone()))
                .await;
        }
        if consent_state.all_required_accepted {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Refreshing,
            )
            .await;
            self.refresh_community_node_registration_with_token_if_due(
                base_url.as_str(),
                &mut token,
                node.auto_approve,
            )
            .await?;
            self.clear_community_node_retry_state(base_url.as_str())
                .await;
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Ready,
            )
            .await;
            let refreshed = self.require_community_node(base_url.as_str()).await?;
            return self
                .community_node_status(refreshed, Some(consent_state), None)
                .await;
        }
        self.clear_community_node_retry_state(base_url.as_str())
            .await;
        self.set_community_node_session_phase(base_url.as_str(), CommunityNodeSessionPhase::Idle)
            .await;
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
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_last_errors
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_cached_consents
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_session_phases
            .lock()
            .await
            .insert(base_url.clone(), CommunityNodeSessionPhase::Idle);
        self.community_node_ready_refresh_pending
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
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        let status = self
            .fetch_community_node_consent_status_with_retry(base_url.as_str(), &mut token, true)
            .await
            .context("failed to fetch community node consent status")?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(status.clone()))
            .await;
        self.community_node_status(node, Some(status), None).await
    }

    pub async fn accept_community_node_consents(
        &self,
        request: AcceptCommunityNodeConsentsRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Accepting,
        )
        .await;
        let status = self
            .accept_community_node_consents_with_retry(
                base_url.as_str(),
                &mut token,
                &request.policy_slugs,
            )
            .await
            .context("failed to accept community node consents")?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(status.clone()))
            .await;
        if status.all_required_accepted {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Refreshing,
            )
            .await;
            self.refresh_community_node_registration_with_token_if_due(
                base_url.as_str(),
                &mut token,
                node.auto_approve,
            )
            .await?;
            self.clear_community_node_retry_state(base_url.as_str())
                .await;
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Ready,
            )
            .await;
            let refreshed = self.require_community_node(base_url.as_str()).await?;
            return self
                .community_node_status(refreshed, Some(status), None)
                .await;
        }
        self.set_community_node_session_phase(base_url.as_str(), CommunityNodeSessionPhase::Idle)
            .await;
        self.community_node_status(node, Some(status), None).await
    }

    pub async fn refresh_community_node_metadata(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        self.ensure_community_node_session(base_url.as_str())
            .await?;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Refreshing,
        )
        .await;
        let refreshed = self
            .sync_community_node_bootstrap_metadata_with_retry(
                base_url.as_str(),
                &mut token,
                node.auto_approve,
            )
            .await?;
        self.clear_community_node_retry_state(base_url.as_str())
            .await;
        self.set_community_node_session_phase(base_url.as_str(), CommunityNodeSessionPhase::Ready)
            .await;
        self.community_node_status(refreshed, None, None).await
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
