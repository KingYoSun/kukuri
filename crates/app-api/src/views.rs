use kukuri_core::{
    AssetRole, ChannelAudienceKind, ChannelSharingState, GameRoomKind, GameRoomStatus,
    KukuriEnvelope, LiveSessionStatus, MetaverseAssetKind, MetaverseAssetRef,
    MetaverseRoomEventEnvelopeContentV1, MetaverseRoomEventV1, MetaverseRoomStateV1,
};
use kukuri_store::{NotificationKind, TimelineCursor};
use kukuri_transport::{ConnectMode, ConnectionPath, DiscoveryMode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostView {
    pub object_id: String,
    pub envelope_id: String,
    pub author_pubkey: String,
    pub author_name: Option<String>,
    pub author_display_name: Option<String>,
    pub author_picture: Option<String>,
    pub author_picture_asset: Option<ProfileAssetView>,
    pub following: bool,
    pub followed_by: bool,
    pub mutual: bool,
    pub friend_of_friend: bool,
    pub content: String,
    pub content_status: BlobViewStatus,
    pub attachments: Vec<AttachmentView>,
    pub created_at: i64,
    pub reply_to: Option<String>,
    pub reply_preview: Option<ReplyPreviewView>,
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
pub struct ReplyPreviewAuthorView {
    pub pubkey: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub picture: Option<String>,
    pub picture_asset: Option<ProfileAssetView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyPreviewView {
    pub object_id: String,
    pub topic: String,
    pub author: ReplyPreviewAuthorView,
    pub content: String,
    pub attachments: Vec<AttachmentView>,
    pub root_id: Option<String>,
    pub reply_to: Option<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkedPostView {
    pub bookmarked_at: i64,
    pub post: PostView,
}

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
    #[serde(default)]
    pub muted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialConnectionKind {
    Following,
    Followed,
    Muted,
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
pub struct DirectMessageTopicStatusView {
    pub topic: String,
    pub joined: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub status_detail: String,
    pub last_error: Option<String>,
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
pub struct NotificationView {
    pub notification_id: String,
    pub kind: NotificationKind,
    pub actor_pubkey: String,
    pub actor_name: Option<String>,
    pub actor_display_name: Option<String>,
    pub actor_picture: Option<String>,
    pub actor_picture_asset: Option<ProfileAssetView>,
    pub source_envelope_id: Option<String>,
    pub source_replica_id: Option<String>,
    pub topic_id: Option<String>,
    pub channel_id: Option<String>,
    pub object_id: Option<String>,
    pub thread_root_object_id: Option<String>,
    pub dm_id: Option<String>,
    pub message_id: Option<String>,
    pub preview_text: Option<String>,
    pub created_at: i64,
    pub received_at: i64,
    pub read_at: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationStatusView {
    pub unread_count: usize,
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
    pub room_kind: GameRoomKind,
    pub metaverse: Option<MetaverseRoomStateV1>,
    pub manifest_blob_hash: String,
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
pub struct CreateMetaverseRoomInput {
    pub title: String,
    pub description: String,
    pub max_peers: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateGameRoomInput {
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateMetaverseRoomInput {
    pub status: GameRoomStatus,
    pub shared_object_position: [i64; 3],
    pub shared_object_rotation: [i64; 3],
    pub shared_object_scale: [i64; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishMetaverseRoomEventInput {
    pub room_id: String,
    pub peer_id: String,
    pub seq: u64,
    pub event: MetaverseRoomEventV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseRoomEventView {
    pub envelope_id: String,
    pub content: MetaverseRoomEventEnvelopeContentV1,
    pub envelope: KukuriEnvelope,
    pub received_at: i64,
    pub source_peer: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportMetaverseRoomAssetInput {
    pub room_id: String,
    pub kind: MetaverseAssetKind,
    pub mime_type: String,
    pub name: Option<String>,
    pub bytes: Vec<u8>,
}

pub type MetaverseAssetRefView = MetaverseAssetRef;

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
    pub delivery_state: DeliveryState,
    pub last_sync_ts: Option<i64>,
    pub peer_count: usize,
    pub pending_events: usize,
    pub status_detail: String,
    pub last_error: Option<String>,
    pub configured_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub active_path: ConnectionPath,
    pub fallback_peer_ids: Vec<String>,
    pub topic_diagnostics: Vec<TopicSyncStatus>,
    pub local_author_pubkey: String,
    pub discovery: DiscoveryStatus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryState {
    Live,
    DurableRecovering,
    DurableReady,
    #[default]
    Offline,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryStatus {
    pub mode: DiscoveryMode,
    pub connect_mode: ConnectMode,
    pub active_path: ConnectionPath,
    pub fallback_peer_ids: Vec<String>,
    pub env_locked: bool,
    pub configured_seed_peer_ids: Vec<String>,
    pub bootstrap_seed_peer_ids: Vec<String>,
    pub manual_ticket_peer_ids: Vec<String>,
    pub connected_peer_ids: Vec<String>,
    pub docs_assist_peer_ids: Vec<String>,
    pub blob_assist_peer_ids: Vec<String>,
    pub local_endpoint_id: String,
    pub last_discovery_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSyncStatus {
    pub topic: String,
    pub joined: bool,
    pub delivery_state: DeliveryState,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub docs_assist_peer_ids: Vec<String>,
    pub configured_peer_ids: Vec<String>,
    pub missing_peer_ids: Vec<String>,
    pub active_path: ConnectionPath,
    pub rendezvous_peer_ids: Vec<String>,
    pub fallback_peer_ids: Vec<String>,
    pub last_received_at: Option<i64>,
    pub last_docs_activity_at: Option<i64>,
    pub status_detail: String,
    pub last_error: Option<String>,
}
