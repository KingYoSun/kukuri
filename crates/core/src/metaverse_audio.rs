use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    DomeInstanceManifestV1, DomeInstanceStatusV1, MetaverseRoomEventEnvelopeContentV1,
    MetaverseRoomEventV1, SpatialContextV1, validate_dome_instance_manifest,
};

pub const METAVERSE_MEDIA_TTL_MILLIS: i64 = 10_000;
pub const METAVERSE_AUDIO_SAMPLE_RATE_HZ: u32 = 16_000;
pub const METAVERSE_AUDIO_MAX_SAMPLES_PER_FRAME: usize = 320;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MetaverseSpatialAudioFrameV1 {
    pub room_id: String,
    pub peer_id: String,
    pub position: [i64; 3],
    pub sample_rate_hz: u32,
    pub samples: Vec<i16>,
    pub captured_at: i64,
}

pub fn validate_metaverse_room_event_content(
    content: &MetaverseRoomEventEnvelopeContentV1,
) -> Result<()> {
    if content.event_id.trim().is_empty()
        || content.room_id.trim().is_empty()
        || content.session_id.trim().is_empty()
        || content.peer_id.trim().is_empty()
        || content.instance_generation == 0
    {
        bail!("metaverse room event identity is incomplete");
    }
    if content.session_id != content.room_id {
        bail!("metaverse room event session does not match its Dome instance");
    }
    let expected_context = match &content.channel_id {
        Some(channel_id) => SpatialContextV1::Channel {
            topic_id: content.topic_id.clone(),
            channel_id: channel_id.clone(),
        },
        None => SpatialContextV1::Topic {
            topic_id: content.topic_id.clone(),
        },
    };
    if content.spatial_context != expected_context {
        bail!("metaverse room event Spatial Context does not match topic/channel identity");
    }
    match &content.event {
        MetaverseRoomEventV1::PresenceJoin { presence } => {
            if presence.room_id != content.room_id
                || presence.peer_id != content.peer_id
                || presence.last_seen_at < presence.joined_at
                || presence.last_seen_at
                    > content.sent_at.saturating_add(METAVERSE_MEDIA_TTL_MILLIS)
            {
                bail!("metaverse presence identity or timestamp is invalid");
            }
        }
        MetaverseRoomEventV1::PresenceLeave {
            room_id,
            peer_id,
            left_at,
        } => {
            if room_id != &content.room_id
                || peer_id != &content.peer_id
                || *left_at > content.sent_at.saturating_add(METAVERSE_MEDIA_TTL_MILLIS)
            {
                bail!("metaverse presence leave identity or timestamp is invalid");
            }
        }
        MetaverseRoomEventV1::ChatMessage { message } => {
            if message.room_id != content.room_id || message.author_peer_id != content.peer_id {
                bail!("metaverse chat identity is invalid");
            }
        }
        MetaverseRoomEventV1::SpatialAudioFrame { frame } => {
            if frame.room_id != content.room_id
                || frame.peer_id != content.peer_id
                || frame.sample_rate_hz != METAVERSE_AUDIO_SAMPLE_RATE_HZ
                || frame.samples.is_empty()
                || frame.samples.len() > METAVERSE_AUDIO_MAX_SAMPLES_PER_FRAME
                || frame.captured_at > content.sent_at.saturating_add(METAVERSE_MEDIA_TTL_MILLIS)
            {
                bail!("metaverse spatial audio frame is invalid");
            }
        }
    }
    Ok(())
}

pub fn metaverse_room_event_is_live(
    content: &MetaverseRoomEventEnvelopeContentV1,
    now_millis: i64,
) -> bool {
    match content.event {
        MetaverseRoomEventV1::PresenceJoin { .. }
        | MetaverseRoomEventV1::PresenceLeave { .. }
        | MetaverseRoomEventV1::SpatialAudioFrame { .. } => {
            content.sent_at <= now_millis.saturating_add(METAVERSE_MEDIA_TTL_MILLIS)
                && now_millis <= content.sent_at.saturating_add(METAVERSE_MEDIA_TTL_MILLIS)
        }
        MetaverseRoomEventV1::ChatMessage { .. } => true,
    }
}

pub fn validate_metaverse_room_event_for_instance(
    content: &MetaverseRoomEventEnvelopeContentV1,
    instance: &DomeInstanceManifestV1,
) -> Result<()> {
    validate_metaverse_room_event_content(content)?;
    validate_dome_instance_manifest(instance)?;
    if instance.status != DomeInstanceStatusV1::Active || instance.relationship_detach.is_some() {
        bail!("metaverse room events require an active attached Dome instance");
    }
    if content.room_id != instance.instance_id
        || content.session_id != instance.instance_id
        || content.spatial_context != instance.spatial_context
        || content.instance_generation != instance.generation
    {
        bail!("metaverse room event does not match the current Dome instance generation");
    }
    Ok(())
}

pub fn spatial_audio_gain_milli(distance_cm: u64) -> u16 {
    const REFERENCE_DISTANCE_CM: u64 = 100;
    if distance_cm <= REFERENCE_DISTANCE_CM {
        return 1_000;
    }
    ((REFERENCE_DISTANCE_CM.saturating_mul(1_000) / distance_cm).min(1_000)) as u16
}

pub fn connection_opening_audio_distance_cm(
    speaker: [i64; 3],
    speaker_opening: [i64; 3],
    listener_opening: [i64; 3],
    listener: [i64; 3],
) -> u64 {
    fn distance(left: [i64; 3], right: [i64; 3]) -> u64 {
        let squared = left
            .into_iter()
            .zip(right)
            .fold(0_f64, |sum, (left, right)| {
                let delta = (left - right) as f64;
                sum + delta * delta
            });
        squared.sqrt().round() as u64
    }
    distance(speaker, speaker_opening).saturating_add(distance(listener_opening, listener))
}
