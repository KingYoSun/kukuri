use super::*;

const METAVERSE_ROOM_EVENT_BUFFER_LIMIT: usize = 512;

pub(crate) async fn push_metaverse_room_event_buffer(
    buffers: &Arc<Mutex<HashMap<String, VecDeque<MetaverseRoomEventView>>>>,
    view: MetaverseRoomEventView,
) {
    let key = metaverse_room_event_buffer_key(
        view.content.topic_id.as_str(),
        view.content.room_id.as_str(),
    );
    let mut guard = buffers.lock().await;
    let queue = guard.entry(key).or_default();
    if queue
        .iter()
        .any(|existing| existing.envelope_id == view.envelope_id)
    {
        return;
    }
    if queue.iter().any(|existing| {
        existing.envelope.pubkey == view.envelope.pubkey
            && existing.content.peer_id == view.content.peer_id
            && existing.content.session_id == view.content.session_id
            && existing.content.seq >= view.content.seq
    }) {
        return;
    }
    let now = Utc::now().timestamp_millis();
    queue.retain(|existing| metaverse_room_event_is_live(&existing.content, now));
    queue.push_back(view);
    while queue.len() > METAVERSE_ROOM_EVENT_BUFFER_LIMIT {
        queue.pop_front();
    }
}

pub(crate) fn metaverse_room_event_buffer_key(topic_id: &str, room_id: &str) -> String {
    format!("{topic_id}::{room_id}")
}

pub(crate) fn parse_metaverse_room_event_envelope(
    envelope: KukuriEnvelope,
    received_at: i64,
    source_peer: String,
) -> Result<Option<MetaverseRoomEventView>> {
    if envelope.kind != "metaverse-room-event" {
        return Ok(None);
    }
    envelope.verify()?;
    let content: MetaverseRoomEventEnvelopeContentV1 =
        serde_json::from_str(envelope.content.as_str())
            .context("failed to decode metaverse room event content")?;
    validate_metaverse_room_event_content(&content)?;
    Ok(Some(MetaverseRoomEventView {
        envelope_id: envelope.id.0.clone(),
        content,
        envelope,
        received_at,
        source_peer,
    }))
}

impl AppService {
    pub(crate) async fn preflight_spatial_audio_event(
        &self,
        content: &MetaverseRoomEventEnvelopeContentV1,
        now: i64,
    ) -> Result<()> {
        let MetaverseRoomEventV1::SpatialAudioFrame { frame } = &content.event else {
            return Ok(());
        };
        let key =
            metaverse_room_event_buffer_key(content.topic_id.as_str(), content.room_id.as_str());
        let guard = self.metaverse_room_events.lock().await;
        let author = self.current_author_pubkey();
        let recent = guard
            .get(key.as_str())
            .into_iter()
            .flatten()
            .filter(|event| {
                event.envelope.pubkey.as_str() == author.as_str()
                    && event.content.sent_at >= now.saturating_sub(1_000)
                    && matches!(
                        event.content.event,
                        MetaverseRoomEventV1::SpatialAudioFrame { .. }
                    )
            })
            .collect::<Vec<_>>();
        let next_frames = recent.len() as u64 + 1;
        let frame_limit = u64::from(
            self.metaverse_resource_budget
                .player
                .max_audio_frames_per_second,
        );
        if next_frames > frame_limit {
            return Err(kukuri_core::MetaverseResourceRejection::new(
                kukuri_core::MetaverseBudgetScope::Player,
                kukuri_core::MetaverseBudgetResource::AudioFrameRate,
                kukuri_core::MetaverseResourceRejectionReason::RateExceeded,
                next_frames,
                frame_limit,
            )
            .into());
        }
        let next_bytes = recent
            .iter()
            .filter_map(|event| match &event.content.event {
                MetaverseRoomEventV1::SpatialAudioFrame { frame } => {
                    Some(frame.samples.len() as u64 * 2)
                }
                _ => None,
            })
            .sum::<u64>()
            .saturating_add(frame.samples.len() as u64 * 2);
        let byte_limit = self
            .metaverse_resource_budget
            .player
            .max_audio_bytes_per_second;
        if next_bytes > byte_limit {
            return Err(kukuri_core::MetaverseResourceRejection::new(
                kukuri_core::MetaverseBudgetScope::Player,
                kukuri_core::MetaverseBudgetResource::AudioBandwidth,
                kukuri_core::MetaverseResourceRejectionReason::RateExceeded,
                next_bytes,
                byte_limit,
            )
            .into());
        }
        Ok(())
    }
}
