use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{ChannelId, EnvelopeId, ManifestBlobRef, Pubkey, TopicId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameRoomKind {
    #[default]
    ScoreGame,
    MetaverseRoom,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameRoomStatus {
    Waiting,
    Running,
    Paused,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameParticipant {
    pub participant_id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameScoreEntry {
    pub participant_id: String,
    pub label: String,
    pub score: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetaverseAssetKind {
    Vrm,
    Glb,
    Texture,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseAssetRef {
    pub kind: MetaverseAssetKind,
    pub blob_hash: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseRoomPresenceV1 {
    pub room_id: String,
    pub peer_id: String,
    pub display_name: Option<String>,
    pub avatar_asset_ref: Option<MetaverseAssetRef>,
    pub joined_at: i64,
    pub last_seen_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseAvatarTransformV1 {
    pub room_id: String,
    pub peer_id: String,
    pub seq: u64,
    pub position: [i64; 3],
    pub rotation: [i64; 3],
    pub animation: Option<String>,
    pub sent_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseRoomChatMessageV1 {
    pub room_id: String,
    pub message_id: String,
    pub author_peer_id: String,
    pub display_name: Option<String>,
    pub body: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetaversePrimitive {
    Cube,
    Sphere,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRoomObjectV1 {
    pub object_id: String,
    pub asset_ref: Option<MetaverseAssetRef>,
    pub primitive_fallback: MetaversePrimitive,
    pub position: [i64; 3],
    pub rotation: [i64; 3],
    pub scale: [i64; 3],
    pub updated_by: Pubkey,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetaverseRoomEventV1 {
    PresenceJoin {
        presence: MetaverseRoomPresenceV1,
    },
    PresenceLeave {
        room_id: String,
        peer_id: String,
        left_at: i64,
    },
    AvatarTransform {
        transform: MetaverseAvatarTransformV1,
    },
    ChatMessage {
        message: MetaverseRoomChatMessageV1,
    },
    ObjectUpdate {
        object: SharedRoomObjectV1,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseRoomEventEnvelopeContentV1 {
    pub event_id: String,
    pub topic_id: TopicId,
    pub channel_id: Option<ChannelId>,
    pub room_id: String,
    pub peer_id: String,
    pub seq: u64,
    pub sent_at: i64,
    pub event: MetaverseRoomEventV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseRoomSceneV1 {
    pub ground: String,
    pub shared_object: SharedRoomObjectV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseRoomSpawnV1 {
    pub position: [i64; 3],
    pub rotation: [i64; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaverseRoomStateV1 {
    pub world_version: u64,
    pub max_peers: Option<u32>,
    pub scene: MetaverseRoomSceneV1,
    pub default_spawn: MetaverseRoomSpawnV1,
    pub asset_refs: Vec<MetaverseAssetRef>,
    #[serde(default)]
    pub chat_history: Vec<MetaverseRoomChatMessageV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomManifestBlobV1 {
    pub room_id: String,
    pub topic_id: TopicId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    pub owner_pubkey: Pubkey,
    pub title: String,
    pub description: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub participants: Vec<GameParticipant>,
    pub scores: Vec<GameScoreEntry>,
    #[serde(default)]
    pub room_kind: GameRoomKind,
    #[serde(default)]
    pub metaverse: Option<MetaverseRoomStateV1>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomStateDocV1 {
    pub room_id: String,
    pub topic_id: TopicId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    pub owner_pubkey: Pubkey,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: GameRoomStatus,
    pub current_manifest: ManifestBlobRef,
    pub last_envelope_id: EnvelopeId,
}

pub fn build_game_session_envelope<T: Serialize>(
    keys: &crate::KukuriKeys,
    topic: &TopicId,
    room_id: &str,
    content: &T,
) -> Result<crate::KukuriEnvelope> {
    crate::sign_envelope_json(
        keys,
        "game-session",
        vec![
            vec!["topic".into(), topic.as_str().into()],
            vec!["object".into(), "game-session".into()],
            vec!["room_id".into(), room_id.to_string()],
        ],
        content,
    )
}

pub fn build_metaverse_room_event_envelope(
    keys: &crate::KukuriKeys,
    topic: &TopicId,
    room_id: &str,
    content: &MetaverseRoomEventEnvelopeContentV1,
) -> Result<crate::KukuriEnvelope> {
    crate::sign_envelope_json(
        keys,
        "metaverse-room-event",
        vec![
            vec!["topic".into(), topic.as_str().into()],
            vec!["object".into(), "metaverse-room-event".into()],
            vec!["room_id".into(), room_id.to_string()],
            vec!["event_id".into(), content.event_id.clone()],
        ],
        content,
    )
}
