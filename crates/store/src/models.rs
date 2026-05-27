use kukuri_core::{
    AssetRef, BlobHash, CustomReactionAssetSnapshotV1, DirectMessageAttachmentManifestV1,
    EnvelopeId, GameRoomKind, GameRoomStatus, GameScoreEntry, LiveSessionStatus,
    MetaverseRoomStateV1, ObjectStatus, PayloadRef, ReactionKeyKind, ReplicaId,
    RepostSourceSnapshotV1,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineCursor {
    pub created_at: i64,
    pub object_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<TimelineCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobCacheStatus {
    Missing,
    Available,
    Pinned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectProjectionRow {
    pub object_id: EnvelopeId,
    pub topic_id: String,
    pub channel_id: String,
    pub author_pubkey: String,
    pub created_at: i64,
    pub object_kind: String,
    pub root_object_id: Option<EnvelopeId>,
    pub reply_to_object_id: Option<EnvelopeId>,
    pub payload_ref: PayloadRef,
    pub content: Option<String>,
    pub attachments: Vec<AssetRef>,
    pub repost_of: Option<RepostSourceSnapshotV1>,
    pub source_replica_id: ReplicaId,
    pub source_key: String,
    pub source_envelope_id: EnvelopeId,
    pub source_blob_hash: Option<BlobHash>,
    pub derived_at: i64,
    pub projection_version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionProjectionRow {
    pub source_replica_id: ReplicaId,
    pub target_object_id: EnvelopeId,
    pub reaction_id: EnvelopeId,
    pub author_pubkey: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub reaction_key_kind: ReactionKeyKind,
    pub normalized_reaction_key: String,
    pub emoji: Option<String>,
    pub custom_asset_id: Option<String>,
    pub custom_asset_snapshot: Option<CustomReactionAssetSnapshotV1>,
    pub status: ObjectStatus,
    pub source_key: String,
    pub source_envelope_id: EnvelopeId,
    pub derived_at: i64,
    pub projection_version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkedCustomReactionRow {
    pub asset_id: String,
    pub owner_pubkey: String,
    pub blob_hash: BlobHash,
    pub search_key: String,
    pub mime: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    pub bookmarked_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkedPostRow {
    pub source_object_id: EnvelopeId,
    pub source_envelope_id: EnvelopeId,
    pub source_replica_id: ReplicaId,
    pub topic_id: String,
    pub channel_id: String,
    pub author_pubkey: String,
    pub created_at: i64,
    pub object_kind: String,
    pub payload_ref: PayloadRef,
    pub content: Option<String>,
    pub attachments: Vec<AssetRef>,
    pub reply_to_object_id: Option<EnvelopeId>,
    pub root_object_id: Option<EnvelopeId>,
    pub repost_of: Option<RepostSourceSnapshotV1>,
    pub bookmarked_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionProjectionRow {
    pub session_id: String,
    pub topic_id: String,
    pub channel_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: LiveSessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub updated_at: i64,
    pub source_replica_id: ReplicaId,
    pub source_key: String,
    pub manifest_blob_hash: BlobHash,
    pub derived_at: i64,
    pub projection_version: i64,
    pub viewer_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomProjectionRow {
    pub room_id: String,
    pub topic_id: String,
    pub channel_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreEntry>,
    pub room_kind: GameRoomKind,
    pub metaverse: Option<MetaverseRoomStateV1>,
    pub updated_at: i64,
    pub source_replica_id: ReplicaId,
    pub source_key: String,
    pub manifest_blob_hash: BlobHash,
    pub derived_at: i64,
    pub projection_version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorRelationshipProjectionRow {
    pub local_author_pubkey: String,
    pub author_pubkey: String,
    pub following: bool,
    pub followed_by: bool,
    pub mutual: bool,
    pub friend_of_friend: bool,
    pub friend_of_friend_via_pubkeys: Vec<String>,
    pub derived_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutedAuthorRow {
    pub author_pubkey: String,
    pub muted_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageConversationRow {
    pub dm_id: String,
    pub peer_pubkey: String,
    pub updated_at: i64,
    pub last_message_at: Option<i64>,
    pub last_message_id: Option<String>,
    pub last_message_preview: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageMessageRow {
    pub dm_id: String,
    pub message_id: String,
    pub sender_pubkey: String,
    pub recipient_pubkey: String,
    pub created_at: i64,
    pub text: Option<String>,
    pub reply_to_message_id: Option<String>,
    pub attachment_manifest: Option<DirectMessageAttachmentManifestV1>,
    pub outgoing: bool,
    pub acked_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageOutboxRow {
    pub dm_id: String,
    pub message_id: String,
    pub peer_pubkey: String,
    pub frame_blob_hash: BlobHash,
    pub created_at: i64,
    pub last_attempt_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageTombstoneRow {
    pub dm_id: String,
    pub message_id: String,
    pub deleted_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    Mention,
    Reply,
    Repost,
    QuoteRepost,
    DirectMessage,
    Followed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRow {
    pub notification_id: String,
    pub recipient_pubkey: String,
    pub kind: NotificationKind,
    pub actor_pubkey: String,
    pub source_envelope_id: Option<EnvelopeId>,
    pub source_replica_id: Option<ReplicaId>,
    pub topic_id: Option<String>,
    pub channel_id: Option<String>,
    pub object_id: Option<EnvelopeId>,
    pub dm_id: Option<String>,
    pub message_id: Option<String>,
    pub preview_text: Option<String>,
    pub created_at: i64,
    pub received_at: i64,
    pub read_at: Option<i64>,
}
