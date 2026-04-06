use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{ChannelId, EnvelopeId, ManifestBlobRef, Pubkey, TopicId};

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
