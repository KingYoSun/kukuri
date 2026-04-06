use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{ChannelId, EnvelopeId, ManifestBlobRef, Pubkey, TopicId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveSignalKind {
    SessionStarted,
    SessionEnded,
    RoomActivity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiveSessionStatus {
    Scheduled,
    Live,
    Paused,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionManifestBlobV1 {
    pub session_id: String,
    pub topic_id: TopicId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    pub owner_pubkey: Pubkey,
    pub title: String,
    pub description: String,
    pub status: LiveSessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionStateDocV1 {
    pub session_id: String,
    pub topic_id: TopicId,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    pub owner_pubkey: Pubkey,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: LiveSessionStatus,
    pub current_manifest: ManifestBlobRef,
    pub last_envelope_id: EnvelopeId,
}

pub fn build_live_session_envelope<T: Serialize>(
    keys: &crate::KukuriKeys,
    topic: &TopicId,
    session_id: &str,
    content: &T,
) -> Result<crate::KukuriEnvelope> {
    crate::sign_envelope_json(
        keys,
        "live-session",
        vec![
            vec!["topic".into(), topic.as_str().into()],
            vec!["object".into(), "live-session".into()],
            vec!["session_id".into(), session_id.to_string()],
        ],
        content,
    )
}
