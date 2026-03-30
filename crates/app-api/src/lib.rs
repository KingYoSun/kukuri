use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use futures_util::StreamExt;
use kukuri_blob_service::{BlobService, BlobStatus, MemoryBlobService, StoredBlob};
use kukuri_core::{
    AssetRole, AuthorProfileDocV1, AuthorProfilePostDocV1, AuthorProfileRepostDocV1,
    CanonicalPostHeader, ChannelAudienceKind, ChannelId, ChannelRef, ChannelSharingState,
    CreatePrivateChannelInput, CustomReactionAssetDocV1, CustomReactionAssetSnapshotV1,
    DirectMessageAttachmentKind, DirectMessageAttachmentManifestV1,
    DirectMessageEncryptedAttachmentV1, DirectMessageEncryptedBlobRefV1, DirectMessageFrameV1,
    DirectMessagePayloadV1, EnvelopeId, FollowEdge, FollowEdgeDocV1, FollowEdgeStatus,
    FriendOnlyGrantPreview, FriendPlusSharePreview, GAME_MANIFEST_MIME, GameParticipant,
    GameRoomManifestBlobV1, GameRoomStateDocV1, GameRoomStatus, GameScoreEntry, GossipHint,
    HintObjectRef, KukuriEnvelope, KukuriKeys, KukuriMediaManifestV1,
    KukuriProfileEnvelopeContentV1, KukuriProfilePostEnvelopeContentV1,
    KukuriProfileRepostEnvelopeContentV1, LIVE_MANIFEST_MIME, LiveSessionManifestBlobV1,
    LiveSessionStateDocV1, LiveSessionStatus, ManifestBlobRef, MediaManifestItem, ObjectStatus,
    ObjectVisibility, PayloadRef, PrivateChannelEpochHandoffGrantDocV1,
    PrivateChannelEpochHandoffGrantPayloadV1, PrivateChannelInvitePreview,
    PrivateChannelInviteTokenParams, PrivateChannelJoinMode, PrivateChannelMetadataDocV1,
    PrivateChannelParticipantDocV1, PrivateChannelPolicyDocV1, Profile, ProfilePost, ProfileRepost,
    Pubkey, ReactionDocV1, ReactionKeyKind, ReactionKeyV1, ReplicaId, RepostSourceSnapshotV1,
    TimelineScope, TopicId, author_profile_topic_id, build_custom_reaction_asset_envelope,
    build_direct_message_ack, build_follow_edge_envelope, build_friend_only_grant_token,
    build_friend_plus_share_token, build_game_session_envelope, build_live_session_envelope,
    build_media_manifest_envelope, build_post_envelope_with_payload_in_channel,
    build_private_channel_epoch_handoff_grant_envelope, build_private_channel_invite_token,
    build_private_channel_participant_envelope, build_private_channel_policy_envelope,
    build_profile_envelope, build_profile_post_envelope, build_profile_repost_envelope,
    build_reaction_envelope, build_repost_envelope, decrypt_direct_message_attachment,
    decrypt_direct_message_frame, decrypt_private_channel_epoch_handoff_grant,
    derive_direct_message_topic, deterministic_reaction_id, direct_message_id_for_participants,
    encrypt_direct_message_attachment, encrypt_direct_message_frame,
    encrypt_private_channel_epoch_handoff_grant, generate_keys, parse_custom_reaction_asset,
    parse_follow_edge, parse_friend_only_grant_token, parse_friend_plus_share_token,
    parse_private_channel_epoch_handoff_grant, parse_private_channel_invite_token,
    parse_private_channel_participant, parse_private_channel_policy, parse_profile,
    parse_profile_post, parse_profile_repost, parse_reaction, timeline_sort_key,
};
use kukuri_docs_sync::{
    DocOp, DocQuery, DocsSync, MemoryDocsSync, author_replica_id, private_channel_epoch_replica_id,
    private_channel_hint_topic, private_channel_replica_id, stable_key, topic_replica_id,
};
use kukuri_store::{
    AuthorRelationshipProjectionRow, BlobCacheStatus, BookmarkedCustomReactionRow,
    DirectMessageConversationRow, DirectMessageMessageRow, DirectMessageOutboxRow,
    DirectMessageTombstoneRow, GameRoomProjectionRow, LiveSessionProjectionRow,
    ObjectProjectionRow, Page, ProjectionStore, ReactionProjectionRow, Store, TimelineCursor,
};
use kukuri_transport::{
    ConnectMode, DiscoveryMode, DiscoverySnapshot, HintTransport, PeerSnapshot, SeedPeer,
    TopicPeerSnapshot, Transport,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};

const REPLICA_SYNC_RESTART_RETRY_SECONDS: i64 = 5;
const PUBLIC_CHANNEL_ID: &str = "public";
const DIRECT_MESSAGE_FRAME_MIME: &str = "application/vnd.kukuri.direct-message-frame+json";
const DIRECT_MESSAGE_ATTACHMENT_MIME: &str =
    "application/vnd.kukuri.direct-message-attachment+json";
const DIRECT_MESSAGE_RETRY_INTERVAL_MS: u64 = 2_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostView {
    pub object_id: String,
    pub envelope_id: String,
    pub author_pubkey: String,
    pub author_name: Option<String>,
    pub author_display_name: Option<String>,
    pub following: bool,
    pub followed_by: bool,
    pub mutual: bool,
    pub friend_of_friend: bool,
    pub content: String,
    pub content_status: BlobViewStatus,
    pub attachments: Vec<AttachmentView>,
    pub created_at: i64,
    pub reply_to: Option<String>,
    pub root_id: Option<String>,
    pub object_kind: String,
    pub published_topic_id: Option<String>,
    pub origin_topic_id: Option<String>,
    pub repost_of: Option<RepostSourceView>,
    pub repost_commentary: Option<String>,
    pub is_threadable: bool,
    pub channel_id: Option<String>,
    pub audience_label: String,
    #[serde(default)]
    pub reaction_summary: Vec<ReactionSummaryView>,
    #[serde(default)]
    pub my_reactions: Vec<ReactionKeyView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionKeyView {
    pub reaction_key_kind: String,
    pub normalized_reaction_key: String,
    pub emoji: Option<String>,
    pub custom_asset: Option<CustomReactionAssetView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionSummaryView {
    pub reaction_key_kind: String,
    pub normalized_reaction_key: String,
    pub emoji: Option<String>,
    pub custom_asset: Option<CustomReactionAssetView>,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionStateView {
    pub target_object_id: String,
    pub source_replica_id: String,
    pub reaction_summary: Vec<ReactionSummaryView>,
    pub my_reactions: Vec<ReactionKeyView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentReactionView {
    pub reaction_key_kind: String,
    pub normalized_reaction_key: String,
    pub emoji: Option<String>,
    pub custom_asset: Option<CustomReactionAssetView>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomReactionAssetView {
    pub asset_id: String,
    pub owner_pubkey: String,
    pub blob_hash: String,
    pub search_key: String,
    pub mime: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
}

pub type BookmarkedCustomReactionView = CustomReactionAssetView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateCustomReactionAssetInput {
    pub search_key: String,
    pub mime: String,
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepostSourceView {
    pub source_object_id: String,
    pub source_topic_id: String,
    pub source_author_pubkey: String,
    pub source_author_name: Option<String>,
    pub source_author_display_name: Option<String>,
    pub source_object_kind: String,
    pub content: String,
    pub attachments: Vec<AttachmentView>,
    pub reply_to: Option<String>,
    pub root_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobViewStatus {
    Missing,
    Available,
    Pinned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentView {
    pub hash: String,
    pub mime: String,
    pub bytes: u64,
    pub role: String,
    pub status: BlobViewStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobMediaPayload {
    pub bytes_base64: String,
    pub mime: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileInput {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub picture_upload: Option<PendingAttachment>,
    #[serde(default)]
    pub clear_picture: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileAssetView {
    pub hash: String,
    pub mime: String,
    pub bytes: u64,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorSocialView {
    pub author_pubkey: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub picture_asset: Option<ProfileAssetView>,
    pub updated_at: Option<i64>,
    pub following: bool,
    pub followed_by: bool,
    pub mutual: bool,
    pub friend_of_friend: bool,
    pub friend_of_friend_via_pubkeys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingAttachment {
    pub mime: String,
    pub bytes: Vec<u8>,
    pub role: AssetRole,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageStatusView {
    pub peer_pubkey: String,
    pub dm_id: String,
    pub mutual: bool,
    pub send_enabled: bool,
    pub peer_count: usize,
    pub pending_outbox_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageMessageView {
    pub dm_id: String,
    pub message_id: String,
    pub sender_pubkey: String,
    pub recipient_pubkey: String,
    pub created_at: i64,
    pub text: String,
    pub reply_to_message_id: Option<String>,
    pub attachments: Vec<AttachmentView>,
    pub outgoing: bool,
    pub delivered: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageConversationView {
    pub dm_id: String,
    pub peer_pubkey: String,
    pub peer_name: Option<String>,
    pub peer_display_name: Option<String>,
    pub peer_picture: Option<String>,
    pub peer_picture_asset: Option<ProfileAssetView>,
    pub updated_at: i64,
    pub last_message_at: Option<i64>,
    pub last_message_id: Option<String>,
    pub last_message_preview: Option<String>,
    pub status: DirectMessageStatusView,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionView {
    pub session_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: LiveSessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub viewer_count: usize,
    pub joined_by_me: bool,
    pub channel_id: Option<String>,
    pub audience_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomView {
    pub room_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreView>,
    pub updated_at: i64,
    pub channel_id: Option<String>,
    pub audience_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameScoreView {
    pub participant_id: String,
    pub label: String,
    pub score: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateLiveSessionInput {
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateGameRoomInput {
    pub title: String,
    pub description: String,
    pub participants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateGameRoomInput {
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineView {
    pub items: Vec<PostView>,
    pub next_cursor: Option<TimelineCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageTimelineView {
    pub items: Vec<DirectMessageMessageView>,
    pub next_cursor: Option<TimelineCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProfileTimelineItem {
    Post(ProfilePost),
    Repost(ProfileRepost),
}

impl ProfileTimelineItem {
    fn created_at(&self) -> i64 {
        match self {
            Self::Post(post) => post.created_at,
            Self::Repost(repost) => repost.created_at,
        }
    }

    fn object_id(&self) -> &EnvelopeId {
        match self {
            Self::Post(post) => &post.object_id,
            Self::Repost(repost) => &repost.object_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedRepostSource {
    repost_of: RepostSourceSnapshotV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinedPrivateChannelView {
    pub topic_id: String,
    pub channel_id: String,
    pub label: String,
    pub creator_pubkey: String,
    pub owner_pubkey: String,
    pub joined_via_pubkey: Option<String>,
    pub audience_kind: ChannelAudienceKind,
    pub is_owner: bool,
    pub current_epoch_id: String,
    pub archived_epoch_ids: Vec<String>,
    pub sharing_state: ChannelSharingState,
    pub rotation_required: bool,
    pub participant_count: usize,
    pub stale_participant_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelEpochCapability {
    pub epoch_id: String,
    pub namespace_secret_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelCapability {
    pub topic_id: String,
    pub channel_id: String,
    pub label: String,
    pub creator_pubkey: String,
    #[serde(default)]
    pub owner_pubkey: String,
    #[serde(default)]
    pub joined_via_pubkey: Option<String>,
    #[serde(default)]
    pub audience_kind: ChannelAudienceKind,
    #[serde(default)]
    pub current_epoch_id: String,
    #[serde(default)]
    pub current_epoch_secret_hex: String,
    #[serde(default)]
    pub archived_epochs: Vec<PrivateChannelEpochCapability>,
    #[serde(default)]
    pub rotation_required: bool,
    #[serde(default)]
    pub participant_count: usize,
    #[serde(default)]
    pub stale_participant_count: usize,
    #[serde(default)]
    pub namespace_secret_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelAccessTokenKind {
    Invite,
    Grant,
    Share,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelAccessTokenExport {
    pub kind: ChannelAccessTokenKind,
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelAccessTokenPreview {
    pub kind: ChannelAccessTokenKind,
    pub topic_id: String,
    pub channel_id: String,
    pub channel_label: String,
    pub owner_pubkey: String,
    pub inviter_pubkey: Option<String>,
    pub sponsor_pubkey: Option<String>,
    pub epoch_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub connected: bool,
    pub last_sync_ts: Option<i64>,
    pub peer_count: usize,
    pub pending_events: usize,
    pub status_detail: String,
    pub last_error: Option<String>,
    pub configured_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub topic_diagnostics: Vec<TopicSyncStatus>,
    pub local_author_pubkey: String,
    pub discovery: DiscoveryStatus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryStatus {
    pub mode: DiscoveryMode,
    pub connect_mode: ConnectMode,
    pub env_locked: bool,
    pub configured_seed_peer_ids: Vec<String>,
    pub bootstrap_seed_peer_ids: Vec<String>,
    pub manual_ticket_peer_ids: Vec<String>,
    pub connected_peer_ids: Vec<String>,
    pub assist_peer_ids: Vec<String>,
    pub local_endpoint_id: String,
    pub last_discovery_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSyncStatus {
    pub topic: String,
    pub joined: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub assist_peer_ids: Vec<String>,
    pub configured_peer_ids: Vec<String>,
    pub missing_peer_ids: Vec<String>,
    pub last_received_at: Option<i64>,
    pub status_detail: String,
    pub last_error: Option<String>,
}

pub struct AppService {
    store: Arc<dyn Store>,
    projection_store: Arc<dyn ProjectionStore>,
    transport: Arc<dyn Transport>,
    hint_transport: Arc<dyn HintTransport>,
    docs_sync: Arc<dyn DocsSync>,
    blob_service: Arc<dyn BlobService>,
    keys: Arc<KukuriKeys>,
    subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    private_channel_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    author_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    joined_private_channels: Arc<Mutex<HashMap<String, JoinedPrivateChannelState>>>,
    live_presence_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    last_sync_ts: Arc<Mutex<Option<i64>>>,
    replica_sync_restart_deadlines: Arc<Mutex<HashMap<String, i64>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct JoinedPrivateChannelState {
    topic_id: String,
    channel_id: ChannelId,
    label: String,
    creator_pubkey: String,
    owner_pubkey: String,
    joined_via_pubkey: Option<String>,
    audience_kind: ChannelAudienceKind,
    current_epoch_id: String,
    current_epoch_secret_hex: String,
    archived_epochs: Vec<PrivateChannelEpochCapability>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PrivateChannelDiagnostics {
    sharing_state: ChannelSharingState,
    participant_count: usize,
    stale_participant_count: usize,
    rotation_required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrivateChannelOwnerAction {
    Write,
    Share,
}

impl AppService {
    pub fn new<S, T>(store: Arc<S>, transport: Arc<T>) -> Self
    where
        S: Store + ProjectionStore + 'static,
        T: Transport + HintTransport + 'static,
    {
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        Self::new_with_services(
            store.clone() as Arc<dyn Store>,
            store as Arc<dyn ProjectionStore>,
            transport.clone(),
            transport as Arc<dyn HintTransport>,
            docs_sync,
            blob_service,
            generate_keys(),
        )
    }

    pub fn new_with_services(
        store: Arc<dyn Store>,
        projection_store: Arc<dyn ProjectionStore>,
        transport: Arc<dyn Transport>,
        hint_transport: Arc<dyn HintTransport>,
        docs_sync: Arc<dyn DocsSync>,
        blob_service: Arc<dyn BlobService>,
        keys: KukuriKeys,
    ) -> Self {
        Self {
            store,
            transport,
            projection_store,
            hint_transport,
            docs_sync,
            blob_service,
            keys: Arc::new(keys),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            direct_message_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            private_channel_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            author_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            joined_private_channels: Arc::new(Mutex::new(HashMap::new())),
            live_presence_tasks: Arc::new(Mutex::new(HashMap::new())),
            last_sync_ts: Arc::new(Mutex::new(None)),
            replica_sync_restart_deadlines: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn warm_social_graph(&self) -> Result<()> {
        let local_author = self.current_author_pubkey();
        self.ensure_author_subscription(local_author.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        for edge in self
            .store
            .list_follow_edges_by_subject(local_author.as_str())
            .await?
        {
            if edge.status == FollowEdgeStatus::Active {
                self.ensure_author_subscription(edge.target_pubkey.as_str())
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_my_profile(&self) -> Result<Profile> {
        let local_author = self.current_author_pubkey();
        self.ensure_author_subscription(local_author.as_str())
            .await?;
        Ok(self
            .store
            .get_profile(local_author.as_str())
            .await?
            .unwrap_or(Profile {
                pubkey: Pubkey::from(local_author),
                ..Profile::default()
            }))
    }

    pub async fn set_my_profile(&self, input: ProfileInput) -> Result<Profile> {
        let author_pubkey = Pubkey::from(self.current_author_pubkey());
        let current_profile = self.get_my_profile().await?;
        let picture = if input.clear_picture || input.picture_upload.is_some() {
            normalize_optional_text(input.picture)
        } else {
            normalize_optional_text(input.picture).or(current_profile.picture.clone())
        };
        let picture_asset = if input.clear_picture {
            None
        } else if let Some(upload) = input.picture_upload {
            let stored = self
                .blob_service
                .put_blob(upload.bytes, upload.mime.as_str())
                .await?;
            Some(kukuri_core::AssetRef {
                hash: stored.hash,
                mime: stored.mime,
                bytes: stored.bytes,
                role: AssetRole::ProfileAvatar,
            })
        } else {
            current_profile.picture_asset.clone()
        };
        let envelope = build_profile_envelope(
            self.keys.as_ref(),
            &KukuriProfileEnvelopeContentV1 {
                author_pubkey: author_pubkey.clone(),
                name: normalize_optional_text(input.name),
                display_name: normalize_optional_text(input.display_name),
                about: normalize_optional_text(input.about),
                picture,
                picture_asset,
            },
        )?;
        let profile = parse_profile(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse profile envelope"))?;
        self.store.put_envelope(envelope.clone()).await?;
        self.projection_store
            .upsert_profile_cache(profile.clone())
            .await?;
        persist_profile_doc(self.docs_sync.as_ref(), &profile, &envelope).await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(profile)
    }

    pub async fn follow_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let target_pubkey = Pubkey::from(normalize_author_pubkey(pubkey)?);
        let envelope = build_follow_edge_envelope(
            self.keys.as_ref(),
            &target_pubkey,
            FollowEdgeStatus::Active,
        )?;
        let edge = parse_follow_edge(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse follow edge"))?;
        self.store.put_envelope(envelope.clone()).await?;
        persist_follow_edge_doc(self.docs_sync.as_ref(), &edge, &envelope).await?;
        self.ensure_author_subscription(target_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        self.build_author_social_view(target_pubkey.as_str()).await
    }

    pub async fn unfollow_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let target_pubkey = Pubkey::from(normalize_author_pubkey(pubkey)?);
        let envelope = build_follow_edge_envelope(
            self.keys.as_ref(),
            &target_pubkey,
            FollowEdgeStatus::Revoked,
        )?;
        let edge = parse_follow_edge(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse follow edge"))?;
        self.store.put_envelope(envelope.clone()).await?;
        persist_follow_edge_doc(self.docs_sync.as_ref(), &edge, &envelope).await?;
        self.ensure_author_subscription(target_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        self.build_author_social_view(target_pubkey.as_str()).await
    }

    pub async fn get_author_social_view(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let author_pubkey = normalize_author_pubkey(pubkey)?;
        self.ensure_author_subscription(author_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        self.build_author_social_view(author_pubkey.as_str()).await
    }

    pub async fn resume_direct_message_state(&self) -> Result<()> {
        let mut peers = self
            .projection_store
            .list_direct_message_conversations()
            .await?
            .into_iter()
            .map(|row| row.peer_pubkey)
            .collect::<BTreeSet<_>>();
        for row in self.projection_store.list_direct_message_outbox().await? {
            peers.insert(row.peer_pubkey);
        }
        for peer_pubkey in peers {
            self.ensure_author_subscription(peer_pubkey.as_str())
                .await?;
            self.rebuild_author_relationships().await?;
            if self
                .direct_message_send_enabled(peer_pubkey.as_str())
                .await?
            {
                self.ensure_direct_message_subscription(peer_pubkey.as_str())
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn open_direct_message(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageConversationView> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        self.ensure_author_subscription(peer_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        let existing = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey.as_str())
            .await?;
        let can_send = self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?;
        if existing.is_none() && !can_send {
            anyhow::bail!("direct message requires a mutual relationship");
        }
        if can_send {
            self.restart_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        self.ensure_direct_message_conversation_row(peer_pubkey.as_str())
            .await?;
        self.direct_message_conversation_view(peer_pubkey.as_str())
            .await
    }

    pub async fn list_direct_messages(&self) -> Result<Vec<DirectMessageConversationView>> {
        let rows = self
            .projection_store
            .list_direct_message_conversations()
            .await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(
                self.direct_message_conversation_view(row.peer_pubkey.as_str())
                    .await?,
            );
        }
        Ok(items)
    }

    pub async fn list_direct_message_messages(
        &self,
        peer_pubkey: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<DirectMessageTimelineView> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let existing = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey.as_str())
            .await?;
        let can_send = self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?;
        if existing.is_none() && !can_send {
            anyhow::bail!("direct message requires a mutual relationship");
        }
        if can_send {
            self.ensure_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        self.ensure_direct_message_conversation_row(peer_pubkey.as_str())
            .await?;
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey.as_str()),
        );
        let page = self
            .projection_store
            .list_direct_message_messages(dm_id.as_str(), cursor, limit)
            .await?;
        let mut items = Vec::with_capacity(page.items.len());
        for row in page.items {
            items.push(self.direct_message_message_view(row).await?);
        }
        Ok(DirectMessageTimelineView {
            items,
            next_cursor: page.next_cursor,
        })
    }

    pub async fn send_direct_message(
        &self,
        peer_pubkey: &str,
        text: Option<&str>,
        reply_to_message_id: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        self.ensure_author_subscription(peer_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        if !self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?
        {
            anyhow::bail!("direct message requires a mutual relationship");
        }
        self.restart_direct_message_subscription(peer_pubkey.as_str())
            .await?;
        self.send_direct_message_internal(
            peer_pubkey.as_str(),
            text,
            reply_to_message_id,
            attachments,
        )
        .await
    }

    pub async fn delete_direct_message_message(
        &self,
        peer_pubkey: &str,
        message_id: &str,
    ) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let message_id = message_id.trim();
        if message_id.is_empty() {
            anyhow::bail!("direct message message_id is required");
        }
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey.as_str()),
        );
        self.projection_store
            .put_direct_message_tombstone(DirectMessageTombstoneRow {
                dm_id: dm_id.clone(),
                message_id: message_id.to_string(),
                deleted_at: Utc::now().timestamp_millis(),
            })
            .await?;
        self.projection_store
            .delete_direct_message_message_local(dm_id.as_str(), message_id)
            .await?;
        self.refresh_direct_message_conversation(peer_pubkey.as_str())
            .await?;
        Ok(())
    }

    pub async fn clear_direct_message(&self, peer_pubkey: &str) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey.as_str()),
        );
        let deleted_at = Utc::now().timestamp_millis();
        let mut cursor = None;
        loop {
            let page = self
                .projection_store
                .list_direct_message_messages(dm_id.as_str(), cursor.clone(), 500)
                .await?;
            for row in &page.items {
                self.projection_store
                    .put_direct_message_tombstone(DirectMessageTombstoneRow {
                        dm_id: dm_id.clone(),
                        message_id: row.message_id.clone(),
                        deleted_at,
                    })
                    .await?;
            }
            if page.next_cursor.is_none() {
                break;
            }
            cursor = page.next_cursor;
        }
        self.projection_store
            .clear_direct_message_local(dm_id.as_str())
            .await?;
        Ok(())
    }

    pub async fn get_direct_message_status(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageStatusView> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        self.ensure_author_subscription(peer_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        self.direct_message_status_view(peer_pubkey.as_str()).await
    }

    pub async fn list_profile_timeline(
        &self,
        author_pubkey: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        self.ensure_author_subscription(author_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        let mut posts =
            load_profile_posts_from_author_replica(self.docs_sync.as_ref(), author_pubkey.as_str())
                .await?;
        let mut reposts = load_profile_reposts_from_author_replica(
            self.docs_sync.as_ref(),
            author_pubkey.as_str(),
        )
        .await?;
        let mut items = Vec::with_capacity(posts.len() + reposts.len());
        items.extend(posts.drain(..).map(ProfileTimelineItem::Post));
        items.extend(reposts.drain(..).map(ProfileTimelineItem::Repost));
        items.sort_by(|left, right| {
            right
                .created_at()
                .cmp(&left.created_at())
                .then_with(|| right.object_id().cmp(left.object_id()))
        });
        let page = profile_timeline_page(items, cursor, limit);
        let mut views = Vec::with_capacity(page.items.len());
        for item in page.items {
            match item {
                ProfileTimelineItem::Post(post) => {
                    views.push(self.profile_post_to_view(post).await?)
                }
                ProfileTimelineItem::Repost(repost) => {
                    views.push(self.profile_repost_to_view(repost).await?)
                }
            }
        }
        Ok(TimelineView {
            items: views,
            next_cursor: page.next_cursor,
        })
    }

    pub async fn create_repost(
        &self,
        target_topic_id: &str,
        source_topic_id: &str,
        source_object_id: &str,
        commentary: Option<&str>,
    ) -> Result<String> {
        self.ensure_topic_subscription(target_topic_id).await?;
        self.ensure_topic_subscription(source_topic_id).await?;

        let normalized_commentary = normalize_repost_commentary(commentary.map(str::to_string));
        if let Some(existing_object_id) = self
            .find_existing_simple_repost(
                target_topic_id,
                source_object_id,
                normalized_commentary.as_deref(),
            )
            .await?
        {
            return Ok(existing_object_id);
        }

        let source_object = self
            .resolve_repost_source(source_topic_id, source_object_id)
            .await?;
        let topic = TopicId::new(target_topic_id);
        let envelope = build_repost_envelope(
            self.keys.as_ref(),
            &topic,
            source_object.repost_of.clone(),
            normalized_commentary.as_deref(),
        )?;
        let repost_object = envelope
            .to_post_object()?
            .ok_or_else(|| anyhow::anyhow!("failed to parse repost object"))?;
        self.ingest_event(
            &topic_replica_id(target_topic_id),
            envelope.clone(),
            None,
            Vec::new(),
        )
        .await?;

        let local_author_pubkey = self.current_author_pubkey();
        let profile_repost_envelope = build_profile_repost_envelope(
            self.keys.as_ref(),
            &KukuriProfileRepostEnvelopeContentV1 {
                author_pubkey: Pubkey::from(local_author_pubkey.as_str()),
                profile_topic_id: author_profile_topic_id(local_author_pubkey.as_str()),
                published_topic_id: topic.clone(),
                object_id: repost_object.object_id.clone(),
                created_at: repost_object.created_at,
                commentary: normalized_commentary.clone(),
                repost_of: source_object.repost_of,
            },
        )?;
        let profile_repost = parse_profile_repost(&profile_repost_envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse profile repost envelope"))?;
        persist_profile_repost_doc(
            self.docs_sync.as_ref(),
            &profile_repost,
            &profile_repost_envelope,
        )
        .await?;

        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(target_topic_id, None),
                GossipHint::TopicObjectsChanged {
                    topic_id: topic,
                    objects: vec![HintObjectRef {
                        object_id: envelope.id.0.clone(),
                        object_kind: envelope.kind.clone(),
                    }],
                },
            )
            .await?;
        Ok(envelope.id.0)
    }

    pub async fn toggle_reaction(
        &self,
        target_topic_id: &str,
        target_object_id: &str,
        reaction_key: ReactionKeyV1,
        channel_ref: Option<ChannelRef>,
    ) -> Result<ReactionStateView> {
        let target_topic_id = TopicId::new(target_topic_id);
        self.ensure_topic_subscription(target_topic_id.as_str())
            .await?;
        let target_object_id = EnvelopeId::from(target_object_id);
        let target = self
            .projection_store
            .get_object_projection(&target_object_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("reaction target was not found"))?;
        if !matches!(target.object_kind.as_str(), "post" | "comment") {
            anyhow::bail!("reaction target must be a post or comment");
        }
        if target.topic_id != target_topic_id.as_str() {
            anyhow::bail!("reaction target topic does not match");
        }
        let target_channel_id = channel_id_from_storage(target.channel_id.as_str());
        match (channel_ref.as_ref(), target_channel_id.as_ref()) {
            (Some(ChannelRef::Public), None) | (None, None) => {}
            (Some(ChannelRef::PrivateChannel { channel_id }), Some(target_channel_id))
                if channel_id == target_channel_id => {}
            (None, Some(_)) => {}
            _ => anyhow::bail!("reaction channel does not match the target object"),
        }
        let current_author = Pubkey::from(self.current_author_pubkey());
        let normalized_reaction_key = reaction_key.normalized_key()?;
        let reaction_id = deterministic_reaction_id(
            &target.source_replica_id,
            &target_object_id,
            &current_author,
            normalized_reaction_key.as_str(),
        );
        let next_status = match self
            .projection_store
            .get_reaction_cache(&target.source_replica_id, &target_object_id, &reaction_id)
            .await?
        {
            Some(existing) if existing.status == ObjectStatus::Active => ObjectStatus::Deleted,
            _ => ObjectStatus::Active,
        };
        let envelope = build_reaction_envelope(
            self.keys.as_ref(),
            &target_topic_id,
            target_channel_id.as_ref(),
            &target_object_id,
            reaction_key,
            &reaction_id,
            next_status.clone(),
        )?;
        let reaction = parse_reaction(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse reaction envelope"))?;
        persist_reaction_doc(
            self.docs_sync.as_ref(),
            &target.source_replica_id,
            &reaction,
            &envelope,
        )
        .await?;
        self.store.put_envelope(envelope.clone()).await?;
        self.projection_store
            .upsert_reaction_cache(reaction_projection_row_from_doc(
                &reaction,
                &target.source_replica_id,
            ))
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(target_topic_id.as_str(), target_channel_id.as_ref()),
                GossipHint::TopicObjectsChanged {
                    topic_id: target_topic_id.clone(),
                    objects: vec![HintObjectRef {
                        object_id: target_object_id.as_str().to_string(),
                        object_kind: "reaction".into(),
                    }],
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        self.reaction_state_for_target(&target.source_replica_id, &target_object_id)
            .await
    }

    pub async fn create_custom_reaction_asset(
        &self,
        input: CreateCustomReactionAssetInput,
    ) -> Result<CustomReactionAssetView> {
        let stored_blob = self
            .blob_service
            .put_blob(input.bytes, input.mime.as_str())
            .await?;
        let envelope = build_custom_reaction_asset_envelope(
            self.keys.as_ref(),
            stored_blob.hash.clone(),
            input.search_key,
            input.mime,
            stored_blob.bytes,
            input.width,
            input.height,
        )?;
        let asset = parse_custom_reaction_asset(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse custom reaction asset envelope"))?;
        persist_custom_reaction_asset_doc(self.docs_sync.as_ref(), &asset, &envelope).await?;
        self.store.put_envelope(envelope).await?;
        self.projection_store
            .mark_blob_status(&stored_blob.hash, BlobCacheStatus::Available)
            .await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(custom_reaction_asset_view_from_doc(&asset))
    }

    pub async fn list_my_custom_reaction_assets(&self) -> Result<Vec<CustomReactionAssetView>> {
        let author_pubkey = self.current_author_pubkey();
        let mut items = load_custom_reaction_assets_from_author_replica(
            self.docs_sync.as_ref(),
            &author_pubkey,
        )
        .await?;
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.asset_id.cmp(&left.asset_id))
        });
        Ok(items
            .into_iter()
            .map(|asset| custom_reaction_asset_view_from_doc(&asset))
            .collect())
    }

    pub async fn list_recent_reactions(&self, limit: usize) -> Result<Vec<RecentReactionView>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let author_pubkey = self.current_author_pubkey();
        let mut seen = BTreeSet::new();
        let mut items = Vec::new();
        for row in self
            .projection_store
            .list_recent_reaction_cache_by_author(author_pubkey.as_str())
            .await?
        {
            if !seen.insert(row.normalized_reaction_key.clone()) {
                continue;
            }
            items.push(recent_reaction_view_from_projection(&row));
            if items.len() >= limit {
                break;
            }
        }
        Ok(items)
    }

    pub async fn list_bookmarked_custom_reactions(
        &self,
    ) -> Result<Vec<BookmarkedCustomReactionView>> {
        Ok(self
            .projection_store
            .list_bookmarked_custom_reactions()
            .await?
            .into_iter()
            .map(bookmarked_custom_reaction_view_from_row)
            .collect())
    }

    pub async fn bookmark_custom_reaction(
        &self,
        asset: CustomReactionAssetSnapshotV1,
    ) -> Result<BookmarkedCustomReactionView> {
        if asset.owner_pubkey.as_str() == self.current_author_pubkey() {
            anyhow::bail!("bookmarking your own custom reaction is not supported");
        }
        let row = BookmarkedCustomReactionRow {
            asset_id: asset.asset_id.clone(),
            owner_pubkey: asset.owner_pubkey.as_str().to_string(),
            blob_hash: asset.blob_hash,
            search_key: search_key_or_asset_id(asset.search_key.as_str(), asset.asset_id.as_str()),
            mime: asset.mime,
            bytes: asset.bytes,
            width: asset.width,
            height: asset.height,
            bookmarked_at: Utc::now().timestamp_millis(),
        };
        self.projection_store
            .put_bookmarked_custom_reaction(row.clone())
            .await?;
        Ok(bookmarked_custom_reaction_view_from_row(row))
    }

    pub async fn remove_bookmarked_custom_reaction(&self, asset_id: &str) -> Result<()> {
        self.projection_store
            .remove_bookmarked_custom_reaction(asset_id)
            .await
    }

    pub async fn create_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<String> {
        self.create_post_in_channel(topic_id, ChannelRef::Public, content, reply_to)
            .await
    }

    pub async fn create_post_with_attachments(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        self.create_post_with_attachments_in_channel(
            topic_id,
            ChannelRef::Public,
            content,
            reply_to,
            attachments,
        )
        .await
    }

    pub async fn create_post_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<String> {
        self.create_post_with_attachments_in_channel(
            topic_id,
            channel_ref,
            content,
            reply_to,
            Vec::new(),
        )
        .await
    }

    pub async fn create_post_with_attachments_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        content: &str,
        reply_to: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        self.ensure_topic_subscription(topic_id).await?;
        let topic = TopicId::new(topic_id);
        let parent = if let Some(reply_to) = reply_to {
            self.resolve_parent_object(&EnvelopeId::from(reply_to))
                .await?
        } else {
            None
        };
        let private_state = if let Some(parent) = parent.as_ref() {
            let content = parent
                .post_content()?
                .ok_or_else(|| anyhow::anyhow!("reply target is not a post object"))?;
            if content.object_kind == "repost"
                && normalize_repost_commentary(content_from_payload_ref(&content.payload_ref))
                    .is_none()
            {
                anyhow::bail!("simple repost cannot be a reply parent");
            }
            if content.topic_id.as_str() != topic_id {
                anyhow::bail!("reply target topic does not match");
            }
            if let Some(channel_id) = content.channel_id.clone() {
                Some(
                    self.private_channel_write_state(topic_id, &channel_id)
                        .await?,
                )
            } else {
                None
            }
        } else {
            match channel_ref {
                ChannelRef::Public => None,
                ChannelRef::PrivateChannel { channel_id } => Some(
                    self.private_channel_write_state(topic_id, &channel_id)
                        .await?,
                ),
            }
        };
        let effective_channel_id = private_state.as_ref().map(|state| state.channel_id.clone());
        let write_replica = private_state
            .as_ref()
            .map(current_private_channel_replica_id)
            .unwrap_or_else(|| topic_replica_id(topic_id));
        let now = Utc::now().timestamp_millis();
        let stored_blob = self
            .blob_service
            .put_blob(content.as_bytes().to_vec(), "text/plain")
            .await?;
        let stored_attachments = futures_util::future::try_join_all(attachments.into_iter().map(
            |attachment| async move {
                let stored = self
                    .blob_service
                    .put_blob(attachment.bytes, attachment.mime.as_str())
                    .await?;
                Ok::<_, anyhow::Error>((attachment.role, stored))
            },
        ))
        .await?;
        let manifest_ids = if stored_attachments.is_empty() {
            Vec::new()
        } else {
            let manifest_id = format!(
                "media-{}-{}",
                now,
                short_id_suffix(self.current_author_pubkey().as_str())
            );
            let manifest = KukuriMediaManifestV1 {
                manifest_id: manifest_id.clone(),
                owner_pubkey: Pubkey::from(self.current_author_pubkey()),
                created_at: now,
                items: stored_attachments
                    .iter()
                    .map(|(role, stored)| MediaManifestItem {
                        blob_hash: stored.hash.clone(),
                        mime: stored.mime.clone(),
                        size: stored.bytes,
                        width: None,
                        height: None,
                        duration_ms: None,
                        codec: None,
                        thumbnail_blob_hash: match role {
                            AssetRole::VideoManifest => None,
                            _ => None,
                        },
                    })
                    .collect(),
            };
            let envelope = build_media_manifest_envelope(self.keys.as_ref(), &topic, &manifest)?;
            persist_media_manifest(
                &write_replica,
                &envelope,
                &manifest,
                self.docs_sync.as_ref(),
            )
            .await?;
            vec![manifest_id]
        };
        let envelope = build_post_envelope_with_payload_in_channel(
            self.keys.as_ref(),
            &topic,
            PayloadRef::BlobText {
                hash: stored_blob.hash.clone(),
                mime: stored_blob.mime.clone(),
                bytes: stored_blob.bytes,
            },
            stored_attachments
                .iter()
                .map(|(role, stored)| kukuri_core::AssetRef {
                    hash: stored.hash.clone(),
                    mime: stored.mime.clone(),
                    bytes: stored.bytes,
                    role: role.clone(),
                })
                .collect(),
            manifest_ids,
            parent.as_ref(),
            if effective_channel_id.is_some() {
                ObjectVisibility::Private
            } else {
                ObjectVisibility::Public
            },
            effective_channel_id.as_ref(),
        )?;
        let post_object = envelope
            .to_post_object()?
            .ok_or_else(|| anyhow::anyhow!("failed to parse post object for profile topic"))?;
        self.ingest_event(
            &write_replica,
            envelope.clone(),
            Some(stored_blob.clone()),
            stored_attachments,
        )
        .await?;
        if effective_channel_id.is_none() {
            let local_author_pubkey = self.current_author_pubkey();
            let profile_post_envelope = build_profile_post_envelope(
                self.keys.as_ref(),
                &KukuriProfilePostEnvelopeContentV1 {
                    author_pubkey: Pubkey::from(local_author_pubkey.as_str()),
                    profile_topic_id: author_profile_topic_id(local_author_pubkey.as_str()),
                    published_topic_id: topic.clone(),
                    object_id: post_object.object_id.clone(),
                    created_at: post_object.created_at,
                    object_kind: post_object.object_kind.clone(),
                    content: content.to_string(),
                    attachments: post_object.attachments.clone(),
                    reply_to_object_id: post_object.reply_to.clone(),
                    root_id: post_object.root.clone(),
                },
            )?;
            let profile_post = parse_profile_post(&profile_post_envelope)?
                .ok_or_else(|| anyhow::anyhow!("failed to parse profile post envelope"))?;
            persist_profile_post_doc(
                self.docs_sync.as_ref(),
                &profile_post,
                &profile_post_envelope,
            )
            .await?;
        }
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, effective_channel_id.as_ref()),
                GossipHint::TopicObjectsChanged {
                    topic_id: topic.clone(),
                    objects: vec![HintObjectRef {
                        object_id: envelope.id.0.clone(),
                        object_kind: envelope.kind.clone(),
                    }],
                },
            )
            .await?;
        Ok(envelope.id.0)
    }

    async fn resolve_repost_source(
        &self,
        source_topic_id: &str,
        source_object_id: &str,
    ) -> Result<ResolvedRepostSource> {
        let source_object_id = EnvelopeId::from(source_object_id);
        if ProjectionStore::get_object_projection(self.projection_store.as_ref(), &source_object_id)
            .await?
            .is_none()
        {
            let _ = hydrate_topic_state_with_services(
                self.docs_sync.as_ref(),
                self.blob_service.as_ref(),
                self.projection_store.as_ref(),
                source_topic_id,
            )
            .await?;
        }
        let projection = ProjectionStore::get_object_projection(
            self.projection_store.as_ref(),
            &source_object_id,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("repost source object not found"))?;
        if projection.topic_id != source_topic_id {
            anyhow::bail!("repost source topic does not match");
        }
        if projection.channel_id != PUBLIC_CHANNEL_ID {
            anyhow::bail!("only public posts and comments can be reposted");
        }
        if !matches!(projection.object_kind.as_str(), "post" | "comment") {
            anyhow::bail!("only public posts and comments can be reposted");
        }

        let header = fetch_post_object_for_projection(
            self.docs_sync.as_ref(),
            &projection.source_replica_id,
            projection.source_key.as_str(),
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("repost source header not found"))?;
        let content = match &projection.payload_ref {
            PayloadRef::InlineText { text } => text.clone(),
            PayloadRef::BlobText { hash, .. } => {
                fetch_projection_blob_text(self.blob_service.as_ref(), hash)
                    .await
                    .ok_or_else(|| anyhow::anyhow!("repost source content is unavailable"))?
            }
        };
        Ok(ResolvedRepostSource {
            repost_of: RepostSourceSnapshotV1 {
                source_object_id: header.object_id,
                source_topic_id: header.topic_id,
                source_author_pubkey: header.author,
                source_object_kind: header.object_kind,
                content,
                attachments: header.attachments,
                reply_to_object_id: header.reply_to,
                root_id: header.root,
            },
        })
    }

    async fn find_existing_simple_repost(
        &self,
        target_topic_id: &str,
        source_object_id: &str,
        commentary: Option<&str>,
    ) -> Result<Option<String>> {
        if commentary.is_some() {
            return Ok(None);
        }
        let target_replica = topic_replica_id(target_topic_id);
        let local_author_pubkey = self.current_author_pubkey();
        for record in self
            .docs_sync
            .query_replica(&target_replica, DocQuery::Prefix("objects/".into()))
            .await?
        {
            if !record.key.ends_with("/state") {
                continue;
            }
            let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
            if header.object_kind != "repost"
                || header.author.as_str() != local_author_pubkey
                || header.channel_id.is_some()
            {
                continue;
            }
            let Some(repost_of) = header.repost_of.as_ref() else {
                continue;
            };
            if repost_of.source_object_id.as_str() != source_object_id {
                continue;
            }
            let commentary = match &header.payload_ref {
                PayloadRef::InlineText { text } => normalize_repost_commentary(Some(text.clone())),
                PayloadRef::BlobText { .. } => None,
            };
            if commentary.is_none() {
                return Ok(Some(header.object_id.as_str().to_string()));
            }
        }
        Ok(None)
    }

    pub async fn list_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.list_timeline_scoped(topic_id, TimelineScope::Public, cursor, limit)
            .await
    }

    pub async fn list_timeline_scoped(
        &self,
        topic_id: &str,
        scope: TimelineScope,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.ensure_scope_subscriptions(topic_id, &scope).await?;
        let mut page = filtered_timeline_page(
            self.projection_store.as_ref(),
            topic_id,
            cursor.clone(),
            limit,
            &self.allowed_channel_ids_for_scope(topic_id, &scope).await?,
        )
        .await?;
        if page.items.is_empty()
            || projection_page_needs_hydration(&page)
            || self
                .scope_needs_current_private_epoch_hydration(topic_id, &scope, &page)
                .await
        {
            self.maybe_restart_scope_replica_sync(topic_id, &scope)
                .await;
            if self.hydrate_scope_projection(topic_id, &scope).await? > 0 {
                *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
            }
            page = filtered_timeline_page(
                self.projection_store.as_ref(),
                topic_id,
                cursor,
                limit,
                &self.allowed_channel_ids_for_scope(topic_id, &scope).await?,
            )
            .await?;
        }
        self.ensure_author_subscriptions_for_rows(&page.items)
            .await?;
        let view = self.page_to_view(page).await?;
        let mut last_sync = self.last_sync_ts.lock().await;
        if !view.items.is_empty() && last_sync.is_none() {
            *last_sync = Some(Utc::now().timestamp_millis());
        }
        Ok(view)
    }

    pub async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.ensure_scope_subscriptions(topic_id, &TimelineScope::AllJoined)
            .await?;
        let thread_root = EnvelopeId::from(thread_id);
        let mut page = filtered_thread_page(
            self.projection_store.as_ref(),
            topic_id,
            &thread_root,
            cursor.clone(),
            limit,
            None,
        )
        .await?;
        if page.items.is_empty() || projection_page_needs_hydration(&page) {
            self.maybe_restart_scope_replica_sync(topic_id, &TimelineScope::AllJoined)
                .await;
            if self
                .hydrate_scope_projection(topic_id, &TimelineScope::AllJoined)
                .await?
                > 0
            {
                *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
            }
            let root_channel = self
                .projection_store
                .get_object_projection(&thread_root)
                .await?
                .map(|row| row.channel_id);
            page = filtered_thread_page(
                self.projection_store.as_ref(),
                topic_id,
                &thread_root,
                cursor,
                limit,
                root_channel.as_deref(),
            )
            .await?;
        }
        self.ensure_author_subscriptions_for_rows(&page.items)
            .await?;
        let view = self.page_to_view(page).await?;
        let mut last_sync = self.last_sync_ts.lock().await;
        if !view.items.is_empty() && last_sync.is_none() {
            *last_sync = Some(Utc::now().timestamp_millis());
        }
        Ok(view)
    }

    pub async fn list_live_sessions(&self, topic_id: &str) -> Result<Vec<LiveSessionView>> {
        self.list_live_sessions_scoped(topic_id, TimelineScope::Public)
            .await
    }

    pub async fn list_live_sessions_scoped(
        &self,
        topic_id: &str,
        scope: TimelineScope,
    ) -> Result<Vec<LiveSessionView>> {
        self.ensure_scope_subscriptions(topic_id, &scope).await?;
        self.projection_store
            .clear_expired_live_presence(Utc::now().timestamp_millis())
            .await?;
        let allowed = self.allowed_channel_ids_for_scope(topic_id, &scope).await?;
        let mut rows = filter_channel_rows(
            self.projection_store
                .list_topic_live_sessions(topic_id)
                .await?,
            &allowed,
            |row| row.channel_id.as_str(),
        );
        let needs_refresh = rows
            .iter()
            .any(|row| row.status == LiveSessionStatus::Live && row.viewer_count == 0);
        if rows.is_empty() || needs_refresh {
            self.maybe_restart_scope_replica_sync(topic_id, &scope)
                .await;
            self.hydrate_scope_projection(topic_id, &scope).await?;
            self.projection_store
                .clear_expired_live_presence(Utc::now().timestamp_millis())
                .await?;
            rows = filter_channel_rows(
                self.projection_store
                    .list_topic_live_sessions(topic_id)
                    .await?,
                &allowed,
                |row| row.channel_id.as_str(),
            );
        }
        self.cleanup_ended_live_presence_tasks(&rows).await;
        let joined_sessions = self.live_presence_tasks.lock().await;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(LiveSessionView {
                session_id: row.session_id.clone(),
                host_pubkey: row.host_pubkey,
                title: row.title,
                description: row.description,
                status: row.status,
                started_at: row.started_at,
                ended_at: row.ended_at,
                viewer_count: row.viewer_count,
                joined_by_me: joined_sessions.contains_key(
                    live_presence_task_key(
                        topic_id,
                        row.channel_id.as_str(),
                        row.session_id.as_str(),
                    )
                    .as_str(),
                ),
                channel_id: channel_id_for_view(row.channel_id.as_str()),
                audience_label: self
                    .audience_label_for_storage(topic_id, row.channel_id.as_str())
                    .await,
            });
        }
        Ok(items)
    }

    pub async fn create_live_session(
        &self,
        topic_id: &str,
        input: CreateLiveSessionInput,
    ) -> Result<String> {
        self.create_live_session_in_channel(topic_id, ChannelRef::Public, input)
            .await
    }

    pub async fn create_live_session_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        input: CreateLiveSessionInput,
    ) -> Result<String> {
        self.ensure_topic_subscription(topic_id).await?;
        let now = Utc::now().timestamp_millis();
        let title = input.title.trim();
        if title.is_empty() {
            anyhow::bail!("live session title is required");
        }
        let private_state = match channel_ref {
            ChannelRef::Public => None,
            ChannelRef::PrivateChannel { channel_id } => Some(
                self.private_channel_write_state(topic_id, &channel_id)
                    .await?,
            ),
        };
        let channel_id = private_state.as_ref().map(|state| state.channel_id.clone());
        let source_replica_id = private_state
            .as_ref()
            .map(current_private_channel_replica_id)
            .unwrap_or_else(|| topic_replica_id(topic_id));
        let session_id = format!(
            "live-{}-{}",
            now,
            short_id_suffix(self.current_author_pubkey().as_str())
        );
        let topic = TopicId::new(topic_id);
        let manifest = LiveSessionManifestBlobV1 {
            session_id: session_id.clone(),
            topic_id: topic.clone(),
            channel_id: channel_id.clone(),
            owner_pubkey: Pubkey::from(self.current_author_pubkey()),
            title: title.to_string(),
            description: input.description.trim().to_string(),
            status: LiveSessionStatus::Live,
            started_at: now,
            ended_at: None,
        };
        let envelope = build_live_session_envelope(
            self.keys.as_ref(),
            &topic,
            session_id.as_str(),
            &serde_json::json!({
                "session_id": session_id,
                "topic_id": topic,
                "channel_id": channel_id.as_ref().map(|value| value.as_str()),
                "status": "live",
                "title": manifest.title,
                "description": manifest.description,
            }),
        )?;
        let state = self
            .persist_live_session_manifest(
                &source_replica_id,
                topic_id,
                manifest.clone(),
                now,
                envelope.id.clone(),
            )
            .await?;
        self.projection_store
            .upsert_live_session_cache(live_projection_row_from_state(
                &state,
                &manifest,
                topic_id,
                &source_replica_id,
            ))
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, channel_id.as_ref()),
                GossipHint::SessionChanged {
                    topic_id: topic.clone(),
                    session_id: session_id.clone(),
                    object_kind: "live-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(now);
        Ok(session_id)
    }

    pub async fn end_live_session(&self, topic_id: &str, session_id: &str) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let (source_replica_id, state, mut manifest) = self
            .fetch_live_session_state_and_manifest(topic_id, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("live session not found"))?;
        let owner = self.current_author_pubkey();
        if state.owner_pubkey.as_str() != owner {
            anyhow::bail!("only the live session owner can end the session");
        }
        let channel_key = channel_storage_id(state.channel_id.as_ref());
        let hint_topic = channel_hint_topic_for(topic_id, state.channel_id.as_ref());
        if manifest.status == LiveSessionStatus::Ended {
            self.stop_live_presence_task(topic_id, channel_key.as_str(), session_id)
                .await;
            return Ok(());
        }
        let now = Utc::now().timestamp_millis();
        manifest.status = LiveSessionStatus::Ended;
        manifest.ended_at = Some(now);
        let envelope = build_live_session_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            session_id,
            &serde_json::json!({
                "session_id": session_id,
                "topic_id": topic_id,
                "channel_id": state.channel_id.as_ref().map(|value| value.as_str()),
                "status": "ended",
            }),
        )?;
        let state = self
            .persist_live_session_manifest(
                &source_replica_id,
                topic_id,
                manifest.clone(),
                state.created_at,
                envelope.id.clone(),
            )
            .await?;
        self.projection_store
            .upsert_live_session_cache(live_projection_row_from_state(
                &state,
                &manifest,
                topic_id,
                &source_replica_id,
            ))
            .await?;
        self.stop_live_presence_task(topic_id, channel_key.as_str(), session_id)
            .await;
        self.hint_transport
            .publish_hint(
                &hint_topic,
                GossipHint::SessionChanged {
                    topic_id: TopicId::new(topic_id),
                    session_id: session_id.to_string(),
                    object_kind: "live-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(now);
        Ok(())
    }

    pub async fn join_live_session(&self, topic_id: &str, session_id: &str) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let Some((_, state, manifest)) = self
            .fetch_live_session_state_and_manifest(topic_id, session_id)
            .await?
        else {
            anyhow::bail!("live session not found");
        };
        if manifest.status == LiveSessionStatus::Ended {
            anyhow::bail!("cannot join an ended live session");
        }
        let channel_key = channel_storage_id(state.channel_id.as_ref());
        let task_key = live_presence_task_key(topic_id, channel_key.as_str(), session_id);
        if self
            .live_presence_tasks
            .lock()
            .await
            .contains_key(task_key.as_str())
        {
            return Ok(());
        }
        self.apply_live_presence(topic_id, state.channel_id.as_ref(), session_id, 30_000)
            .await?;
        let hint_transport = Arc::clone(&self.hint_transport);
        let projection_store = Arc::clone(&self.projection_store);
        let hint_topic = channel_hint_topic_for(topic_id, state.channel_id.as_ref());
        let topic_key = topic_id.to_string();
        let channel_key_for_task = channel_key.clone();
        let session_key = session_id.to_string();
        let author = Pubkey::from(self.current_author_pubkey());
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let now = Utc::now().timestamp_millis();
                let _ = projection_store
                    .upsert_live_presence(
                        topic_key.as_str(),
                        channel_key_for_task.as_str(),
                        session_key.as_str(),
                        author.as_str(),
                        now + 30_000,
                        now,
                    )
                    .await;
                let _ = hint_transport
                    .publish_hint(
                        &hint_topic,
                        GossipHint::LivePresence {
                            topic_id: TopicId::new(topic_key.clone()),
                            session_id: session_key.clone(),
                            author: author.clone(),
                            ttl_ms: 30_000,
                        },
                    )
                    .await;
            }
        });
        self.live_presence_tasks
            .lock()
            .await
            .insert(task_key, handle);
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    pub async fn leave_live_session(&self, topic_id: &str, session_id: &str) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let (_, state, _) = self
            .fetch_live_session_state_and_manifest(topic_id, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("live session not found"))?;
        let channel_key = channel_storage_id(state.channel_id.as_ref());
        self.stop_live_presence_task(topic_id, channel_key.as_str(), session_id)
            .await;
        self.apply_live_presence(topic_id, state.channel_id.as_ref(), session_id, 0)
            .await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    pub async fn list_game_rooms(&self, topic_id: &str) -> Result<Vec<GameRoomView>> {
        self.list_game_rooms_scoped(topic_id, TimelineScope::Public)
            .await
    }

    pub async fn list_game_rooms_scoped(
        &self,
        topic_id: &str,
        scope: TimelineScope,
    ) -> Result<Vec<GameRoomView>> {
        self.ensure_scope_subscriptions(topic_id, &scope).await?;
        let allowed = self.allowed_channel_ids_for_scope(topic_id, &scope).await?;
        let mut rows = filter_channel_rows(
            self.projection_store
                .list_topic_game_rooms(topic_id)
                .await?,
            &allowed,
            |row| row.channel_id.as_str(),
        );
        if rows.is_empty() {
            self.hydrate_scope_projection(topic_id, &scope).await?;
            rows = filter_channel_rows(
                self.projection_store
                    .list_topic_game_rooms(topic_id)
                    .await?,
                &allowed,
                |row| row.channel_id.as_str(),
            );
        }
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(GameRoomView {
                room_id: row.room_id,
                host_pubkey: row.host_pubkey,
                title: row.title,
                description: row.description,
                status: row.status,
                phase_label: row.phase_label,
                scores: row
                    .scores
                    .into_iter()
                    .map(|score| GameScoreView {
                        participant_id: score.participant_id,
                        label: score.label,
                        score: score.score,
                    })
                    .collect(),
                updated_at: row.updated_at,
                channel_id: channel_id_for_view(row.channel_id.as_str()),
                audience_label: self
                    .audience_label_for_storage(topic_id, row.channel_id.as_str())
                    .await,
            });
        }
        Ok(items)
    }

    pub async fn create_game_room(
        &self,
        topic_id: &str,
        input: CreateGameRoomInput,
    ) -> Result<String> {
        self.create_game_room_in_channel(topic_id, ChannelRef::Public, input)
            .await
    }

    pub async fn create_game_room_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        input: CreateGameRoomInput,
    ) -> Result<String> {
        self.ensure_topic_subscription(topic_id).await?;
        let private_state = match channel_ref {
            ChannelRef::Public => None,
            ChannelRef::PrivateChannel { channel_id } => Some(
                self.private_channel_write_state(topic_id, &channel_id)
                    .await?,
            ),
        };
        let channel_id = private_state.as_ref().map(|state| state.channel_id.clone());
        let source_replica_id = private_state
            .as_ref()
            .map(current_private_channel_replica_id)
            .unwrap_or_else(|| topic_replica_id(topic_id));
        let participants = sanitize_game_participants(input.participants)?;
        let now = Utc::now().timestamp_millis();
        let title = input.title.trim();
        if title.is_empty() {
            anyhow::bail!("game room title is required");
        }
        let room_id = format!(
            "game-{}-{}",
            now,
            short_id_suffix(self.current_author_pubkey().as_str())
        );
        let manifest = GameRoomManifestBlobV1 {
            room_id: room_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: channel_id.clone(),
            owner_pubkey: Pubkey::from(self.current_author_pubkey()),
            title: title.to_string(),
            description: input.description.trim().to_string(),
            status: GameRoomStatus::Waiting,
            phase_label: None,
            participants: participants
                .iter()
                .enumerate()
                .map(|(index, label)| GameParticipant {
                    participant_id: format!("participant-{}", index + 1),
                    label: label.clone(),
                })
                .collect(),
            scores: participants
                .iter()
                .enumerate()
                .map(|(index, label)| GameScoreEntry {
                    participant_id: format!("participant-{}", index + 1),
                    label: label.clone(),
                    score: 0,
                })
                .collect(),
            updated_at: now,
        };
        let envelope = build_game_session_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            room_id.as_str(),
            &serde_json::json!({
                "room_id": room_id,
                "topic_id": topic_id,
                "channel_id": channel_id.as_ref().map(|value| value.as_str()),
                "status": "waiting",
            }),
        )?;
        let state = self
            .persist_game_room_manifest(
                &source_replica_id,
                topic_id,
                manifest.clone(),
                now,
                envelope.id.clone(),
            )
            .await?;
        self.projection_store
            .upsert_game_room_cache(game_projection_row_from_state(
                &state,
                &manifest,
                topic_id,
                &source_replica_id,
            ))
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, channel_id.as_ref()),
                GossipHint::SessionChanged {
                    topic_id: TopicId::new(topic_id),
                    session_id: room_id.clone(),
                    object_kind: "game-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(now);
        Ok(room_id)
    }

    pub async fn update_game_room(
        &self,
        topic_id: &str,
        room_id: &str,
        input: UpdateGameRoomInput,
    ) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let (source_replica_id, state, mut manifest) = self
            .fetch_game_room_state_and_manifest(topic_id, room_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("game room not found"))?;
        let owner = self.current_author_pubkey();
        if state.owner_pubkey.as_str() != owner {
            anyhow::bail!("only the game room owner can update the room");
        }
        validate_game_room_transition(&manifest.status, &input.status)?;
        validate_game_room_scores(&manifest, &input.scores)?;
        manifest.status = input.status;
        manifest.phase_label = input
            .phase_label
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        manifest.scores = input
            .scores
            .into_iter()
            .map(|score| GameScoreEntry {
                participant_id: score.participant_id,
                label: score.label,
                score: score.score,
            })
            .collect();
        manifest.updated_at = Utc::now().timestamp_millis();
        let envelope = build_game_session_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            room_id,
            &serde_json::json!({
                "room_id": room_id,
                "topic_id": topic_id,
                "channel_id": state.channel_id.as_ref().map(|value| value.as_str()),
                "status": format!("{:?}", manifest.status).to_lowercase(),
                "phase_label": manifest.phase_label,
            }),
        )?;
        let state = self
            .persist_game_room_manifest(
                &source_replica_id,
                topic_id,
                manifest.clone(),
                state.created_at,
                envelope.id.clone(),
            )
            .await?;
        self.projection_store
            .upsert_game_room_cache(game_projection_row_from_state(
                &state,
                &manifest,
                topic_id,
                &source_replica_id,
            ))
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, state.channel_id.as_ref()),
                GossipHint::SessionChanged {
                    topic_id: TopicId::new(topic_id),
                    session_id: room_id.to_string(),
                    object_kind: "game-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(manifest.updated_at);
        Ok(())
    }

    pub async fn create_private_channel(
        &self,
        input: CreatePrivateChannelInput,
    ) -> Result<JoinedPrivateChannelView> {
        self.ensure_topic_subscription(input.topic_id.as_str())
            .await?;
        let label = input.label.trim();
        if label.is_empty() {
            anyhow::bail!("private channel label is required");
        }
        let now = Utc::now().timestamp_millis();
        let owner_pubkey = self.current_author_pubkey();
        let channel_id = ChannelId::new(format!(
            "channel-{}-{}",
            now,
            short_id_suffix(owner_pubkey.as_str())
        ));
        let current_epoch_id =
            initial_private_channel_epoch_id(&input.audience_kind, now, owner_pubkey.as_str());
        let current_epoch_secret_hex = generate_keys().export_secret_hex();
        let state = JoinedPrivateChannelState {
            topic_id: input.topic_id.as_str().to_string(),
            channel_id: channel_id.clone(),
            label: label.to_string(),
            creator_pubkey: owner_pubkey.clone(),
            owner_pubkey: owner_pubkey.clone(),
            joined_via_pubkey: None,
            audience_kind: input.audience_kind.clone(),
            current_epoch_id: current_epoch_id.clone(),
            current_epoch_secret_hex: current_epoch_secret_hex.clone(),
            archived_epochs: Vec::new(),
        };
        self.register_joined_private_channel(state.clone()).await?;
        let metadata = PrivateChannelMetadataDocV1 {
            channel_id: channel_id.clone(),
            topic_id: input.topic_id.clone(),
            label: label.to_string(),
            creator_pubkey: Pubkey::from(state.creator_pubkey.clone()),
            created_at: now,
            audience_kind: input.audience_kind.clone(),
            owner_pubkey: Pubkey::from(owner_pubkey.clone()),
        };
        persist_private_channel_metadata(
            self.docs_sync.as_ref(),
            &current_private_channel_replica_id(&state),
            &metadata,
        )
        .await?;
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                channel_id: channel_id.clone(),
                topic_id: input.topic_id.clone(),
                audience_kind: input.audience_kind.clone(),
                owner_pubkey: Pubkey::from(owner_pubkey.clone()),
                epoch_id: current_epoch_id,
                sharing_state: ChannelSharingState::Open,
                rotated_at: None,
                previous_epoch_id: None,
            },
            &current_private_channel_replica_id(&state),
        )
        .await?;
        persist_private_channel_participant(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelParticipantDocV1 {
                channel_id,
                topic_id: input.topic_id,
                epoch_id: state.current_epoch_id.clone(),
                participant_pubkey: Pubkey::from(owner_pubkey),
                joined_at: now,
                is_owner: true,
                join_mode: Some(PrivateChannelJoinMode::OwnerSeed),
                sponsor_pubkey: None,
                share_token_id: None,
            },
            &current_private_channel_replica_id(&state),
        )
        .await?;
        self.joined_private_channel_view_for_state(&state).await
    }

    pub async fn export_private_channel_invite(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<String> {
        let state = self
            .private_channel_state_for_owner_action(
                topic_id,
                &ChannelId::new(channel_id),
                PrivateChannelOwnerAction::Share,
            )
            .await?;
        if state.audience_kind != ChannelAudienceKind::InviteOnly {
            anyhow::bail!(
                "private channel invite export is only available for invite-only channels"
            );
        }
        build_private_channel_invite_token(
            self.keys.as_ref(),
            PrivateChannelInviteTokenParams {
                topic: &TopicId::new(topic_id),
                channel_id: &state.channel_id,
                channel_label: state.label.as_str(),
                owner_pubkey: &Pubkey::from(state.owner_pubkey.clone()),
                epoch_id: state.current_epoch_id.as_str(),
                namespace_secret_hex: state.current_epoch_secret_hex.as_str(),
                expires_at,
            },
        )
    }

    pub async fn import_private_channel_invite(
        &self,
        token: &str,
    ) -> Result<PrivateChannelInvitePreview> {
        let preview = parse_private_channel_invite_token(token)?;
        if let Some(expires_at) = preview.expires_at
            && expires_at < Utc::now().timestamp_millis()
        {
            anyhow::bail!("private channel invite is expired");
        }
        self.ensure_topic_subscription(preview.topic_id.as_str())
            .await?;
        let replica = private_channel_replica_for_epoch(
            preview.channel_id.as_str(),
            preview.epoch_id.as_str(),
        );
        self.docs_sync
            .register_private_replica_secret(&replica, preview.namespace_secret_hex.as_str())
            .await?;
        let import_result = async {
            let (metadata, policy, participants) = wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &replica,
                "invite-only channel replica sync",
            )
            .await?;
            if policy.audience_kind != ChannelAudienceKind::InviteOnly {
                anyhow::bail!("invite-only replica audience must be invite_only");
            }
            if policy.sharing_state != ChannelSharingState::Open {
                anyhow::bail!("invite-only access token is no longer open for import");
            }
            if policy.epoch_id != preview.epoch_id {
                anyhow::bail!("invite-only access token epoch does not match the current policy");
            }
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == policy.owner_pubkey
                    && participant.epoch_id == policy.epoch_id
                    && participant.is_owner
            }) {
                anyhow::bail!("invite-only channel owner is not an active participant");
            }
            let local_pubkey = Pubkey::from(self.current_author_pubkey());
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == local_pubkey
                    && participant.epoch_id == policy.epoch_id
            }) {
                persist_private_channel_participant(
                    self.docs_sync.as_ref(),
                    self.keys.as_ref(),
                    &PrivateChannelParticipantDocV1 {
                        channel_id: metadata.channel_id.clone(),
                        topic_id: metadata.topic_id.clone(),
                        epoch_id: policy.epoch_id.clone(),
                        participant_pubkey: local_pubkey,
                        joined_at: Utc::now().timestamp_millis(),
                        is_owner: false,
                        join_mode: Some(PrivateChannelJoinMode::InviteToken),
                        sponsor_pubkey: Some(preview.inviter_pubkey.clone()),
                        share_token_id: None,
                    },
                    &replica,
                )
                .await?;
            }
            let next_state = merged_private_channel_state_from_epoch_join(
                self.joined_private_channel_state(
                    preview.topic_id.as_str(),
                    preview.channel_id.as_str(),
                )
                .await,
                preview.topic_id.as_str(),
                preview.channel_id.clone(),
                preview.channel_label.as_str(),
                metadata.creator_pubkey.as_str(),
                preview.owner_pubkey.as_str(),
                Some(preview.inviter_pubkey.as_str()),
                ChannelAudienceKind::InviteOnly,
                preview.epoch_id.as_str(),
                preview.namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if import_result.is_err() {
            let _ = self.docs_sync.remove_private_replica_secret(&replica).await;
        }
        import_result?;
        Ok(preview)
    }

    pub async fn export_channel_access_token(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<ChannelAccessTokenExport> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        let (kind, token) = match state.audience_kind {
            ChannelAudienceKind::InviteOnly => (
                ChannelAccessTokenKind::Invite,
                self.export_private_channel_invite(topic_id, channel_id, expires_at)
                    .await?,
            ),
            ChannelAudienceKind::FriendOnly => (
                ChannelAccessTokenKind::Grant,
                self.export_friend_only_grant(topic_id, channel_id, expires_at)
                    .await?,
            ),
            ChannelAudienceKind::FriendPlus => (
                ChannelAccessTokenKind::Share,
                self.export_friend_plus_share(topic_id, channel_id, expires_at)
                    .await?,
            ),
        };
        Ok(ChannelAccessTokenExport { kind, token })
    }

    pub async fn import_channel_access_token(
        &self,
        token: &str,
    ) -> Result<ChannelAccessTokenPreview> {
        if parse_private_channel_invite_token(token).is_ok() {
            let preview = self.import_private_channel_invite(token).await?;
            return Ok(ChannelAccessTokenPreview {
                kind: ChannelAccessTokenKind::Invite,
                topic_id: preview.topic_id.as_str().to_string(),
                channel_id: preview.channel_id.as_str().to_string(),
                channel_label: preview.channel_label,
                owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                inviter_pubkey: Some(preview.inviter_pubkey.as_str().to_string()),
                sponsor_pubkey: None,
                epoch_id: preview.epoch_id,
            });
        }
        if parse_friend_only_grant_token(token).is_ok() {
            let preview = self.import_friend_only_grant(token).await?;
            return Ok(ChannelAccessTokenPreview {
                kind: ChannelAccessTokenKind::Grant,
                topic_id: preview.topic_id.as_str().to_string(),
                channel_id: preview.channel_id.as_str().to_string(),
                channel_label: preview.channel_label,
                owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                inviter_pubkey: None,
                sponsor_pubkey: Some(preview.owner_pubkey.as_str().to_string()),
                epoch_id: preview.epoch_id,
            });
        }
        if parse_friend_plus_share_token(token).is_ok() {
            let preview = self.import_friend_plus_share(token).await?;
            return Ok(ChannelAccessTokenPreview {
                kind: ChannelAccessTokenKind::Share,
                topic_id: preview.topic_id.as_str().to_string(),
                channel_id: preview.channel_id.as_str().to_string(),
                channel_label: preview.channel_label,
                owner_pubkey: preview.owner_pubkey.as_str().to_string(),
                inviter_pubkey: None,
                sponsor_pubkey: Some(preview.sponsor_pubkey.as_str().to_string()),
                epoch_id: preview.epoch_id,
            });
        }
        anyhow::bail!("unrecognized private channel access token")
    }

    pub async fn export_friend_only_grant(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<String> {
        let state = self
            .private_channel_state_for_owner_action(
                topic_id,
                &ChannelId::new(channel_id),
                PrivateChannelOwnerAction::Share,
            )
            .await?;
        if state.audience_kind != ChannelAudienceKind::FriendOnly {
            anyhow::bail!("friend-only grant export is only available for friends channels");
        }
        if state.owner_pubkey != self.current_author_pubkey() {
            anyhow::bail!("only the channel owner can create friend-only grants");
        }
        let diagnostics = self.private_channel_diagnostics(&state).await?;
        if diagnostics.sharing_state != ChannelSharingState::Open {
            anyhow::bail!("friend-only grant export is disabled while sharing is frozen");
        }
        build_friend_only_grant_token(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            &state.channel_id,
            state.label.as_str(),
            state.current_epoch_id.as_str(),
            state.current_epoch_secret_hex.as_str(),
            expires_at,
        )
    }

    pub async fn import_friend_only_grant(&self, token: &str) -> Result<FriendOnlyGrantPreview> {
        let preview = parse_friend_only_grant_token(token)?;
        if let Some(expires_at) = preview.expires_at
            && expires_at < Utc::now().timestamp_millis()
        {
            anyhow::bail!("friend-only grant is expired");
        }
        self.ensure_topic_subscription(preview.topic_id.as_str())
            .await?;
        self.ensure_author_subscription(preview.owner_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                preview.owner_pubkey.as_str(),
            )
            .await?;
        if !relationship.as_ref().is_some_and(|value| value.mutual) {
            anyhow::bail!(
                "friend-only grant import requires a mutual relationship with the channel owner"
            );
        }

        let replica = private_channel_epoch_replica_id(
            preview.channel_id.as_str(),
            preview.epoch_id.as_str(),
        );
        self.docs_sync
            .register_private_replica_secret(&replica, preview.namespace_secret_hex.as_str())
            .await?;
        let import_result = async {
            let (metadata, policy, participants) = wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &replica,
                "friend-only channel replica sync",
            )
            .await?;
            if policy.audience_kind != ChannelAudienceKind::FriendOnly {
                anyhow::bail!("friend-only grant replica audience must be friend_only");
            }
            if policy.sharing_state != ChannelSharingState::Open {
                anyhow::bail!("friend-only grant is no longer open for import");
            }
            if policy.epoch_id != preview.epoch_id {
                anyhow::bail!("friend-only grant epoch does not match the current policy");
            }
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == policy.owner_pubkey
                    && participant.epoch_id == policy.epoch_id
                    && participant.is_owner
            }) {
                anyhow::bail!("friend-only grant owner is not an active participant");
            }
            let joined_at = Utc::now().timestamp_millis();
            persist_private_channel_participant(
                self.docs_sync.as_ref(),
                self.keys.as_ref(),
                &PrivateChannelParticipantDocV1 {
                    channel_id: metadata.channel_id.clone(),
                    topic_id: metadata.topic_id.clone(),
                    epoch_id: policy.epoch_id.clone(),
                    participant_pubkey: Pubkey::from(self.current_author_pubkey()),
                    joined_at,
                    is_owner: false,
                    join_mode: Some(PrivateChannelJoinMode::FriendOnlyGrant),
                    sponsor_pubkey: Some(policy.owner_pubkey.clone()),
                    share_token_id: None,
                },
                &replica,
            )
            .await?;
            let next_state = merged_private_channel_state_from_epoch_join(
                self.joined_private_channel_state(
                    preview.topic_id.as_str(),
                    preview.channel_id.as_str(),
                )
                .await,
                preview.topic_id.as_str(),
                preview.channel_id.clone(),
                preview.channel_label.as_str(),
                metadata.creator_pubkey.as_str(),
                preview.owner_pubkey.as_str(),
                Some(preview.owner_pubkey.as_str()),
                ChannelAudienceKind::FriendOnly,
                preview.epoch_id.as_str(),
                preview.namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if import_result.is_err() {
            let _ = self.docs_sync.remove_private_replica_secret(&replica).await;
        }
        import_result?;
        Ok(preview)
    }

    pub async fn export_friend_plus_share(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<String> {
        let state = self
            .private_channel_state_for_owner_action(
                topic_id,
                &ChannelId::new(channel_id),
                PrivateChannelOwnerAction::Share,
            )
            .await?;
        if state.audience_kind != ChannelAudienceKind::FriendPlus {
            anyhow::bail!("friend-plus share export is only available for friends+ channels");
        }
        let replica = current_private_channel_replica_id(&state);
        let Some(policy) =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &replica).await?
        else {
            anyhow::bail!("friend-plus channel policy is missing");
        };
        if policy.sharing_state != ChannelSharingState::Open {
            anyhow::bail!("friend-plus share export is disabled while sharing is frozen");
        }
        let participants =
            fetch_private_channel_participants_from_replica(self.docs_sync.as_ref(), &replica)
                .await?;
        let local_author = self.current_author_pubkey();
        if !participants.iter().any(|participant| {
            participant.epoch_id == state.current_epoch_id
                && participant.participant_pubkey.as_str() == local_author
        }) {
            anyhow::bail!("only active participants can create friend-plus shares");
        }
        let effective_expires_at =
            expires_at.or_else(|| Some(Utc::now().timestamp_millis() + 24 * 60 * 60 * 1000));
        build_friend_plus_share_token(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            &state.channel_id,
            state.label.as_str(),
            &Pubkey::from(state.owner_pubkey.clone()),
            state.current_epoch_id.as_str(),
            state.current_epoch_secret_hex.as_str(),
            effective_expires_at,
        )
    }

    pub async fn import_friend_plus_share(&self, token: &str) -> Result<FriendPlusSharePreview> {
        let preview = parse_friend_plus_share_token(token)?;
        self.ensure_topic_subscription(preview.topic_id.as_str())
            .await?;
        self.ensure_author_subscription(preview.sponsor_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                preview.sponsor_pubkey.as_str(),
            )
            .await?;
        if !relationship.as_ref().is_some_and(|value| value.mutual) {
            anyhow::bail!(
                "friend-plus share import requires a mutual relationship with the sponsor"
            );
        }

        let replica = private_channel_epoch_replica_id(
            preview.channel_id.as_str(),
            preview.epoch_id.as_str(),
        );
        self.docs_sync
            .register_private_replica_secret(&replica, preview.namespace_secret_hex.as_str())
            .await?;
        let import_result = async {
            let (metadata, policy, _participants) = wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &replica,
                "friend-plus channel replica sync",
            )
            .await?;
            let participants =
                fetch_private_channel_participants_from_replica(self.docs_sync.as_ref(), &replica)
                    .await?;
            if policy.audience_kind != ChannelAudienceKind::FriendPlus {
                anyhow::bail!("friend-plus share replica audience must be friend_plus");
            }
            if policy.sharing_state != ChannelSharingState::Open {
                anyhow::bail!("friend-plus share is no longer open for import");
            }
            if policy.epoch_id != preview.epoch_id {
                anyhow::bail!("friend-plus share epoch does not match the current policy");
            }
            let local_author = self.current_author_pubkey();
            if !participants.iter().any(|participant| {
                participant.participant_pubkey.as_str() == local_author
                    && participant.epoch_id == policy.epoch_id
            }) {
                persist_private_channel_participant(
                    self.docs_sync.as_ref(),
                    self.keys.as_ref(),
                    &PrivateChannelParticipantDocV1 {
                        channel_id: metadata.channel_id.clone(),
                        topic_id: metadata.topic_id.clone(),
                        epoch_id: policy.epoch_id.clone(),
                        participant_pubkey: Pubkey::from(local_author),
                        joined_at: Utc::now().timestamp_millis(),
                        is_owner: false,
                        join_mode: Some(PrivateChannelJoinMode::FriendPlusShare),
                        sponsor_pubkey: Some(preview.sponsor_pubkey.clone()),
                        share_token_id: Some(preview.share_token_id.clone()),
                    },
                    &replica,
                )
                .await?;
            }
            let next_state = merged_private_channel_state_from_epoch_join(
                self.joined_private_channel_state(
                    preview.topic_id.as_str(),
                    preview.channel_id.as_str(),
                )
                .await,
                preview.topic_id.as_str(),
                preview.channel_id.clone(),
                preview.channel_label.as_str(),
                metadata.creator_pubkey.as_str(),
                preview.owner_pubkey.as_str(),
                Some(preview.sponsor_pubkey.as_str()),
                ChannelAudienceKind::FriendPlus,
                preview.epoch_id.as_str(),
                preview.namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if import_result.is_err() {
            let _ = self.docs_sync.remove_private_replica_secret(&replica).await;
        }
        import_result?;
        Ok(preview)
    }

    pub async fn freeze_private_channel(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<JoinedPrivateChannelView> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        if state.audience_kind != ChannelAudienceKind::FriendPlus {
            anyhow::bail!("freeze is only available for friend-plus channels");
        }
        if state.owner_pubkey != self.current_author_pubkey() {
            anyhow::bail!("only the channel owner can freeze the channel");
        }
        let current_replica = current_private_channel_replica_id(&state);
        let Some(current_policy) =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &current_replica)
                .await?
        else {
            anyhow::bail!("friend-plus channel policy is missing");
        };
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                sharing_state: ChannelSharingState::Frozen,
                rotated_at: current_policy.rotated_at,
                ..current_policy
            },
            &current_replica,
        )
        .await?;
        self.joined_private_channel_view_for_state(&state).await
    }

    pub async fn rotate_private_channel(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<JoinedPrivateChannelView> {
        let Some(mut state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        if !private_channel_is_epoch_aware(&state.audience_kind) {
            anyhow::bail!("rotate is only available for epoch-aware private channels");
        }
        if state.owner_pubkey != self.current_author_pubkey() {
            anyhow::bail!("only the channel owner can rotate the channel");
        }
        let current_replica = current_private_channel_replica_id(&state);
        let current_policy =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &current_replica)
                .await?
                .unwrap_or(PrivateChannelPolicyDocV1 {
                    channel_id: state.channel_id.clone(),
                    topic_id: TopicId::new(topic_id),
                    audience_kind: state.audience_kind.clone(),
                    owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                    epoch_id: state.current_epoch_id.clone(),
                    sharing_state: ChannelSharingState::Open,
                    rotated_at: None,
                    previous_epoch_id: None,
                });
        let current_participants = fetch_private_channel_participants_from_replica(
            self.docs_sync.as_ref(),
            &current_replica,
        )
        .await?;
        let mut rotation_recipients = BTreeMap::new();
        for participant in active_private_channel_participants(
            &current_participants,
            state.current_epoch_id.as_str(),
        ) {
            if participant.is_owner {
                continue;
            }
            rotation_recipients
                .entry(participant.participant_pubkey.as_str().to_string())
                .or_insert(participant);
        }
        for epoch in &state.archived_epochs {
            let archived_replica =
                private_channel_epoch_replica_id(channel_id, epoch.epoch_id.as_str());
            let archived_participants = fetch_private_channel_participants_from_replica(
                self.docs_sync.as_ref(),
                &archived_replica,
            )
            .await?;
            for participant in
                active_private_channel_participants(&archived_participants, epoch.epoch_id.as_str())
            {
                if participant.is_owner {
                    continue;
                }
                rotation_recipients
                    .entry(participant.participant_pubkey.as_str().to_string())
                    .or_insert(participant);
            }
        }
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                sharing_state: ChannelSharingState::Frozen,
                rotated_at: Some(Utc::now().timestamp_millis()),
                ..current_policy
            },
            &current_replica,
        )
        .await?;

        let next_epoch_id = next_private_channel_epoch_id(self.current_author_pubkey().as_str());
        let next_secret = generate_keys().export_secret_hex();
        let next_replica = private_channel_epoch_replica_id(channel_id, next_epoch_id.as_str());
        self.docs_sync
            .register_private_replica_secret(&next_replica, next_secret.as_str())
            .await?;
        let metadata = PrivateChannelMetadataDocV1 {
            channel_id: state.channel_id.clone(),
            topic_id: TopicId::new(topic_id),
            label: state.label.clone(),
            creator_pubkey: Pubkey::from(state.creator_pubkey.clone()),
            created_at: Utc::now().timestamp_millis(),
            audience_kind: state.audience_kind.clone(),
            owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
        };
        persist_private_channel_metadata(self.docs_sync.as_ref(), &next_replica, &metadata).await?;
        persist_private_channel_policy(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelPolicyDocV1 {
                channel_id: state.channel_id.clone(),
                topic_id: TopicId::new(topic_id),
                audience_kind: state.audience_kind.clone(),
                owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                epoch_id: next_epoch_id.clone(),
                sharing_state: ChannelSharingState::Open,
                rotated_at: None,
                previous_epoch_id: Some(state.current_epoch_id.clone()),
            },
            &next_replica,
        )
        .await?;
        persist_private_channel_participant(
            self.docs_sync.as_ref(),
            self.keys.as_ref(),
            &PrivateChannelParticipantDocV1 {
                channel_id: state.channel_id.clone(),
                topic_id: TopicId::new(topic_id),
                epoch_id: next_epoch_id.clone(),
                participant_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                joined_at: Utc::now().timestamp_millis(),
                is_owner: true,
                join_mode: Some(PrivateChannelJoinMode::OwnerSeed),
                sponsor_pubkey: None,
                share_token_id: None,
            },
            &next_replica,
        )
        .await?;
        for participant in rotation_recipients.into_values() {
            if state.audience_kind == ChannelAudienceKind::FriendOnly {
                self.ensure_author_subscription(participant.participant_pubkey.as_str())
                    .await?;
                let relationship = self
                    .projection_store
                    .get_author_relationship(
                        self.current_author_pubkey().as_str(),
                        participant.participant_pubkey.as_str(),
                    )
                    .await?;
                if !relationship.as_ref().is_some_and(|value| value.mutual) {
                    continue;
                }
            }
            let grant_doc = encrypt_private_channel_epoch_handoff_grant(
                self.keys.as_ref(),
                &PrivateChannelEpochHandoffGrantPayloadV1 {
                    channel_id: state.channel_id.clone(),
                    topic_id: TopicId::new(topic_id),
                    owner_pubkey: Pubkey::from(state.owner_pubkey.clone()),
                    recipient_pubkey: participant.participant_pubkey.clone(),
                    old_epoch_id: state.current_epoch_id.clone(),
                    new_epoch_id: next_epoch_id.clone(),
                    new_namespace_secret_hex: next_secret.clone(),
                },
            )?;
            persist_private_channel_rotation_grant(
                self.docs_sync.as_ref(),
                self.keys.as_ref(),
                &grant_doc,
                &current_replica,
            )
            .await?;
        }

        let archived_epoch_id = state.current_epoch_id.clone();
        let archived_secret = state.current_epoch_secret_hex.clone();
        archive_private_channel_epoch(
            &mut state,
            archived_epoch_id.as_str(),
            archived_secret.as_str(),
        );
        state.current_epoch_id = next_epoch_id;
        state.current_epoch_secret_hex = next_secret;
        self.register_joined_private_channel(state.clone()).await?;
        self.joined_private_channel_view_for_state(&state).await
    }

    pub async fn restore_private_channel_capability(
        &self,
        capability: PrivateChannelCapability,
    ) -> Result<()> {
        let state = joined_private_channel_state_from_capability(capability)?;
        self.ensure_topic_subscription(state.topic_id.as_str())
            .await?;
        self.register_joined_private_channel(state).await
    }

    pub async fn list_joined_private_channels(
        &self,
        topic_id: &str,
    ) -> Result<Vec<JoinedPrivateChannelView>> {
        self.ensure_topic_subscription(topic_id).await?;
        self.ensure_joined_private_channel_subscriptions(topic_id)
            .await?;
        self.maybe_restart_scope_replica_sync(topic_id, &TimelineScope::AllJoined)
            .await;
        self.maybe_redeem_rotation_grants_for_topic(topic_id)
            .await?;
        let mut items = Vec::new();
        for state in self.joined_private_channel_states_for_topic(topic_id).await {
            items.push(self.joined_private_channel_view_for_state(&state).await?);
        }
        Ok(items)
    }

    pub async fn get_private_channel_capability(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<Option<PrivateChannelCapability>> {
        self.maybe_redeem_rotation_grants_for_channel(topic_id, channel_id)
            .await?;
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            return Ok(None);
        };
        Ok(Some(
            self.private_channel_capability_from_state(&state).await?,
        ))
    }

    pub async fn list_private_channel_capabilities(&self) -> Result<Vec<PrivateChannelCapability>> {
        let states = self
            .joined_private_channels
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut items = Vec::with_capacity(states.len());
        for state in states {
            items.push(self.private_channel_capability_from_state(&state).await?);
        }
        Ok(items)
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let PeerSnapshot {
            connected,
            peer_count,
            connected_peers,
            configured_peers,
            subscribed_topics,
            pending_events,
            status_detail,
            last_error,
            topic_diagnostics,
        } = self.transport.peers().await?;
        let subscribed_topics = normalize_topics(subscribed_topics);
        let topic_diagnostics = normalize_topic_diagnostics(topic_diagnostics);
        let assist_peer_ids = self.assisted_peer_ids().await?;
        let effective_connected_peer_ids =
            merge_peer_ids(connected_peers.clone(), assist_peer_ids.clone());
        let discovery = self.get_discovery_status().await?;

        Ok(SyncStatus {
            connected: connected || !assist_peer_ids.is_empty(),
            last_sync_ts: *self.last_sync_ts.lock().await,
            peer_count: peer_count.max(effective_connected_peer_ids.len()),
            pending_events,
            status_detail: effective_sync_status_detail(
                status_detail.as_str(),
                connected_peers.len(),
                assist_peer_ids.len(),
                subscribed_topics.len(),
            ),
            last_error,
            configured_peers,
            subscribed_topics,
            topic_diagnostics: topic_diagnostics
                .into_iter()
                .map(|diagnostic| {
                    let gossip_peer_count = diagnostic.connected_peers.len();
                    TopicSyncStatus {
                        topic: diagnostic.topic,
                        joined: diagnostic.joined || !assist_peer_ids.is_empty(),
                        peer_count: diagnostic.peer_count.max(
                            merge_peer_ids(
                                diagnostic.connected_peers.clone(),
                                assist_peer_ids.clone(),
                            )
                            .len(),
                        ),
                        connected_peers: diagnostic.connected_peers,
                        assist_peer_ids: assist_peer_ids.clone(),
                        configured_peer_ids: diagnostic.configured_peer_ids,
                        missing_peer_ids: diagnostic.missing_peer_ids,
                        last_received_at: diagnostic.last_received_at,
                        status_detail: effective_topic_status_detail(
                            diagnostic.status_detail.as_str(),
                            gossip_peer_count,
                            assist_peer_ids.len(),
                        ),
                        last_error: diagnostic.last_error,
                    }
                })
                .collect(),
            local_author_pubkey: self.current_author_pubkey(),
            discovery,
        })
    }

    pub async fn get_discovery_status(&self) -> Result<DiscoveryStatus> {
        let DiscoverySnapshot {
            mode,
            connect_mode,
            env_locked,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            local_endpoint_id,
            last_discovery_error,
        } = self.transport.discovery().await?;
        let assist_peer_ids = self.assisted_peer_ids().await?;
        Ok(DiscoveryStatus {
            mode,
            connect_mode,
            env_locked,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            assist_peer_ids,
            local_endpoint_id,
            last_discovery_error,
        })
    }

    async fn assisted_peer_ids(&self) -> Result<Vec<String>> {
        Ok(merge_peer_ids(
            self.docs_sync.assist_peer_ids().await?,
            self.blob_service.assist_peer_ids().await?,
        ))
    }

    pub async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.transport.import_ticket(ticket).await?;
        self.docs_sync.import_peer_ticket(ticket).await?;
        self.blob_service.import_peer_ticket(ticket).await?;
        let existing_topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in existing_topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }
        let existing_private_topics = self
            .joined_private_channels
            .lock()
            .await
            .values()
            .map(|state| {
                (
                    state.topic_id.clone(),
                    state.channel_id.as_str().to_string(),
                )
            })
            .collect::<Vec<_>>();
        for (topic_id, channel_id) in existing_private_topics {
            self.restart_private_channel_subscription(topic_id.as_str(), channel_id.as_str())
                .await?;
        }
        let existing_authors = self
            .author_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for author in existing_authors {
            self.restart_author_subscription(author.as_str()).await?;
        }
        let existing_direct_messages = self
            .direct_message_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for peer_pubkey in existing_direct_messages {
            self.restart_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        Ok(())
    }

    pub async fn set_discovery_seeds(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        let effective_seed_peers =
            merge_seed_peers(configured_seed_peers.clone(), bootstrap_seed_peers.clone());
        self.transport
            .configure_discovery(
                mode,
                env_locked,
                configured_seed_peers,
                bootstrap_seed_peers,
            )
            .await?;
        self.docs_sync
            .set_seed_peers(effective_seed_peers.clone())
            .await?;
        self.blob_service
            .set_seed_peers(effective_seed_peers)
            .await?;
        let existing_topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in existing_topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }
        let existing_private_topics = self
            .joined_private_channels
            .lock()
            .await
            .values()
            .map(|state| {
                (
                    state.topic_id.clone(),
                    state.channel_id.as_str().to_string(),
                )
            })
            .collect::<Vec<_>>();
        for (topic_id, channel_id) in existing_private_topics {
            self.restart_private_channel_subscription(topic_id.as_str(), channel_id.as_str())
                .await?;
        }
        let existing_authors = self
            .author_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for author in existing_authors {
            self.restart_author_subscription(author.as_str()).await?;
        }
        let existing_direct_messages = self
            .direct_message_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for peer_pubkey in existing_direct_messages {
            self.restart_direct_message_subscription(peer_pubkey.as_str())
                .await?;
        }
        Ok(())
    }

    pub async fn unsubscribe_topic(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        let private_keys = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(&format!("{topic_id}::")))
            .cloned()
            .collect::<Vec<_>>();
        for key in private_keys {
            if let Some(handle) = self
                .private_channel_subscriptions
                .lock()
                .await
                .remove(key.as_str())
            {
                handle.abort();
            }
            let mut parts = key.splitn(3, "::");
            let _ = parts.next();
            if let Some(channel_id) = parts.next() {
                self.hint_transport
                    .unsubscribe_hints(&private_channel_hint_topic(channel_id))
                    .await?;
            }
        }
        let keys_to_remove = self
            .live_presence_tasks
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(&format!("{topic_id}::")))
            .cloned()
            .collect::<Vec<_>>();
        for key in keys_to_remove {
            let mut parts = key.splitn(3, "::");
            let _ = parts.next();
            let channel_id = parts.next().unwrap_or(PUBLIC_CHANNEL_ID).to_string();
            let session_id = parts.next().unwrap_or_default().to_string();
            self.stop_live_presence_task(topic_id, channel_id.as_str(), session_id.as_str())
                .await;
        }
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await
    }

    pub async fn peer_ticket(&self) -> Result<Option<String>> {
        self.transport.export_ticket().await
    }

    pub async fn blob_media_payload(
        &self,
        hash: &str,
        mime: &str,
    ) -> Result<Option<BlobMediaPayload>> {
        let hash = hash.trim();
        if hash.is_empty() {
            warn!(mime = %mime, "blob media payload fetch skipped because hash was blank");
            return Ok(None);
        }
        info!(hash = %hash, mime = %mime, "blob media payload fetch requested");
        let bytes = match self
            .blob_service
            .fetch_blob(&kukuri_core::BlobHash::new(hash.to_string()))
            .await
        {
            Ok(Some(bytes)) => {
                info!(
                    hash = %hash,
                    mime = %mime,
                    byte_len = bytes.len(),
                    "blob media payload fetch hit"
                );
                bytes
            }
            Ok(None) => {
                warn!(hash = %hash, mime = %mime, "blob media payload fetch miss");
                return Ok(None);
            }
            Err(error) => {
                warn!(
                    hash = %hash,
                    mime = %mime,
                    error = %error,
                    "blob media payload fetch failed"
                );
                return Err(error);
            }
        };
        Ok(Some(BlobMediaPayload {
            bytes_base64: BASE64_STANDARD.encode(bytes),
            mime: mime.to_string(),
        }))
    }

    pub async fn blob_preview_data_url(&self, hash: &str, mime: &str) -> Result<Option<String>> {
        let Some(payload) = self.blob_media_payload(hash, mime).await? else {
            return Ok(None);
        };
        Ok(Some(format!(
            "data:{};base64,{}",
            payload.mime, payload.bytes_base64
        )))
    }

    pub async fn shutdown(&self) {
        let topics_to_unsubscribe = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let private_channels_to_unsubscribe = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter_map(|key| key.split("::").nth(1).map(str::to_owned))
            .collect::<BTreeSet<_>>();
        let handles = {
            let mut subscriptions = self.subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        let private_handles = {
            let mut subscriptions = self.private_channel_subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in private_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        for channel_id in private_channels_to_unsubscribe {
            let _ = self
                .hint_transport
                .unsubscribe_hints(&private_channel_hint_topic(channel_id.as_str()))
                .await;
        }
        for topic_id in topics_to_unsubscribe {
            let _ = self
                .hint_transport
                .unsubscribe_hints(&TopicId::new(topic_id))
                .await;
        }
        let dm_peers_to_unsubscribe = self
            .direct_message_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let dm_handles = {
            let mut subscriptions = self.direct_message_subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in dm_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        for peer_pubkey in dm_peers_to_unsubscribe {
            if let Ok(topic) =
                derive_direct_message_topic(self.keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))
            {
                let _ = self.hint_transport.unsubscribe_hints(&topic).await;
            }
        }
        let author_handles = {
            let mut subscriptions = self.author_subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in author_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        let presence_handles = {
            let mut tasks = self.live_presence_tasks.lock().await;
            tasks.drain().map(|(_, handle)| handle).collect::<Vec<_>>()
        };
        for handle in presence_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
    }

    fn current_author_pubkey(&self) -> String {
        self.keys.public_key_hex()
    }

    async fn reaction_state_for_target(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
    ) -> Result<ReactionStateView> {
        let rows = self
            .projection_store
            .list_reaction_cache_for_target(source_replica_id, target_object_id)
            .await?;
        let current_author = self.current_author_pubkey();
        let mut summary = BTreeMap::<String, ReactionSummaryView>::new();
        let mut my_reactions = Vec::new();
        for row in rows {
            let key_view = reaction_key_view_from_projection(&row);
            if row.status == ObjectStatus::Active {
                summary
                    .entry(row.normalized_reaction_key.clone())
                    .and_modify(|value| value.count += 1)
                    .or_insert_with(|| ReactionSummaryView {
                        reaction_key_kind: key_view.reaction_key_kind.clone(),
                        normalized_reaction_key: key_view.normalized_reaction_key.clone(),
                        emoji: key_view.emoji.clone(),
                        custom_asset: key_view.custom_asset.clone(),
                        count: 1,
                    });
                if row.author_pubkey == current_author {
                    my_reactions.push(key_view);
                }
            }
        }
        Ok(ReactionStateView {
            target_object_id: target_object_id.as_str().to_string(),
            source_replica_id: source_replica_id.as_str().to_string(),
            reaction_summary: summary.into_values().collect(),
            my_reactions,
        })
    }

    async fn maybe_redeem_rotation_grants_for_scope(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<()> {
        match scope {
            TimelineScope::Public => Ok(()),
            TimelineScope::AllJoined => self.maybe_redeem_rotation_grants_for_topic(topic_id).await,
            TimelineScope::Channel { channel_id } => self
                .maybe_redeem_rotation_grants_for_channel(topic_id, channel_id.as_str())
                .await
                .map(|_| ()),
        }
    }

    async fn maybe_redeem_rotation_grants_for_topic(&self, topic_id: &str) -> Result<()> {
        for state in self.joined_private_channel_states_for_topic(topic_id).await {
            self.maybe_redeem_rotation_grants_for_channel(topic_id, state.channel_id.as_str())
                .await?;
        }
        Ok(())
    }

    async fn maybe_redeem_rotation_grants_for_channel(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<bool> {
        let mut redeemed_any = false;
        loop {
            let Some(state) = self
                .joined_private_channel_state(topic_id, channel_id)
                .await
            else {
                return Ok(redeemed_any);
            };
            let local_author = self.current_author_pubkey();
            let replica = current_private_channel_replica_id(&state);
            let grant_doc = fetch_private_channel_rotation_grant_from_replica(
                self.docs_sync.as_ref(),
                &replica,
                local_author.as_str(),
            )
            .await?;
            let grant_doc = if let Some(grant_doc) = grant_doc {
                Some(grant_doc)
            } else {
                if let Err(error) = self.docs_sync.restart_replica_sync(&replica).await {
                    warn!(
                        topic = %topic_id,
                        channel_id = %channel_id,
                        epoch_id = %state.current_epoch_id,
                        error = %error,
                        "failed to restart private channel replica sync while polling epoch handoff"
                    );
                }
                fetch_private_channel_rotation_grant_from_replica(
                    self.docs_sync.as_ref(),
                    &replica,
                    local_author.as_str(),
                )
                .await?
            };
            let Some(grant_doc) = grant_doc else {
                return Ok(redeemed_any);
            };
            let payload =
                match decrypt_private_channel_epoch_handoff_grant(self.keys.as_ref(), &grant_doc) {
                    Ok(payload) => payload,
                    Err(error) => {
                        warn!(
                            topic = %topic_id,
                            channel_id = %channel_id,
                            epoch_id = %state.current_epoch_id,
                            error = %error,
                            "failed to decrypt private channel epoch handoff grant"
                        );
                        return Ok(redeemed_any);
                    }
                };
            if payload.old_epoch_id != state.current_epoch_id
                || private_channel_epoch_capabilities(&state)
                    .iter()
                    .any(|known_epoch| known_epoch.epoch_id == payload.new_epoch_id)
            {
                return Ok(redeemed_any);
            }
            let next_replica =
                private_channel_epoch_replica_id(channel_id, payload.new_epoch_id.as_str());
            self.docs_sync
                .register_private_replica_secret(
                    &next_replica,
                    payload.new_namespace_secret_hex.as_str(),
                )
                .await?;
            if let Err(error) = self.docs_sync.restart_replica_sync(&next_replica).await {
                warn!(
                    topic = %topic_id,
                    channel_id = %channel_id,
                    epoch_id = %payload.new_epoch_id,
                    error = %error,
                    "failed to restart rotated private channel replica sync"
                );
            }
            let (metadata, policy, participants) = match wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &next_replica,
                "private channel epoch handoff sync",
            )
            .await
            {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    warn!(
                        topic = %topic_id,
                        channel_id = %channel_id,
                        epoch_id = %payload.new_epoch_id,
                        error = %error,
                        "failed to load rotated private channel replica"
                    );
                    return Ok(redeemed_any);
                }
            };
            if policy.audience_kind != state.audience_kind
                || policy.epoch_id != payload.new_epoch_id
                || policy.previous_epoch_id.as_deref() != Some(payload.old_epoch_id.as_str())
            {
                warn!(
                    topic = %topic_id,
                    channel_id = %channel_id,
                    epoch_id = %payload.new_epoch_id,
                    audience_kind = ?policy.audience_kind,
                    "private channel epoch handoff payload does not match rotated policy"
                );
                return Ok(redeemed_any);
            }
            let local_pubkey = Pubkey::from(local_author.clone());
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == local_pubkey
                    && participant.epoch_id == policy.epoch_id
            }) {
                persist_private_channel_participant(
                    self.docs_sync.as_ref(),
                    self.keys.as_ref(),
                    &PrivateChannelParticipantDocV1 {
                        channel_id: metadata.channel_id.clone(),
                        topic_id: metadata.topic_id.clone(),
                        epoch_id: policy.epoch_id.clone(),
                        participant_pubkey: local_pubkey,
                        joined_at: Utc::now().timestamp_millis(),
                        is_owner: false,
                        join_mode: Some(PrivateChannelJoinMode::RotationRedeem),
                        sponsor_pubkey: Some(policy.owner_pubkey.clone()),
                        share_token_id: None,
                    },
                    &next_replica,
                )
                .await?;
            }
            let next_state = merged_private_channel_state_from_epoch_join(
                Some(state.clone()),
                metadata.topic_id.as_str(),
                metadata.channel_id.clone(),
                metadata.label.as_str(),
                metadata.creator_pubkey.as_str(),
                policy.owner_pubkey.as_str(),
                state.joined_via_pubkey.as_deref(),
                policy.audience_kind.clone(),
                payload.new_epoch_id.as_str(),
                payload.new_namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            redeemed_any = true;
        }
    }

    async fn private_channel_diagnostics(
        &self,
        state: &JoinedPrivateChannelState,
    ) -> Result<PrivateChannelDiagnostics> {
        let replica = current_private_channel_replica_id(state);
        let sharing_state =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &replica)
                .await?
                .map(|policy| policy.sharing_state)
                .unwrap_or(ChannelSharingState::Open);
        let participants =
            fetch_private_channel_participants_from_replica(self.docs_sync.as_ref(), &replica)
                .await?;
        let participant_count = participants.len();
        let mut stale_participant_count = 0usize;
        if state.audience_kind == ChannelAudienceKind::FriendOnly
            && state.owner_pubkey == self.current_author_pubkey()
        {
            for participant in &participants {
                if participant.is_owner {
                    continue;
                }
                self.ensure_author_subscription(participant.participant_pubkey.as_str())
                    .await?;
                let relationship = self
                    .projection_store
                    .get_author_relationship(
                        self.current_author_pubkey().as_str(),
                        participant.participant_pubkey.as_str(),
                    )
                    .await?;
                if relationship.as_ref().is_some_and(|value| !value.mutual) {
                    stale_participant_count += 1;
                }
            }
        }
        Ok(PrivateChannelDiagnostics {
            sharing_state,
            participant_count,
            stale_participant_count,
            rotation_required: state.audience_kind == ChannelAudienceKind::FriendOnly
                && stale_participant_count > 0,
        })
    }

    async fn joined_private_channel_view_for_state(
        &self,
        state: &JoinedPrivateChannelState,
    ) -> Result<JoinedPrivateChannelView> {
        let diagnostics = self.private_channel_diagnostics(state).await?;
        Ok(JoinedPrivateChannelView {
            topic_id: state.topic_id.clone(),
            channel_id: state.channel_id.as_str().to_string(),
            label: state.label.clone(),
            creator_pubkey: state.creator_pubkey.clone(),
            owner_pubkey: state.owner_pubkey.clone(),
            joined_via_pubkey: state.joined_via_pubkey.clone(),
            audience_kind: state.audience_kind.clone(),
            is_owner: state.owner_pubkey == self.current_author_pubkey(),
            current_epoch_id: state.current_epoch_id.clone(),
            archived_epoch_ids: state
                .archived_epochs
                .iter()
                .map(|epoch| epoch.epoch_id.clone())
                .collect(),
            sharing_state: diagnostics.sharing_state,
            rotation_required: diagnostics.rotation_required,
            participant_count: diagnostics.participant_count,
            stale_participant_count: diagnostics.stale_participant_count,
        })
    }

    async fn private_channel_capability_from_state(
        &self,
        state: &JoinedPrivateChannelState,
    ) -> Result<PrivateChannelCapability> {
        let diagnostics = self.private_channel_diagnostics(state).await?;
        Ok(PrivateChannelCapability {
            topic_id: state.topic_id.clone(),
            channel_id: state.channel_id.as_str().to_string(),
            label: state.label.clone(),
            creator_pubkey: state.creator_pubkey.clone(),
            owner_pubkey: state.owner_pubkey.clone(),
            joined_via_pubkey: state.joined_via_pubkey.clone(),
            audience_kind: state.audience_kind.clone(),
            current_epoch_id: state.current_epoch_id.clone(),
            current_epoch_secret_hex: state.current_epoch_secret_hex.clone(),
            archived_epochs: state.archived_epochs.clone(),
            rotation_required: diagnostics.rotation_required,
            participant_count: diagnostics.participant_count,
            stale_participant_count: diagnostics.stale_participant_count,
            namespace_secret_hex: state.current_epoch_secret_hex.clone(),
        })
    }

    async fn audience_label_for_storage(&self, topic_id: &str, channel_id: &str) -> String {
        if channel_id == PUBLIC_CHANNEL_ID {
            return "Public".to_string();
        }
        self.joined_private_channels
            .lock()
            .await
            .get(joined_private_channel_key(topic_id, channel_id).as_str())
            .map(|channel| channel.label.clone())
            .unwrap_or_else(|| "Private channel".to_string())
    }

    async fn joined_private_channel_states_for_topic(
        &self,
        topic_id: &str,
    ) -> Vec<JoinedPrivateChannelState> {
        self.joined_private_channels
            .lock()
            .await
            .values()
            .filter(|state| state.topic_id == topic_id)
            .cloned()
            .collect()
    }

    async fn joined_private_channel_state(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Option<JoinedPrivateChannelState> {
        self.joined_private_channels
            .lock()
            .await
            .get(joined_private_channel_key(topic_id, channel_id).as_str())
            .cloned()
    }

    async fn ensure_private_channel_access(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
    ) -> Result<()> {
        if self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
            .is_none()
        {
            anyhow::bail!("private channel is not joined");
        }
        Ok(())
    }

    async fn maybe_auto_rotate_private_channel_for_owner(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
        action: PrivateChannelOwnerAction,
    ) -> Result<()> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        if state.owner_pubkey != self.current_author_pubkey() {
            return Ok(());
        }
        match state.audience_kind {
            ChannelAudienceKind::InviteOnly | ChannelAudienceKind::FriendPlus => {
                if matches!(
                    action,
                    PrivateChannelOwnerAction::Write | PrivateChannelOwnerAction::Share
                ) {
                    let _ = self
                        .rotate_private_channel(topic_id, channel_id.as_str())
                        .await?;
                }
            }
            ChannelAudienceKind::FriendOnly => {
                let diagnostics = self.private_channel_diagnostics(&state).await?;
                if diagnostics.rotation_required {
                    let _ = self
                        .rotate_private_channel(topic_id, channel_id.as_str())
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn private_channel_state_for_owner_action(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
        action: PrivateChannelOwnerAction,
    ) -> Result<JoinedPrivateChannelState> {
        self.maybe_redeem_rotation_grants_for_channel(topic_id, channel_id.as_str())
            .await?;
        self.ensure_private_channel_access(topic_id, channel_id)
            .await?;
        self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
            .await?;
        self.maybe_auto_rotate_private_channel_for_owner(topic_id, channel_id, action)
            .await?;
        self.maybe_redeem_rotation_grants_for_channel(topic_id, channel_id.as_str())
            .await?;
        self.ensure_private_channel_access(topic_id, channel_id)
            .await?;
        self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
            .await?;
        let state = self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
            .ok_or_else(|| anyhow::anyhow!("private channel is not joined"))?;
        if private_channel_rotation_is_pending(self.docs_sync.as_ref(), self.keys.as_ref(), &state)
            .await?
        {
            anyhow::bail!(
                "private channel epoch handoff is pending; wait for automatic redemption or use a fresh access token"
            );
        }
        Ok(state)
    }

    async fn private_channel_write_state(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
    ) -> Result<JoinedPrivateChannelState> {
        self.private_channel_state_for_owner_action(
            topic_id,
            channel_id,
            PrivateChannelOwnerAction::Write,
        )
        .await
    }

    async fn register_joined_private_channel(
        &self,
        state: JoinedPrivateChannelState,
    ) -> Result<()> {
        register_private_channel_replica_secrets(self.docs_sync.as_ref(), &state).await?;
        self.joined_private_channels.lock().await.insert(
            joined_private_channel_key(state.topic_id.as_str(), state.channel_id.as_str()),
            state.clone(),
        );
        self.ensure_private_channel_subscription(
            state.topic_id.as_str(),
            state.channel_id.as_str(),
        )
        .await?;
        Ok(())
    }

    async fn ensure_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        self.spawn_private_channel_subscription(state).await
    }

    async fn ensure_joined_private_channel_subscriptions(&self, topic_id: &str) -> Result<()> {
        for state in self.joined_private_channel_states_for_topic(topic_id).await {
            self.ensure_private_channel_subscription(topic_id, state.channel_id.as_str())
                .await?;
        }
        Ok(())
    }

    async fn restart_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let prefix = joined_private_channel_subscription_prefix(topic_id, channel_id);
        let keys = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(prefix.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            if let Some(handle) = self
                .private_channel_subscriptions
                .lock()
                .await
                .remove(key.as_str())
            {
                handle.abort();
            }
        }
        self.hint_transport
            .unsubscribe_hints(&private_channel_hint_topic(channel_id))
            .await?;
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            return Ok(());
        };
        self.spawn_private_channel_subscription(state).await
    }

    async fn spawn_private_channel_subscription(
        &self,
        state: JoinedPrivateChannelState,
    ) -> Result<()> {
        let docs_sync = Arc::clone(&self.docs_sync);
        for epoch in private_channel_epoch_capabilities(&state) {
            let replica = private_channel_replica_for_epoch(
                state.channel_id.as_str(),
                epoch.epoch_id.as_str(),
            );
            let key = joined_private_channel_subscription_key(
                state.topic_id.as_str(),
                state.channel_id.as_str(),
                &replica,
            );
            if self
                .private_channel_subscriptions
                .lock()
                .await
                .contains_key(key.as_str())
            {
                continue;
            }
            docs_sync
                .register_private_replica_secret(&replica, epoch.namespace_secret_hex.as_str())
                .await?;
            self.spawn_subscription_task(
                state.topic_id.as_str(),
                Some(state.channel_id.clone()),
                replica,
                private_channel_hint_topic(state.channel_id.as_str()),
                Some(key),
            )
            .await?;
        }
        Ok(())
    }

    async fn spawn_subscription_task(
        &self,
        topic_id: &str,
        channel_id: Option<ChannelId>,
        replica: ReplicaId,
        hint_topic: TopicId,
        private_key: Option<String>,
    ) -> Result<()> {
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let blob_service = Arc::clone(&self.blob_service);
        let hint_transport = Arc::clone(&self.hint_transport);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let topic = topic_id.to_string();
        let storage_channel_id = channel_storage_id(channel_id.as_ref());
        docs_sync.open_replica(&replica).await?;
        let mut doc_stream = docs_sync.subscribe_replica(&replica).await?;
        let mut hint_stream = hint_transport.subscribe_hints(&hint_topic).await?;
        let replica_for_task = replica.clone();
        let hint_topic_for_task = hint_topic.clone();
        let handle = tokio::spawn(async move {
            let _ = hydrate_subscription_state_with_services(
                docs_sync.as_ref(),
                blob_service.as_ref(),
                projection_store.as_ref(),
                topic.as_str(),
                &replica_for_task,
            )
            .await;
            loop {
                tokio::select! {
                    Some(event) = doc_stream.next() => {
                        if let Ok(event) = event {
                            if let Some(source_peer) = event.source_peer.as_deref()
                                && let Err(error) = blob_service.learn_peer(source_peer).await
                            {
                                warn!(
                                    topic = %topic,
                                    source_peer = %source_peer,
                                    error = %error,
                                    "failed to learn blob peer from docs sync event"
                                );
                            }
                            if let Ok(count) = hydrate_subscription_state_with_services(
                                docs_sync.as_ref(),
                                blob_service.as_ref(),
                                projection_store.as_ref(),
                                topic.as_str(),
                                &replica_for_task,
                            ).await
                            && count > 0
                            {
                                *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            }
                        }
                    }
                    Some(event) = hint_stream.next() => {
                        if hint_targets_topic(&event.hint, topic.as_str()) {
                            match &event.hint {
                                GossipHint::LivePresence { session_id, author, ttl_ms, .. } => {
                                    let now = Utc::now().timestamp_millis();
                                    let _ = projection_store
                                        .upsert_live_presence(
                                            topic.as_str(),
                                            storage_channel_id.as_str(),
                                            session_id.as_str(),
                                            author.as_str(),
                                            now + i64::from(*ttl_ms),
                                            now,
                                        )
                                        .await;
                                    let _ = projection_store.clear_expired_live_presence(now).await;
                                    *last_sync.lock().await = Some(now);
                                }
                                _ => {
                                    if let Ok(count) = hydrate_subscription_state_with_services(
                                        docs_sync.as_ref(),
                                        blob_service.as_ref(),
                                        projection_store.as_ref(),
                                        topic.as_str(),
                                        &replica_for_task,
                                    ).await
                                    && count > 0
                                    {
                                        *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                                    }
                                }
                            }
                        }
                    }
                    else => {
                        let _ = hint_transport.unsubscribe_hints(&hint_topic_for_task).await;
                        break;
                    },
                }
            }
        });

        if let Some(private_key) = private_key {
            self.private_channel_subscriptions
                .lock()
                .await
                .insert(private_key, handle);
        } else {
            self.subscriptions
                .lock()
                .await
                .insert(topic_id.to_string(), handle);
        }
        Ok(())
    }

    async fn stop_live_presence_task(&self, topic_id: &str, channel_id: &str, session_id: &str) {
        let key = live_presence_task_key(topic_id, channel_id, session_id);
        let handle = self.live_presence_tasks.lock().await.remove(key.as_str());
        if let Some(handle) = handle {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
    }

    async fn cleanup_ended_live_presence_tasks(&self, rows: &[LiveSessionProjectionRow]) {
        for row in rows {
            if row.status == LiveSessionStatus::Ended {
                self.stop_live_presence_task(
                    row.topic_id.as_str(),
                    row.channel_id.as_str(),
                    row.session_id.as_str(),
                )
                .await;
            }
        }
    }

    async fn apply_live_presence(
        &self,
        topic_id: &str,
        channel_id: Option<&ChannelId>,
        session_id: &str,
        ttl_ms: u32,
    ) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let author = self.current_author_pubkey();
        self.projection_store
            .upsert_live_presence(
                topic_id,
                channel_storage_id(channel_id).as_str(),
                session_id,
                author.as_str(),
                now + i64::from(ttl_ms),
                now,
            )
            .await?;
        self.projection_store
            .clear_expired_live_presence(now)
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, channel_id),
                GossipHint::LivePresence {
                    topic_id: TopicId::new(topic_id),
                    session_id: session_id.to_string(),
                    author: Pubkey::from(author),
                    ttl_ms,
                },
            )
            .await?;
        Ok(())
    }

    async fn persist_live_session_manifest(
        &self,
        replica: &ReplicaId,
        topic_id: &str,
        manifest: LiveSessionManifestBlobV1,
        created_at: i64,
        last_envelope_id: EnvelopeId,
    ) -> Result<LiveSessionStateDocV1> {
        let now = Utc::now().timestamp_millis();
        let stored =
            store_manifest_blob(self.blob_service.as_ref(), &manifest, LIVE_MANIFEST_MIME).await?;
        let state = LiveSessionStateDocV1 {
            session_id: manifest.session_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: manifest.channel_id.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            created_at,
            updated_at: now,
            status: manifest.status.clone(),
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            last_envelope_id,
        };
        persist_live_session_state(self.docs_sync.as_ref(), replica, &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    async fn persist_game_room_manifest(
        &self,
        replica: &ReplicaId,
        topic_id: &str,
        manifest: GameRoomManifestBlobV1,
        created_at: i64,
        last_envelope_id: EnvelopeId,
    ) -> Result<GameRoomStateDocV1> {
        let now = Utc::now().timestamp_millis();
        let stored =
            store_manifest_blob(self.blob_service.as_ref(), &manifest, GAME_MANIFEST_MIME).await?;
        let state = GameRoomStateDocV1 {
            room_id: manifest.room_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: manifest.channel_id.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            created_at,
            updated_at: now,
            status: manifest.status.clone(),
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            last_envelope_id,
        };
        persist_game_room_state(self.docs_sync.as_ref(), replica, &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    async fn fetch_live_session_state_and_manifest(
        &self,
        topic_id: &str,
        session_id: &str,
    ) -> Result<Option<(ReplicaId, LiveSessionStateDocV1, LiveSessionManifestBlobV1)>> {
        for replica in subscription_replicas_for_topic(
            topic_id,
            self.joined_private_channel_states_for_topic(topic_id).await,
        ) {
            let Some(state) = fetch_live_session_state_from_replica(
                self.docs_sync.as_ref(),
                &replica,
                session_id,
            )
            .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<LiveSessionManifestBlobV1>(
                self.blob_service.as_ref(),
                &state.current_manifest,
            )
            .await?
            else {
                continue;
            };
            return Ok(Some((replica, state, manifest)));
        }
        Ok(None)
    }

    async fn fetch_game_room_state_and_manifest(
        &self,
        topic_id: &str,
        room_id: &str,
    ) -> Result<Option<(ReplicaId, GameRoomStateDocV1, GameRoomManifestBlobV1)>> {
        for replica in subscription_replicas_for_topic(
            topic_id,
            self.joined_private_channel_states_for_topic(topic_id).await,
        ) {
            let Some(state) =
                fetch_game_room_state_from_replica(self.docs_sync.as_ref(), &replica, room_id)
                    .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<GameRoomManifestBlobV1>(
                self.blob_service.as_ref(),
                &state.current_manifest,
            )
            .await?
            else {
                continue;
            };
            return Ok(Some((replica, state, manifest)));
        }
        Ok(None)
    }

    async fn build_author_social_view(&self, author_pubkey: &str) -> Result<AuthorSocialView> {
        let profile = self.store.get_profile(author_pubkey).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), author_pubkey)
            .await?;
        Ok(author_social_view_from_parts(
            author_pubkey,
            profile.as_ref(),
            relationship.as_ref(),
        ))
    }

    async fn rebuild_author_relationships(&self) -> Result<()> {
        rebuild_author_relationships_with_services(
            self.store.as_ref(),
            self.projection_store.as_ref(),
            self.current_author_pubkey().as_str(),
        )
        .await
    }

    async fn ensure_author_subscriptions_for_rows(
        &self,
        rows: &[ObjectProjectionRow],
    ) -> Result<()> {
        let mut author_pubkeys = BTreeSet::new();
        for row in rows {
            author_pubkeys.insert(row.author_pubkey.clone());
            if let Some(repost_of) = row.repost_of.as_ref() {
                author_pubkeys.insert(repost_of.source_author_pubkey.as_str().to_string());
            }
        }
        for author_pubkey in author_pubkeys {
            self.ensure_author_subscription(author_pubkey.as_str())
                .await?;
        }
        Ok(())
    }

    async fn ensure_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        if self
            .author_subscriptions
            .lock()
            .await
            .contains_key(author_pubkey.as_str())
        {
            return Ok(());
        }

        self.spawn_author_subscription(author_pubkey.as_str()).await
    }

    async fn restart_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        if let Some(handle) = self
            .author_subscriptions
            .lock()
            .await
            .remove(author_pubkey.as_str())
        {
            handle.abort();
        }
        self.spawn_author_subscription(author_pubkey.as_str()).await
    }

    async fn spawn_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let store = Arc::clone(&self.store);
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let author_key = normalize_author_pubkey(author_pubkey)?;
        let local_author_pubkey = self.current_author_pubkey();
        let replica = author_replica_id(author_key.as_str());
        docs_sync.open_replica(&replica).await?;
        let initial_count = hydrate_author_state_with_services(
            docs_sync.as_ref(),
            store.as_ref(),
            projection_store.as_ref(),
            local_author_pubkey.as_str(),
            author_key.as_str(),
        )
        .await?;
        if initial_count > 0 {
            *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        }
        let mut doc_stream = docs_sync.subscribe_replica(&replica).await?;
        let author_key_for_task = author_key.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = doc_stream.next() => {
                        if event.is_err() {
                            continue;
                        }
                        if let Ok(count) = hydrate_author_state_with_services(
                            docs_sync.as_ref(),
                            store.as_ref(),
                            projection_store.as_ref(),
                            local_author_pubkey.as_str(),
                            author_key_for_task.as_str(),
                        ).await
                        && count > 0
                        {
                            *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                        }
                    }
                    else => break,
                }
            }
        });
        self.author_subscriptions
            .lock()
            .await
            .insert(author_key, handle);
        Ok(())
    }

    async fn direct_message_send_enabled(&self, peer_pubkey: &str) -> Result<bool> {
        Ok(self
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), peer_pubkey)
            .await?
            .as_ref()
            .is_some_and(|relationship| relationship.mutual))
    }

    async fn direct_message_status_view(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageStatusView> {
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        let send_enabled = self.direct_message_send_enabled(peer_pubkey).await?;
        let peer_count = if send_enabled {
            self.direct_message_topic_peer_count(peer_pubkey).await?
        } else {
            0
        };
        let pending_outbox_count = self
            .projection_store
            .list_direct_message_outbox()
            .await?
            .into_iter()
            .filter(|row| row.peer_pubkey == peer_pubkey)
            .count();
        Ok(DirectMessageStatusView {
            peer_pubkey: peer_pubkey.to_string(),
            dm_id,
            mutual: send_enabled,
            send_enabled,
            peer_count,
            pending_outbox_count,
        })
    }

    async fn ensure_direct_message_conversation_row(&self, peer_pubkey: &str) -> Result<()> {
        if self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?
            .is_some()
        {
            return Ok(());
        }
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        self.projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id,
                peer_pubkey: peer_pubkey.to_string(),
                updated_at: Utc::now().timestamp_millis(),
                last_message_at: None,
                last_message_id: None,
                last_message_preview: None,
            })
            .await
    }

    async fn refresh_direct_message_conversation(&self, peer_pubkey: &str) -> Result<()> {
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        let existing = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?;
        let page = self
            .projection_store
            .list_direct_message_messages(dm_id.as_str(), None, 1)
            .await?;
        let (updated_at, last_message_at, last_message_id, last_message_preview) =
            if let Some(message) = page.items.first() {
                (
                    message.created_at,
                    Some(message.created_at),
                    Some(message.message_id.clone()),
                    Some(direct_message_preview(message)),
                )
            } else if let Some(existing) = existing.as_ref() {
                (existing.updated_at, None, None, None)
            } else if self.direct_message_send_enabled(peer_pubkey).await? {
                (Utc::now().timestamp_millis(), None, None, None)
            } else {
                return Ok(());
            };
        self.projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id,
                peer_pubkey: peer_pubkey.to_string(),
                updated_at,
                last_message_at,
                last_message_id,
                last_message_preview,
            })
            .await
    }

    async fn direct_message_conversation_view(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageConversationView> {
        let conversation = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?
            .ok_or_else(|| anyhow::anyhow!("direct message conversation is not initialized"))?;
        let profile = self.store.get_profile(peer_pubkey).await?;
        let status = self.direct_message_status_view(peer_pubkey).await?;
        Ok(DirectMessageConversationView {
            dm_id: conversation.dm_id,
            peer_pubkey: peer_pubkey.to_string(),
            peer_name: profile.as_ref().and_then(|value| value.name.clone()),
            peer_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            peer_picture: profile.as_ref().and_then(|value| value.picture.clone()),
            peer_picture_asset: profile_asset_view_from_ref(
                profile
                    .as_ref()
                    .and_then(|value| value.picture_asset.as_ref()),
            ),
            updated_at: conversation.updated_at,
            last_message_at: conversation.last_message_at,
            last_message_id: conversation.last_message_id,
            last_message_preview: conversation.last_message_preview,
            status,
        })
    }

    async fn direct_message_message_view(
        &self,
        row: DirectMessageMessageRow,
    ) -> Result<DirectMessageMessageView> {
        Ok(DirectMessageMessageView {
            dm_id: row.dm_id,
            message_id: row.message_id,
            sender_pubkey: row.sender_pubkey,
            recipient_pubkey: row.recipient_pubkey,
            created_at: row.created_at,
            text: row.text.unwrap_or_default(),
            reply_to_message_id: row.reply_to_message_id,
            attachments: direct_message_attachment_views(
                self.blob_service.as_ref(),
                row.attachment_manifest.as_ref(),
            )
            .await?,
            outgoing: row.outgoing,
            delivered: row.acked_at.is_some() || !row.outgoing,
        })
    }

    async fn ensure_direct_message_subscription(&self, peer_pubkey: &str) -> Result<()> {
        if !self.direct_message_send_enabled(peer_pubkey).await? {
            return Ok(());
        }
        let mut subscriptions = self.direct_message_subscriptions.lock().await;
        if subscriptions
            .get(peer_pubkey)
            .is_some_and(|handle| !handle.is_finished())
        {
            return Ok(());
        }
        subscriptions.remove(peer_pubkey);
        drop(subscriptions);
        self.spawn_direct_message_subscription(peer_pubkey).await
    }

    async fn restart_direct_message_subscription(&self, peer_pubkey: &str) -> Result<()> {
        if let Some(handle) = self
            .direct_message_subscriptions
            .lock()
            .await
            .remove(peer_pubkey)
        {
            handle.abort();
        }
        let topic = derive_direct_message_topic(self.keys.as_ref(), &Pubkey::from(peer_pubkey))?;
        self.hint_transport.unsubscribe_hints(&topic).await?;
        if self.direct_message_send_enabled(peer_pubkey).await? {
            self.spawn_direct_message_subscription(peer_pubkey).await?;
        }
        Ok(())
    }

    async fn spawn_direct_message_subscription(&self, peer_pubkey: &str) -> Result<()> {
        let projection_store = Arc::clone(&self.projection_store);
        let blob_service = Arc::clone(&self.blob_service);
        let hint_transport = Arc::clone(&self.hint_transport);
        let transport = Arc::clone(&self.transport);
        let keys = Arc::clone(&self.keys);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let local_author_pubkey = self.current_author_pubkey();
        let topic =
            derive_direct_message_topic(keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))?;
        let mut hint_stream = hint_transport.subscribe_hints(&topic).await?;
        let topic_for_task = topic.clone();
        let peer_for_task = peer_pubkey.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(
                DIRECT_MESSAGE_RETRY_INTERVAL_MS,
            ));
            let _ = AppService::flush_direct_message_outbox_for_peer_with_services(
                projection_store.as_ref(),
                hint_transport.as_ref(),
                transport.as_ref(),
                local_author_pubkey.as_str(),
                keys.as_ref(),
                peer_for_task.as_str(),
            )
            .await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let _ = AppService::flush_direct_message_outbox_for_peer_with_services(
                            projection_store.as_ref(),
                            hint_transport.as_ref(),
                            transport.as_ref(),
                            local_author_pubkey.as_str(),
                            keys.as_ref(),
                            peer_for_task.as_str(),
                        ).await;
                    }
                    Some(event) = hint_stream.next() => {
                        if !matches!(
                            &event.hint,
                            GossipHint::DirectMessageFrame { topic_id, .. } | GossipHint::DirectMessageAck { topic_id, .. }
                            if topic_id.as_str() == topic_for_task.as_str()
                        ) {
                            continue;
                        }
                        if let Err(error) = blob_service.learn_peer(event.source_peer.as_str()).await {
                            warn!(
                                peer_pubkey = %peer_for_task,
                                source_peer = %event.source_peer,
                                error = %error,
                                "failed to learn direct message blob peer"
                            );
                        }
                        match AppService::handle_direct_message_hint_with_services(
                            projection_store.as_ref(),
                            blob_service.as_ref(),
                            hint_transport.as_ref(),
                            keys.as_ref(),
                            local_author_pubkey.as_str(),
                            peer_for_task.as_str(),
                            &topic_for_task,
                            &event.hint,
                        ).await {
                            Ok(true) => {
                                *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            }
                            Ok(false) => {}
                            Err(error) => {
                                warn!(
                                    peer_pubkey = %peer_for_task,
                                    error = %error,
                                    "failed to handle direct message hint"
                                );
                            }
                        }
                    }
                    else => {
                        let _ = hint_transport.unsubscribe_hints(&topic_for_task).await;
                        break;
                    }
                }
            }
        });
        self.direct_message_subscriptions
            .lock()
            .await
            .insert(peer_pubkey, handle);
        Ok(())
    }

    async fn handle_direct_message_hint_with_services(
        projection_store: &dyn ProjectionStore,
        blob_service: &dyn BlobService,
        hint_transport: &dyn HintTransport,
        keys: &KukuriKeys,
        local_author_pubkey: &str,
        peer_pubkey: &str,
        topic: &TopicId,
        hint: &GossipHint,
    ) -> Result<bool> {
        match hint {
            GossipHint::DirectMessageFrame {
                dm_id,
                message_id,
                frame_hash,
                ..
            } => {
                AppService::ingest_direct_message_frame_with_services(
                    projection_store,
                    blob_service,
                    hint_transport,
                    keys,
                    local_author_pubkey,
                    peer_pubkey,
                    topic,
                    dm_id.as_str(),
                    message_id.as_str(),
                    frame_hash,
                )
                .await
            }
            GossipHint::DirectMessageAck { ack, .. } => {
                ack.verify()?;
                if ack.sender.as_str() != peer_pubkey
                    || ack.recipient.as_str() != local_author_pubkey
                {
                    return Ok(false);
                }
                projection_store
                    .set_direct_message_acked_at(
                        ack.dm_id.as_str(),
                        ack.message_id.as_str(),
                        ack.acked_at,
                    )
                    .await?;
                projection_store
                    .remove_direct_message_outbox(ack.dm_id.as_str(), ack.message_id.as_str())
                    .await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn ingest_direct_message_frame_with_services(
        projection_store: &dyn ProjectionStore,
        blob_service: &dyn BlobService,
        hint_transport: &dyn HintTransport,
        keys: &KukuriKeys,
        local_author_pubkey: &str,
        peer_pubkey: &str,
        topic: &TopicId,
        dm_id: &str,
        message_id: &str,
        frame_hash: &kukuri_core::BlobHash,
    ) -> Result<bool> {
        let expected_dm_id = direct_message_id_for_participants(
            &Pubkey::from(local_author_pubkey),
            &Pubkey::from(peer_pubkey),
        );
        if dm_id != expected_dm_id {
            return Ok(false);
        }
        let Some(frame_bytes) = blob_service.fetch_blob(frame_hash).await? else {
            return Ok(false);
        };
        let frame: DirectMessageFrameV1 = serde_json::from_slice(frame_bytes.as_slice())
            .context("failed to decode direct message frame blob")?;
        if frame.message_id != message_id || frame.dm_id != dm_id {
            return Ok(false);
        }
        if frame.sender.as_str() != peer_pubkey || frame.recipient.as_str() != local_author_pubkey {
            return Ok(false);
        }
        let payload = decrypt_direct_message_frame(keys, &frame)?;
        let ack = build_direct_message_ack(
            keys,
            dm_id,
            message_id,
            &frame.sender,
            Utc::now().timestamp_millis(),
        )?;
        if projection_store
            .has_direct_message_tombstone(dm_id, message_id)
            .await?
        {
            hint_transport
                .publish_hint(
                    topic,
                    GossipHint::DirectMessageAck {
                        topic_id: topic.clone(),
                        ack,
                    },
                )
                .await?;
            return Ok(false);
        }
        if projection_store
            .get_direct_message_message(dm_id, message_id)
            .await?
            .is_some()
        {
            hint_transport
                .publish_hint(
                    topic,
                    GossipHint::DirectMessageAck {
                        topic_id: topic.clone(),
                        ack,
                    },
                )
                .await?;
            return Ok(false);
        }
        let local_manifest = materialize_direct_message_manifest(
            blob_service,
            keys,
            &frame.sender,
            frame.message_id.as_str(),
            payload.attachment_manifest.as_ref(),
        )
        .await?;
        projection_store
            .put_direct_message_message(DirectMessageMessageRow {
                dm_id: dm_id.to_string(),
                message_id: message_id.to_string(),
                sender_pubkey: frame.sender.as_str().to_string(),
                recipient_pubkey: frame.recipient.as_str().to_string(),
                created_at: frame.created_at,
                text: payload.text,
                reply_to_message_id: payload.reply_to,
                attachment_manifest: local_manifest,
                outgoing: false,
                acked_at: None,
            })
            .await?;
        projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id: dm_id.to_string(),
                peer_pubkey: peer_pubkey.to_string(),
                updated_at: frame.created_at,
                last_message_at: Some(frame.created_at),
                last_message_id: Some(message_id.to_string()),
                last_message_preview: projection_store
                    .get_direct_message_message(dm_id, message_id)
                    .await?
                    .as_ref()
                    .map(direct_message_preview),
            })
            .await?;
        hint_transport
            .publish_hint(
                topic,
                GossipHint::DirectMessageAck {
                    topic_id: topic.clone(),
                    ack,
                },
            )
            .await?;
        Ok(true)
    }

    async fn flush_direct_message_outbox_for_peer_with_services(
        projection_store: &dyn ProjectionStore,
        hint_transport: &dyn HintTransport,
        transport: &dyn Transport,
        local_author_pubkey: &str,
        keys: &KukuriKeys,
        peer_pubkey: &str,
    ) -> Result<usize> {
        let relationship = projection_store
            .get_author_relationship(local_author_pubkey, peer_pubkey)
            .await?;
        if !relationship.as_ref().is_some_and(|value| value.mutual) {
            return Ok(0);
        }
        let topic = derive_direct_message_topic(keys, &Pubkey::from(peer_pubkey))?;
        let peer_count = direct_message_topic_peer_count(transport, &topic).await?;
        if peer_count == 0 {
            return Ok(0);
        }
        let mut published = 0usize;
        let attempted_at = Utc::now().timestamp_millis();
        for row in projection_store.list_direct_message_outbox().await? {
            if row.peer_pubkey != peer_pubkey {
                continue;
            }
            projection_store
                .touch_direct_message_outbox_attempt(
                    row.dm_id.as_str(),
                    row.message_id.as_str(),
                    attempted_at,
                )
                .await?;
            hint_transport
                .publish_hint(
                    &topic,
                    GossipHint::DirectMessageFrame {
                        topic_id: topic.clone(),
                        dm_id: row.dm_id.clone(),
                        message_id: row.message_id.clone(),
                        frame_hash: row.frame_blob_hash.clone(),
                    },
                )
                .await?;
            published += 1;
        }
        Ok(published)
    }

    async fn direct_message_topic_peer_count(&self, peer_pubkey: &str) -> Result<usize> {
        let topic = derive_direct_message_topic(self.keys.as_ref(), &Pubkey::from(peer_pubkey))?;
        direct_message_topic_peer_count(self.transport.as_ref(), &topic).await
    }

    async fn send_direct_message_internal(
        &self,
        peer_pubkey: &str,
        text: Option<&str>,
        reply_to_message_id: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        let text = normalize_optional_text(text.map(str::to_string));
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        if text.is_none() && attachments.is_empty() {
            anyhow::bail!("direct message text or attachment is required");
        }
        let message_id = format!(
            "dm-message-{}-{}",
            Utc::now().timestamp_millis(),
            short_id_suffix(self.current_author_pubkey().as_str())
        );
        if let Some(reply_to_message_id) = reply_to_message_id
            && self
                .projection_store
                .get_direct_message_message(dm_id.as_str(), reply_to_message_id.trim())
                .await?
                .is_none()
        {
            anyhow::bail!("direct message reply target was not found");
        }
        let (local_manifest, encrypted_manifest) = self
            .prepare_direct_message_manifests(peer_pubkey, message_id.as_str(), attachments)
            .await?;
        let created_at = Utc::now().timestamp_millis();
        let frame = encrypt_direct_message_frame(
            self.keys.as_ref(),
            &Pubkey::from(peer_pubkey),
            dm_id.as_str(),
            message_id.as_str(),
            created_at,
            &DirectMessagePayloadV1 {
                text: text.clone(),
                reply_to: normalize_optional_text(reply_to_message_id.map(str::to_string)),
                attachment_manifest: encrypted_manifest,
            },
        )?;
        let frame_bytes =
            serde_json::to_vec(&frame).context("failed to encode direct message frame blob")?;
        let frame_blob = self
            .blob_service
            .put_blob(frame_bytes, DIRECT_MESSAGE_FRAME_MIME)
            .await?;
        self.projection_store
            .put_direct_message_message(DirectMessageMessageRow {
                dm_id: dm_id.clone(),
                message_id: message_id.clone(),
                sender_pubkey: self.current_author_pubkey(),
                recipient_pubkey: peer_pubkey.to_string(),
                created_at,
                text,
                reply_to_message_id: normalize_optional_text(
                    reply_to_message_id.map(str::to_string),
                ),
                attachment_manifest: local_manifest,
                outgoing: true,
                acked_at: None,
            })
            .await?;
        self.projection_store
            .put_direct_message_outbox(DirectMessageOutboxRow {
                dm_id: dm_id.clone(),
                message_id: message_id.clone(),
                peer_pubkey: peer_pubkey.to_string(),
                frame_blob_hash: frame_blob.hash,
                created_at,
                last_attempt_at: None,
            })
            .await?;
        self.refresh_direct_message_conversation(peer_pubkey)
            .await?;
        let _ = Self::flush_direct_message_outbox_for_peer_with_services(
            self.projection_store.as_ref(),
            self.hint_transport.as_ref(),
            self.transport.as_ref(),
            self.current_author_pubkey().as_str(),
            self.keys.as_ref(),
            peer_pubkey,
        )
        .await?;
        Ok(message_id)
    }

    async fn prepare_direct_message_manifests(
        &self,
        peer_pubkey: &str,
        message_id: &str,
        attachments: Vec<PendingAttachment>,
    ) -> Result<(
        Option<DirectMessageAttachmentManifestV1>,
        Option<DirectMessageAttachmentManifestV1>,
    )> {
        if attachments.is_empty() {
            return Ok((None, None));
        }
        let image = attachments
            .iter()
            .find(|attachment| attachment.role == AssetRole::ImageOriginal);
        let video = attachments
            .iter()
            .find(|attachment| attachment.role == AssetRole::VideoManifest);
        let poster = attachments
            .iter()
            .find(|attachment| attachment.role == AssetRole::VideoPoster);
        match (image, video, poster) {
            (Some(image), None, None) => {
                if attachments.len() != 1 || !image.mime.starts_with("image/") {
                    anyhow::bail!(
                        "direct message image attachment must be a single image/* payload"
                    );
                }
                let local_blob = self
                    .blob_service
                    .put_blob(image.bytes.clone(), image.mime.as_str())
                    .await?;
                let encrypted = encrypt_direct_message_attachment(
                    self.keys.as_ref(),
                    &Pubkey::from(peer_pubkey),
                    message_id,
                    "original",
                    image.bytes.as_slice(),
                )?;
                let encrypted_blob = self
                    .blob_service
                    .put_blob(
                        serde_json::to_vec(&encrypted)
                            .context("failed to encode encrypted direct message attachment")?,
                        DIRECT_MESSAGE_ATTACHMENT_MIME,
                    )
                    .await?;
                Ok((
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Image,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: local_blob.hash,
                            mime: image.mime.clone(),
                            bytes: image.bytes.len() as u64,
                            nonce_hex: String::new(),
                        },
                        poster: None,
                    }),
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Image,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: encrypted_blob.hash,
                            mime: image.mime.clone(),
                            bytes: image.bytes.len() as u64,
                            nonce_hex: encrypted.nonce_hex,
                        },
                        poster: None,
                    }),
                ))
            }
            (None, Some(video), Some(poster)) => {
                if attachments.len() != 2
                    || !video.mime.starts_with("video/")
                    || !poster.mime.starts_with("image/")
                {
                    anyhow::bail!(
                        "direct message video attachment must contain one video/* payload and one image/* poster"
                    );
                }
                let local_video = self
                    .blob_service
                    .put_blob(video.bytes.clone(), video.mime.as_str())
                    .await?;
                let local_poster = self
                    .blob_service
                    .put_blob(poster.bytes.clone(), poster.mime.as_str())
                    .await?;
                let encrypted_video = encrypt_direct_message_attachment(
                    self.keys.as_ref(),
                    &Pubkey::from(peer_pubkey),
                    message_id,
                    "original",
                    video.bytes.as_slice(),
                )?;
                let encrypted_poster = encrypt_direct_message_attachment(
                    self.keys.as_ref(),
                    &Pubkey::from(peer_pubkey),
                    message_id,
                    "poster",
                    poster.bytes.as_slice(),
                )?;
                let encrypted_video_blob = self
                    .blob_service
                    .put_blob(
                        serde_json::to_vec(&encrypted_video)
                            .context("failed to encode encrypted direct message video")?,
                        DIRECT_MESSAGE_ATTACHMENT_MIME,
                    )
                    .await?;
                let encrypted_poster_blob = self
                    .blob_service
                    .put_blob(
                        serde_json::to_vec(&encrypted_poster)
                            .context("failed to encode encrypted direct message poster")?,
                        DIRECT_MESSAGE_ATTACHMENT_MIME,
                    )
                    .await?;
                Ok((
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Video,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: local_video.hash,
                            mime: video.mime.clone(),
                            bytes: video.bytes.len() as u64,
                            nonce_hex: String::new(),
                        },
                        poster: Some(DirectMessageEncryptedBlobRefV1 {
                            blob_id: "poster".into(),
                            hash: local_poster.hash,
                            mime: poster.mime.clone(),
                            bytes: poster.bytes.len() as u64,
                            nonce_hex: String::new(),
                        }),
                    }),
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Video,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: encrypted_video_blob.hash,
                            mime: video.mime.clone(),
                            bytes: video.bytes.len() as u64,
                            nonce_hex: encrypted_video.nonce_hex,
                        },
                        poster: Some(DirectMessageEncryptedBlobRefV1 {
                            blob_id: "poster".into(),
                            hash: encrypted_poster_blob.hash,
                            mime: poster.mime.clone(),
                            bytes: poster.bytes.len() as u64,
                            nonce_hex: encrypted_poster.nonce_hex,
                        }),
                    }),
                ))
            }
            _ => anyhow::bail!(
                "direct message attachment must be one image or one video with a poster"
            ),
        }
    }

    async fn ensure_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if self.subscriptions.lock().await.contains_key(topic_id) {
            return Ok(());
        }

        self.spawn_topic_subscription(topic_id).await
    }

    async fn restart_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await?;
        self.spawn_topic_subscription(topic_id).await
    }

    async fn spawn_topic_subscription(&self, topic_id: &str) -> Result<()> {
        self.spawn_subscription_task(
            topic_id,
            None,
            topic_replica_id(topic_id),
            TopicId::new(topic_id),
            None,
        )
        .await
    }

    async fn ingest_event(
        &self,
        replica: &ReplicaId,
        envelope: KukuriEnvelope,
        _stored_blob: Option<StoredBlob>,
        attachments: Vec<(AssetRole, StoredBlob)>,
    ) -> Result<()> {
        self.store.put_envelope(envelope.clone()).await?;
        let mut object = envelope
            .to_post_object()?
            .ok_or_else(|| anyhow::anyhow!("expected timeline envelope"))?;
        if object.object_kind != "repost" {
            object.attachments = attachments
                .iter()
                .map(|(role, stored)| kukuri_core::AssetRef {
                    hash: stored.hash.clone(),
                    mime: stored.mime.clone(),
                    bytes: stored.bytes,
                    role: role.clone(),
                })
                .collect();
        }
        let content = match &object.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => self
                .blob_service
                .fetch_blob(hash)
                .await?
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string()),
        };
        persist_post_object(
            self.docs_sync.as_ref(),
            replica,
            object.clone(),
            envelope.clone(),
        )
        .await?;
        ProjectionStore::put_object_projection(
            self.projection_store.as_ref(),
            projection_row_from_header(&object, content, replica),
        )
        .await?;
        if let PayloadRef::BlobText { hash, .. } = &object.payload_ref {
            ProjectionStore::mark_blob_status(
                self.projection_store.as_ref(),
                hash,
                BlobCacheStatus::Available,
            )
            .await?;
        }
        for (_, attachment) in attachments {
            ProjectionStore::mark_blob_status(
                self.projection_store.as_ref(),
                &attachment.hash,
                BlobCacheStatus::Available,
            )
            .await?;
        }
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    async fn resolve_parent_object(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<KukuriEnvelope>> {
        if let Some(envelope) = self.store.get_envelope(object_id).await? {
            return Ok(Some(envelope));
        }

        let Some(projection) =
            ProjectionStore::get_object_projection(self.projection_store.as_ref(), object_id)
                .await?
        else {
            return Ok(None);
        };

        let object_kind = projection.object_kind.as_str();
        let mut tags = vec![
            vec!["topic".into(), projection.topic_id.clone()],
            vec!["object".into(), object_kind.to_string()],
        ];
        if projection.channel_id != PUBLIC_CHANNEL_ID {
            tags.push(vec!["channel".into(), projection.channel_id.clone()]);
        }

        Ok(Some(KukuriEnvelope {
            id: projection.object_id,
            pubkey: projection.author_pubkey.into(),
            created_at: projection.created_at,
            kind: object_kind.into(),
            tags,
            content: serde_json::to_string(&kukuri_core::KukuriPostEnvelopeContentV1 {
                object_kind: object_kind.into(),
                topic_id: TopicId::new(projection.topic_id.clone()),
                channel_id: channel_id_from_storage(projection.channel_id.as_str()),
                payload_ref: projection.payload_ref.clone(),
                attachments: Vec::new(),
                media_manifest_refs: Vec::new(),
                visibility: if projection.channel_id == PUBLIC_CHANNEL_ID {
                    ObjectVisibility::Public
                } else {
                    ObjectVisibility::Private
                },
                reply_to: projection.reply_to_object_id.clone(),
                root_id: projection.root_object_id.clone(),
                repost_of: projection.repost_of.clone(),
            })?,
            sig: String::new(),
        }))
    }

    async fn ensure_scope_subscriptions(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<()> {
        self.maybe_redeem_rotation_grants_for_scope(topic_id, scope)
            .await?;
        self.ensure_topic_subscription(topic_id).await?;
        match scope {
            TimelineScope::Public => Ok(()),
            TimelineScope::AllJoined => {
                self.ensure_joined_private_channel_subscriptions(topic_id)
                    .await
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
                    .await
            }
        }
    }

    async fn scope_needs_current_private_epoch_hydration(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
        page: &Page<ObjectProjectionRow>,
    ) -> bool {
        let TimelineScope::Channel { channel_id } = scope else {
            return false;
        };
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
        else {
            return false;
        };
        if state.archived_epochs.is_empty() {
            return false;
        }
        let current_replica = current_private_channel_replica_id(&state);
        !page
            .items
            .iter()
            .any(|item| item.source_replica_id == current_replica)
    }

    async fn allowed_channel_ids_for_scope(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<BTreeSet<String>> {
        let mut allowed = BTreeSet::new();
        match scope {
            TimelineScope::Public => {
                allowed.insert(PUBLIC_CHANNEL_ID.to_string());
            }
            TimelineScope::AllJoined => {
                allowed.insert(PUBLIC_CHANNEL_ID.to_string());
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    allowed.insert(state.channel_id.as_str().to_string());
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                allowed.insert(channel_id.as_str().to_string());
            }
        }
        Ok(allowed)
    }

    async fn hydrate_scope_projection(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<usize> {
        let mut hydrated = hydrate_topic_state_with_services(
            self.docs_sync.as_ref(),
            self.blob_service.as_ref(),
            self.projection_store.as_ref(),
            topic_id,
        )
        .await?;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        hydrated += hydrate_subscription_state_with_services(
                            self.docs_sync.as_ref(),
                            self.blob_service.as_ref(),
                            self.projection_store.as_ref(),
                            topic_id,
                            &replica,
                        )
                        .await?;
                    }
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                if let Some(state) = self
                    .joined_private_channel_state(topic_id, channel_id.as_str())
                    .await
                {
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        hydrated += hydrate_subscription_state_with_services(
                            self.docs_sync.as_ref(),
                            self.blob_service.as_ref(),
                            self.projection_store.as_ref(),
                            topic_id,
                            &replica,
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(hydrated)
    }

    async fn maybe_restart_scope_replica_sync(&self, topic_id: &str, scope: &TimelineScope) {
        self.maybe_restart_replica_sync(topic_id, &topic_replica_id(topic_id))
            .await;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    self.maybe_restart_private_channel_subscription(
                        topic_id,
                        state.channel_id.as_str(),
                    )
                    .await;
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        self.maybe_restart_replica_sync(topic_id, &replica).await;
                    }
                }
            }
            TimelineScope::Channel { channel_id } => {
                if let Some(state) = self
                    .joined_private_channel_state(topic_id, channel_id.as_str())
                    .await
                {
                    self.maybe_restart_private_channel_subscription(topic_id, channel_id.as_str())
                        .await;
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        self.maybe_restart_replica_sync(topic_id, &replica).await;
                    }
                }
            }
        }
    }

    async fn maybe_restart_replica_sync(&self, topic_id: &str, replica: &ReplicaId) {
        let key = replica.as_str().to_string();
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self.docs_sync.restart_replica_sync(replica).await {
            warn!(
                topic = %topic_id,
                replica = %replica.as_str(),
                error = %error,
                "failed to restart replica sync"
            );
        }
    }

    async fn maybe_restart_private_channel_subscription(&self, topic_id: &str, channel_id: &str) {
        let key = format!("private-channel:{topic_id}:{channel_id}");
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self
            .restart_private_channel_subscription(topic_id, channel_id)
            .await
        {
            warn!(
                topic = %topic_id,
                channel_id = %channel_id,
                error = %error,
                "failed to restart private channel subscription"
            );
        }
    }

    async fn page_to_view(&self, page: Page<ObjectProjectionRow>) -> Result<TimelineView> {
        let mut items = Vec::with_capacity(page.items.len());
        for row in page.items {
            items.push(self.row_to_view(row).await?);
        }
        Ok(TimelineView {
            items,
            next_cursor: page.next_cursor,
        })
    }

    async fn row_to_view(&self, row: ObjectProjectionRow) -> Result<PostView> {
        let post_object = fetch_post_object_for_projection(
            self.docs_sync.as_ref(),
            &row.source_replica_id,
            row.source_key.as_str(),
        )
        .await?;
        let profile = self.store.get_profile(row.author_pubkey.as_str()).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                row.author_pubkey.as_str(),
            )
            .await?;
        let repost_commentary = normalize_repost_commentary(row.content.clone());
        let content_status = if row.object_kind == "repost" {
            BlobViewStatus::Available
        } else {
            blob_view_status_for_payload(self.blob_service.as_ref(), &row.payload_ref).await?
        };
        let attachments = if row.object_kind == "repost" {
            Vec::new()
        } else if let Some(post_object) = post_object {
            attachment_views(self.blob_service.as_ref(), &post_object).await?
        } else {
            Vec::new()
        };
        let repost_of = match row.repost_of.clone() {
            Some(snapshot) => Some(self.repost_snapshot_to_view(snapshot).await?),
            None => None,
        };
        let audience_label = self
            .audience_label_for_storage(row.topic_id.as_str(), row.channel_id.as_str())
            .await;
        let reaction_state = self
            .reaction_state_for_target(&row.source_replica_id, &row.object_id)
            .await?;

        Ok(PostView {
            object_id: row.object_id.0.clone(),
            envelope_id: row.source_envelope_id.0.clone(),
            author_pubkey: row.author_pubkey.clone(),
            author_name: profile.as_ref().and_then(|profile| profile.name.clone()),
            author_display_name: profile
                .as_ref()
                .and_then(|profile| profile.display_name.clone()),
            following: relationship.as_ref().is_some_and(|value| value.following),
            followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
            mutual: relationship.as_ref().is_some_and(|value| value.mutual),
            friend_of_friend: relationship
                .as_ref()
                .is_some_and(|value| value.friend_of_friend),
            content: row.content.unwrap_or_else(|| "[blob pending]".to_string()),
            content_status,
            attachments,
            created_at: row.created_at,
            reply_to: row.reply_to_object_id.clone().map(|id| id.0),
            root_id: row.root_object_id.clone().map(|id| id.0),
            object_kind: row.object_kind.clone(),
            published_topic_id: Some(row.topic_id.clone()),
            origin_topic_id: Some(row.topic_id.clone()),
            repost_of,
            repost_commentary: repost_commentary.clone(),
            is_threadable: row.object_kind != "repost" || repost_commentary.is_some(),
            channel_id: channel_id_for_view(row.channel_id.as_str()),
            audience_label,
            reaction_summary: reaction_state.reaction_summary,
            my_reactions: reaction_state.my_reactions,
        })
    }

    async fn profile_post_to_view(&self, profile_post: ProfilePost) -> Result<PostView> {
        let profile = self
            .store
            .get_profile(profile_post.author_pubkey.as_str())
            .await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                profile_post.author_pubkey.as_str(),
            )
            .await?;

        Ok(PostView {
            object_id: profile_post.object_id.0.clone(),
            envelope_id: profile_post.object_id.0.clone(),
            author_pubkey: profile_post.author_pubkey.as_str().to_string(),
            author_name: profile.as_ref().and_then(|value| value.name.clone()),
            author_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            following: relationship.as_ref().is_some_and(|value| value.following),
            followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
            mutual: relationship.as_ref().is_some_and(|value| value.mutual),
            friend_of_friend: relationship
                .as_ref()
                .is_some_and(|value| value.friend_of_friend),
            object_kind: profile_post.object_kind,
            content: profile_post.content,
            content_status: BlobViewStatus::Available,
            attachments: attachment_views_from_refs(
                self.blob_service.as_ref(),
                &profile_post.attachments,
            )
            .await?,
            created_at: profile_post.created_at,
            reply_to: profile_post.reply_to_object_id.map(|id| id.0),
            root_id: profile_post.root_id.map(|id| id.0),
            published_topic_id: Some(profile_post.published_topic_id.as_str().to_string()),
            origin_topic_id: Some(profile_post.published_topic_id.as_str().to_string()),
            repost_of: None,
            repost_commentary: None,
            is_threadable: true,
            channel_id: None,
            audience_label: "Public".into(),
            reaction_summary: Vec::new(),
            my_reactions: Vec::new(),
        })
    }

    async fn profile_repost_to_view(&self, profile_repost: ProfileRepost) -> Result<PostView> {
        let profile = self
            .store
            .get_profile(profile_repost.author_pubkey.as_str())
            .await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                profile_repost.author_pubkey.as_str(),
            )
            .await?;

        Ok(PostView {
            object_id: profile_repost.object_id.0.clone(),
            envelope_id: profile_repost.envelope_id.0.clone(),
            author_pubkey: profile_repost.author_pubkey.as_str().to_string(),
            author_name: profile.as_ref().and_then(|value| value.name.clone()),
            author_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            following: relationship.as_ref().is_some_and(|value| value.following),
            followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
            mutual: relationship.as_ref().is_some_and(|value| value.mutual),
            friend_of_friend: relationship
                .as_ref()
                .is_some_and(|value| value.friend_of_friend),
            object_kind: "repost".into(),
            content: profile_repost.commentary.clone().unwrap_or_default(),
            content_status: BlobViewStatus::Available,
            attachments: Vec::new(),
            created_at: profile_repost.created_at,
            reply_to: None,
            root_id: None,
            published_topic_id: Some(profile_repost.published_topic_id.as_str().to_string()),
            origin_topic_id: Some(profile_repost.published_topic_id.as_str().to_string()),
            repost_of: Some(
                self.repost_snapshot_to_view(profile_repost.repost_of)
                    .await?,
            ),
            repost_commentary: profile_repost.commentary.clone(),
            is_threadable: profile_repost.commentary.is_some(),
            channel_id: None,
            audience_label: "Public".into(),
            reaction_summary: Vec::new(),
            my_reactions: Vec::new(),
        })
    }

    async fn repost_snapshot_to_view(
        &self,
        snapshot: RepostSourceSnapshotV1,
    ) -> Result<RepostSourceView> {
        let source_profile = self
            .store
            .get_profile(snapshot.source_author_pubkey.as_str())
            .await?;
        Ok(RepostSourceView {
            source_object_id: snapshot.source_object_id.as_str().to_string(),
            source_topic_id: snapshot.source_topic_id.as_str().to_string(),
            source_author_pubkey: snapshot.source_author_pubkey.as_str().to_string(),
            source_author_name: source_profile.as_ref().and_then(|value| value.name.clone()),
            source_author_display_name: source_profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            source_object_kind: snapshot.source_object_kind,
            content: snapshot.content,
            attachments: attachment_views_from_refs(
                self.blob_service.as_ref(),
                &snapshot.attachments,
            )
            .await?,
            reply_to: snapshot.reply_to_object_id.map(|id| id.0),
            root_id: snapshot.root_id.map(|id| id.0),
        })
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn profile_asset_view_from_ref(asset: Option<&kukuri_core::AssetRef>) -> Option<ProfileAssetView> {
    asset.map(|asset| ProfileAssetView {
        hash: asset.hash.as_str().to_string(),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        role: "profile_avatar".into(),
    })
}

fn normalize_repost_commentary(value: Option<String>) -> Option<String> {
    normalize_optional_text(value)
}

fn content_from_payload_ref(payload_ref: &PayloadRef) -> Option<String> {
    match payload_ref {
        PayloadRef::InlineText { text } => Some(text.clone()),
        PayloadRef::BlobText { .. } => None,
    }
}

fn normalize_author_pubkey(pubkey: &str) -> Result<String> {
    let trimmed = pubkey.trim();
    if trimmed.len() != 64 || !trimmed.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!("invalid author pubkey"));
    }
    Ok(trimmed.to_string())
}

fn author_social_view_from_parts(
    author_pubkey: &str,
    profile: Option<&Profile>,
    relationship: Option<&AuthorRelationshipProjectionRow>,
) -> AuthorSocialView {
    AuthorSocialView {
        author_pubkey: author_pubkey.to_string(),
        name: profile.and_then(|profile| profile.name.clone()),
        display_name: profile.and_then(|profile| profile.display_name.clone()),
        about: profile.and_then(|profile| profile.about.clone()),
        picture: profile.and_then(|profile| profile.picture.clone()),
        picture_asset: profile_asset_view_from_ref(
            profile.and_then(|profile| profile.picture_asset.as_ref()),
        ),
        updated_at: profile.map(|profile| profile.updated_at),
        following: relationship.is_some_and(|relationship| relationship.following),
        followed_by: relationship.is_some_and(|relationship| relationship.followed_by),
        mutual: relationship.is_some_and(|relationship| relationship.mutual),
        friend_of_friend: relationship.is_some_and(|relationship| relationship.friend_of_friend),
        friend_of_friend_via_pubkeys: relationship
            .map(|relationship| relationship.friend_of_friend_via_pubkeys.clone())
            .unwrap_or_default(),
    }
}

async fn rebuild_author_relationships_with_services(
    store: &dyn Store,
    projection_store: &dyn ProjectionStore,
    local_author_pubkey: &str,
) -> Result<()> {
    let following_edges = store
        .list_follow_edges_by_subject(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .collect::<Vec<_>>();
    let followed_by_edges = store
        .list_follow_edges_by_target(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .collect::<Vec<_>>();

    let following = following_edges
        .iter()
        .map(|edge| edge.target_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let followed_by = followed_by_edges
        .iter()
        .map(|edge| edge.subject_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();

    let mut friend_of_friend_via = BTreeMap::<String, BTreeSet<String>>::new();
    for via_author in &following {
        for edge in store
            .list_follow_edges_by_subject(via_author.as_str())
            .await?
        {
            if edge.status != FollowEdgeStatus::Active {
                continue;
            }
            let target = edge.target_pubkey.as_str();
            if target == local_author_pubkey || following.contains(target) {
                continue;
            }
            friend_of_friend_via
                .entry(target.to_string())
                .or_default()
                .insert(via_author.clone());
        }
    }

    let derived_at = Utc::now().timestamp_millis();
    let mut author_pubkeys = BTreeSet::new();
    author_pubkeys.extend(following.iter().cloned());
    author_pubkeys.extend(followed_by.iter().cloned());
    author_pubkeys.extend(friend_of_friend_via.keys().cloned());
    author_pubkeys.remove(local_author_pubkey);

    let rows = author_pubkeys
        .into_iter()
        .map(|author_pubkey| {
            let following_flag = following.contains(author_pubkey.as_str());
            let followed_by_flag = followed_by.contains(author_pubkey.as_str());
            let via_pubkeys = friend_of_friend_via
                .get(author_pubkey.as_str())
                .map(|values| values.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            AuthorRelationshipProjectionRow {
                local_author_pubkey: local_author_pubkey.to_string(),
                author_pubkey: author_pubkey.clone(),
                following: following_flag,
                followed_by: followed_by_flag,
                mutual: following_flag && followed_by_flag,
                friend_of_friend: !following_flag && !via_pubkeys.is_empty(),
                friend_of_friend_via_pubkeys: via_pubkeys,
                derived_at,
            }
        })
        .collect::<Vec<_>>();
    projection_store
        .rebuild_author_relationships(local_author_pubkey, rows)
        .await
}

async fn persist_profile_doc(
    docs_sync: &dyn DocsSync,
    profile: &Profile,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(profile.pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("profile", "latest"),
                value: serde_json::to_value(AuthorProfileDocV1 {
                    author_pubkey: profile.pubkey.clone(),
                    name: profile.name.clone(),
                    display_name: profile.display_name.clone(),
                    about: profile.about.clone(),
                    picture: profile.picture.clone(),
                    picture_asset: profile.picture_asset.clone(),
                    updated_at: profile.updated_at,
                    envelope_id: envelope.id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn persist_profile_post_doc(
    docs_sync: &dyn DocsSync,
    profile_post: &ProfilePost,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(profile_post.author_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("profile/posts", profile_post.object_id.as_str()),
                value: serde_json::to_value(AuthorProfilePostDocV1 {
                    author_pubkey: profile_post.author_pubkey.clone(),
                    profile_topic_id: profile_post.profile_topic_id.clone(),
                    published_topic_id: profile_post.published_topic_id.clone(),
                    object_id: profile_post.object_id.clone(),
                    created_at: profile_post.created_at,
                    object_kind: profile_post.object_kind.clone(),
                    content: profile_post.content.clone(),
                    attachments: profile_post.attachments.clone(),
                    reply_to_object_id: profile_post.reply_to_object_id.clone(),
                    root_id: profile_post.root_id.clone(),
                    envelope_id: envelope.id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn persist_profile_repost_doc(
    docs_sync: &dyn DocsSync,
    profile_repost: &ProfileRepost,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(profile_repost.author_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("profile/reposts", profile_repost.object_id.as_str()),
                value: serde_json::to_value(AuthorProfileRepostDocV1 {
                    author_pubkey: profile_repost.author_pubkey.clone(),
                    profile_topic_id: profile_repost.profile_topic_id.clone(),
                    published_topic_id: profile_repost.published_topic_id.clone(),
                    object_id: profile_repost.object_id.clone(),
                    created_at: profile_repost.created_at,
                    commentary: profile_repost.commentary.clone(),
                    repost_of: profile_repost.repost_of.clone(),
                    envelope_id: envelope.id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn persist_follow_edge_doc(
    docs_sync: &dyn DocsSync,
    edge: &FollowEdge,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(edge.subject_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("graph/follows", edge.target_pubkey.as_str()),
                value: serde_json::to_value(FollowEdgeDocV1 {
                    subject_pubkey: edge.subject_pubkey.clone(),
                    target_pubkey: edge.target_pubkey.clone(),
                    status: edge.status.clone(),
                    updated_at: edge.updated_at,
                    envelope_id: edge.envelope_id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn persist_custom_reaction_asset_doc(
    docs_sync: &dyn DocsSync,
    asset: &CustomReactionAssetDocV1,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(asset.author_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("reactions/assets", &format!("{}/state", asset.asset_id)),
                value: serde_json::to_value(asset)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("reactions/assets", &format!("{}/envelope", asset.asset_id)),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn persist_reaction_doc(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    reaction: &ReactionDocV1,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "reactions",
                    &format!(
                        "{}/{}/state",
                        reaction.target_object_id.as_str(),
                        reaction.reaction_id.as_str()
                    ),
                ),
                value: serde_json::to_value(reaction)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "reactions",
                    &format!(
                        "{}/{}/envelope",
                        reaction.target_object_id.as_str(),
                        reaction.reaction_id.as_str()
                    ),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn hydrate_author_state_with_services(
    docs_sync: &dyn DocsSync,
    store: &dyn Store,
    projection_store: &dyn ProjectionStore,
    local_author_pubkey: &str,
    author_pubkey: &str,
) -> Result<usize> {
    let replica = author_replica_id(author_pubkey);
    let mut count = 0usize;
    if let Some(record) = docs_sync
        .query_replica(&replica, DocQuery::Exact(stable_key("profile", "latest")))
        .await?
        .into_iter()
        .next()
    {
        match serde_json::from_slice::<AuthorProfileDocV1>(record.value.as_slice()) {
            Ok(doc) if doc.author_pubkey.as_str() == author_pubkey => {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                {
                    store.put_envelope(envelope.clone()).await?;
                    if let Some(profile) = parse_profile(&envelope)? {
                        projection_store.upsert_profile_cache(profile).await?;
                    }
                    count += 1;
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring profile doc with mismatched author"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode author profile doc"
                );
            }
        }
    }

    for record in docs_sync
        .query_replica(&replica, DocQuery::Prefix("graph/follows/".into()))
        .await?
    {
        match serde_json::from_slice::<FollowEdgeDocV1>(record.value.as_slice()) {
            Ok(doc) if doc.subject_pubkey.as_str() == author_pubkey => {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                    && let Some(edge) = parse_follow_edge(&envelope)?
                    && edge.target_pubkey == doc.target_pubkey
                    && edge.status == doc.status
                {
                    store.put_envelope(envelope).await?;
                    count += 1;
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring follow doc with mismatched subject"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode follow edge doc"
                );
            }
        }
    }

    rebuild_author_relationships_with_services(store, projection_store, local_author_pubkey)
        .await?;
    Ok(count)
}

async fn fetch_author_envelope_by_id(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    envelope_id: &EnvelopeId,
) -> Result<Option<KukuriEnvelope>> {
    let Some(record) = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("envelopes", envelope_id.as_str())),
        )
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(record.value.as_slice())?;
    envelope.verify()?;
    Ok(Some(envelope))
}

async fn load_custom_reaction_assets_from_author_replica(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Result<Vec<CustomReactionAssetDocV1>> {
    let replica = author_replica_id(author_pubkey);
    let mut items = Vec::new();
    for record in docs_sync
        .query_replica(
            &replica,
            DocQuery::Prefix(stable_key("reactions/assets", "")),
        )
        .await?
    {
        if !record.key.ends_with("/state") {
            continue;
        }
        let doc: CustomReactionAssetDocV1 = serde_json::from_slice(record.value.as_slice())?;
        if doc.author_pubkey.as_str() == author_pubkey {
            items.push(doc);
        }
    }
    Ok(items)
}

async fn load_profile_posts_from_author_replica(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Result<Vec<ProfilePost>> {
    let author_pubkey = normalize_author_pubkey(author_pubkey)?;
    let replica = author_replica_id(author_pubkey.as_str());
    let expected_profile_topic_id = author_profile_topic_id(author_pubkey.as_str());
    let mut items = Vec::new();
    let mut seen_object_ids = BTreeSet::new();

    for record in docs_sync
        .query_replica(&replica, DocQuery::Prefix("profile/posts/".into()))
        .await?
    {
        match serde_json::from_slice::<AuthorProfilePostDocV1>(record.value.as_slice()) {
            Ok(doc)
                if doc.author_pubkey.as_str() == author_pubkey
                    && doc.profile_topic_id == expected_profile_topic_id =>
            {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                {
                    match parse_profile_post(&envelope) {
                        Ok(Some(profile_post))
                            if profile_post.author_pubkey == doc.author_pubkey
                                && profile_post.profile_topic_id == doc.profile_topic_id
                                && profile_post.published_topic_id == doc.published_topic_id
                                && profile_post.object_id == doc.object_id
                                && profile_post.created_at == doc.created_at
                                && profile_post.object_kind == doc.object_kind
                                && profile_post.content == doc.content
                                && profile_post.attachments == doc.attachments
                                && profile_post.reply_to_object_id == doc.reply_to_object_id
                                && profile_post.root_id == doc.root_id =>
                        {
                            if seen_object_ids.insert(profile_post.object_id.clone()) {
                                items.push(profile_post);
                            }
                        }
                        Ok(Some(_)) | Ok(None) => {}
                        Err(error) => {
                            warn!(
                                author_pubkey = %author_pubkey,
                                key = %record.key,
                                envelope_id = %doc.envelope_id.as_str(),
                                error = %error,
                                "ignoring invalid profile post envelope"
                            );
                        }
                    }
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring profile post doc with mismatched author or topic"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode profile post doc"
                );
            }
        }
    }

    Ok(items)
}

async fn load_profile_reposts_from_author_replica(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Result<Vec<ProfileRepost>> {
    let author_pubkey = normalize_author_pubkey(author_pubkey)?;
    let replica = author_replica_id(author_pubkey.as_str());
    let expected_profile_topic_id = author_profile_topic_id(author_pubkey.as_str());
    let mut items = Vec::new();
    let mut seen_object_ids = BTreeSet::new();

    for record in docs_sync
        .query_replica(&replica, DocQuery::Prefix("profile/reposts/".into()))
        .await?
    {
        match serde_json::from_slice::<AuthorProfileRepostDocV1>(record.value.as_slice()) {
            Ok(doc)
                if doc.author_pubkey.as_str() == author_pubkey
                    && doc.profile_topic_id == expected_profile_topic_id =>
            {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                {
                    match parse_profile_repost(&envelope) {
                        Ok(Some(profile_repost))
                            if profile_repost.author_pubkey == doc.author_pubkey
                                && profile_repost.profile_topic_id == doc.profile_topic_id
                                && profile_repost.published_topic_id == doc.published_topic_id
                                && profile_repost.object_id == doc.object_id
                                && profile_repost.created_at == doc.created_at
                                && profile_repost.commentary == doc.commentary
                                && profile_repost.repost_of == doc.repost_of =>
                        {
                            if seen_object_ids.insert(profile_repost.object_id.clone()) {
                                items.push(profile_repost);
                            }
                        }
                        Ok(Some(_)) | Ok(None) => {}
                        Err(error) => {
                            warn!(
                                author_pubkey = %author_pubkey,
                                key = %record.key,
                                envelope_id = %doc.envelope_id.as_str(),
                                error = %error,
                                "ignoring invalid profile repost envelope"
                            );
                        }
                    }
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring profile repost doc with mismatched author or topic"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode profile repost doc"
                );
            }
        }
    }

    Ok(items)
}

fn merge_seed_peers(
    configured_seed_peers: Vec<SeedPeer>,
    bootstrap_seed_peers: Vec<SeedPeer>,
) -> Vec<SeedPeer> {
    let mut deduped = BTreeMap::new();
    for seed_peer in configured_seed_peers
        .into_iter()
        .chain(bootstrap_seed_peers.into_iter())
    {
        let key = match seed_peer.addr_hint.as_deref() {
            Some(addr_hint) => format!("{}@{}", seed_peer.endpoint_id, addr_hint),
            None => seed_peer.endpoint_id.clone(),
        };
        deduped.insert(key, seed_peer);
    }
    deduped.into_values().collect()
}

async fn persist_post_object(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    object: CanonicalPostHeader,
    envelope: KukuriEnvelope,
) -> Result<()> {
    let sort_key = timeline_sort_key(object.created_at, &object.object_id);
    let object_json = serde_json::to_value(&object)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("objects", &format!("{}/state", object.object_id.as_str())),
                value: object_json,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "objects",
                    &format!("{}/envelope", object.object_id.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "indexes/timeline",
                    &format!("{sort_key}/{}", object.object_id.as_str()),
                ),
                value: serde_json::json!({
                    "object_id": object.object_id,
                    "created_at": object.created_at,
                    "object_kind": object.object_kind,
                }),
            },
        )
        .await?;
    let root_id = object
        .root
        .clone()
        .unwrap_or_else(|| object.object_id.clone());
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "indexes/thread",
                    &format!(
                        "{}/{sort_key}/{}",
                        root_id.as_str(),
                        object.object_id.as_str()
                    ),
                ),
                value: serde_json::json!({
                    "object_id": object.object_id,
                    "root_id": root_id,
                    "reply_to": object.reply_to,
                }),
            },
        )
        .await?;
    Ok(())
}

async fn persist_media_manifest(
    replica: &ReplicaId,
    envelope: &KukuriEnvelope,
    manifest: &KukuriMediaManifestV1,
    docs_sync: &dyn DocsSync,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "manifests/media",
                    &format!("{}/state", manifest.manifest_id),
                ),
                value: serde_json::to_value(manifest)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "manifests/media",
                    &format!("{}/envelope", manifest.manifest_id),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    Ok(())
}

async fn persist_live_session_state(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    state: &LiveSessionStateDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

async fn persist_game_room_state(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    state: &GameRoomStateDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

async fn persist_private_channel_metadata(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    metadata: &PrivateChannelMetadataDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "metadata"),
                value: serde_json::to_value(metadata)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "topic"),
                value: serde_json::json!({ "topic_id": metadata.topic_id }),
            },
        )
        .await
}

async fn persist_private_channel_policy(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    policy: &PrivateChannelPolicyDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_policy_envelope(keys, policy)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "policy/envelope"),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn persist_private_channel_participant(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    participant: &PrivateChannelParticipantDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_participant_envelope(keys, participant)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "channels/participants",
                    &format!("{}/envelope", participant.participant_pubkey.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn persist_private_channel_rotation_grant(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    grant: &PrivateChannelEpochHandoffGrantDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_epoch_handoff_grant_envelope(keys, grant)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "channels/rotation-grants",
                    &format!("{}/envelope", grant.recipient_pubkey.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

async fn fetch_private_channel_metadata_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Option<PrivateChannelMetadataDocV1>> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(stable_key("channels", "metadata")))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let mut metadata: PrivateChannelMetadataDocV1 = serde_json::from_slice(&record.value)?;
    if metadata.owner_pubkey.as_str().trim().is_empty() {
        metadata.owner_pubkey = metadata.creator_pubkey.clone();
    }
    Ok(Some(metadata))
}

async fn fetch_private_channel_policy_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Option<PrivateChannelPolicyDocV1>> {
    let Some(record) = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("channels", "policy/envelope")),
        )
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
    envelope.verify()?;
    parse_private_channel_policy(&envelope)
}

async fn fetch_private_channel_participants_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Vec<PrivateChannelParticipantDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Prefix(stable_key("channels/participants", "")),
        )
        .await?;
    let mut items = Vec::new();
    for record in records {
        if !record.key.ends_with("/envelope") {
            continue;
        }
        let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
        envelope.verify()?;
        if let Some(participant) = parse_private_channel_participant(&envelope)? {
            items.push(participant);
        }
    }
    items.sort_by(|left, right| left.participant_pubkey.cmp(&right.participant_pubkey));
    Ok(items)
}

async fn fetch_private_channel_rotation_grant_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    recipient_pubkey: &str,
) -> Result<Option<PrivateChannelEpochHandoffGrantDocV1>> {
    let Some(record) = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key(
                "channels/rotation-grants",
                &format!("{recipient_pubkey}/envelope"),
            )),
        )
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
    envelope.verify()?;
    parse_private_channel_epoch_handoff_grant(&envelope)
}

async fn wait_for_private_channel_epoch_snapshot(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    timeout_label: &str,
) -> Result<(
    PrivateChannelMetadataDocV1,
    PrivateChannelPolicyDocV1,
    Vec<PrivateChannelParticipantDocV1>,
)> {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let metadata = fetch_private_channel_metadata_from_replica(docs_sync, replica).await?;
            let policy = fetch_private_channel_policy_from_replica(docs_sync, replica).await?;
            let participants =
                fetch_private_channel_participants_from_replica(docs_sync, replica).await?;
            let owner_participant_visible = policy.as_ref().is_some_and(|policy| {
                participants.iter().any(|participant| {
                    participant.participant_pubkey == policy.owner_pubkey
                        && participant.epoch_id == policy.epoch_id
                        && participant.is_owner
                })
            });
            if let (Some(metadata), Some(policy)) = (metadata, policy)
                && owner_participant_visible
            {
                return Ok::<_, anyhow::Error>((metadata, policy, participants));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("timed out waiting for {timeout_label}"))?
}

async fn private_channel_rotation_is_pending(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    state: &JoinedPrivateChannelState,
) -> Result<bool> {
    let replica = current_private_channel_replica_id(state);
    let Some(policy) = fetch_private_channel_policy_from_replica(docs_sync, &replica).await? else {
        return Ok(false);
    };
    if policy.sharing_state != ChannelSharingState::Frozen || policy.rotated_at.is_none() {
        return Ok(false);
    }
    let local_author = keys.public_key_hex();
    let Some(grant) = fetch_private_channel_rotation_grant_from_replica(
        docs_sync,
        &replica,
        local_author.as_str(),
    )
    .await?
    else {
        return Ok(false);
    };
    let payload = decrypt_private_channel_epoch_handoff_grant(keys, &grant)?;
    Ok(payload.new_epoch_id != state.current_epoch_id)
}

async fn store_manifest_blob<T: Serialize>(
    blob_service: &dyn BlobService,
    manifest: &T,
    mime: &str,
) -> Result<StoredBlob> {
    let payload = serde_json::to_vec(manifest)?;
    blob_service.put_blob(payload, mime).await
}

async fn fetch_manifest_blob<T: DeserializeOwned>(
    blob_service: &dyn BlobService,
    blob_ref: &ManifestBlobRef,
) -> Result<Option<T>> {
    let Some(bytes) = blob_service.fetch_blob(&blob_ref.hash).await? else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

fn projection_blob_fetch_timeout() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_secs(5)
    } else {
        tokio::time::Duration::from_secs(2)
    }
}

fn projection_blob_status_timeout() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_secs(1)
    } else {
        tokio::time::Duration::from_millis(250)
    }
}

async fn fetch_projection_blob_text(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> Option<String> {
    match tokio::time::timeout(
        projection_blob_fetch_timeout(),
        blob_service.fetch_blob(hash),
    )
    .await
    {
        Ok(Ok(Some(bytes))) => Some(String::from_utf8_lossy(&bytes).to_string()),
        Ok(Ok(None)) | Ok(Err(_)) | Err(_) => None,
    }
}

async fn best_effort_blob_cache_status(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> BlobCacheStatus {
    match tokio::time::timeout(
        projection_blob_status_timeout(),
        blob_service.blob_status(hash),
    )
    .await
    {
        Ok(Ok(status)) => blob_status(status),
        Ok(Err(_)) | Err(_) => BlobCacheStatus::Missing,
    }
}

async fn best_effort_blob_view_status(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> BlobViewStatus {
    match tokio::time::timeout(
        projection_blob_status_timeout(),
        blob_service.blob_status(hash),
    )
    .await
    {
        Ok(Ok(status)) => blob_view_status(status),
        Ok(Err(_)) | Err(_) => BlobViewStatus::Missing,
    }
}

async fn fetch_live_session_state_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    session_id: &str,
) -> Result<Option<LiveSessionStateDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("sessions/live", &format!("{session_id}/state"))),
        )
        .await?;
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&record.value)?))
}

async fn fetch_game_room_state_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    room_id: &str,
) -> Result<Option<GameRoomStateDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("sessions/game", &format!("{room_id}/state"))),
        )
        .await?;
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&record.value)?))
}

fn live_projection_row_from_state(
    state: &LiveSessionStateDocV1,
    manifest: &LiveSessionManifestBlobV1,
    topic_id: &str,
    source_replica_id: &ReplicaId,
) -> LiveSessionProjectionRow {
    LiveSessionProjectionRow {
        session_id: state.session_id.clone(),
        topic_id: topic_id.to_string(),
        channel_id: channel_storage_id(state.channel_id.as_ref()),
        host_pubkey: state.owner_pubkey.as_str().to_string(),
        title: manifest.title.clone(),
        description: manifest.description.clone(),
        status: state.status.clone(),
        started_at: manifest.started_at,
        ended_at: manifest.ended_at,
        updated_at: state.updated_at,
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
        viewer_count: 0,
    }
}

fn game_projection_row_from_state(
    state: &GameRoomStateDocV1,
    manifest: &GameRoomManifestBlobV1,
    topic_id: &str,
    source_replica_id: &ReplicaId,
) -> GameRoomProjectionRow {
    GameRoomProjectionRow {
        room_id: state.room_id.clone(),
        topic_id: topic_id.to_string(),
        channel_id: channel_storage_id(state.channel_id.as_ref()),
        host_pubkey: state.owner_pubkey.as_str().to_string(),
        title: manifest.title.clone(),
        description: manifest.description.clone(),
        status: state.status.clone(),
        phase_label: manifest.phase_label.clone(),
        scores: manifest.scores.clone(),
        updated_at: state.updated_at,
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

fn projection_row_from_header(
    header: &CanonicalPostHeader,
    content: Option<String>,
    source_replica_id: &ReplicaId,
) -> ObjectProjectionRow {
    let source_blob_hash = match &header.payload_ref {
        PayloadRef::BlobText { hash, .. } => Some(hash.clone()),
        PayloadRef::InlineText { .. } => None,
    };
    ObjectProjectionRow {
        object_id: header.object_id.clone(),
        topic_id: header.topic_id.as_str().to_string(),
        channel_id: channel_storage_id(header.channel_id.as_ref()),
        author_pubkey: header.author.as_str().to_string(),
        created_at: header.created_at,
        object_kind: header.object_kind.clone(),
        root_object_id: header.root.clone(),
        reply_to_object_id: header.reply_to.clone(),
        payload_ref: header.payload_ref.clone(),
        content,
        repost_of: header.repost_of.clone(),
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("objects", &format!("{}/state", header.object_id.as_str())),
        source_envelope_id: header.envelope_id.clone(),
        source_blob_hash,
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

fn reaction_projection_row_from_doc(
    reaction: &ReactionDocV1,
    source_replica_id: &ReplicaId,
) -> ReactionProjectionRow {
    ReactionProjectionRow {
        source_replica_id: source_replica_id.clone(),
        target_object_id: reaction.target_object_id.clone(),
        reaction_id: reaction.reaction_id.clone(),
        author_pubkey: reaction.author_pubkey.as_str().to_string(),
        created_at: reaction.created_at,
        updated_at: reaction.updated_at,
        reaction_key_kind: reaction.reaction_key_kind.clone(),
        normalized_reaction_key: reaction.normalized_reaction_key.clone(),
        emoji: reaction.emoji.clone(),
        custom_asset_id: reaction.custom_asset_id.clone(),
        custom_asset_snapshot: reaction.custom_asset_snapshot.clone(),
        status: reaction.status.clone(),
        source_key: stable_key(
            "reactions",
            &format!(
                "{}/{}/state",
                reaction.target_object_id.as_str(),
                reaction.reaction_id.as_str()
            ),
        ),
        source_envelope_id: reaction.envelope_id.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

fn custom_reaction_asset_view_from_snapshot(
    snapshot: &CustomReactionAssetSnapshotV1,
) -> CustomReactionAssetView {
    CustomReactionAssetView {
        asset_id: snapshot.asset_id.clone(),
        owner_pubkey: snapshot.owner_pubkey.as_str().to_string(),
        blob_hash: snapshot.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(
            snapshot.search_key.as_str(),
            snapshot.asset_id.as_str(),
        ),
        mime: snapshot.mime.clone(),
        bytes: snapshot.bytes,
        width: snapshot.width,
        height: snapshot.height,
    }
}

fn custom_reaction_asset_view_from_doc(
    asset: &CustomReactionAssetDocV1,
) -> CustomReactionAssetView {
    CustomReactionAssetView {
        asset_id: asset.asset_id.clone(),
        owner_pubkey: asset.author_pubkey.as_str().to_string(),
        blob_hash: asset.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(asset.search_key.as_str(), asset.asset_id.as_str()),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        width: asset.width,
        height: asset.height,
    }
}

fn bookmarked_custom_reaction_view_from_row(
    row: BookmarkedCustomReactionRow,
) -> BookmarkedCustomReactionView {
    let asset_id = row.asset_id;
    BookmarkedCustomReactionView {
        asset_id: asset_id.clone(),
        owner_pubkey: row.owner_pubkey,
        blob_hash: row.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(row.search_key.as_str(), asset_id.as_str()),
        mime: row.mime,
        bytes: row.bytes,
        width: row.width,
        height: row.height,
    }
}

fn recent_reaction_view_from_projection(row: &ReactionProjectionRow) -> RecentReactionView {
    RecentReactionView {
        reaction_key_kind: reaction_key_kind_label(&row.reaction_key_kind).to_string(),
        normalized_reaction_key: row.normalized_reaction_key.clone(),
        emoji: row.emoji.clone(),
        custom_asset: row
            .custom_asset_snapshot
            .as_ref()
            .map(custom_reaction_asset_view_from_snapshot),
        updated_at: row.updated_at,
    }
}

fn reaction_key_kind_label(kind: &ReactionKeyKind) -> &'static str {
    match kind {
        ReactionKeyKind::Emoji => "emoji",
        ReactionKeyKind::CustomAsset => "custom_asset",
    }
}

fn reaction_key_view_from_projection(row: &ReactionProjectionRow) -> ReactionKeyView {
    ReactionKeyView {
        reaction_key_kind: reaction_key_kind_label(&row.reaction_key_kind).to_string(),
        normalized_reaction_key: row.normalized_reaction_key.clone(),
        emoji: row.emoji.clone(),
        custom_asset: row
            .custom_asset_snapshot
            .as_ref()
            .map(custom_reaction_asset_view_from_snapshot),
    }
}

fn search_key_or_asset_id(search_key: &str, asset_id: &str) -> String {
    let normalized = search_key.trim();
    if normalized.is_empty() {
        return asset_id.to_string();
    }
    normalized.to_string()
}

async fn hydrate_object_projection_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("objects/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
        let content = match &header.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => {
                let payload = fetch_projection_blob_text(blob_service, hash).await;
                projection_store
                    .mark_blob_status(
                        hash,
                        match payload {
                            Some(_) => BlobCacheStatus::Available,
                            None => BlobCacheStatus::Missing,
                        },
                    )
                    .await?;
                payload
            }
        };
        for attachment in &header.attachments {
            let status = best_effort_blob_cache_status(blob_service, &attachment.hash).await;
            projection_store
                .mark_blob_status(&attachment.hash, status)
                .await?;
        }
        projection_store
            .put_object_projection(projection_row_from_header(&header, content, replica))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

async fn hydrate_reaction_cache_from_replica(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("reactions/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        let reaction: ReactionDocV1 = serde_json::from_slice(record.value.as_slice())?;
        projection_store
            .upsert_reaction_cache(reaction_projection_row_from_doc(&reaction, replica))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

async fn hydrate_topic_state_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
) -> Result<usize> {
    hydrate_subscription_state_with_services(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        &topic_replica_id(topic_id),
    )
    .await
}

async fn hydrate_subscription_state_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    let post_count =
        hydrate_object_projection_from_replica(docs_sync, blob_service, projection_store, replica)
            .await?;
    let reaction_count =
        hydrate_reaction_cache_from_replica(docs_sync, projection_store, replica).await?;
    let live_count = hydrate_live_sessions_from_replica(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
    )
    .await?;
    let game_count = hydrate_game_rooms_from_replica(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
    )
    .await?;
    Ok(post_count + reaction_count + live_count + game_count)
}

async fn hydrate_live_sessions_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("sessions/live/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        let state: LiveSessionStateDocV1 = serde_json::from_slice(&record.value)?;
        projection_store
            .mark_blob_status(
                &state.current_manifest.hash,
                blob_status(
                    blob_service
                        .blob_status(&state.current_manifest.hash)
                        .await?,
                ),
            )
            .await?;
        let Some(manifest) =
            fetch_manifest_blob::<LiveSessionManifestBlobV1>(blob_service, &state.current_manifest)
                .await?
        else {
            continue;
        };
        projection_store
            .upsert_live_session_cache(live_projection_row_from_state(
                &state, &manifest, topic_id, replica,
            ))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

async fn hydrate_game_rooms_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("sessions/game/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        let state: GameRoomStateDocV1 = serde_json::from_slice(&record.value)?;
        projection_store
            .mark_blob_status(
                &state.current_manifest.hash,
                blob_status(
                    blob_service
                        .blob_status(&state.current_manifest.hash)
                        .await?,
                ),
            )
            .await?;
        let Some(manifest) =
            fetch_manifest_blob::<GameRoomManifestBlobV1>(blob_service, &state.current_manifest)
                .await?
        else {
            continue;
        };
        projection_store
            .upsert_game_room_cache(game_projection_row_from_state(
                &state, &manifest, topic_id, replica,
            ))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

fn hint_targets_topic(hint: &GossipHint, topic: &str) -> bool {
    match hint {
        GossipHint::TopicObjectsChanged { topic_id, .. }
        | GossipHint::Presence { topic_id, .. }
        | GossipHint::Typing { topic_id, .. }
        | GossipHint::SessionChanged { topic_id, .. }
        | GossipHint::LivePresence { topic_id, .. }
        | GossipHint::DirectMessageFrame { topic_id, .. }
        | GossipHint::DirectMessageAck { topic_id, .. } => topic_id.as_str() == topic,
        GossipHint::ThreadUpdated { .. } | GossipHint::ProfileUpdated { .. } => true,
    }
}

fn projection_page_needs_hydration(page: &Page<ObjectProjectionRow>) -> bool {
    page.items.iter().any(|item| item.content.is_none())
}

fn profile_timeline_page(
    posts: Vec<ProfileTimelineItem>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<ProfileTimelineItem> {
    if limit == 0 {
        return Page {
            items: Vec::new(),
            next_cursor: cursor,
        };
    }

    let mut items = Vec::new();
    let mut next_cursor = None;
    for post in posts {
        let include = cursor.as_ref().is_none_or(|current| {
            post.created_at() < current.created_at
                || (post.created_at() == current.created_at
                    && post.object_id() < &current.object_id)
        });
        if !include {
            continue;
        }
        if items.len() >= limit {
            next_cursor = Some(TimelineCursor {
                created_at: post.created_at(),
                object_id: post.object_id().clone(),
            });
            break;
        }
        items.push(post);
    }

    Page { items, next_cursor }
}

async fn filtered_timeline_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channels: &BTreeSet<String>,
) -> Result<Page<ObjectProjectionRow>> {
    if limit == 0 {
        return Ok(Page {
            items: Vec::new(),
            next_cursor: cursor,
        });
    }
    let mut current_cursor = cursor;
    let mut items = Vec::new();
    let page_size = limit.max(20);
    loop {
        let page = ProjectionStore::list_topic_timeline(
            projection_store,
            topic_id,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if allowed_channels.contains(row.channel_id.as_str()) {
                items.push(row);
                if items.len() >= limit {
                    return Ok(Page { items, next_cursor });
                }
            }
        }
        if next_cursor.is_none() {
            return Ok(Page { items, next_cursor });
        }
        current_cursor = next_cursor;
    }
}

async fn filtered_thread_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    thread_root_object_id: &EnvelopeId,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channel: Option<&str>,
) -> Result<Page<ObjectProjectionRow>> {
    if limit == 0 {
        return Ok(Page {
            items: Vec::new(),
            next_cursor: cursor,
        });
    }
    let mut current_cursor = cursor;
    let mut items = Vec::new();
    let page_size = limit.max(20);
    loop {
        let page = ProjectionStore::list_thread(
            projection_store,
            topic_id,
            thread_root_object_id,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if allowed_channel.is_none_or(|channel_id| row.channel_id == channel_id) {
                items.push(row);
                if items.len() >= limit {
                    return Ok(Page { items, next_cursor });
                }
            }
        }
        if next_cursor.is_none() {
            return Ok(Page { items, next_cursor });
        }
        current_cursor = next_cursor;
    }
}

fn filter_channel_rows<T>(
    rows: Vec<T>,
    allowed_channels: &BTreeSet<String>,
    channel_id: impl Fn(&T) -> &str,
) -> Vec<T> {
    rows.into_iter()
        .filter(|row| allowed_channels.contains(channel_id(row)))
        .collect()
}

async fn fetch_post_object_for_projection(
    docs_sync: &dyn DocsSync,
    replica_id: &ReplicaId,
    source_key: &str,
) -> Result<Option<CanonicalPostHeader>> {
    let Ok(records) = docs_sync
        .query_replica(replica_id, DocQuery::Exact(source_key.to_string()))
        .await
    else {
        return Ok(None);
    };
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    let header = serde_json::from_slice(&record.value)?;
    Ok(Some(header))
}

fn legacy_epoch_id() -> &'static str {
    "legacy"
}

fn private_channel_is_epoch_aware(audience_kind: &ChannelAudienceKind) -> bool {
    let _ = audience_kind;
    true
}

fn initial_private_channel_epoch_id(
    audience_kind: &ChannelAudienceKind,
    now_ms: i64,
    owner_pubkey: &str,
) -> String {
    let _ = audience_kind;
    format!("epoch-{now_ms}-{}", short_id_suffix(owner_pubkey))
}

fn next_private_channel_epoch_id(owner_pubkey: &str) -> String {
    format!(
        "epoch-{}-{}",
        Utc::now().timestamp_millis(),
        short_id_suffix(owner_pubkey)
    )
}

fn private_channel_replica_for_epoch(channel_id: &str, epoch_id: &str) -> ReplicaId {
    if epoch_id == legacy_epoch_id() {
        return private_channel_replica_id(channel_id);
    }
    private_channel_epoch_replica_id(channel_id, epoch_id)
}

fn current_private_channel_replica_id(state: &JoinedPrivateChannelState) -> ReplicaId {
    private_channel_replica_for_epoch(state.channel_id.as_str(), state.current_epoch_id.as_str())
}

fn private_channel_epoch_capabilities(
    state: &JoinedPrivateChannelState,
) -> Vec<PrivateChannelEpochCapability> {
    let mut items = vec![PrivateChannelEpochCapability {
        epoch_id: state.current_epoch_id.clone(),
        namespace_secret_hex: state.current_epoch_secret_hex.clone(),
    }];
    for epoch in &state.archived_epochs {
        if items.iter().any(|item| item.epoch_id == epoch.epoch_id) {
            continue;
        }
        items.push(epoch.clone());
    }
    items
}

fn joined_private_channel_state_from_capability(
    capability: PrivateChannelCapability,
) -> Result<JoinedPrivateChannelState> {
    let current_epoch_id = if capability.current_epoch_id.trim().is_empty() {
        legacy_epoch_id().to_string()
    } else {
        capability.current_epoch_id
    };
    let current_epoch_secret_hex = if capability.current_epoch_secret_hex.trim().is_empty() {
        capability.namespace_secret_hex.clone()
    } else {
        capability.current_epoch_secret_hex
    };
    if current_epoch_secret_hex.trim().is_empty() {
        anyhow::bail!("private channel capability is missing current epoch secret");
    }
    let owner_pubkey = if capability.owner_pubkey.trim().is_empty() {
        capability.creator_pubkey.clone()
    } else {
        capability.owner_pubkey
    };
    Ok(JoinedPrivateChannelState {
        topic_id: capability.topic_id,
        channel_id: ChannelId::new(capability.channel_id),
        label: capability.label.trim().to_string(),
        creator_pubkey: capability.creator_pubkey,
        owner_pubkey,
        joined_via_pubkey: capability.joined_via_pubkey,
        audience_kind: capability.audience_kind,
        current_epoch_id,
        current_epoch_secret_hex,
        archived_epochs: capability.archived_epochs,
    })
}

#[allow(clippy::too_many_arguments)]
fn merged_private_channel_state_from_epoch_join(
    existing: Option<JoinedPrivateChannelState>,
    topic_id: &str,
    channel_id: ChannelId,
    label: &str,
    creator_pubkey: &str,
    owner_pubkey: &str,
    joined_via_pubkey: Option<&str>,
    audience_kind: ChannelAudienceKind,
    epoch_id: &str,
    namespace_secret_hex: &str,
) -> JoinedPrivateChannelState {
    let mut archived_epochs = existing
        .as_ref()
        .map(|state| state.archived_epochs.clone())
        .unwrap_or_default();
    archived_epochs.retain(|epoch| epoch.epoch_id != epoch_id);
    if let Some(existing_state) = existing.as_ref()
        && existing_state.current_epoch_id != epoch_id
        && !archived_epochs
            .iter()
            .any(|epoch| epoch.epoch_id == existing_state.current_epoch_id)
    {
        archived_epochs.push(PrivateChannelEpochCapability {
            epoch_id: existing_state.current_epoch_id.clone(),
            namespace_secret_hex: existing_state.current_epoch_secret_hex.clone(),
        });
    }
    JoinedPrivateChannelState {
        topic_id: topic_id.to_string(),
        channel_id,
        label: label.to_string(),
        creator_pubkey: creator_pubkey.to_string(),
        owner_pubkey: owner_pubkey.to_string(),
        joined_via_pubkey: joined_via_pubkey.map(str::to_string),
        audience_kind,
        current_epoch_id: epoch_id.to_string(),
        current_epoch_secret_hex: namespace_secret_hex.to_string(),
        archived_epochs,
    }
}

fn archive_private_channel_epoch(
    state: &mut JoinedPrivateChannelState,
    epoch_id: &str,
    namespace_secret_hex: &str,
) {
    if state
        .archived_epochs
        .iter()
        .any(|epoch| epoch.epoch_id == epoch_id)
    {
        return;
    }
    state.archived_epochs.push(PrivateChannelEpochCapability {
        epoch_id: epoch_id.to_string(),
        namespace_secret_hex: namespace_secret_hex.to_string(),
    });
}

fn active_private_channel_participants(
    participants: &[PrivateChannelParticipantDocV1],
    epoch_id: &str,
) -> Vec<PrivateChannelParticipantDocV1> {
    participants
        .iter()
        .filter(|participant| participant.epoch_id == epoch_id)
        .cloned()
        .collect()
}

async fn register_private_channel_replica_secrets(
    docs_sync: &dyn DocsSync,
    state: &JoinedPrivateChannelState,
) -> Result<()> {
    for epoch in private_channel_epoch_capabilities(state) {
        let replica =
            private_channel_replica_for_epoch(state.channel_id.as_str(), epoch.epoch_id.as_str());
        docs_sync
            .register_private_replica_secret(&replica, epoch.namespace_secret_hex.as_str())
            .await?;
    }
    Ok(())
}

fn joined_private_channel_subscription_prefix(topic_id: &str, channel_id: &str) -> String {
    format!("{topic_id}::{channel_id}::")
}

fn joined_private_channel_subscription_key(
    topic_id: &str,
    channel_id: &str,
    replica: &ReplicaId,
) -> String {
    format!("{topic_id}::{channel_id}::{}", replica.as_str())
}

fn subscription_replicas_for_topic(
    topic_id: &str,
    joined_channels: Vec<JoinedPrivateChannelState>,
) -> Vec<ReplicaId> {
    let mut replicas = vec![topic_replica_id(topic_id)];
    replicas.extend(joined_channels.into_iter().flat_map(|state| {
        private_channel_epoch_capabilities(&state)
            .into_iter()
            .map(move |epoch| {
                private_channel_replica_for_epoch(
                    state.channel_id.as_str(),
                    epoch.epoch_id.as_str(),
                )
            })
    }));
    replicas
}

async fn blob_view_status_for_payload(
    blob_service: &dyn BlobService,
    payload_ref: &PayloadRef,
) -> Result<BlobViewStatus> {
    match payload_ref {
        PayloadRef::InlineText { .. } => Ok(BlobViewStatus::Available),
        PayloadRef::BlobText { hash, .. } => {
            Ok(best_effort_blob_view_status(blob_service, hash).await)
        }
    }
}

async fn attachment_views(
    blob_service: &dyn BlobService,
    header: &CanonicalPostHeader,
) -> Result<Vec<AttachmentView>> {
    let mut attachments = Vec::with_capacity(header.attachments.len());
    for attachment in &header.attachments {
        attachments.push(AttachmentView {
            hash: attachment.hash.as_str().to_string(),
            mime: attachment.mime.clone(),
            bytes: attachment.bytes,
            role: attachment_role_name(&attachment.role).to_string(),
            status: best_effort_blob_view_status(blob_service, &attachment.hash).await,
        });
    }
    Ok(attachments)
}

async fn attachment_views_from_refs(
    blob_service: &dyn BlobService,
    refs: &[kukuri_core::AssetRef],
) -> Result<Vec<AttachmentView>> {
    let mut attachments = Vec::with_capacity(refs.len());
    for attachment in refs {
        attachments.push(AttachmentView {
            hash: attachment.hash.as_str().to_string(),
            mime: attachment.mime.clone(),
            bytes: attachment.bytes,
            role: attachment_role_name(&attachment.role).to_string(),
            status: best_effort_blob_view_status(blob_service, &attachment.hash).await,
        });
    }
    Ok(attachments)
}

async fn direct_message_attachment_views(
    blob_service: &dyn BlobService,
    manifest: Option<&DirectMessageAttachmentManifestV1>,
) -> Result<Vec<AttachmentView>> {
    let Some(manifest) = manifest else {
        return Ok(Vec::new());
    };
    let mut attachments = Vec::new();
    attachments.push(AttachmentView {
        hash: manifest.original.hash.as_str().to_string(),
        mime: manifest.original.mime.clone(),
        bytes: manifest.original.bytes,
        role: match manifest.kind {
            DirectMessageAttachmentKind::Image => "image_original".into(),
            DirectMessageAttachmentKind::Video => "video_manifest".into(),
        },
        status: best_effort_blob_view_status(blob_service, &manifest.original.hash).await,
    });
    if let Some(poster) = manifest.poster.as_ref() {
        attachments.push(AttachmentView {
            hash: poster.hash.as_str().to_string(),
            mime: poster.mime.clone(),
            bytes: poster.bytes,
            role: "video_poster".into(),
            status: best_effort_blob_view_status(blob_service, &poster.hash).await,
        });
    }
    Ok(attachments)
}

fn direct_message_preview(row: &DirectMessageMessageRow) -> String {
    if let Some(text) = row
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return text.chars().take(80).collect();
    }
    match row
        .attachment_manifest
        .as_ref()
        .map(|manifest| &manifest.kind)
    {
        Some(DirectMessageAttachmentKind::Image) => "[Image]".into(),
        Some(DirectMessageAttachmentKind::Video) => "[Video]".into(),
        None => String::new(),
    }
}

async fn materialize_direct_message_manifest(
    blob_service: &dyn BlobService,
    keys: &KukuriKeys,
    sender_pubkey: &Pubkey,
    message_id: &str,
    manifest: Option<&DirectMessageAttachmentManifestV1>,
) -> Result<Option<DirectMessageAttachmentManifestV1>> {
    let Some(manifest) = manifest else {
        return Ok(None);
    };
    let original = materialize_direct_message_blob_ref(
        blob_service,
        keys,
        sender_pubkey,
        message_id,
        &manifest.original,
    )
    .await?;
    let poster = match manifest.poster.as_ref() {
        Some(poster) => Some(
            materialize_direct_message_blob_ref(
                blob_service,
                keys,
                sender_pubkey,
                message_id,
                poster,
            )
            .await?,
        ),
        None => None,
    };
    Ok(Some(DirectMessageAttachmentManifestV1 {
        attachment_id: manifest.attachment_id.clone(),
        kind: manifest.kind.clone(),
        original,
        poster,
    }))
}

async fn materialize_direct_message_blob_ref(
    blob_service: &dyn BlobService,
    keys: &KukuriKeys,
    sender_pubkey: &Pubkey,
    message_id: &str,
    encrypted_ref: &DirectMessageEncryptedBlobRefV1,
) -> Result<DirectMessageEncryptedBlobRefV1> {
    let Some(bytes) = blob_service.fetch_blob(&encrypted_ref.hash).await? else {
        anyhow::bail!("direct message attachment blob is missing");
    };
    let encrypted: DirectMessageEncryptedAttachmentV1 = serde_json::from_slice(bytes.as_slice())
        .context("failed to decode direct message attachment blob")?;
    let decrypted = decrypt_direct_message_attachment(keys, sender_pubkey, message_id, &encrypted)?;
    let local = blob_service
        .put_blob(decrypted, encrypted_ref.mime.as_str())
        .await?;
    Ok(DirectMessageEncryptedBlobRefV1 {
        blob_id: encrypted_ref.blob_id.clone(),
        hash: local.hash,
        mime: encrypted_ref.mime.clone(),
        bytes: encrypted_ref.bytes,
        nonce_hex: String::new(),
    })
}

async fn direct_message_topic_peer_count(
    transport: &dyn Transport,
    topic: &TopicId,
) -> Result<usize> {
    let snapshot = transport.peers().await?;
    let hint_topic = format!("hint/{}", topic.as_str());
    let topic_peer_count = snapshot
        .topic_diagnostics
        .iter()
        .find(|diagnostic| diagnostic.topic == hint_topic || diagnostic.topic == topic.as_str())
        .map(|diagnostic| diagnostic.peer_count)
        .unwrap_or(0);
    if topic_peer_count > 0 {
        return Ok(topic_peer_count);
    }
    if snapshot.connected && snapshot.peer_count > 0 {
        return Ok(snapshot.peer_count);
    }
    Ok(0)
}

fn blob_view_status(status: BlobStatus) -> BlobViewStatus {
    match status {
        BlobStatus::Missing => BlobViewStatus::Missing,
        BlobStatus::Available => BlobViewStatus::Available,
        BlobStatus::Pinned => BlobViewStatus::Pinned,
    }
}

fn blob_status(status: BlobStatus) -> BlobCacheStatus {
    match status {
        BlobStatus::Missing => BlobCacheStatus::Missing,
        BlobStatus::Available => BlobCacheStatus::Available,
        BlobStatus::Pinned => BlobCacheStatus::Pinned,
    }
}

fn attachment_role_name(role: &AssetRole) -> &'static str {
    match role {
        AssetRole::ImageOriginal => "image_original",
        AssetRole::ImagePreview => "image_preview",
        AssetRole::VideoPoster => "video_poster",
        AssetRole::VideoManifest => "video_manifest",
        AssetRole::ProfileAvatar => "profile_avatar",
        AssetRole::Attachment => "attachment",
    }
}

fn sanitize_game_participants(participants: Vec<String>) -> Result<Vec<String>> {
    let mut seen = BTreeSet::new();
    let normalized = participants
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect::<Vec<_>>();
    if normalized.len() < 2 {
        anyhow::bail!("game room requires at least two unique participants");
    }
    Ok(normalized)
}

fn validate_game_room_transition(current: &GameRoomStatus, next: &GameRoomStatus) -> Result<()> {
    match (current, next) {
        (GameRoomStatus::Ended, GameRoomStatus::Ended) => {
            anyhow::bail!("ended game room cannot be updated")
        }
        (GameRoomStatus::Ended, _) => anyhow::bail!("ended game room cannot be updated"),
        (GameRoomStatus::Waiting, GameRoomStatus::Waiting)
        | (GameRoomStatus::Waiting, GameRoomStatus::Running)
        | (GameRoomStatus::Waiting, GameRoomStatus::Ended)
        | (GameRoomStatus::Running, GameRoomStatus::Running)
        | (GameRoomStatus::Running, GameRoomStatus::Paused)
        | (GameRoomStatus::Running, GameRoomStatus::Ended)
        | (GameRoomStatus::Paused, GameRoomStatus::Paused)
        | (GameRoomStatus::Paused, GameRoomStatus::Running)
        | (GameRoomStatus::Paused, GameRoomStatus::Ended) => Ok(()),
        (GameRoomStatus::Waiting, GameRoomStatus::Paused) => {
            anyhow::bail!("game room cannot pause before it starts")
        }
        (GameRoomStatus::Running, GameRoomStatus::Waiting)
        | (GameRoomStatus::Paused, GameRoomStatus::Waiting) => {
            anyhow::bail!("game room cannot move back to waiting")
        }
    }
}

fn validate_game_room_scores(
    manifest: &GameRoomManifestBlobV1,
    scores: &[GameScoreView],
) -> Result<()> {
    if manifest.scores.len() != scores.len() {
        anyhow::bail!("score update must include all participants");
    }
    let expected = manifest
        .scores
        .iter()
        .map(|score| score.participant_id.clone())
        .collect::<BTreeSet<_>>();
    let provided = scores
        .iter()
        .map(|score| score.participant_id.clone())
        .collect::<BTreeSet<_>>();
    if expected != provided {
        anyhow::bail!("score update participants do not match the room roster");
    }
    let expected_labels = manifest
        .scores
        .iter()
        .map(|score| (score.participant_id.as_str(), score.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    for score in scores {
        if expected_labels.get(score.participant_id.as_str()) != Some(&score.label.as_str()) {
            anyhow::bail!("score update labels do not match the room roster");
        }
    }
    Ok(())
}

fn channel_storage_id(channel_id: Option<&ChannelId>) -> String {
    channel_id
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| PUBLIC_CHANNEL_ID.to_string())
}

fn channel_hint_topic_for(topic_id: &str, channel_id: Option<&ChannelId>) -> TopicId {
    channel_id
        .map(|value| private_channel_hint_topic(value.as_str()))
        .unwrap_or_else(|| TopicId::new(topic_id))
}

fn channel_id_from_storage(channel_id: &str) -> Option<ChannelId> {
    (channel_id != PUBLIC_CHANNEL_ID).then(|| ChannelId::new(channel_id.to_string()))
}

fn channel_id_for_view(channel_id: &str) -> Option<String> {
    channel_id_from_storage(channel_id).map(|value| value.as_str().to_string())
}

fn joined_private_channel_key(topic_id: &str, channel_id: &str) -> String {
    format!("{topic_id}::{channel_id}")
}

fn live_presence_task_key(topic_id: &str, channel_id: &str, session_id: &str) -> String {
    format!("{topic_id}::{channel_id}::{session_id}")
}

fn short_id_suffix(author_pubkey: &str) -> &str {
    author_pubkey.get(..8).unwrap_or(author_pubkey)
}

fn normalize_topic_name(topic: String) -> Option<String> {
    let normalized = topic
        .strip_prefix("hint/")
        .map_or(topic.clone(), ToOwned::to_owned);
    if normalized.starts_with("private/") || normalized.starts_with("kukuri:dm:") {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_topics(topics: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for topic in topics {
        let Some(topic) = normalize_topic_name(topic) else {
            continue;
        };
        if seen.insert(topic.clone()) {
            normalized.push(topic);
        }
    }
    normalized
}

fn normalize_topic_diagnostics(diagnostics: Vec<TopicPeerSnapshot>) -> Vec<TopicPeerSnapshot> {
    let mut merged = BTreeMap::<String, TopicPeerSnapshot>::new();
    for diagnostic in diagnostics {
        let Some(topic) = normalize_topic_name(diagnostic.topic) else {
            continue;
        };
        let entry = merged
            .entry(topic.clone())
            .or_insert_with(|| TopicPeerSnapshot {
                topic: topic.clone(),
                joined: false,
                peer_count: 0,
                connected_peers: Vec::new(),
                configured_peer_ids: Vec::new(),
                missing_peer_ids: Vec::new(),
                last_received_at: None,
                status_detail: diagnostic.status_detail.clone(),
                last_error: diagnostic.last_error.clone(),
            });
        entry.joined |= diagnostic.joined;
        entry.peer_count = entry.peer_count.max(diagnostic.peer_count);
        for peer in diagnostic.connected_peers {
            if !entry.connected_peers.contains(&peer) {
                entry.connected_peers.push(peer);
            }
        }
        for peer in diagnostic.configured_peer_ids {
            if !entry.configured_peer_ids.contains(&peer) {
                entry.configured_peer_ids.push(peer);
            }
        }
        for peer in diagnostic.missing_peer_ids {
            if !entry.missing_peer_ids.contains(&peer) {
                entry.missing_peer_ids.push(peer);
            }
        }
        entry.last_received_at = match (entry.last_received_at, diagnostic.last_received_at) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (None, value) | (value, None) => value,
        };
        if entry.status_detail.starts_with("No peers configured")
            || entry.status_detail.starts_with("Waiting")
        {
            entry.status_detail = diagnostic.status_detail;
        }
        if entry.last_error.is_none() {
            entry.last_error = diagnostic.last_error;
        }
    }
    merged.into_values().collect()
}

fn merge_peer_ids(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    left.into_iter()
        .chain(right)
        .filter(|peer| !peer.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn effective_sync_status_detail(
    base: &str,
    gossip_peer_count: usize,
    assist_peer_count: usize,
    subscribed_topic_count: usize,
) -> String {
    if gossip_peer_count > 0 || assist_peer_count == 0 {
        return base.to_string();
    }
    if subscribed_topic_count > 0 {
        format!("relay-assisted sync available via {assist_peer_count} peer(s)")
    } else {
        format!("relay-assisted connectivity available via {assist_peer_count} peer(s)")
    }
}

fn effective_topic_status_detail(
    base: &str,
    gossip_peer_count: usize,
    assist_peer_count: usize,
) -> String {
    if gossip_peer_count > 0 || assist_peer_count == 0 {
        return base.to_string();
    }
    format!("relay-assisted sync available via {assist_peer_count} peer(s)")
}

impl Drop for AppService {
    fn drop(&mut self) {
        if let Ok(mut subscriptions) = self.subscriptions.try_lock() {
            for (_, handle) in subscriptions.drain() {
                handle.abort();
            }
        }
        if let Ok(mut subscriptions) = self.private_channel_subscriptions.try_lock() {
            for (_, handle) in subscriptions.drain() {
                handle.abort();
            }
        }
        if let Ok(mut subscriptions) = self.author_subscriptions.try_lock() {
            for (_, handle) in subscriptions.drain() {
                handle.abort();
            }
        }
        if let Ok(mut tasks) = self.live_presence_tasks.try_lock() {
            for (_, handle) in tasks.drain() {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use iroh::address_lookup::EndpointInfo;
    use kukuri_blob_service::IrohBlobService;
    use kukuri_core::build_post_envelope_with_payload;
    use kukuri_docs_sync::IrohDocsNode;
    use kukuri_docs_sync::IrohDocsSync;
    use kukuri_store::{BookmarkedCustomReactionRow, MemoryStore, SqliteStore};
    use kukuri_transport::{
        DhtDiscoveryOptions, DiscoveryMode, FakeNetwork, FakeTransport, HintEnvelope, HintStream,
        IrohGossipTransport, SeedPeer,
    };
    use pkarr::errors::{ConcurrencyError, PublishError, QueryError};
    use pkarr::{Client as PkarrClient, SignedPacket, Timestamp, mainline::Testnet};
    use std::sync::OnceLock;
    use tempfile::tempdir;
    use tokio::sync::{Mutex as TokioMutex, broadcast};
    use tokio::time::{Duration, sleep, timeout};
    use tokio_stream::wrappers::BroadcastStream;

    fn social_graph_propagation_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(300)
        } else {
            Duration::from_secs(10)
        }
    }

    fn p2p_replication_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(60)
        } else {
            Duration::from_secs(10)
        }
    }

    fn seeded_dht_publish_attempts() -> usize {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            60
        } else {
            20
        }
    }

    fn seeded_dht_publish_resolve_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(15)
        } else {
            Duration::from_secs(5)
        }
    }

    fn iroh_integration_test_lock() -> Arc<TokioMutex<()>> {
        static LOCK: OnceLock<Arc<TokioMutex<()>>> = OnceLock::new();
        LOCK.get_or_init(|| Arc::new(TokioMutex::new(()))).clone()
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

    async fn wait_for_connected_peer_count(app: &AppService, expected: usize) {
        match timeout(social_graph_propagation_timeout(), async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status = app.get_sync_status().await.expect("sync status");
                if status.connected && status.peer_count >= expected {
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
                let status = app.get_sync_status().await.expect("sync status");
                panic!(
                    "peer connection timeout; connected={}, peer_count={}, status_detail={}, last_error={:?}, discovery_connected_peers={:?}",
                    status.connected,
                    status.peer_count,
                    status.status_detail,
                    status.last_error,
                    status.discovery.connected_peer_ids
                );
            }
        }
    }

    async fn wait_for_topic_peer_count(app: &AppService, topic: &str, expected: usize) {
        match timeout(social_graph_propagation_timeout(), async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status = app.get_sync_status().await.expect("sync status");
                let ready = status.topic_diagnostics.iter().any(|entry| {
                    let relay_assisted_ready = entry.assist_peer_ids.len() >= expected;
                    entry.topic == topic
                        && entry.joined
                        && entry.peer_count >= expected
                        && (entry.connected_peers.len() >= expected || relay_assisted_ready)
                });
                if ready {
                    stable_ready_polls += 1;
                    if stable_ready_polls >= 3 {
                        return;
                    }
                } else {
                    stable_ready_polls = 0;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let snapshot = app
                    .get_sync_status()
                    .await
                    .map(|status| format_sync_snapshot(&status, topic))
                    .unwrap_or_else(|_| "failed to read sync status".to_string());
                panic!("topic connected-peer timeout for {topic}; {snapshot}");
            }
        }
    }

    async fn warm_author_social_view(app: &AppService, author_pubkey: &str, topic: &str) {
        match timeout(social_graph_propagation_timeout(), async {
            loop {
                if app.get_author_social_view(author_pubkey).await.is_ok() {
                    return;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let snapshot = app
                    .get_sync_status()
                    .await
                    .map(|status| format_sync_snapshot(&status, topic))
                    .unwrap_or_else(|_| "failed to read sync status".to_string());
                panic!("author social view warmup timeout for {author_pubkey}; {snapshot}");
            }
        }
    }

    async fn wait_for_mutual_author_view(app: &AppService, author_pubkey: &str, topic: &str) {
        match timeout(social_graph_propagation_timeout(), async {
            loop {
                let view = app
                    .get_author_social_view(author_pubkey)
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
                let social_view = app
                    .get_author_social_view(author_pubkey)
                    .await
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
                    .unwrap_or_else(|_| "social_view=unavailable".to_string());
                let snapshot = app
                    .get_sync_status()
                    .await
                    .map(|status| format_sync_snapshot(&status, topic))
                    .unwrap_or_else(|_| "failed to read sync status".to_string());
                panic!(
                    "mutual relationship timeout for {author_pubkey}; {social_view}, {snapshot}"
                );
            }
        }
    }

    fn is_retryable_friend_only_grant_import_error(message: &str) -> bool {
        message.contains("mutual relationship")
            || message.contains("friend-only grant epoch does not match the current policy")
            || message.contains("friend-only grant owner is not an active participant")
            || message.contains("timed out waiting for friend-only channel replica sync")
    }

    async fn wait_for_friend_only_grant_import(
        app: &AppService,
        token: &str,
        step_timeout: Duration,
    ) -> kukuri_core::FriendOnlyGrantPreview {
        match timeout(step_timeout, async {
            loop {
                match app.import_friend_only_grant(token).await {
                    Ok(preview) => return preview,
                    Err(error)
                        if is_retryable_friend_only_grant_import_error(
                            error.to_string().as_str(),
                        ) =>
                    {
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(error) => panic!("friend-only grant import failed: {error:#}"),
                }
            }
        })
        .await
        {
            Ok(preview) => preview,
            Err(_) => {
                let preview =
                    kukuri_core::parse_friend_only_grant_token(token).expect("parse grant token");
                let social_view = app
                    .get_author_social_view(preview.owner_pubkey.as_str())
                    .await
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
                    .unwrap_or_else(|_| "social_view=unavailable".to_string());
                let snapshot = app
                    .get_sync_status()
                    .await
                    .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                    .unwrap_or_else(|_| "failed to read sync status".to_string());
                panic!(
                    "friend-only grant import timeout for {}; {social_view}, {snapshot}",
                    preview.owner_pubkey.as_str()
                );
            }
        }
    }

    fn is_retryable_friend_plus_share_import_error(message: &str) -> bool {
        message.contains("mutual relationship")
            || message.contains("sponsor is not an active participant")
            || message.contains("timed out waiting for friend-plus sponsor participant sync")
            || message.contains("timed out waiting for friend-plus channel replica sync")
    }

    async fn wait_for_friend_plus_share_import(
        app: &AppService,
        token: &str,
        step_timeout: Duration,
    ) -> kukuri_core::FriendPlusSharePreview {
        let preview = kukuri_core::parse_friend_plus_share_token(token).expect("parse share token");
        match timeout(step_timeout, async {
            loop {
                match app.import_friend_plus_share(token).await {
                    Ok(preview) => return preview,
                    Err(error)
                        if is_retryable_friend_plus_share_import_error(
                            error.to_string().as_str(),
                        ) =>
                    {
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(error) => panic!("friend-plus share import failed: {error:#}"),
                }
            }
        })
        .await
        {
            Ok(preview) => preview,
            Err(_) => {
                let social_view = app
                    .get_author_social_view(preview.sponsor_pubkey.as_str())
                    .await
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
                    .unwrap_or_else(|_| "social_view=unavailable".to_string());
                let snapshot = app
                    .get_sync_status()
                    .await
                    .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                    .unwrap_or_else(|_| "failed to read sync status".to_string());
                panic!(
                    "friend-plus share import timeout; sponsor_pubkey={}, {social_view}, {snapshot}",
                    preview.sponsor_pubkey.as_str()
                );
            }
        }
    }

    #[derive(Clone)]
    struct StaticTransport {
        peers: Arc<TokioMutex<PeerSnapshot>>,
        hints: Arc<TokioMutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
        local_ticket: String,
    }

    impl StaticTransport {
        fn new(peers: PeerSnapshot) -> Self {
            Self {
                peers: Arc::new(TokioMutex::new(peers)),
                hints: Arc::new(TokioMutex::new(HashMap::new())),
                local_ticket: "static-peer".into(),
            }
        }

        async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
            let mut guard = self.hints.lock().await;
            guard
                .entry(topic.as_str().to_string())
                .or_insert_with(|| broadcast::channel(64).0)
                .clone()
        }
    }

    #[derive(Clone, Default)]
    struct AssistedDocsSync {
        peer_ids: Vec<String>,
    }

    impl AssistedDocsSync {
        fn new(peer_ids: Vec<&str>) -> Self {
            Self {
                peer_ids: peer_ids.into_iter().map(str::to_string).collect(),
            }
        }
    }

    #[async_trait]
    impl DocsSync for AssistedDocsSync {
        async fn open_replica(&self, _replica_id: &ReplicaId) -> Result<()> {
            Ok(())
        }

        async fn apply_doc_op(&self, _replica_id: &ReplicaId, _op: DocOp) -> Result<()> {
            Ok(())
        }

        async fn query_replica(
            &self,
            _replica_id: &ReplicaId,
            _query: DocQuery,
        ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
            Ok(Vec::new())
        }

        async fn subscribe_replica(
            &self,
            _replica_id: &ReplicaId,
        ) -> Result<kukuri_docs_sync::DocEventStream> {
            let (sender, _) = broadcast::channel::<kukuri_docs_sync::DocEvent>(1);
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok().map(Ok) });
            Ok(Box::pin(stream))
        }

        async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }

        async fn assist_peer_ids(&self) -> Result<Vec<String>> {
            Ok(self.peer_ids.clone())
        }
    }

    #[derive(Clone, Default)]
    struct TrackingDocsSync {
        restarted_replicas: Arc<TokioMutex<Vec<String>>>,
    }

    #[async_trait]
    impl DocsSync for TrackingDocsSync {
        async fn open_replica(&self, _replica_id: &ReplicaId) -> Result<()> {
            Ok(())
        }

        async fn apply_doc_op(&self, _replica_id: &ReplicaId, _op: DocOp) -> Result<()> {
            Ok(())
        }

        async fn query_replica(
            &self,
            _replica_id: &ReplicaId,
            _query: DocQuery,
        ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
            Ok(Vec::new())
        }

        async fn subscribe_replica(
            &self,
            _replica_id: &ReplicaId,
        ) -> Result<kukuri_docs_sync::DocEventStream> {
            let (sender, _) = broadcast::channel::<kukuri_docs_sync::DocEvent>(1);
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok().map(Ok) });
            Ok(Box::pin(stream))
        }

        async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }

        async fn restart_replica_sync(&self, replica_id: &ReplicaId) -> Result<()> {
            self.restarted_replicas
                .lock()
                .await
                .push(replica_id.as_str().to_string());
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct AssistedBlobService {
        peer_ids: Vec<String>,
    }

    impl AssistedBlobService {
        fn new(peer_ids: Vec<&str>) -> Self {
            Self {
                peer_ids: peer_ids.into_iter().map(str::to_string).collect(),
            }
        }
    }

    #[async_trait]
    impl BlobService for AssistedBlobService {
        async fn put_blob(&self, _data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
            Ok(StoredBlob {
                hash: kukuri_core::BlobHash::new("test-hash"),
                mime: mime.to_string(),
                bytes: 0,
            })
        }

        async fn fetch_blob(&self, _hash: &kukuri_core::BlobHash) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }

        async fn pin_blob(&self, _hash: &kukuri_core::BlobHash) -> Result<()> {
            Ok(())
        }

        async fn blob_status(&self, _hash: &kukuri_core::BlobHash) -> Result<BlobStatus> {
            Ok(BlobStatus::Missing)
        }

        async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }

        async fn assist_peer_ids(&self) -> Result<Vec<String>> {
            Ok(self.peer_ids.clone())
        }
    }

    async fn persist_test_post(
        docs_sync: &dyn DocsSync,
        projection_store: Option<&dyn ProjectionStore>,
        keys: &KukuriKeys,
        topic: &TopicId,
        payload_ref: PayloadRef,
        attachments: Vec<kukuri_core::AssetRef>,
        reply_to: Option<&KukuriEnvelope>,
    ) -> KukuriEnvelope {
        let envelope = build_post_envelope_with_payload(
            keys,
            topic,
            payload_ref,
            attachments,
            Vec::new(),
            reply_to,
            ObjectVisibility::Public,
        )
        .expect("event");
        let object = envelope
            .to_post_object()
            .expect("post object")
            .expect("post object");
        let replica = topic_replica_id(topic.as_str());
        persist_post_object(docs_sync, &replica, object.clone(), envelope.clone())
            .await
            .expect("persist post object");
        if let Some(projection_store) = projection_store {
            ProjectionStore::put_object_projection(
                projection_store,
                projection_row_from_header(&object, None, &replica),
            )
            .await
            .expect("put placeholder projection");
        }
        envelope
    }

    #[async_trait]
    impl Transport for StaticTransport {
        async fn peers(&self) -> Result<PeerSnapshot> {
            Ok(self.peers.lock().await.clone())
        }

        async fn export_ticket(&self) -> Result<Option<String>> {
            Ok(Some(self.local_ticket.clone()))
        }

        async fn import_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl HintTransport for StaticTransport {
        async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
            let sender = self.hint_sender(topic).await;
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok() });
            Ok(Box::pin(stream))
        }

        async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
            let sender = self.hint_sender(topic).await;
            let _ = sender.send(HintEnvelope {
                hint,
                received_at: Utc::now().timestamp_millis(),
                source_peer: "static".into(),
            });
            Ok(())
        }
    }

    struct TestIrohStack {
        _node: Arc<IrohDocsNode>,
        transport: Arc<IrohGossipTransport>,
        docs_sync: Arc<IrohDocsSync>,
        blob_service: Arc<IrohBlobService>,
    }

    impl TestIrohStack {
        async fn new(root: &std::path::Path) -> Self {
            Self::new_with_discovery(root, DhtDiscoveryOptions::disabled()).await
        }

        async fn new_with_dht(root: &std::path::Path, testnet: &Testnet) -> Self {
            let stack = Self::new_with_discovery(
                root,
                DhtDiscoveryOptions::with_client(dht_test_client(testnet)),
            )
            .await;
            publish_endpoint_to_testnet(stack._node.endpoint(), testnet).await;
            stack
        }

        async fn new_with_discovery(
            root: &std::path::Path,
            dht_options: DhtDiscoveryOptions,
        ) -> Self {
            let node = IrohDocsNode::persistent_with_discovery_config(
                root,
                kukuri_transport::TransportNetworkConfig::loopback(),
                dht_options,
                kukuri_transport::TransportRelayConfig::default(),
            )
            .await
            .expect("iroh docs node");
            let transport = Arc::new(
                IrohGossipTransport::from_shared_parts(
                    node.endpoint().clone(),
                    node.gossip().clone(),
                    node.discovery(),
                    kukuri_transport::TransportNetworkConfig::loopback(),
                    kukuri_transport::TransportRelayConfig::default(),
                )
                .expect("transport"),
            );
            let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
            let blob_service = Arc::new(IrohBlobService::new(node.clone()));
            Self {
                _node: node,
                transport,
                docs_sync,
                blob_service,
            }
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

    async fn publish_endpoint_to_testnet(endpoint: &iroh::Endpoint, testnet: &Testnet) {
        let client = dht_test_client(testnet);
        let public_key =
            pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
        let expected_info = EndpointInfo::from(endpoint.addr());
        let mut last_error = None;
        for _ in 0..seeded_dht_publish_attempts() {
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
                Err(
                    error @ PublishError::Query(QueryError::Timeout | QueryError::NoClosestNodes),
                ) => {
                    last_error = Some(error);
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("publish endpoint info: {error}"),
            }
        }
        if let Some(error) = last_error.take()
            && client
                .resolve_most_recent(&public_key)
                .await
                .as_ref()
                .and_then(|packet| EndpointInfo::from_pkarr_signed_packet(packet).ok())
                .is_none_or(|packet_info| {
                    packet_info.to_txt_strings() != expected_info.to_txt_strings()
                })
        {
            panic!("publish endpoint info: {error}");
        }
        timeout(seeded_dht_publish_resolve_timeout(), async {
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

    async fn configure_seeded_dht(app: &AppService, remote_endpoint_id: String) {
        app.set_discovery_seeds(
            DiscoveryMode::SeededDht,
            false,
            vec![SeedPeer {
                endpoint_id: remote_endpoint_id,
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("configure seeded dht");
    }

    fn app_with_iroh_services(store: Arc<MemoryStore>, stack: &TestIrohStack) -> AppService {
        AppService::new_with_services(
            store.clone(),
            store,
            stack.transport.clone(),
            stack.transport.clone(),
            stack.docs_sync.clone(),
            stack.blob_service.clone(),
            generate_keys(),
        )
    }

    fn pending_image_attachment(mime: &str, bytes: &[u8]) -> PendingAttachment {
        PendingAttachment {
            mime: mime.to_string(),
            bytes: bytes.to_vec(),
            role: AssetRole::ImageOriginal,
        }
    }

    fn pending_video_attachment(role: AssetRole, mime: &str, bytes: &[u8]) -> PendingAttachment {
        PendingAttachment {
            mime: mime.to_string(),
            bytes: bytes.to_vec(),
            role,
        }
    }

    fn tiny_png_bytes() -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO7ZPioAAAAASUVORK5CYII=")
            .expect("decode png")
    }

    fn reaction_snapshot_from_view(
        asset: &CustomReactionAssetView,
    ) -> CustomReactionAssetSnapshotV1 {
        CustomReactionAssetSnapshotV1 {
            asset_id: asset.asset_id.clone(),
            owner_pubkey: Pubkey::from(asset.owner_pubkey.as_str()),
            blob_hash: kukuri_core::BlobHash::new(asset.blob_hash.clone()),
            search_key: asset.search_key.clone(),
            mime: asset.mime.clone(),
            bytes: asset.bytes,
            width: asset.width,
            height: asset.height,
        }
    }

    fn local_app_with_memory_services() -> (
        AppService,
        Arc<MemoryStore>,
        Arc<MemoryDocsSync>,
        Arc<MemoryBlobService>,
    ) {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport,
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service.clone(),
            generate_keys(),
        );
        (app, store, docs_sync, blob_service)
    }

    async fn author_profile_post_docs(
        docs_sync: &dyn DocsSync,
        author_pubkey: &str,
    ) -> Vec<AuthorProfilePostDocV1> {
        docs_sync
            .query_replica(
                &author_replica_id(author_pubkey),
                DocQuery::Prefix("profile/posts/".into()),
            )
            .await
            .expect("profile post docs")
            .into_iter()
            .map(|record| {
                serde_json::from_slice::<AuthorProfilePostDocV1>(record.value.as_slice())
                    .expect("decode profile post doc")
            })
            .collect()
    }

    async fn author_profile_repost_docs(
        docs_sync: &dyn DocsSync,
        author_pubkey: &str,
    ) -> Vec<AuthorProfileRepostDocV1> {
        docs_sync
            .query_replica(
                &author_replica_id(author_pubkey),
                DocQuery::Prefix("profile/reposts/".into()),
            )
            .await
            .expect("profile repost docs")
            .into_iter()
            .map(|record| {
                serde_json::from_slice::<AuthorProfileRepostDocV1>(record.value.as_slice())
                    .expect("decode profile repost doc")
            })
            .collect()
    }

    async fn author_profile_doc(
        docs_sync: &dyn DocsSync,
        author_pubkey: &str,
    ) -> Option<AuthorProfileDocV1> {
        docs_sync
            .query_replica(
                &author_replica_id(author_pubkey),
                DocQuery::Exact(stable_key("profile", "latest")),
            )
            .await
            .expect("profile doc")
            .into_iter()
            .next()
            .map(|record| {
                serde_json::from_slice::<AuthorProfileDocV1>(record.value.as_slice())
                    .expect("decode profile doc")
            })
    }

    #[derive(Clone)]
    struct NoopHintTransport;

    #[async_trait]
    impl HintTransport for NoopHintTransport {
        async fn subscribe_hints(&self, _topic: &TopicId) -> Result<HintStream> {
            Ok(Box::pin(futures_util::stream::empty()))
        }

        async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish_hint(&self, _topic: &TopicId, _hint: GossipHint) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct CountingClosingHintTransport {
        subscribe_count: Arc<TokioMutex<usize>>,
    }

    #[async_trait]
    impl HintTransport for CountingClosingHintTransport {
        async fn subscribe_hints(&self, _topic: &TopicId) -> Result<HintStream> {
            *self.subscribe_count.lock().await += 1;
            Ok(Box::pin(futures_util::stream::empty()))
        }

        async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish_hint(&self, _topic: &TopicId, _hint: GossipHint) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct TrackingHintTransport {
        hints: Arc<TokioMutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
        unsubscribed_topics: Arc<TokioMutex<Vec<String>>>,
    }

    impl TrackingHintTransport {
        async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
            let mut guard = self.hints.lock().await;
            guard
                .entry(topic.as_str().to_string())
                .or_insert_with(|| broadcast::channel(64).0)
                .clone()
        }
    }

    #[async_trait]
    impl HintTransport for TrackingHintTransport {
        async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
            let sender = self.hint_sender(topic).await;
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok() });
            Ok(Box::pin(stream))
        }

        async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
            self.unsubscribed_topics
                .lock()
                .await
                .push(topic.as_str().to_string());
            Ok(())
        }

        async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
            let sender = self.hint_sender(topic).await;
            let _ = sender.send(HintEnvelope {
                hint,
                received_at: Utc::now().timestamp_millis(),
                source_peer: "tracking".into(),
            });
            Ok(())
        }
    }

    async fn assert_docs_sync_recovers_post_without_hints(topic: &str, content: &str) {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a,
            stack_a.transport.clone(),
            Arc::new(NoopHintTransport),
            stack_a.docs_sync.clone(),
            stack_a.blob_service.clone(),
            generate_keys(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b,
            stack_b.transport.clone(),
            Arc::new(NoopHintTransport),
            stack_b.docs_sync.clone(),
            stack_b.blob_service.clone(),
            generate_keys(),
        );

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let object_id = app_a
            .create_post(topic, content, None)
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
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
        .expect("missing gossip timeout");

        assert_eq!(received.content, content);
    }

    #[tokio::test]
    async fn create_post_and_list_timeline() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post("kukuri:topic:api", "hello app", None)
            .await
            .expect("create post");
        let timeline = app
            .list_timeline("kukuri:topic:api", None, 10)
            .await
            .expect("timeline");

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].object_id, object_id);
        assert_eq!(timeline.items[0].content, "hello app");
    }

    #[tokio::test]
    async fn dm_send_requires_mutual_relationship() {
        let (app, _, _, _) = local_app_with_memory_services();
        let peer_keys = generate_keys();

        let error = app
            .send_direct_message(
                peer_keys.public_key_hex().as_str(),
                Some("hello"),
                None,
                Vec::new(),
            )
            .await
            .expect_err("direct message send should require mutual relationship");

        assert!(
            error
                .to_string()
                .contains("direct message requires a mutual relationship")
        );
    }

    #[tokio::test]
    async fn dm_reopens_finished_subscription_when_hint_stream_closes() {
        let transport = Arc::new(StaticTransport::new(PeerSnapshot {
            connected: true,
            peer_count: 1,
            connected_peers: vec!["peer-b".into()],
            configured_peers: vec!["peer-b".into()],
            subscribed_topics: Vec::new(),
            pending_events: 0,
            status_detail: "connected".into(),
            last_error: None,
            topic_diagnostics: Vec::new(),
        }));
        let hint_transport = Arc::new(CountingClosingHintTransport::default());
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let store = Arc::new(MemoryStore::default());
        let keys_local = generate_keys();
        let keys_peer = generate_keys();
        let local_pubkey = keys_local.public_key_hex();
        let peer_pubkey = keys_peer.public_key_hex();
        let follow_local_to_peer = parse_follow_edge(
            &build_follow_edge_envelope(
                &keys_local,
                &Pubkey::from(peer_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build follow edge local->peer"),
        )
        .expect("parse follow edge local->peer")
        .expect("follow edge local->peer");
        let follow_peer_to_local = parse_follow_edge(
            &build_follow_edge_envelope(
                &keys_peer,
                &Pubkey::from(local_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build follow edge peer->local"),
        )
        .expect("parse follow edge peer->local")
        .expect("follow edge peer->local");
        store
            .upsert_follow_edge(follow_local_to_peer)
            .await
            .expect("seed local->peer follow edge");
        store
            .upsert_follow_edge(follow_peer_to_local)
            .await
            .expect("seed peer->local follow edge");

        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport,
            hint_transport.clone(),
            docs_sync,
            blob_service,
            keys_local,
        );

        app.open_direct_message(peer_pubkey.as_str())
            .await
            .expect("open direct message first time");
        sleep(Duration::from_millis(50)).await;
        app.open_direct_message(peer_pubkey.as_str())
            .await
            .expect("open direct message second time");
        sleep(Duration::from_millis(50)).await;

        assert_eq!(*hint_transport.subscribe_count.lock().await, 2);
    }

    #[tokio::test]
    async fn dm_restart_resumes_pending_outbox_and_local_delete_prevents_duplicate_reinsert() {
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let hint_transport = Arc::new(TrackingHintTransport::default());
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let keys_a = generate_keys();
        let keys_b = generate_keys();
        let a_pubkey = keys_a.public_key_hex();
        let b_pubkey = keys_b.public_key_hex();
        let follow_a_to_b = parse_follow_edge(
            &build_follow_edge_envelope(
                &keys_a,
                &Pubkey::from(b_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build follow edge a->b"),
        )
        .expect("parse follow edge a->b")
        .expect("follow edge a->b");
        let follow_b_to_a = parse_follow_edge(
            &build_follow_edge_envelope(
                &keys_b,
                &Pubkey::from(a_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build follow edge b->a"),
        )
        .expect("parse follow edge b->a")
        .expect("follow edge b->a");

        store_a
            .upsert_follow_edge(follow_a_to_b.clone())
            .await
            .expect("seed follow edge a->b in store a");
        store_a
            .upsert_follow_edge(follow_b_to_a.clone())
            .await
            .expect("seed follow edge b->a in store a");
        store_b
            .upsert_follow_edge(follow_a_to_b)
            .await
            .expect("seed follow edge a->b in store b");
        store_b
            .upsert_follow_edge(follow_b_to_a)
            .await
            .expect("seed follow edge b->a in store b");

        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a.clone(),
            transport.clone(),
            hint_transport.clone(),
            docs_sync.clone(),
            blob_service.clone(),
            keys_a.clone(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b.clone(),
            transport.clone(),
            hint_transport.clone(),
            docs_sync.clone(),
            blob_service.clone(),
            keys_b.clone(),
        );

        app_b
            .open_direct_message(a_pubkey.as_str())
            .await
            .expect("recipient opens direct message");

        let message_id = app_a
            .send_direct_message(
                b_pubkey.as_str(),
                None,
                None,
                vec![pending_image_attachment(
                    "image/png",
                    tiny_png_bytes().as_slice(),
                )],
            )
            .await
            .expect("queue direct message while offline");
        let queued_status = app_a
            .get_direct_message_status(b_pubkey.as_str())
            .await
            .expect("queued status");
        assert_eq!(queued_status.pending_outbox_count, 1);
        let queued_outbox = store_a
            .list_direct_message_outbox()
            .await
            .expect("list queued outbox");
        assert_eq!(queued_outbox.len(), 1);
        let queued_frame = queued_outbox[0].clone();

        let initial_timeline = app_b
            .list_direct_message_messages(a_pubkey.as_str(), None, 20)
            .await
            .expect("initial recipient timeline");
        assert!(initial_timeline.items.is_empty());

        drop(app_a);

        let reopened_app_a = AppService::new_with_services(
            store_a.clone(),
            store_a.clone(),
            transport.clone(),
            hint_transport.clone(),
            docs_sync.clone(),
            blob_service.clone(),
            keys_a.clone(),
        );
        reopened_app_a
            .resume_direct_message_state()
            .await
            .expect("resume direct message state");
        assert!(
            reopened_app_a
                .direct_message_subscriptions
                .lock()
                .await
                .contains_key(b_pubkey.as_str()),
            "resume should restore the direct message subscription",
        );

        let topic = derive_direct_message_topic(&keys_a, &Pubkey::from(b_pubkey.as_str()))
            .expect("derive dm topic");
        {
            let mut snapshot = transport.peers.lock().await;
            snapshot.connected = true;
            snapshot.peer_count = 1;
            snapshot.connected_peers = vec!["peer-b".into()];
            snapshot.topic_diagnostics = vec![TopicPeerSnapshot {
                topic: format!("hint/{}", topic.as_str()),
                joined: true,
                peer_count: 1,
                connected_peers: vec!["peer-b".into()],
                configured_peer_ids: vec!["peer-b".into()],
                missing_peer_ids: Vec::new(),
                last_received_at: None,
                status_detail: "connected".into(),
                last_error: None,
            }];
        }
        let published = AppService::flush_direct_message_outbox_for_peer_with_services(
            store_a.as_ref(),
            hint_transport.as_ref(),
            transport.as_ref(),
            a_pubkey.as_str(),
            &keys_a,
            b_pubkey.as_str(),
        )
        .await
        .expect("flush queued direct message after restart");
        assert_eq!(published, 1);

        let delivered = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_direct_message_messages(a_pubkey.as_str(), None, 20)
                    .await
                    .expect("recipient timeline");
                if let Some(message) = timeline
                    .items
                    .iter()
                    .find(|item| item.message_id == message_id)
                {
                    break message.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("wait for delivered direct message");
        assert_eq!(delivered.text, "");
        assert_eq!(delivered.attachments.len(), 1);
        assert_eq!(delivered.attachments[0].role, "image_original");
        assert_eq!(delivered.attachments[0].mime, "image/png");
        assert!(!delivered.outgoing);
        assert!(delivered.delivered);

        timeout(Duration::from_secs(10), async {
            loop {
                let status = reopened_app_a
                    .get_direct_message_status(b_pubkey.as_str())
                    .await
                    .expect("sender status");
                if status.pending_outbox_count == 0 {
                    break;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("ack clears outbox");

        app_b
            .delete_direct_message_message(a_pubkey.as_str(), message_id.as_str())
            .await
            .expect("delete direct message locally");
        let after_delete = app_b
            .list_direct_message_messages(a_pubkey.as_str(), None, 20)
            .await
            .expect("timeline after delete");
        assert!(after_delete.items.is_empty());

        hint_transport
            .publish_hint(
                &topic,
                GossipHint::DirectMessageFrame {
                    topic_id: topic.clone(),
                    dm_id: queued_frame.dm_id.clone(),
                    message_id: queued_frame.message_id.clone(),
                    frame_hash: queued_frame.frame_blob_hash.clone(),
                },
            )
            .await
            .expect("republish duplicate direct message frame");
        sleep(Duration::from_millis(200)).await;

        let after_duplicate = app_b
            .list_direct_message_messages(a_pubkey.as_str(), None, 20)
            .await
            .expect("timeline after duplicate frame");
        assert!(after_duplicate.items.is_empty());
    }

    #[tokio::test]
    async fn public_post_reaction_persists_and_aggregates_emoji_and_custom_keys() {
        let (app, store, docs_sync, blob_service) = local_app_with_memory_services();
        let topic = "kukuri:topic:reaction-public";
        let object_id = app
            .create_post(topic, "reactable post", None)
            .await
            .expect("create post");
        let custom_asset = app
            .create_custom_reaction_asset(CreateCustomReactionAssetInput {
                search_key: "party".into(),
                mime: "image/png".into(),
                bytes: tiny_png_bytes(),
                width: 128,
                height: 128,
            })
            .await
            .expect("create custom reaction asset");

        let emoji_state = app
            .toggle_reaction(
                topic,
                object_id.as_str(),
                ReactionKeyV1::Emoji {
                    emoji: "👍".into()
                },
                None,
            )
            .await
            .expect("toggle emoji reaction");
        let custom_state = app
            .toggle_reaction(
                topic,
                object_id.as_str(),
                ReactionKeyV1::CustomAsset {
                    asset_id: custom_asset.asset_id.clone(),
                    snapshot: reaction_snapshot_from_view(&custom_asset),
                },
                None,
            )
            .await
            .expect("toggle custom reaction");
        let target = store
            .get_object_projection(&EnvelopeId::from(object_id.clone()))
            .await
            .expect("object projection")
            .expect("target projection");
        let reaction_rows = store
            .list_reaction_cache_for_target(
                &target.source_replica_id,
                &EnvelopeId::from(object_id.clone()),
            )
            .await
            .expect("reaction rows");
        let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
        let post = timeline
            .items
            .iter()
            .find(|item| item.object_id == object_id)
            .expect("reaction post");
        let author_replica = author_replica_id(custom_asset.owner_pubkey.as_str());
        let asset_docs = docs_sync
            .query_replica(
                &author_replica,
                DocQuery::Prefix("reactions/assets/".into()),
            )
            .await
            .expect("asset docs");
        let stored_blob = blob_service
            .fetch_blob(&kukuri_core::BlobHash::new(custom_asset.blob_hash.clone()))
            .await
            .expect("fetch stored blob")
            .expect("stored blob bytes");

        assert_eq!(emoji_state.target_object_id, object_id);
        assert_eq!(custom_state.target_object_id, object_id);
        assert_eq!(reaction_rows.len(), 2);
        assert!(
            reaction_rows
                .iter()
                .all(|row| row.status == ObjectStatus::Active)
        );
        assert_eq!(post.reaction_summary.len(), 2);
        assert_eq!(post.my_reactions.len(), 2);
        assert!(post.reaction_summary.iter().any(|entry| {
            entry.reaction_key_kind == "emoji"
                && entry.emoji.as_deref() == Some("👍")
                && entry.count == 1
        }));
        assert!(post.reaction_summary.iter().any(|entry| {
            entry.reaction_key_kind == "custom_asset"
                && entry
                    .custom_asset
                    .as_ref()
                    .map(|asset| asset.asset_id.as_str())
                    == Some(custom_asset.asset_id.as_str())
                && entry.count == 1
        }));
        assert_eq!(asset_docs.len(), 2);
        assert_eq!(stored_blob, tiny_png_bytes());
    }

    #[tokio::test]
    async fn same_author_same_reaction_key_toggles_off() {
        let (app, store, _, _) = local_app_with_memory_services();
        let topic = "kukuri:topic:reaction-toggle";
        let object_id = app
            .create_post(topic, "toggle me", None)
            .await
            .expect("create post");

        let first = app
            .toggle_reaction(
                topic,
                object_id.as_str(),
                ReactionKeyV1::Emoji {
                    emoji: "🎉".into()
                },
                None,
            )
            .await
            .expect("first toggle");
        let second = app
            .toggle_reaction(
                topic,
                object_id.as_str(),
                ReactionKeyV1::Emoji {
                    emoji: "🎉".into()
                },
                None,
            )
            .await
            .expect("second toggle");
        let target = store
            .get_object_projection(&EnvelopeId::from(object_id.clone()))
            .await
            .expect("object projection")
            .expect("target projection");
        let reaction_rows = store
            .list_reaction_cache_for_target(
                &target.source_replica_id,
                &EnvelopeId::from(object_id.clone()),
            )
            .await
            .expect("reaction rows");

        assert_eq!(first.reaction_summary.len(), 1);
        assert!(second.reaction_summary.is_empty());
        assert!(second.my_reactions.is_empty());
        assert_eq!(reaction_rows.len(), 1);
        assert_eq!(reaction_rows[0].status, ObjectStatus::Deleted);
    }

    #[tokio::test]
    async fn different_reaction_keys_can_coexist_on_same_target() {
        let (app, _, _, _) = local_app_with_memory_services();
        let topic = "kukuri:topic:reaction-coexist";
        let object_id = app
            .create_post(topic, "multiple reactions", None)
            .await
            .expect("create post");

        app.toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "🔥".into()
            },
            None,
        )
        .await
        .expect("fire reaction");
        let state = app
            .toggle_reaction(
                topic,
                object_id.as_str(),
                ReactionKeyV1::Emoji {
                    emoji: "😂".into()
                },
                None,
            )
            .await
            .expect("laugh reaction");

        assert_eq!(state.reaction_summary.len(), 2);
        assert_eq!(state.my_reactions.len(), 2);
        assert!(
            state
                .reaction_summary
                .iter()
                .any(|entry| entry.normalized_reaction_key == "emoji:🔥" && entry.count == 1)
        );
        assert!(
            state
                .reaction_summary
                .iter()
                .any(|entry| entry.normalized_reaction_key == "emoji:😂" && entry.count == 1)
        );
    }

    #[tokio::test]
    async fn custom_reaction_asset_is_author_owned_public_blob_backed_object() {
        let (app, _, docs_sync, blob_service) = local_app_with_memory_services();
        let asset = app
            .create_custom_reaction_asset(CreateCustomReactionAssetInput {
                search_key: "party".into(),
                mime: "image/png".into(),
                bytes: tiny_png_bytes(),
                width: 128,
                height: 128,
            })
            .await
            .expect("create custom reaction asset");
        let listed = app
            .list_my_custom_reaction_assets()
            .await
            .expect("list owned assets");
        let author_replica = author_replica_id(asset.owner_pubkey.as_str());
        let asset_docs = docs_sync
            .query_replica(
                &author_replica,
                DocQuery::Prefix("reactions/assets/".into()),
            )
            .await
            .expect("asset docs");
        let stored_blob = blob_service
            .fetch_blob(&kukuri_core::BlobHash::new(asset.blob_hash.clone()))
            .await
            .expect("fetch blob")
            .expect("stored blob");

        assert_eq!(listed, vec![asset.clone()]);
        assert_eq!(asset.search_key, "party");
        assert_eq!(asset_docs.len(), 2);
        assert_eq!(stored_blob, tiny_png_bytes());
    }

    #[tokio::test]
    async fn recent_reactions_return_latest_unique_keys_even_after_toggle_off() {
        let (app, _, _, _) = local_app_with_memory_services();
        let topic = "kukuri:topic:reaction-recent";
        let object_id = app
            .create_post(topic, "recent reactions", None)
            .await
            .expect("create post");

        app.toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "🔥".into()
            },
            None,
        )
        .await
        .expect("fire reaction");
        sleep(Duration::from_millis(5)).await;
        app.toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "😂".into()
            },
            None,
        )
        .await
        .expect("laugh reaction");
        sleep(Duration::from_millis(5)).await;
        app.toggle_reaction(
            topic,
            object_id.as_str(),
            ReactionKeyV1::Emoji {
                emoji: "🔥".into()
            },
            None,
        )
        .await
        .expect("toggle fire reaction off");

        let recent = app
            .list_recent_reactions(8)
            .await
            .expect("list recent reactions");

        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].normalized_reaction_key, "emoji:🔥");
        assert_eq!(recent[1].normalized_reaction_key, "emoji:😂");
    }

    #[tokio::test]
    async fn local_bookmarks_restore_saved_custom_reactions_after_restart() {
        let dir = tempdir().expect("tempdir");
        let database_path = dir.path().join("bookmark-store.sqlite");
        let store = Arc::new(
            SqliteStore::connect_file(&database_path)
                .await
                .expect("sqlite store"),
        );
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let local_keys = generate_keys();
        let foreign_keys = generate_keys();
        let foreign_pubkey = foreign_keys.public_key_hex();
        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service.clone(),
            local_keys,
        );

        app.bookmark_custom_reaction(CustomReactionAssetSnapshotV1 {
            asset_id: "asset-bookmarked".into(),
            owner_pubkey: Pubkey::from(foreign_pubkey.as_str()),
            blob_hash: kukuri_core::BlobHash::new("blob-bookmarked"),
            search_key: "bookmark".into(),
            mime: "image/png".into(),
            bytes: 128,
            width: 128,
            height: 128,
        })
        .await
        .expect("bookmark custom reaction");
        drop(app);
        store.close().await;

        let reopened = Arc::new(
            SqliteStore::connect_file(&database_path)
                .await
                .expect("reopen sqlite store"),
        );
        let reopened_app = AppService::new_with_services(
            reopened.clone(),
            reopened.clone(),
            transport,
            Arc::new(NoopHintTransport),
            docs_sync,
            blob_service,
            generate_keys(),
        );
        let bookmarks = reopened_app
            .list_bookmarked_custom_reactions()
            .await
            .expect("list bookmarks after restart");

        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].asset_id, "asset-bookmarked");
        assert_eq!(bookmarks[0].owner_pubkey, foreign_pubkey);
        assert_eq!(bookmarks[0].search_key, "bookmark");
    }

    #[test]
    fn legacy_custom_reaction_records_fall_back_to_asset_id_for_search_key() {
        let owner_pubkey = "b".repeat(64);
        let snapshot = CustomReactionAssetSnapshotV1 {
            asset_id: "asset-legacy".into(),
            owner_pubkey: Pubkey::from(owner_pubkey.as_str()),
            blob_hash: kukuri_core::BlobHash::new("blob-legacy"),
            search_key: "   ".into(),
            mime: "image/png".into(),
            bytes: 128,
            width: 128,
            height: 128,
        };
        let row = BookmarkedCustomReactionRow {
            asset_id: "asset-bookmarked-legacy".into(),
            owner_pubkey: owner_pubkey.clone(),
            blob_hash: kukuri_core::BlobHash::new("blob-bookmarked-legacy"),
            search_key: String::new(),
            mime: "image/png".into(),
            bytes: 128,
            width: 128,
            height: 128,
            bookmarked_at: 1,
        };

        assert_eq!(
            custom_reaction_asset_view_from_snapshot(&snapshot).search_key,
            "asset-legacy"
        );
        assert_eq!(
            bookmarked_custom_reaction_view_from_row(row).search_key,
            "asset-bookmarked-legacy"
        );
    }

    #[tokio::test]
    async fn private_channel_reaction_stays_epoch_scoped_after_rotate() {
        let (app, store, _, _) = local_app_with_memory_services();
        let topic = "kukuri:topic:reaction-private";
        let channel = app
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(topic),
                label: "friends".into(),
                audience_kind: ChannelAudienceKind::FriendOnly,
            })
            .await
            .expect("create private channel");
        let channel_id = ChannelId::new(channel.channel_id.clone());
        let channel_ref = ChannelRef::PrivateChannel {
            channel_id: channel_id.clone(),
        };
        let old_post_id = app
            .create_post_in_channel(topic, channel_ref.clone(), "before rotate", None)
            .await
            .expect("create old epoch post");
        let old_state = app
            .toggle_reaction(
                topic,
                old_post_id.as_str(),
                ReactionKeyV1::Emoji {
                    emoji: "👍".into()
                },
                Some(channel_ref.clone()),
            )
            .await
            .expect("toggle old epoch reaction");
        let old_target = store
            .get_object_projection(&EnvelopeId::from(old_post_id.clone()))
            .await
            .expect("old projection")
            .expect("old target");

        let rotated = app
            .rotate_private_channel(topic, channel.channel_id.as_str())
            .await
            .expect("rotate private channel");
        let new_post_id = app
            .create_post_in_channel(topic, channel_ref.clone(), "after rotate", None)
            .await
            .expect("create new epoch post");
        let new_state = app
            .toggle_reaction(
                topic,
                new_post_id.as_str(),
                ReactionKeyV1::Emoji {
                    emoji: "👍".into()
                },
                Some(channel_ref),
            )
            .await
            .expect("toggle new epoch reaction");
        let new_target = store
            .get_object_projection(&EnvelopeId::from(new_post_id.clone()))
            .await
            .expect("new projection")
            .expect("new target");
        let old_rows = store
            .list_reaction_cache_for_target(
                &old_target.source_replica_id,
                &EnvelopeId::from(old_post_id),
            )
            .await
            .expect("old reaction rows");
        let new_rows = store
            .list_reaction_cache_for_target(
                &new_target.source_replica_id,
                &EnvelopeId::from(new_post_id),
            )
            .await
            .expect("new reaction rows");

        assert_ne!(rotated.current_epoch_id, channel.current_epoch_id);
        assert_ne!(old_target.source_replica_id, new_target.source_replica_id);
        assert_eq!(
            old_state.source_replica_id,
            old_target.source_replica_id.as_str()
        );
        assert_eq!(
            new_state.source_replica_id,
            new_target.source_replica_id.as_str()
        );
        assert_eq!(old_rows.len(), 1);
        assert_eq!(new_rows.len(), 1);
        assert_eq!(old_rows[0].status, ObjectStatus::Active);
        assert_eq!(new_rows[0].status, ObjectStatus::Active);
        assert_ne!(old_rows[0].source_replica_id, new_rows[0].source_replica_id);
    }

    #[tokio::test]
    async fn create_public_post_persists_profile_post_doc_and_lists_profile_timeline() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let author_pubkey = keys.public_key_hex();
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service,
            keys,
        );
        let topic = "kukuri:topic:profile-doc";

        let object_id = app
            .create_post(topic, "hello profile", None)
            .await
            .expect("create post");
        let profile_docs =
            author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;

        assert_eq!(profile_docs.len(), 1);
        assert_eq!(profile_docs[0].author_pubkey.as_str(), author_pubkey);
        assert_eq!(
            profile_docs[0].profile_topic_id,
            author_profile_topic_id(author_pubkey.as_str())
        );
        assert_eq!(profile_docs[0].published_topic_id.as_str(), topic);
        assert_eq!(profile_docs[0].object_id.as_str(), object_id);
        assert_eq!(profile_docs[0].object_kind, "post");

        let timeline = app
            .list_profile_timeline(author_pubkey.as_str(), None, 20)
            .await
            .expect("profile timeline");
        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("profile post");

        assert_eq!(post.content, "hello profile");
        assert_eq!(post.published_topic_id.as_deref(), Some(topic));
        assert_eq!(post.channel_id, None);
        assert_eq!(post.audience_label, "Public");
        assert!(post.reaction_summary.is_empty());
        assert!(post.my_reactions.is_empty());
    }

    #[tokio::test]
    async fn public_reply_is_indexed_in_profile_timeline() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let author_pubkey = keys.public_key_hex();
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service,
            keys,
        );
        let topic = "kukuri:topic:profile-replies";

        let root_id = app
            .create_post(topic, "root", None)
            .await
            .expect("root post");
        let reply_id = app
            .create_post(topic, "reply", Some(root_id.as_str()))
            .await
            .expect("reply post");
        let profile_docs =
            author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
        let reply_doc = profile_docs
            .iter()
            .find(|doc| doc.object_id.as_str() == reply_id)
            .expect("reply profile doc");

        assert_eq!(reply_doc.object_kind, "comment");
        assert_eq!(
            reply_doc.reply_to_object_id.as_ref().map(|id| id.as_str()),
            Some(root_id.as_str())
        );
        assert_eq!(
            reply_doc.root_id.as_ref().map(|id| id.as_str()),
            Some(root_id.as_str())
        );

        let timeline = app
            .list_profile_timeline(author_pubkey.as_str(), None, 20)
            .await
            .expect("profile timeline");
        let reply = timeline
            .items
            .iter()
            .find(|post| post.object_id == reply_id)
            .expect("profile reply");

        assert_eq!(reply.object_kind, "comment");
        assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
        assert_eq!(reply.root_id.as_deref(), Some(root_id.as_str()));
        assert_eq!(reply.published_topic_id.as_deref(), Some(topic));
    }

    #[tokio::test]
    async fn private_channel_post_is_not_indexed_in_profile_timeline() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let author_pubkey = keys.public_key_hex();
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service,
            keys,
        );
        let topic = "kukuri:topic:profile-private";
        let channel = app
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(topic),
                label: "core".into(),
                audience_kind: ChannelAudienceKind::InviteOnly,
            })
            .await
            .expect("create private channel");

        let private_object_id = app
            .create_post_in_channel(
                topic,
                ChannelRef::PrivateChannel {
                    channel_id: ChannelId::new(channel.channel_id.clone()),
                },
                "private hello",
                None,
            )
            .await
            .expect("create private post");

        let profile_docs =
            author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
        assert!(profile_docs.is_empty());

        let timeline = app
            .list_profile_timeline(author_pubkey.as_str(), None, 20)
            .await
            .expect("profile timeline");
        assert!(
            timeline
                .items
                .iter()
                .all(|post| post.object_id != private_object_id)
        );
    }

    #[tokio::test]
    async fn set_my_profile_with_avatar_upload_persists_blob_backed_profile_and_author_view() {
        let (app, store, docs_sync, blob_service) = local_app_with_memory_services();
        let avatar_bytes = tiny_png_bytes();

        let updated = app
            .set_my_profile(ProfileInput {
                name: Some("avatar-owner".into()),
                display_name: Some("Avatar Owner".into()),
                about: Some("blob avatar".into()),
                picture: None,
                picture_upload: Some(PendingAttachment {
                    mime: "image/png".into(),
                    bytes: avatar_bytes.clone(),
                    role: AssetRole::ProfileAvatar,
                }),
                clear_picture: false,
            })
            .await
            .expect("set profile");

        let asset = updated.picture_asset.clone().expect("profile avatar asset");
        let stored_profile = store
            .get_profile(updated.pubkey.as_str())
            .await
            .expect("stored profile")
            .expect("stored profile value");
        let profile_doc = author_profile_doc(docs_sync.as_ref(), updated.pubkey.as_str())
            .await
            .expect("profile doc");
        let stored_blob = blob_service
            .fetch_blob(&asset.hash)
            .await
            .expect("fetch avatar blob")
            .expect("avatar blob");
        let local_profile = app.get_my_profile().await.expect("get my profile");
        let author_social = app
            .get_author_social_view(updated.pubkey.as_str())
            .await
            .expect("author social view");

        assert_eq!(updated.picture, None);
        assert_eq!(asset.mime, "image/png");
        assert_eq!(asset.role, AssetRole::ProfileAvatar);
        assert_eq!(stored_blob, avatar_bytes);
        assert_eq!(stored_profile.picture_asset, updated.picture_asset);
        assert_eq!(profile_doc.picture_asset, updated.picture_asset);
        assert_eq!(local_profile.picture_asset, updated.picture_asset);
        assert_eq!(author_social.picture, None);
        assert_eq!(
            author_social
                .picture_asset
                .as_ref()
                .map(|value| value.hash.as_str()),
            Some(asset.hash.as_str())
        );
        assert_eq!(
            author_social
                .picture_asset
                .as_ref()
                .map(|value| value.mime.as_str()),
            Some("image/png")
        );
        assert_eq!(
            author_social
                .picture_asset
                .as_ref()
                .map(|value| value.role.as_str()),
            Some("profile_avatar")
        );
    }

    #[tokio::test]
    async fn set_my_profile_keeps_legacy_picture_url_backward_compatible() {
        let (app, store, docs_sync, _) = local_app_with_memory_services();
        let legacy_picture = "https://example.com/avatar.png".to_string();

        let updated = app
            .set_my_profile(ProfileInput {
                name: Some("legacy-owner".into()),
                display_name: Some("Legacy Owner".into()),
                about: Some("legacy avatar".into()),
                picture: Some(legacy_picture.clone()),
                picture_upload: None,
                clear_picture: false,
            })
            .await
            .expect("set profile");

        let stored_profile = store
            .get_profile(updated.pubkey.as_str())
            .await
            .expect("stored profile")
            .expect("stored profile value");
        let profile_doc = author_profile_doc(docs_sync.as_ref(), updated.pubkey.as_str())
            .await
            .expect("profile doc");
        let local_profile = app.get_my_profile().await.expect("get my profile");
        let author_social = app
            .get_author_social_view(updated.pubkey.as_str())
            .await
            .expect("author social view");

        assert_eq!(updated.picture.as_deref(), Some(legacy_picture.as_str()));
        assert_eq!(updated.picture_asset, None);
        assert_eq!(
            stored_profile.picture.as_deref(),
            Some(legacy_picture.as_str())
        );
        assert_eq!(stored_profile.picture_asset, None);
        assert_eq!(
            profile_doc.picture.as_deref(),
            Some(legacy_picture.as_str())
        );
        assert_eq!(profile_doc.picture_asset, None);
        assert_eq!(
            local_profile.picture.as_deref(),
            Some(legacy_picture.as_str())
        );
        assert_eq!(local_profile.picture_asset, None);
        assert_eq!(
            author_social.picture.as_deref(),
            Some(legacy_picture.as_str())
        );
        assert_eq!(author_social.picture_asset, None);
    }

    #[tokio::test]
    async fn create_same_topic_repost_persists_repost_object_and_profile_repost_doc() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let author_pubkey = keys.public_key_hex();
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service,
            keys,
        );
        let topic = "kukuri:topic:repost-same";

        let source_object_id = app
            .create_post(topic, "hello repost", None)
            .await
            .expect("create source post");
        let repost_object_id = app
            .create_repost(topic, topic, source_object_id.as_str(), None)
            .await
            .expect("create repost");

        let timeline = app.list_timeline(topic, None, 20).await.expect("timeline");
        let repost = timeline
            .items
            .iter()
            .find(|post| post.object_id == repost_object_id)
            .expect("repost item");

        assert_eq!(repost.object_kind, "repost");
        assert_eq!(repost.published_topic_id.as_deref(), Some(topic));
        assert!(repost.repost_of.is_some());
        assert_eq!(repost.repost_commentary, None);
        assert!(!repost.is_threadable);

        let profile_docs =
            author_profile_repost_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
        assert_eq!(profile_docs.len(), 1);
        assert_eq!(profile_docs[0].object_id.as_str(), repost_object_id);
        assert_eq!(profile_docs[0].published_topic_id.as_str(), topic);
        assert_eq!(profile_docs[0].repost_of.source_topic_id.as_str(), topic);
    }

    #[tokio::test]
    async fn create_cross_topic_repost_renders_from_target_topic_without_tracking_source_topic() {
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let peer_snapshot = PeerSnapshot::default();
        let transport_a = Arc::new(StaticTransport::new(peer_snapshot.clone()));
        let transport_b = Arc::new(StaticTransport::new(peer_snapshot));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys_a = generate_keys();
        let author_pubkey = keys_a.public_key_hex();
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a,
            transport_a,
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service.clone(),
            keys_a,
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b,
            transport_b,
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service,
            generate_keys(),
        );
        let source_topic = "kukuri:topic:repost-source";
        let target_topic = "kukuri:topic:repost-target";

        let source_object_id = app_a
            .create_post(source_topic, "source post", None)
            .await
            .expect("create source post");
        let repost_object_id = app_a
            .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
            .await
            .expect("create cross-topic repost");

        let target_timeline = app_b
            .list_timeline(target_topic, None, 20)
            .await
            .expect("target timeline");
        let repost = target_timeline
            .items
            .iter()
            .find(|post| post.object_id == repost_object_id)
            .expect("repost item");

        assert_eq!(repost.object_kind, "repost");
        assert_eq!(repost.published_topic_id.as_deref(), Some(target_topic));
        assert_eq!(
            repost
                .repost_of
                .as_ref()
                .map(|value| value.source_topic_id.as_str()),
            Some(source_topic)
        );
        assert_eq!(
            repost
                .repost_of
                .as_ref()
                .map(|value| value.source_object_id.as_str()),
            Some(source_object_id.as_str())
        );
        assert_eq!(
            repost
                .repost_of
                .as_ref()
                .map(|value| value.content.as_str()),
            Some("source post")
        );

        let profile_timeline = app_a
            .list_profile_timeline(author_pubkey.as_str(), None, 20)
            .await
            .expect("profile timeline");
        assert!(
            profile_timeline
                .items
                .iter()
                .any(|post| post.object_id == repost_object_id)
        );
    }

    #[tokio::test]
    async fn simple_repost_is_unique_per_author_target_and_original() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync,
            blob_service,
            generate_keys(),
        );
        let source_topic = "kukuri:topic:repost-unique-source";
        let target_topic = "kukuri:topic:repost-unique-target";

        let source_object_id = app
            .create_post(source_topic, "source post", None)
            .await
            .expect("create source post");
        let repost_a = app
            .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
            .await
            .expect("create first repost");
        let repost_b = app
            .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
            .await
            .expect("create second repost");

        assert_eq!(repost_a, repost_b);

        let timeline = app
            .list_timeline(target_topic, None, 20)
            .await
            .expect("timeline");
        assert_eq!(
            timeline
                .items
                .iter()
                .filter(|post| post.object_id == repost_a)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn quote_repost_allows_multiple_distinct_quotes_for_same_original() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync,
            blob_service,
            generate_keys(),
        );
        let source_topic = "kukuri:topic:quote-source";
        let target_topic = "kukuri:topic:quote-target";

        let source_object_id = app
            .create_post(source_topic, "quoted source", None)
            .await
            .expect("create source post");
        let quote_a = app
            .create_repost(
                target_topic,
                source_topic,
                source_object_id.as_str(),
                Some("first quote"),
            )
            .await
            .expect("create first quote repost");
        let quote_b = app
            .create_repost(
                target_topic,
                source_topic,
                source_object_id.as_str(),
                Some("second quote"),
            )
            .await
            .expect("create second quote repost");

        assert_ne!(quote_a, quote_b);

        let timeline = app
            .list_timeline(target_topic, None, 20)
            .await
            .expect("timeline");
        assert!(
            timeline
                .items
                .iter()
                .filter(|post| post.object_kind == "repost")
                .count()
                >= 2
        );
    }

    #[tokio::test]
    async fn quote_repost_opens_own_thread_and_simple_repost_cannot_be_reply_parent() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync,
            blob_service,
            generate_keys(),
        );
        let source_topic = "kukuri:topic:reply-source";
        let target_topic = "kukuri:topic:reply-target";

        let source_object_id = app
            .create_post(source_topic, "source post", None)
            .await
            .expect("create source post");
        let simple_repost_id = app
            .create_repost(target_topic, source_topic, source_object_id.as_str(), None)
            .await
            .expect("create simple repost");
        let quote_repost_id = app
            .create_repost(
                target_topic,
                source_topic,
                source_object_id.as_str(),
                Some("quoted reply target"),
            )
            .await
            .expect("create quote repost");

        let simple_reply_error = app
            .create_post(
                target_topic,
                "reply to simple repost",
                Some(simple_repost_id.as_str()),
            )
            .await
            .expect_err("simple repost should reject replies");
        assert!(
            simple_reply_error
                .to_string()
                .contains("simple repost cannot be a reply parent")
        );

        let reply_id = app
            .create_post(
                target_topic,
                "reply to quote repost",
                Some(quote_repost_id.as_str()),
            )
            .await
            .expect("reply to quote repost");
        let thread = app
            .list_thread(target_topic, quote_repost_id.as_str(), None, 20)
            .await
            .expect("quote repost thread");
        assert!(
            thread
                .items
                .iter()
                .any(|post| post.object_id == quote_repost_id)
        );
        assert!(thread.items.iter().any(|post| post.object_id == reply_id));
    }

    #[tokio::test]
    async fn private_channel_post_cannot_be_reposted_publicly() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync,
            blob_service,
            generate_keys(),
        );
        let topic = "kukuri:topic:repost-private";
        let channel = app
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(topic),
                label: "core".into(),
                audience_kind: ChannelAudienceKind::InviteOnly,
            })
            .await
            .expect("create private channel");
        let private_object_id = app
            .create_post_in_channel(
                topic,
                ChannelRef::PrivateChannel {
                    channel_id: ChannelId::new(channel.channel_id.clone()),
                },
                "private source",
                None,
            )
            .await
            .expect("create private post");

        let error = app
            .create_repost(topic, topic, private_object_id.as_str(), None)
            .await
            .expect_err("private post should not be repostable");
        assert!(
            error
                .to_string()
                .contains("only public posts and comments can be reposted")
        );
    }

    #[tokio::test]
    async fn list_profile_timeline_ignores_profile_post_with_signer_mismatch() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let author_pubkey = keys.public_key_hex();
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            Arc::new(NoopHintTransport),
            docs_sync.clone(),
            blob_service,
            keys,
        );
        let topic = "kukuri:topic:profile-invalid";
        let valid_object_id = app
            .create_post(topic, "valid profile post", None)
            .await
            .expect("valid post");
        let forged_content = KukuriProfilePostEnvelopeContentV1 {
            author_pubkey: Pubkey::from(author_pubkey.as_str()),
            profile_topic_id: author_profile_topic_id(author_pubkey.as_str()),
            published_topic_id: TopicId::new(topic),
            object_id: EnvelopeId::from("forged-profile-post"),
            created_at: 123,
            object_kind: "post".into(),
            content: "forged profile post".into(),
            attachments: Vec::new(),
            reply_to_object_id: None,
            root_id: None,
        };
        let forged_envelope = kukuri_core::sign_envelope_json(
            &generate_keys(),
            "profile-post",
            vec![
                vec!["author".into(), author_pubkey.clone()],
                vec!["object".into(), "profile-post".into()],
                vec!["published_topic".into(), topic.into()],
                vec!["post".into(), forged_content.object_id.as_str().to_string()],
            ],
            &forged_content,
        )
        .expect("forged envelope");
        let replica = author_replica_id(author_pubkey.as_str());
        docs_sync
            .open_replica(&replica)
            .await
            .expect("open author replica");
        docs_sync
            .apply_doc_op(
                &replica,
                DocOp::SetJson {
                    key: stable_key("profile/posts", forged_content.object_id.as_str()),
                    value: serde_json::to_value(AuthorProfilePostDocV1 {
                        author_pubkey: forged_content.author_pubkey.clone(),
                        profile_topic_id: forged_content.profile_topic_id.clone(),
                        published_topic_id: forged_content.published_topic_id.clone(),
                        object_id: forged_content.object_id.clone(),
                        created_at: forged_content.created_at,
                        object_kind: forged_content.object_kind.clone(),
                        content: forged_content.content.clone(),
                        attachments: forged_content.attachments.clone(),
                        reply_to_object_id: None,
                        root_id: None,
                        envelope_id: forged_envelope.id.clone(),
                    })
                    .expect("forged doc json"),
                },
            )
            .await
            .expect("persist forged profile doc");
        docs_sync
            .apply_doc_op(
                &replica,
                DocOp::SetJson {
                    key: stable_key("envelopes", forged_envelope.id.as_str()),
                    value: serde_json::to_value(&forged_envelope).expect("forged envelope json"),
                },
            )
            .await
            .expect("persist forged envelope");

        let profile_docs =
            author_profile_post_docs(docs_sync.as_ref(), author_pubkey.as_str()).await;
        assert_eq!(profile_docs.len(), 2);

        let timeline = app
            .list_profile_timeline(author_pubkey.as_str(), None, 20)
            .await
            .expect("profile timeline");

        assert!(
            timeline
                .items
                .iter()
                .any(|post| post.object_id == valid_object_id)
        );
        assert!(
            timeline
                .items
                .iter()
                .all(|post| post.object_id != forged_content.object_id.as_str())
        );
    }

    #[tokio::test]
    async fn create_post_with_image_attachment_surfaces_attachment_metadata() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post_with_attachments(
                "kukuri:topic:image-write",
                "caption",
                None,
                vec![PendingAttachment {
                    mime: "image/png".into(),
                    bytes: b"fake-image".to_vec(),
                    role: AssetRole::ImageOriginal,
                }],
            )
            .await
            .expect("create image post");
        let timeline = app
            .list_timeline("kukuri:topic:image-write", None, 10)
            .await
            .expect("timeline");

        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("image post");
        assert_eq!(post.content, "caption");
        assert_eq!(post.attachments.len(), 1);
        assert_eq!(post.attachments[0].mime, "image/png");
        assert_eq!(post.attachments[0].role, "image_original");
        assert_eq!(post.attachments[0].status, BlobViewStatus::Available);
    }

    #[tokio::test]
    async fn create_post_with_image_only_succeeds() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post_with_attachments(
                "kukuri:topic:image-only",
                "",
                None,
                vec![PendingAttachment {
                    mime: "image/jpeg".into(),
                    bytes: b"fake-jpeg".to_vec(),
                    role: AssetRole::ImageOriginal,
                }],
            )
            .await
            .expect("create image-only post");
        let timeline = app
            .list_timeline("kukuri:topic:image-only", None, 10)
            .await
            .expect("timeline");

        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("image-only post");
        assert_eq!(post.attachments.len(), 1);
        assert_eq!(post.attachments[0].mime, "image/jpeg");
    }

    #[tokio::test]
    async fn create_post_with_video_attachments_surfaces_video_metadata() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post_with_attachments(
                "kukuri:topic:video-write",
                "video caption",
                None,
                vec![
                    pending_video_attachment(
                        AssetRole::VideoManifest,
                        "video/mp4",
                        b"fake-video-manifest",
                    ),
                    pending_video_attachment(
                        AssetRole::VideoPoster,
                        "image/jpeg",
                        b"fake-video-poster",
                    ),
                ],
            )
            .await
            .expect("create video post");
        let timeline = app
            .list_timeline("kukuri:topic:video-write", None, 10)
            .await
            .expect("timeline");

        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("video post");
        assert_eq!(post.attachments.len(), 2);
        assert!(
            post.attachments
                .iter()
                .any(|attachment| attachment.role == "video_manifest")
        );
        assert!(
            post.attachments
                .iter()
                .any(|attachment| attachment.role == "video_poster")
        );
    }

    #[tokio::test]
    async fn tracking_multiple_topics_updates_sync_status() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let _ = app
            .list_timeline("kukuri:topic:one", None, 10)
            .await
            .expect("timeline one");
        let _ = app
            .list_timeline("kukuri:topic:two", None, 10)
            .await
            .expect("timeline two");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:one")
        );
        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:two")
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .any(|topic| topic.topic == "kukuri:topic:one")
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .any(|topic| topic.topic == "kukuri:topic:two")
        );
        assert_eq!(status.status_detail, "No peers configured");
        assert!(
            status
                .topic_diagnostics
                .iter()
                .all(|topic| !topic.status_detail.is_empty())
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .all(|topic| topic.last_error.is_none())
        );
    }

    #[tokio::test]
    async fn discovery_status_separates_bootstrap_seed_peers_from_manual_tickets() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        transport
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                vec![SeedPeer {
                    endpoint_id: "configured-peer".into(),
                    addr_hint: None,
                }],
                vec![SeedPeer {
                    endpoint_id: "bootstrap-peer".into(),
                    addr_hint: None,
                }],
            )
            .await
            .expect("configure discovery");
        transport
            .import_ticket("manual-ticket-peer")
            .await
            .expect("import ticket");
        let app = AppService::new(store, transport);

        let discovery = app.get_discovery_status().await.expect("discovery status");

        assert_eq!(
            discovery.configured_seed_peer_ids,
            vec!["configured-peer".to_string()]
        );
        assert_eq!(
            discovery.bootstrap_seed_peer_ids,
            vec!["bootstrap-peer".to_string()]
        );
        assert_eq!(
            discovery.manual_ticket_peer_ids,
            vec!["manual-ticket-peer".to_string()]
        );
        assert!(discovery.assist_peer_ids.is_empty());
    }

    #[tokio::test]
    async fn relay_assisted_peers_contribute_to_sync_status_and_topic_counts() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot {
            connected: false,
            peer_count: 0,
            connected_peers: Vec::new(),
            configured_peers: vec!["peer-a".into(), "peer-b".into()],
            subscribed_topics: vec!["kukuri:topic:relay-assisted".into()],
            pending_events: 0,
            status_detail: "No peers configured".into(),
            last_error: None,
            topic_diagnostics: vec![TopicPeerSnapshot {
                topic: "kukuri:topic:relay-assisted".into(),
                joined: false,
                peer_count: 0,
                connected_peers: Vec::new(),
                configured_peer_ids: vec!["peer-a".into(), "peer-b".into()],
                missing_peer_ids: vec!["peer-a".into(), "peer-b".into()],
                last_received_at: None,
                status_detail: "No peers configured".into(),
                last_error: None,
            }],
        }));
        let docs_sync = Arc::new(AssistedDocsSync::new(vec!["peer-a", "peer-b"]));
        let blob_service = Arc::new(AssistedBlobService::new(vec!["peer-b", "peer-c"]));
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            generate_keys(),
        );

        let status = app.get_sync_status().await.expect("sync status");

        assert!(status.connected);
        assert_eq!(status.peer_count, 3);
        assert_eq!(
            status.status_detail,
            "relay-assisted sync available via 3 peer(s)"
        );
        assert_eq!(
            status.discovery.assist_peer_ids,
            vec![
                "peer-a".to_string(),
                "peer-b".to_string(),
                "peer-c".to_string()
            ]
        );
        assert_eq!(status.topic_diagnostics.len(), 1);
        assert!(status.topic_diagnostics[0].joined);
        assert_eq!(status.topic_diagnostics[0].peer_count, 3);
        assert_eq!(
            status.topic_diagnostics[0].assist_peer_ids,
            vec![
                "peer-a".to_string(),
                "peer-b".to_string(),
                "peer-c".to_string()
            ]
        );
        assert_eq!(
            status.topic_diagnostics[0].status_detail,
            "relay-assisted sync available via 3 peer(s)"
        );
    }

    #[tokio::test]
    async fn list_timeline_restarts_topic_replica_sync_with_cooldown_when_projection_is_empty() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(TrackingDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync.clone(),
            blob_service,
            generate_keys(),
        );

        let timeline = app
            .list_timeline("kukuri:topic:replica-restart", None, 20)
            .await
            .expect("timeline");
        assert!(timeline.items.is_empty());

        let second_timeline = app
            .list_timeline("kukuri:topic:replica-restart", None, 20)
            .await
            .expect("second timeline");
        assert!(second_timeline.items.is_empty());

        let restarted = docs_sync.restarted_replicas.lock().await.clone();
        assert_eq!(
            restarted,
            vec![
                topic_replica_id("kukuri:topic:replica-restart")
                    .as_str()
                    .to_string()
            ]
        );
    }

    #[tokio::test]
    async fn set_discovery_seeds_restarts_topic_hint_subscription() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let hint_transport = Arc::new(TrackingHintTransport::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            hint_transport.clone(),
            Arc::new(MemoryDocsSync::default()),
            Arc::new(MemoryBlobService::default()),
            generate_keys(),
        );
        let topic = "kukuri:topic:hint-restart";

        let _ = app
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe timeline");

        app.set_discovery_seeds(
            DiscoveryMode::StaticPeer,
            false,
            vec![SeedPeer {
                endpoint_id: "peer-a".into(),
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("set discovery seeds");

        assert_eq!(
            hint_transport.unsubscribed_topics.lock().await.clone(),
            vec![topic.to_string()]
        );
    }

    #[tokio::test]
    async fn shutdown_unsubscribes_active_hint_topics() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let hint_transport = Arc::new(TrackingHintTransport::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport,
            hint_transport.clone(),
            Arc::new(MemoryDocsSync::default()),
            Arc::new(MemoryBlobService::default()),
            generate_keys(),
        );
        let topic = "kukuri:topic:shutdown";

        let _ = app
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe timeline");

        app.shutdown().await;

        assert_eq!(
            hint_transport.unsubscribed_topics.lock().await.clone(),
            vec![topic.to_string()]
        );
    }

    #[tokio::test]
    async fn list_timeline_rehydrates_placeholder_from_blob_store() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:hydrate");
        let stored_blob = blob_service
            .put_blob(b"hello after blob fetch".to_vec(), "text/plain")
            .await
            .expect("put blob");
        persist_test_post(
            docs_sync.as_ref(),
            Some(store.as_ref()),
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: stored_blob.hash.clone(),
                mime: stored_blob.mime.clone(),
                bytes: stored_blob.bytes,
            },
            Vec::new(),
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            keys,
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].content, "hello after blob fetch");
    }

    #[tokio::test]
    async fn on_demand_hydration_updates_last_sync_ts() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:on-demand-sync-ts");
        let stored_blob = blob_service
            .put_blob(b"hydrate updates sync ts".to_vec(), "text/plain")
            .await
            .expect("put blob");
        persist_test_post(
            docs_sync.as_ref(),
            None,
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: stored_blob.hash,
                mime: stored_blob.mime,
                bytes: stored_blob.bytes,
            },
            Vec::new(),
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            keys,
        );

        assert!(
            app.get_sync_status()
                .await
                .expect("status")
                .last_sync_ts
                .is_none()
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert_eq!(timeline.items.len(), 1);

        assert!(
            app.get_sync_status()
                .await
                .expect("status")
                .last_sync_ts
                .is_some()
        );
    }

    #[tokio::test]
    async fn sync_status_normalizes_hint_topic_names() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot {
            connected: true,
            peer_count: 1,
            connected_peers: vec!["peer-a".into()],
            configured_peers: vec!["peer-a".into()],
            subscribed_topics: vec!["hint/kukuri:topic:demo".into()],
            pending_events: 0,
            status_detail: "Connected".into(),
            last_error: None,
            topic_diagnostics: vec![TopicPeerSnapshot {
                topic: "hint/kukuri:topic:demo".into(),
                joined: true,
                peer_count: 1,
                connected_peers: vec!["peer-a".into()],
                configured_peer_ids: vec!["peer-a".into()],
                missing_peer_ids: Vec::new(),
                last_received_at: Some(1),
                status_detail: "Connected".into(),
                last_error: None,
            }],
        }));
        let app = AppService::new(store, transport);

        let status = app.get_sync_status().await.expect("sync status");

        assert_eq!(status.subscribed_topics, vec!["kukuri:topic:demo"]);
        assert_eq!(status.topic_diagnostics.len(), 1);
        assert_eq!(status.topic_diagnostics[0].topic, "kukuri:topic:demo");
    }

    #[tokio::test]
    async fn direct_message_peer_count_falls_back_to_connected_peers_when_topic_diagnostic_is_missing()
     {
        let transport = StaticTransport::new(PeerSnapshot {
            connected: true,
            peer_count: 1,
            connected_peers: vec!["peer-a".into()],
            configured_peers: vec!["peer-a".into()],
            subscribed_topics: vec!["kukuri:topic:demo".into()],
            pending_events: 0,
            status_detail: "Connected".into(),
            last_error: None,
            topic_diagnostics: Vec::new(),
        });
        let keys_a = generate_keys();
        let keys_b = generate_keys();
        let topic = derive_direct_message_topic(&keys_a, &keys_b.public_key())
            .expect("direct message topic");

        let peer_count = direct_message_topic_peer_count(&transport, &topic)
            .await
            .expect("direct message peer count");

        assert_eq!(peer_count, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_ticket_updates_sync_status_error_reason() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport should bind"),
        );
        let app = AppService::new(store, transport);

        let error = app
            .import_peer_ticket("not-a-ticket")
            .await
            .expect_err("invalid ticket should fail");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(error.to_string().contains("failed to import peer ticket"));
        assert!(
            status
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("failed to import peer ticket"))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn missing_gossip_but_docs_sync_recovers_post() {
        assert_docs_sync_recovers_post_without_hints("kukuri:topic:missing-gossip", "docs recover")
            .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gossip_loss_does_not_lose_durable_post() {
        assert_docs_sync_recovers_post_without_hints(
            "kukuri:topic:gossip-loss",
            "durable docs payload",
        )
        .await;
    }

    #[tokio::test]
    async fn thread_open_triggers_lazy_blob_fetch() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:thread-lazy");
        let root_blob = blob_service
            .put_blob(b"root body".to_vec(), "text/plain")
            .await
            .expect("put root blob");
        let root = persist_test_post(
            docs_sync.as_ref(),
            Some(store.as_ref()),
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: root_blob.hash,
                mime: root_blob.mime,
                bytes: root_blob.bytes,
            },
            Vec::new(),
            None,
        )
        .await;
        let reply_blob = blob_service
            .put_blob(b"reply body".to_vec(), "text/plain")
            .await
            .expect("put reply blob");
        let _reply = persist_test_post(
            docs_sync.as_ref(),
            Some(store.as_ref()),
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: reply_blob.hash,
                mime: reply_blob.mime,
                bytes: reply_blob.bytes,
            },
            Vec::new(),
            Some(&root),
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            generate_keys(),
        );

        let thread = app
            .list_thread(topic.as_str(), root.id.as_str(), None, 20)
            .await
            .expect("thread");

        assert_eq!(
            thread.items.len(),
            2,
            "thread items: {:?}",
            thread
                .items
                .iter()
                .map(|post| format!(
                    "{}|reply={:?}|root={:?}",
                    post.object_id, post.reply_to, post.root_id
                ))
                .collect::<Vec<_>>()
        );
        assert!(thread.items.iter().any(|post| post.content == "root body"));
        assert!(thread.items.iter().any(|post| post.content == "reply body"));
    }

    #[tokio::test]
    async fn image_post_visible_before_full_blob_download() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:image");
        let image_bytes = b"fake image bytes".to_vec();
        let image_hash = kukuri_core::blob_hash(&image_bytes);
        persist_test_post(
            docs_sync.as_ref(),
            None,
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: kukuri_core::BlobHash::new("f".repeat(64)),
                mime: "text/plain".into(),
                bytes: 0,
            },
            vec![kukuri_core::AssetRef {
                hash: image_hash.clone(),
                mime: "image/png".into(),
                bytes: image_bytes.len() as u64,
                role: kukuri_core::AssetRole::ImageOriginal,
            }],
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport.clone(),
            transport,
            docs_sync,
            blob_service.clone(),
            generate_keys(),
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].content, "[blob pending]");
        assert_eq!(timeline.items[0].content_status, BlobViewStatus::Missing);
        assert_eq!(timeline.items[0].attachments.len(), 1);
        assert_eq!(
            timeline.items[0].attachments[0].status,
            BlobViewStatus::Missing
        );
        assert_eq!(timeline.items[0].attachments[0].role, "image_original");

        blob_service
            .put_blob(image_bytes, "image/png")
            .await
            .expect("put image blob");

        let refreshed = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline after image fetch");
        assert_eq!(refreshed.items.len(), 1);
        assert_eq!(
            refreshed.items[0].attachments[0].status,
            BlobViewStatus::Available
        );
        assert_eq!(refreshed.items[0].attachments[0].mime, "image/png");
    }

    #[tokio::test]
    async fn video_post_visible_before_full_blob_download() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:video");
        let poster_hash = kukuri_core::blob_hash(b"poster-bytes");
        persist_test_post(
            docs_sync.as_ref(),
            None,
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: kukuri_core::BlobHash::new("f".repeat(64)),
                mime: "text/plain".into(),
                bytes: 13,
            },
            vec![
                kukuri_core::AssetRef {
                    hash: kukuri_core::blob_hash(b"video-bytes"),
                    mime: "video/mp4".into(),
                    bytes: 8192,
                    role: kukuri_core::AssetRole::VideoManifest,
                },
                kukuri_core::AssetRef {
                    hash: poster_hash.clone(),
                    mime: "image/jpeg".into(),
                    bytes: 1024,
                    role: kukuri_core::AssetRole::VideoPoster,
                },
            ],
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport.clone(),
            transport,
            docs_sync,
            blob_service.clone(),
            generate_keys(),
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        let post = &timeline.items[0];
        assert!(
            post.attachments
                .iter()
                .any(|attachment| attachment.role == "video_manifest")
        );
        assert!(
            post.attachments
                .iter()
                .find(|attachment| attachment.role == "video_poster")
                .is_some_and(|attachment| attachment.status == BlobViewStatus::Missing)
        );

        blob_service
            .put_blob(b"poster-bytes".to_vec(), "image/jpeg")
            .await
            .expect("put poster blob");
        let refreshed = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert!(
            refreshed.items[0]
                .attachments
                .iter()
                .find(|attachment| attachment.role == "video_poster")
                .is_some_and(|attachment| attachment.status == BlobViewStatus::Available)
        );
    }

    #[tokio::test]
    async fn new_writes_use_blob_text_payload_refs() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store.clone(), transport);
        let topic = "kukuri:topic:blobtext";

        let object_id = app
            .create_post(topic, "blob text only", None)
            .await
            .expect("create post");
        let projection =
            ProjectionStore::get_object_projection(store.as_ref(), &EnvelopeId::from(object_id))
                .await
                .expect("projection")
                .expect("projection row");

        assert!(matches!(
            projection.payload_ref,
            PayloadRef::BlobText { .. }
        ));
        assert!(!matches!(
            projection.payload_ref,
            PayloadRef::InlineText { .. }
        ));
    }

    #[tokio::test]
    async fn blob_media_payload_roundtrip() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            Arc::new(MemoryDocsSync::default()),
            blob_service.clone(),
            generate_keys(),
        );

        let stored = blob_service
            .put_blob(b"fake-image".to_vec(), "image/png")
            .await
            .expect("put image");
        let payload = app
            .blob_media_payload(stored.hash.as_str(), "image/png")
            .await
            .expect("media payload")
            .expect("media payload present");

        assert_eq!(payload.bytes_base64, "ZmFrZS1pbWFnZQ==");
        assert_eq!(payload.mime, "image/png");
        assert!(
            app.blob_media_payload(&"f".repeat(64), "image/png")
                .await
                .expect("missing payload")
                .is_none()
        );
    }

    #[tokio::test]
    async fn unsubscribe_topic_removes_subscription_from_sync_status() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let _ = app
            .list_timeline("kukuri:topic:one", None, 10)
            .await
            .expect("timeline one");
        let _ = app
            .list_timeline("kukuri:topic:two", None, 10)
            .await
            .expect("timeline two");
        app.unsubscribe_topic("kukuri:topic:two")
            .await
            .expect("unsubscribe topic");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:one")
        );
        assert!(
            !status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:two")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_post_between_apps() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("post-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("post-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b.clone(), &stack_b);

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let topic = "kukuri:topic:app-api-iroh";
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("app b should subscribe to topic");

        let object_id = app_a
            .create_post(topic, "hello over iroh transport", None)
            .await
            .expect("app a should create post");

        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
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
        .expect("timeline sync timeout");

        assert_eq!(received.content, "hello over iroh transport");
        let status_b = app_b.get_sync_status().await.expect("sync status b");
        assert!(status_b.last_sync_ts.is_some());
        assert!(
            status_b
                .subscribed_topics
                .iter()
                .any(|value| value == topic)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_image_post_between_apps() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("image-post-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("image-post-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b.clone(), &stack_b);

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let topic = "kukuri:topic:image-sync";
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("app b should subscribe to topic");

        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "caption over iroh",
                None,
                vec![pending_image_attachment("image/png", b"fake-image-sync")],
            )
            .await
            .expect("create image post");

        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
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
        .expect("image sync timeout");

        assert_eq!(received.content, "caption over iroh");
        assert_eq!(received.attachments.len(), 1);
        assert_eq!(received.attachments[0].mime, "image/png");
        assert_eq!(received.attachments[0].status, BlobViewStatus::Available);
        assert!(
            app_b
                .blob_preview_data_url(received.attachments[0].hash.as_str(), "image/png")
                .await
                .expect("preview data url")
                .is_some()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_video_manifest_payload_available_after_sync() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("video-post-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("video-post-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let topic = "kukuri:topic:video-sync";
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");

        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "video caption",
                None,
                vec![
                    pending_video_attachment(AssetRole::VideoManifest, "video/mp4", b"video-sync"),
                    pending_video_attachment(AssetRole::VideoPoster, "image/jpeg", b"poster-sync"),
                ],
            )
            .await
            .expect("create video post");

        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
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
        .expect("video sync timeout");

        assert!(
            received
                .attachments
                .iter()
                .any(|attachment| attachment.role == "video_manifest")
        );
        let poster = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("video poster");
        assert_eq!(poster.status, BlobViewStatus::Available);
        let poster_payload = app_b
            .blob_media_payload(poster.hash.as_str(), "image/jpeg")
            .await
            .expect("poster media payload")
            .expect("poster payload present");
        assert_eq!(poster_payload.mime, "image/jpeg");
        let manifest = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("video manifest");
        let manifest_payload = app_b
            .blob_media_payload(manifest.hash.as_str(), "video/mp4")
            .await
            .expect("video media payload")
            .expect("manifest payload present");
        assert_eq!(manifest_payload.mime, "video/mp4");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn import_peer_ticket_rebuilds_existing_topic_subscription() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("rebind-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("rebind-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:rebind-after-import";

        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a before import");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b before import");

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        timeout(Duration::from_secs(10), async {
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
                let ready_a = status_a.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && topic_status.peer_count > 0
                });
                let ready_b = status_b.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && topic_status.peer_count > 0
                });
                if ready_a && ready_b {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("subscription rebuild timeout");

        let object_id = app_a
            .create_post(topic, "hello after import", None)
            .await
            .expect("create post");
        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
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
        .expect("timeline sync timeout");

        assert_eq!(received.content, "hello after import");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seeded_dht_syncs_post_between_apps_without_ticket_import() {
        let dir = tempdir().expect("tempdir");
        let testnet = Testnet::new(5).expect("testnet");
        let stack_a = TestIrohStack::new_with_dht(&dir.path().join("seeded-dht-a"), &testnet).await;
        let stack_b = TestIrohStack::new_with_dht(&dir.path().join("seeded-dht-b"), &testnet).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let endpoint_a = app_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = app_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;

        configure_seeded_dht(&app_a, endpoint_b.clone()).await;
        configure_seeded_dht(&app_b, endpoint_a.clone()).await;
        let topic = "kukuri:topic:seeded-dht-app";
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a timeline");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        timeout(Duration::from_secs(90), async {
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
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
        .expect("seeded dht ready timeout");

        let object_id = app_a
            .create_post(topic, "seeded dht app sync", None)
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
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
        .expect("seeded dht sync timeout");

        assert_eq!(received.content, "seeded dht app sync");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seeded_dht_rebuilds_existing_topic_subscription_after_seed_update() {
        let dir = tempdir().expect("tempdir");
        let testnet = Testnet::new(5).expect("testnet");
        let stack_a =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-rebind-a"), &testnet).await;
        let stack_b =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-rebind-b"), &testnet).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:seeded-rebind";

        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a before seed update");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b before seed update");

        let endpoint_a = app_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = app_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;
        configure_seeded_dht(&app_a, endpoint_b.clone()).await;
        configure_seeded_dht(&app_b, endpoint_a.clone()).await;

        timeout(Duration::from_secs(20), async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
                let ready_a = status_a
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                let ready_b = status_b
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
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
        .expect("seeded dht topic rebind timeout");

        let object_id = app_a
            .create_post(topic, "seeded dht rebind", None)
            .await
            .expect("create post");

        timeout(Duration::from_secs(90), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
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
        .expect("seeded dht propagation timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seeded_dht_backfills_docs_and_blobs_with_id_only_seed() {
        let dir = tempdir().expect("tempdir");
        let testnet = Testnet::new(5).expect("testnet");
        let stack_a =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-image-a"), &testnet).await;
        let stack_b =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-image-b"), &testnet).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let endpoint_a = app_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = app_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;
        configure_seeded_dht(&app_a, endpoint_b.clone()).await;
        configure_seeded_dht(&app_b, endpoint_a.clone()).await;
        let topic = "kukuri:topic:seeded-image";
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a timeline");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        timeout(Duration::from_secs(20), async {
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
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
        .expect("seeded dht image ready timeout");

        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "seeded image",
                None,
                vec![pending_image_attachment("image/png", b"seeded-image-bytes")],
            )
            .await
            .expect("create image post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
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
        .expect("seeded dht image backfill timeout");

        assert_eq!(received.attachments.len(), 1);
        assert_eq!(received.attachments[0].status, BlobViewStatus::Available);
        assert!(
            app_b
                .blob_preview_data_url(received.attachments[0].hash.as_str(), "image/png")
                .await
                .expect("preview")
                .is_some()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_image_post_from_docs() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("late-image-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("late-image-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);

        let topic = "kukuri:topic:late-image";
        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "late image caption",
                None,
                vec![pending_image_attachment("image/png", b"late-image-bytes")],
            )
            .await
            .expect("create image post before join");
        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(60), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    let post = post.clone();
                    if post.attachments.len() == 1 {
                        let preview = app_b
                            .blob_preview_data_url(post.attachments[0].hash.as_str(), "image/png")
                            .await
                            .expect("preview data url");
                        if preview.is_some() {
                            let refreshed_timeline = app_b
                                .list_timeline(topic, None, 20)
                                .await
                                .expect("timeline b refreshed");
                            if let Some(refreshed_post) = refreshed_timeline
                                .items
                                .iter()
                                .find(|candidate| candidate.object_id == object_id)
                                .cloned()
                                && refreshed_post.attachments.len() == 1
                                && refreshed_post.attachments[0].status == BlobViewStatus::Available
                            {
                                return refreshed_post;
                            }
                        }
                    }
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late image join timeout");

        assert_eq!(received.attachments.len(), 1);
        assert_eq!(received.attachments[0].status, BlobViewStatus::Available);
        assert!(
            app_b
                .blob_preview_data_url(received.attachments[0].hash.as_str(), "image/png")
                .await
                .expect("preview data url")
                .is_some()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_video_media_payload() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("late-video-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("late-video-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);

        let topic = "kukuri:topic:late-video";
        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "late video caption",
                None,
                vec![
                    pending_video_attachment(AssetRole::VideoManifest, "video/mp4", b"late-video"),
                    pending_video_attachment(AssetRole::VideoPoster, "image/jpeg", b"late-poster"),
                ],
            )
            .await
            .expect("create video post before join");
        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
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
        .expect("late video join timeout");

        let poster = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("video poster");
        assert_eq!(poster.status, BlobViewStatus::Available);
        let poster_payload = app_b
            .blob_media_payload(poster.hash.as_str(), "image/jpeg")
            .await
            .expect("poster media payload")
            .expect("poster payload present");
        assert_eq!(poster_payload.mime, "image/jpeg");
        let manifest = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("video manifest");
        let manifest_payload = app_b
            .blob_media_payload(manifest.hash.as_str(), "video/mp4")
            .await
            .expect("video media payload")
            .expect("manifest payload present");
        assert_eq!(manifest_payload.mime, "video/mp4");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_reply_into_thread() {
        let _guard = iroh_integration_test_lock().lock_owned().await;
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("reply-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("reply-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a.clone(), &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:reply-thread";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a timeline");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        wait_for_topic_peer_count(&app_a, topic, 1).await;
        wait_for_topic_peer_count(&app_b, topic, 1).await;

        let root_id = app_a
            .create_post(topic, "root over iroh", None)
            .await
            .expect("create root");

        timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if timeline.items.iter().any(|post| post.object_id == root_id) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("root propagation timeout");

        let reply_id = app_b
            .create_post(topic, "reply over iroh", Some(root_id.as_str()))
            .await
            .expect("create reply");
        let thread = timeout(p2p_replication_timeout(), async {
            loop {
                let thread = app_b
                    .list_thread(topic, root_id.as_str(), None, 20)
                    .await
                    .expect("thread b");
                if thread.items.iter().any(|post| post.object_id == reply_id) {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("local reply propagation timeout");

        let thread_ids = thread
            .items
            .iter()
            .map(|post| post.object_id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            thread_ids.len(),
            2,
            "thread items: {:?}",
            thread
                .items
                .iter()
                .map(|post| format!(
                    "{}|reply={:?}|root={:?}",
                    post.object_id, post.reply_to, post.root_id
                ))
                .collect::<Vec<_>>()
        );
        assert!(thread_ids.contains(root_id.as_str()));
        assert!(thread_ids.contains(reply_id.as_str()));
        let reply = thread
            .items
            .iter()
            .find(|post| post.object_id == reply_id)
            .expect("reply in thread");
        assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
        assert_eq!(reply.root_id.as_deref(), Some(root_id.as_str()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn image_reply_thread_syncs() {
        let _guard = iroh_integration_test_lock().lock_owned().await;
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("image-thread-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("image-thread-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a.clone(), &stack_a);
        let app_b = app_with_iroh_services(store_b.clone(), &stack_b);
        let topic = "kukuri:topic:image-thread";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a timeline");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        wait_for_topic_peer_count(&app_a, topic, 1).await;
        wait_for_topic_peer_count(&app_b, topic, 1).await;

        let root_id = app_a
            .create_post_with_attachments(
                topic,
                "root image",
                None,
                vec![pending_image_attachment("image/png", b"root-image")],
            )
            .await
            .expect("create root image");

        timeout(p2p_replication_timeout(), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if timeline.items.iter().any(|post| post.object_id == root_id) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("root image propagation timeout");

        let reply_id = app_b
            .create_post_with_attachments(
                topic,
                "reply image",
                Some(root_id.as_str()),
                vec![pending_image_attachment("image/jpeg", b"reply-image")],
            )
            .await
            .expect("create reply image");
        let thread = timeout(p2p_replication_timeout(), async {
            loop {
                let thread = app_b
                    .list_thread(topic, root_id.as_str(), None, 20)
                    .await
                    .expect("thread b");
                if thread.items.iter().any(|post| post.object_id == reply_id) {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("image reply propagation timeout");

        let root = thread
            .items
            .iter()
            .find(|post| post.object_id == root_id)
            .expect("root in thread");
        let reply = thread
            .items
            .iter()
            .find(|post| post.object_id == reply_id)
            .expect("reply in thread");
        assert_eq!(root.attachments[0].mime, "image/png");
        assert_eq!(reply.attachments[0].mime, "image/jpeg");
        assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_multiple_topics_bidirectionally() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("multi-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("multi-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic_one = "kukuri:topic:one";
        let topic_two = "kukuri:topic:two";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let _ = app_a
            .list_timeline(topic_one, None, 20)
            .await
            .expect("subscribe a topic one");
        let _ = app_a
            .list_timeline(topic_two, None, 20)
            .await
            .expect("subscribe a topic two");
        let _ = app_b
            .list_timeline(topic_one, None, 20)
            .await
            .expect("subscribe b topic one");
        let _ = app_b
            .list_timeline(topic_two, None, 20)
            .await
            .expect("subscribe b topic two");

        let id_one = app_a
            .create_post(topic_one, "topic one from a", None)
            .await
            .expect("post one");
        let id_two = app_b
            .create_post(topic_two, "topic two from b", None)
            .await
            .expect("post two");

        timeout(Duration::from_secs(10), async {
            loop {
                let timeline_b = app_b
                    .list_timeline(topic_one, None, 20)
                    .await
                    .expect("timeline b");
                let timeline_a = app_a
                    .list_timeline(topic_two, None, 20)
                    .await
                    .expect("timeline a");
                let has_one = timeline_b.items.iter().any(|post| post.object_id == id_one);
                let has_two = timeline_a.items.iter().any(|post| post.object_id == id_two);
                if has_one && has_two {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("multi topic propagation timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_live_session_manifest() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("live-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("live-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:live-late";

        let session_id = app_a
            .create_live_session(
                topic,
                CreateLiveSessionInput {
                    title: "late live".into(),
                    description: "watch along".into(),
                },
            )
            .await
            .expect("create live session");

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let sessions = app_b
                    .list_live_sessions(topic)
                    .await
                    .expect("list live sessions");
                if let Some(session) = sessions
                    .into_iter()
                    .find(|session| session.session_id == session_id)
                {
                    return session;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("live session backfill timeout");

        assert_eq!(received.title, "late live");
        assert_eq!(received.status, LiveSessionStatus::Live);
    }

    #[tokio::test]
    async fn live_presence_expires_without_heartbeat() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
        let app = AppService::new(store, transport.clone());
        let topic = "kukuri:topic:presence-expiry";
        let session_id = app
            .create_live_session(
                topic,
                CreateLiveSessionInput {
                    title: "presence".into(),
                    description: "ttl".into(),
                },
            )
            .await
            .expect("create live session");

        transport
            .publish_hint(
                &TopicId::new(topic),
                GossipHint::LivePresence {
                    topic_id: TopicId::new(topic),
                    session_id: session_id.clone(),
                    author: Pubkey::from("a".repeat(64)),
                    ttl_ms: 100,
                },
            )
            .await
            .expect("publish live presence");

        timeout(Duration::from_secs(2), async {
            loop {
                let sessions = app
                    .list_live_sessions(topic)
                    .await
                    .expect("list live sessions");
                if sessions
                    .iter()
                    .any(|session| session.session_id == session_id && session.viewer_count == 1)
                {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("viewer count update timeout");

        sleep(Duration::from_millis(150)).await;
        let sessions = app
            .list_live_sessions(topic)
            .await
            .expect("list after expiry");
        let session = sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .expect("session present");
        assert_eq!(session.viewer_count, 0);
    }

    #[tokio::test]
    async fn ended_live_session_rejects_new_viewers() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
        let app = AppService::new(store, transport);
        let topic = "kukuri:topic:ended-live";
        let session_id = app
            .create_live_session(
                topic,
                CreateLiveSessionInput {
                    title: "ended".into(),
                    description: "session".into(),
                },
            )
            .await
            .expect("create live session");
        app.end_live_session(topic, session_id.as_str())
            .await
            .expect("end live session");

        let error = app
            .join_live_session(topic, session_id.as_str())
            .await
            .expect_err("join should fail");
        assert!(error.to_string().contains("ended live session"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn game_room_score_update_replicates() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("game-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("game-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:game-sync";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let room_id = app_a
            .create_game_room(
                topic,
                CreateGameRoomInput {
                    title: "sync room".into(),
                    description: "set".into(),
                    participants: vec!["Alice".into(), "Bob".into()],
                },
            )
            .await
            .expect("create game room");
        app_a
            .update_game_room(
                topic,
                room_id.as_str(),
                UpdateGameRoomInput {
                    status: GameRoomStatus::Running,
                    phase_label: Some("Round 2".into()),
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
                },
            )
            .await
            .expect("update game room");

        let received = timeout(Duration::from_secs(60), async {
            loop {
                let rooms = app_b.list_game_rooms(topic).await.expect("list game rooms");
                if let Some(room) = rooms.into_iter().find(|room| room.room_id == room_id) {
                    let alice_score = room
                        .scores
                        .iter()
                        .find(|score| score.label == "Alice")
                        .map(|score| score.score);
                    if room.status == GameRoomStatus::Running
                        && room.phase_label.as_deref() == Some("Round 2")
                        && alice_score == Some(2)
                    {
                        return room;
                    }
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("game room replication timeout");

        assert_eq!(received.status, GameRoomStatus::Running);
        assert_eq!(received.phase_label.as_deref(), Some("Round 2"));
        assert_eq!(
            received
                .scores
                .iter()
                .find(|score| score.label == "Alice")
                .map(|score| score.score),
            Some(2)
        );
    }

    #[tokio::test]
    async fn finished_game_room_rejects_updates() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
        let app = AppService::new(store, transport);
        let topic = "kukuri:topic:game-finished";
        let room_id = app
            .create_game_room(
                topic,
                CreateGameRoomInput {
                    title: "finished room".into(),
                    description: "set".into(),
                    participants: vec!["Alice".into(), "Bob".into()],
                },
            )
            .await
            .expect("create game room");

        app.update_game_room(
            topic,
            room_id.as_str(),
            UpdateGameRoomInput {
                status: GameRoomStatus::Ended,
                phase_label: Some("Final".into()),
                scores: vec![
                    GameScoreView {
                        participant_id: "participant-1".into(),
                        label: "Alice".into(),
                        score: 2,
                    },
                    GameScoreView {
                        participant_id: "participant-2".into(),
                        label: "Bob".into(),
                        score: 0,
                    },
                ],
            },
        )
        .await
        .expect("finish room");

        let error = app
            .update_game_room(
                topic,
                room_id.as_str(),
                UpdateGameRoomInput {
                    status: GameRoomStatus::Ended,
                    phase_label: Some("After".into()),
                    scores: vec![
                        GameScoreView {
                            participant_id: "participant-1".into(),
                            label: "Alice".into(),
                            score: 3,
                        },
                        GameScoreView {
                            participant_id: "participant-2".into(),
                            label: "Bob".into(),
                            score: 1,
                        },
                    ],
                },
            )
            .await
            .expect_err("ended room update should fail");
        assert!(error.to_string().contains("ended game room"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn private_channel_invite_scopes_posts_and_replies() {
        let _guard = iroh_integration_test_lock().lock_owned().await;
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("private-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("private-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a.clone(), &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:private-channel";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("warm owner public timeline");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("warm invitee public timeline");
        wait_for_topic_peer_count(&app_a, topic, 1).await;
        wait_for_topic_peer_count(&app_b, topic, 1).await;

        let channel = app_a
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(topic),
                label: "core".into(),
                audience_kind: ChannelAudienceKind::InviteOnly,
            })
            .await
            .expect("create private channel");
        let invite = app_a
            .export_private_channel_invite(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export invite");
        let preview = app_b
            .import_private_channel_invite(invite.as_str())
            .await
            .expect("import invite");
        assert_eq!(preview.channel_id.as_str(), channel.channel_id);

        let private_channel_id = ChannelId::new(channel.channel_id.clone());
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: private_channel_id.clone(),
        };
        let private_scope = TimelineScope::Channel {
            channel_id: private_channel_id.clone(),
        };
        let _ = app_a
            .list_timeline_scoped(topic, private_scope.clone(), None, 20)
            .await
            .expect("warm owner private timeline");

        let object_id = app_a
            .create_post_in_channel(topic, private_ref.clone(), "private hello", None)
            .await
            .expect("create private post");

        let received = timeout(p2p_replication_timeout(), async {
            loop {
                let public = app_b
                    .list_timeline_scoped(topic, TimelineScope::Public, None, 20)
                    .await
                    .expect("public timeline");
                assert!(
                    public.items.iter().all(|post| post.object_id != object_id),
                    "private post leaked into public scope"
                );
                let private = app_b
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline");
                if let Some(post) = private
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
        .expect("private timeline timeout");
        assert_eq!(
            received.channel_id.as_deref(),
            Some(channel.channel_id.as_str())
        );

        let reply_id = app_b
            .create_post_in_channel(
                topic,
                ChannelRef::Public,
                "private reply",
                Some(object_id.as_str()),
            )
            .await
            .expect("reply in private channel");

        let thread = timeout(p2p_replication_timeout(), async {
            loop {
                let thread = app_b
                    .list_thread(topic, object_id.as_str(), None, 20)
                    .await
                    .expect("thread b");
                if thread.items.iter().any(|post| post.object_id == reply_id) {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("private local thread timeout");
        let reply = thread
            .items
            .iter()
            .find(|post| post.object_id == reply_id)
            .expect("reply");
        assert_eq!(
            reply.channel_id.as_deref(),
            Some(channel.channel_id.as_str())
        );
        assert_eq!(reply.reply_to.as_deref(), Some(object_id.as_str()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn friend_only_grant_requires_mutual_and_rotate_requires_fresh_grant() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("friend-only-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("friend-only-b")).await;
        let stack_c = TestIrohStack::new(&dir.path().join("friend-only-c")).await;
        let stack_d = TestIrohStack::new(&dir.path().join("friend-only-d")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let store_c = Arc::new(MemoryStore::default());
        let store_d = Arc::new(MemoryStore::default());
        let keys_a = generate_keys();
        let keys_b = generate_keys();
        let keys_c = generate_keys();
        let keys_d = generate_keys();
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a.clone(),
            stack_a.transport.clone(),
            stack_a.transport.clone(),
            stack_a.docs_sync.clone(),
            stack_a.blob_service.clone(),
            keys_a.clone(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b.clone(),
            stack_b.transport.clone(),
            stack_b.transport.clone(),
            stack_b.docs_sync.clone(),
            stack_b.blob_service.clone(),
            keys_b.clone(),
        );
        let app_c = AppService::new_with_services(
            store_c.clone(),
            store_c.clone(),
            stack_c.transport.clone(),
            stack_c.transport.clone(),
            stack_c.docs_sync.clone(),
            stack_c.blob_service.clone(),
            keys_c.clone(),
        );
        let app_d = AppService::new_with_services(
            store_d.clone(),
            store_d.clone(),
            stack_d.transport.clone(),
            stack_d.transport.clone(),
            stack_d.docs_sync.clone(),
            stack_d.blob_service.clone(),
            keys_d.clone(),
        );
        app_a.warm_social_graph().await.expect("warm a");
        app_b.warm_social_graph().await.expect("warm b");
        app_c.warm_social_graph().await.expect("warm c");
        app_d.warm_social_graph().await.expect("warm d");

        let ticket_a = stack_a
            .transport
            .export_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = stack_b
            .transport
            .export_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        let ticket_c = stack_c
            .transport
            .export_ticket()
            .await
            .expect("ticket c")
            .expect("ticket c value");
        let ticket_d = stack_d
            .transport
            .export_ticket()
            .await
            .expect("ticket d")
            .expect("ticket d value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("a imports b");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("b imports a");
        app_a
            .import_peer_ticket(&ticket_c)
            .await
            .expect("a imports c");
        app_c
            .import_peer_ticket(&ticket_a)
            .await
            .expect("c imports a");
        app_a
            .import_peer_ticket(&ticket_d)
            .await
            .expect("a imports d");
        app_d
            .import_peer_ticket(&ticket_a)
            .await
            .expect("d imports a");

        let a_pubkey = keys_a.public_key_hex();
        let b_pubkey = keys_b.public_key_hex();
        let d_pubkey = keys_d.public_key_hex();
        let topic = "kukuri:topic:friend-only";

        wait_for_connected_peer_count(&app_a, 2).await;
        wait_for_connected_peer_count(&app_b, 1).await;
        wait_for_connected_peer_count(&app_d, 1).await;

        for app in [&app_a, &app_b, &app_d] {
            let _ = app
                .list_timeline(topic, None, 20)
                .await
                .expect("subscribe public timeline");
        }
        wait_for_topic_peer_count(&app_a, topic, 2).await;
        wait_for_topic_peer_count(&app_b, topic, 1).await;
        wait_for_topic_peer_count(&app_d, topic, 1).await;
        warm_author_social_view(&app_a, b_pubkey.as_str(), topic).await;
        warm_author_social_view(&app_b, a_pubkey.as_str(), topic).await;
        warm_author_social_view(&app_a, d_pubkey.as_str(), topic).await;
        warm_author_social_view(&app_d, a_pubkey.as_str(), topic).await;

        app_a
            .follow_author(b_pubkey.as_str())
            .await
            .expect("a follows b");
        app_b
            .follow_author(a_pubkey.as_str())
            .await
            .expect("b follows a");
        app_a
            .follow_author(d_pubkey.as_str())
            .await
            .expect("a follows d");
        app_d
            .follow_author(a_pubkey.as_str())
            .await
            .expect("d follows a");
        wait_for_mutual_author_view(&app_a, b_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_b, a_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_a, d_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_d, a_pubkey.as_str(), topic).await;

        let channel = app_a
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(topic),
                label: "friends".into(),
                audience_kind: ChannelAudienceKind::FriendOnly,
            })
            .await
            .expect("create friend-only channel");
        let grant = app_a
            .export_friend_only_grant(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export friend-only grant");
        let preview = wait_for_friend_only_grant_import(
            &app_b,
            grant.as_str(),
            social_graph_propagation_timeout(),
        )
        .await;
        assert_eq!(preview.channel_id.as_str(), channel.channel_id);

        let non_mutual_error = app_c
            .import_friend_only_grant(grant.as_str())
            .await
            .expect_err("c should not join without mutual");
        assert!(non_mutual_error.to_string().contains("mutual relationship"));

        let private_channel_id = ChannelId::new(channel.channel_id.clone());
        let private_scope = TimelineScope::Channel {
            channel_id: private_channel_id.clone(),
        };
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: private_channel_id.clone(),
        };
        let object_id = app_a
            .create_post_in_channel(topic, private_ref, "friends hello", None)
            .await
            .expect("create friend-only post");

        timeout(Duration::from_secs(10), async {
            loop {
                let public = app_b
                    .list_timeline_scoped(topic, TimelineScope::Public, None, 20)
                    .await
                    .expect("public");
                assert!(public.items.iter().all(|post| post.object_id != object_id));
                let private = app_b
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private");
                if private.items.iter().any(|post| post.object_id == object_id) {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("friend-only post propagation timeout");

        app_a
            .unfollow_author(b_pubkey.as_str())
            .await
            .expect("a unfollows b");
        let joined_a = app_a
            .list_joined_private_channels(topic)
            .await
            .expect("list joined channels on a");
        let channel_a = joined_a
            .into_iter()
            .find(|entry| entry.channel_id == channel.channel_id)
            .expect("friend-only channel view");
        assert!(channel_a.rotation_required);
        assert_eq!(channel_a.stale_participant_count, 1);

        let rotated = app_a
            .rotate_private_channel(topic, channel.channel_id.as_str())
            .await
            .expect("rotate friend-only channel");
        assert_ne!(rotated.current_epoch_id, channel_a.current_epoch_id);
        assert_eq!(
            rotated.archived_epoch_ids,
            vec![channel_a.current_epoch_id.clone()]
        );

        app_d
            .import_friend_only_grant(grant.as_str())
            .await
            .expect_err("stale old grant should fail");

        let fresh_grant = app_a
            .export_friend_only_grant(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export fresh friend-only grant");
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("resubscribe a before fresh grant");
        let _ = app_d
            .list_timeline(topic, None, 20)
            .await
            .expect("resubscribe d before fresh grant");
        wait_for_topic_peer_count(&app_a, topic, 2).await;
        wait_for_topic_peer_count(&app_d, topic, 1).await;
        warm_author_social_view(&app_a, d_pubkey.as_str(), topic).await;
        warm_author_social_view(&app_d, a_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_a, d_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_d, a_pubkey.as_str(), topic).await;
        let fresh_preview = wait_for_friend_only_grant_import(
            &app_d,
            fresh_grant.as_str(),
            social_graph_propagation_timeout(),
        )
        .await;
        assert_eq!(fresh_preview.epoch_id, rotated.current_epoch_id);

        let d_private = app_d
            .list_timeline_scoped(topic, private_scope.clone(), None, 20)
            .await
            .expect("d private timeline after rotate");
        assert!(
            d_private
                .items
                .iter()
                .all(|post| post.object_id != object_id)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn friend_plus_share_freeze_rotate_and_new_epoch_visibility() {
        let _guard = iroh_integration_test_lock().lock_owned().await;
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            // CI covers the network path in the harness friend-plus connectivity scenario.
            return;
        }
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("friend-plus-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("friend-plus-b")).await;
        let stack_c = TestIrohStack::new(&dir.path().join("friend-plus-c")).await;
        let stack_d = TestIrohStack::new(&dir.path().join("friend-plus-d")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let store_c = Arc::new(MemoryStore::default());
        let store_d = Arc::new(MemoryStore::default());
        let keys_a = generate_keys();
        let keys_b = generate_keys();
        let keys_c = generate_keys();
        let keys_d = generate_keys();
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a.clone(),
            stack_a.transport.clone(),
            stack_a.transport.clone(),
            stack_a.docs_sync.clone(),
            stack_a.blob_service.clone(),
            keys_a.clone(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b.clone(),
            stack_b.transport.clone(),
            stack_b.transport.clone(),
            stack_b.docs_sync.clone(),
            stack_b.blob_service.clone(),
            keys_b.clone(),
        );
        let app_c = AppService::new_with_services(
            store_c.clone(),
            store_c.clone(),
            stack_c.transport.clone(),
            stack_c.transport.clone(),
            stack_c.docs_sync.clone(),
            stack_c.blob_service.clone(),
            keys_c.clone(),
        );
        let app_d = AppService::new_with_services(
            store_d.clone(),
            store_d.clone(),
            stack_d.transport.clone(),
            stack_d.transport.clone(),
            stack_d.docs_sync.clone(),
            stack_d.blob_service.clone(),
            keys_d.clone(),
        );
        app_a.warm_social_graph().await.expect("warm a");
        app_b.warm_social_graph().await.expect("warm b");
        app_c.warm_social_graph().await.expect("warm c");
        app_d.warm_social_graph().await.expect("warm d");

        let ticket_a = stack_a
            .transport
            .export_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = stack_b
            .transport
            .export_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        let ticket_c = stack_c
            .transport
            .export_ticket()
            .await
            .expect("ticket c")
            .expect("ticket c value");
        let ticket_d = stack_d
            .transport
            .export_ticket()
            .await
            .expect("ticket d")
            .expect("ticket d value");

        let a_pubkey = keys_a.public_key_hex();
        let b_pubkey = keys_b.public_key_hex();
        let c_pubkey = keys_c.public_key_hex();
        let d_pubkey = keys_d.public_key_hex();
        let topic = "kukuri:topic:friend-plus";
        let social_timeout = social_graph_propagation_timeout();
        let replication_timeout = p2p_replication_timeout();
        let rotation_timeout = Duration::from_secs(60);

        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("a imports b");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("b imports a");
        wait_for_connected_peer_count(&app_a, 1).await;
        wait_for_connected_peer_count(&app_b, 1).await;

        for app in [&app_a, &app_b] {
            let _ = app
                .list_timeline(topic, None, 20)
                .await
                .expect("subscribe public timeline");
        }
        wait_for_topic_peer_count(&app_a, topic, 1).await;
        wait_for_topic_peer_count(&app_b, topic, 1).await;

        app_a
            .follow_author(b_pubkey.as_str())
            .await
            .expect("a follows b");
        app_b
            .follow_author(a_pubkey.as_str())
            .await
            .expect("b follows a");
        wait_for_mutual_author_view(&app_a, b_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_b, a_pubkey.as_str(), topic).await;

        let channel = app_a
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(topic),
                label: "friends+".into(),
                audience_kind: ChannelAudienceKind::FriendPlus,
            })
            .await
            .expect("create friend-plus channel");
        let share_ab = app_a
            .export_friend_plus_share(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export a->b share");
        let preview_b =
            wait_for_friend_plus_share_import(&app_b, share_ab.as_str(), social_timeout).await;
        assert_eq!(preview_b.channel_id.as_str(), channel.channel_id);

        let private_channel_id = ChannelId::new(channel.channel_id.clone());
        let private_scope = TimelineScope::Channel {
            channel_id: private_channel_id.clone(),
        };
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: private_channel_id.clone(),
        };
        let _ = app_a
            .list_timeline_scoped(topic, private_scope.clone(), None, 20)
            .await
            .expect("warm private timeline a");
        let _ = app_b
            .list_timeline_scoped(topic, private_scope.clone(), None, 20)
            .await
            .expect("warm private timeline b");
        let old_post_id = app_a
            .create_post_in_channel(topic, private_ref.clone(), "friends+ old", None)
            .await
            .expect("create old friend-plus post");

        timeout(replication_timeout, async {
            loop {
                let private_b = app_b
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline b");
                if private_b
                    .items
                    .iter()
                    .any(|post| post.object_id == old_post_id)
                {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("friend-plus old post propagation to b timeout");

        app_a
            .import_peer_ticket(&ticket_c)
            .await
            .expect("a imports c");
        app_c
            .import_peer_ticket(&ticket_a)
            .await
            .expect("c imports a");
        app_b
            .import_peer_ticket(&ticket_c)
            .await
            .expect("b imports c");
        app_c
            .import_peer_ticket(&ticket_b)
            .await
            .expect("c imports b");
        wait_for_connected_peer_count(&app_a, 2).await;
        wait_for_connected_peer_count(&app_b, 2).await;
        wait_for_connected_peer_count(&app_c, 2).await;

        let _ = app_c
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe public timeline c");
        wait_for_topic_peer_count(&app_a, topic, 2).await;
        wait_for_topic_peer_count(&app_b, topic, 2).await;
        wait_for_topic_peer_count(&app_c, topic, 2).await;

        app_b
            .follow_author(c_pubkey.as_str())
            .await
            .expect("b follows c");
        app_c
            .follow_author(b_pubkey.as_str())
            .await
            .expect("c follows b");
        wait_for_mutual_author_view(&app_b, c_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_c, b_pubkey.as_str(), topic).await;

        let share_bc = app_b
            .export_friend_plus_share(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export b->c share");
        let preview_c =
            wait_for_friend_plus_share_import(&app_c, share_bc.as_str(), social_timeout).await;
        assert_eq!(preview_c.sponsor_pubkey.as_str(), b_pubkey);

        let public_c = app_c
            .list_timeline_scoped(topic, TimelineScope::Public, None, 20)
            .await
            .expect("public c");
        assert!(
            public_c
                .items
                .iter()
                .all(|post| post.object_id != old_post_id)
        );

        timeout(replication_timeout, async {
            loop {
                let private_c = app_c
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline c");
                if private_c
                    .items
                    .iter()
                    .any(|post| post.object_id == old_post_id)
                {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("friend-plus old post propagation to c timeout");

        let stale_share_for_d = app_b
            .export_friend_plus_share(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export b->d share");
        let stale_preview_d =
            kukuri_core::parse_friend_plus_share_token(stale_share_for_d.as_str())
                .expect("parse stale friend-plus share");

        let frozen = app_a
            .freeze_private_channel(topic, channel.channel_id.as_str())
            .await
            .expect("freeze friend-plus channel");
        assert_eq!(frozen.sharing_state, ChannelSharingState::Frozen);

        let freeze_post_id = app_b
            .create_post_in_channel(topic, private_ref.clone(), "friends+ frozen write", None)
            .await
            .expect("write should continue after freeze");

        timeout(replication_timeout, async {
            loop {
                let private_a = app_a
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline a");
                let private_c = app_c
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline c after freeze");
                if private_a
                    .items
                    .iter()
                    .any(|post| post.object_id == freeze_post_id)
                    && private_c
                        .items
                        .iter()
                        .any(|post| post.object_id == freeze_post_id)
                {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("friend-plus frozen write propagation timeout");

        app_b
            .import_peer_ticket(&ticket_d)
            .await
            .expect("b imports d");
        app_d
            .import_peer_ticket(&ticket_b)
            .await
            .expect("d imports b");
        wait_for_connected_peer_count(&app_b, 3).await;
        wait_for_connected_peer_count(&app_d, 1).await;
        let _ = app_d
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe public timeline d");
        wait_for_topic_peer_count(&app_b, topic, 3).await;
        wait_for_topic_peer_count(&app_d, topic, 1).await;
        warm_author_social_view(&app_b, d_pubkey.as_str(), topic).await;
        warm_author_social_view(&app_d, b_pubkey.as_str(), topic).await;

        app_b
            .follow_author(d_pubkey.as_str())
            .await
            .expect("b follows d");
        app_d
            .follow_author(b_pubkey.as_str())
            .await
            .expect("d follows b");
        wait_for_mutual_author_view(&app_b, d_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&app_d, b_pubkey.as_str(), topic).await;

        let freeze_error = app_d
            .import_friend_plus_share(stale_share_for_d.as_str())
            .await
            .expect_err("frozen share should fail");
        assert!(freeze_error.to_string().contains("no longer open"));

        let rotated = app_a
            .rotate_private_channel(topic, channel.channel_id.as_str())
            .await
            .expect("rotate friend-plus channel");
        let rotated_source_replica = private_channel_epoch_replica_id(
            channel.channel_id.as_str(),
            stale_preview_d.epoch_id.as_str(),
        );
        assert!(
            fetch_private_channel_rotation_grant_from_replica(
                app_a.docs_sync.as_ref(),
                &rotated_source_replica,
                b_pubkey.as_str(),
            )
            .await
            .expect("fetch published handoff grant")
            .is_some()
        );
        assert_ne!(rotated.current_epoch_id, stale_preview_d.epoch_id);
        assert!(!rotated.archived_epoch_ids.is_empty());
        assert!(
            rotated
                .archived_epoch_ids
                .iter()
                .any(|epoch_id| epoch_id == &stale_preview_d.epoch_id)
        );

        let joined_b = match timeout(rotation_timeout, async {
            loop {
                let joined = app_b
                    .list_joined_private_channels(topic)
                    .await
                    .expect("list joined on b");
                let Some(item) = joined
                    .iter()
                    .find(|entry| entry.channel_id == channel.channel_id)
                else {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                };
                if item.current_epoch_id == rotated.current_epoch_id {
                    break item.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        {
            Ok(item) => item,
            Err(_) => {
                let joined = app_b
                    .list_joined_private_channels(topic)
                    .await
                    .expect("list joined on b after timeout");
                let current = joined
                    .iter()
                    .find(|entry| entry.channel_id == channel.channel_id)
                    .cloned();
                let grant_visible = fetch_private_channel_rotation_grant_from_replica(
                    app_b.docs_sync.as_ref(),
                    &rotated_source_replica,
                    b_pubkey.as_str(),
                )
                .await
                .expect("fetch handoff grant on b")
                .is_some();
                let snapshot = app_b
                    .get_sync_status()
                    .await
                    .map(|status| format_sync_snapshot(&status, topic))
                    .unwrap_or_else(|_| "failed to read sync status".to_string());
                panic!(
                    "b rotation redeem timeout; current={current:?}, grant_visible={grant_visible}, {snapshot}"
                );
            }
        };
        assert_eq!(
            joined_b.joined_via_pubkey.as_deref(),
            Some(a_pubkey.as_str())
        );
        assert!(
            joined_b
                .archived_epoch_ids
                .iter()
                .any(|epoch_id| epoch_id == &preview_b.epoch_id)
        );

        let joined_c = timeout(rotation_timeout, async {
            loop {
                let joined = app_c
                    .list_joined_private_channels(topic)
                    .await
                    .expect("list joined on c");
                let Some(item) = joined
                    .iter()
                    .find(|entry| entry.channel_id == channel.channel_id)
                else {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                };
                if item.current_epoch_id == rotated.current_epoch_id {
                    break item.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("c rotation redeem timeout");
        assert_eq!(
            joined_c.joined_via_pubkey.as_deref(),
            Some(b_pubkey.as_str())
        );
        assert!(
            joined_c
                .archived_epoch_ids
                .iter()
                .any(|epoch_id| epoch_id == &preview_c.epoch_id)
        );

        let old_share_error = app_d
            .import_friend_plus_share(stale_share_for_d.as_str())
            .await
            .expect_err("old share should still fail after rotate");
        assert!(old_share_error.to_string().contains("no longer open"));

        let new_post_id = app_b
            .create_post_in_channel(topic, private_ref.clone(), "friends+ new", None)
            .await
            .expect("create new epoch post");

        timeout(replication_timeout, async {
            loop {
                let private_a = app_a
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline a after rotate");
                let private_c = app_c
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline c after rotate");
                if private_a
                    .items
                    .iter()
                    .any(|post| post.object_id == new_post_id)
                    && private_c
                        .items
                        .iter()
                        .any(|post| post.object_id == new_post_id)
                {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("friend-plus new epoch propagation timeout");

        let fresh_share = app_b
            .export_friend_plus_share(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export fresh friend-plus share");
        let preview_d =
            wait_for_friend_plus_share_import(&app_d, fresh_share.as_str(), rotation_timeout).await;
        assert_eq!(preview_d.epoch_id, rotated.current_epoch_id);

        let d_private = app_d
            .list_timeline_scoped(topic, private_scope.clone(), None, 20)
            .await
            .expect("d private timeline");
        assert!(
            d_private
                .items
                .iter()
                .all(|post| post.object_id != old_post_id)
        );
        assert!(
            d_private
                .items
                .iter()
                .any(|post| post.object_id == new_post_id)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn social_graph_derives_friend_of_friend_and_clears_after_unfollow() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("author-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("author-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let keys_a = generate_keys();
        let keys_b = generate_keys();
        let keys_c = generate_keys();
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a.clone(),
            stack_a.transport.clone(),
            stack_a.transport.clone(),
            stack_a.docs_sync.clone(),
            stack_a.blob_service.clone(),
            keys_a.clone(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b.clone(),
            stack_b.transport.clone(),
            stack_b.transport.clone(),
            stack_b.docs_sync.clone(),
            stack_b.blob_service.clone(),
            keys_b.clone(),
        );
        app_a
            .warm_social_graph()
            .await
            .expect("warm social graph a");
        app_b
            .warm_social_graph()
            .await
            .expect("warm social graph b");

        let ticket_a = stack_a
            .transport
            .export_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = stack_b
            .transport
            .export_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let b_pubkey = keys_b.public_key_hex();
        let c_pubkey = keys_c.public_key_hex();
        app_a
            .follow_author(b_pubkey.as_str())
            .await
            .expect("a follows b");
        app_b
            .follow_author(c_pubkey.as_str())
            .await
            .expect("b follows c");

        timeout(Duration::from_secs(10), async {
            loop {
                let social_view = app_a
                    .get_author_social_view(c_pubkey.as_str())
                    .await
                    .expect("load c social view");
                if social_view.friend_of_friend {
                    assert_eq!(
                        social_view.friend_of_friend_via_pubkeys,
                        vec![b_pubkey.clone()]
                    );
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("derive friend of friend");

        let b_view = app_a
            .get_author_social_view(b_pubkey.as_str())
            .await
            .expect("load b social view");
        assert!(b_view.following);
        assert!(!b_view.friend_of_friend);

        app_a
            .unfollow_author(b_pubkey.as_str())
            .await
            .expect("a unfollows b");

        let c_view = app_a
            .get_author_social_view(c_pubkey.as_str())
            .await
            .expect("load c social view after unfollow");
        assert!(!c_view.friend_of_friend);
        assert!(c_view.friend_of_friend_via_pubkeys.is_empty());
    }
}
