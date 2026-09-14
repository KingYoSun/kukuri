//! Management is based on the signed Instance, independently of scene assets.
use crate::DomeHostingView;
use crate::service::*;
use kukuri_core::{DomeInstanceStatusV1, SpatialContextV1};

impl AppService {
    pub(crate) async fn get_dome_hosting_authority(
        &self,
        context: SpatialContextV1,
        id: &str,
    ) -> Result<DomeHostingView> {
        let replica = self.hosting_context_replica(&context).await?;
        let instance = self
            .hosting_instance(&replica, id)
            .await?
            .context("Dome instance not found")?;
        let records = self.list_dome_hosting_records(&replica, id).await?;
        self.hosting_authority_view(&instance, &records, Utc::now().timestamp_millis())
            .await
    }

    pub(crate) async fn append_owned_dome_management(
        &self,
        topic: &str,
        channels: &BTreeSet<String>,
        items: &mut Vec<GameRoomView>,
    ) -> Result<()> {
        let owner = self.services.keys.public_key();
        for channel in channels {
            if items.iter().any(|room| {
                room.room_kind == GameRoomKind::MetaverseRoom
                    && room.host_pubkey == owner.as_str()
                    && room.channel_id.as_deref().unwrap_or(PUBLIC_CHANNEL_ID) == channel
            }) {
                continue;
            }
            let context = if channel == PUBLIC_CHANNEL_ID {
                SpatialContextV1::Topic {
                    topic_id: TopicId::new(topic),
                }
            } else {
                SpatialContextV1::Channel {
                    topic_id: TopicId::new(topic),
                    channel_id: kukuri_core::ChannelId::new(channel),
                }
            };
            let replica = self.hosting_context_replica(&context).await?;
            let resolved = match self.fetch_dome_instance_manifest(&replica, &owner).await {
                Ok(value) => value,
                Err(error) if error.downcast_ref::<DomeReadUnavailable>().is_some() => continue,
                Err(error) => return Err(error),
            };
            let Some((state, instance)) = resolved else {
                continue;
            };
            if instance.status != DomeInstanceStatusV1::Active
                || instance.relationship_detach.is_some()
            {
                continue;
            }
            let preset = self
                .fetch_dome_preset_manifest(&instance.preset_ref)
                .await?;
            let mut manifest = instance_management_manifest(&instance);
            let hosting = self
                .get_dome_hosting_authority(context, &instance.instance_id)
                .await?
                .state;
            if let Some(preset) = preset {
                manifest.metaverse = Some(kukuri_core::resolve_metaverse_room_state(
                    &instance, &preset,
                )?);
                manifest.phase_label = None;
            }
            items.push(GameRoomView {
                room_id: instance.instance_id,
                host_pubkey: owner.as_str().into(),
                title: instance.title,
                description: instance.description,
                status: GameRoomStatus::Waiting,
                phase_label: manifest.phase_label,
                scores: vec![],
                room_kind: GameRoomKind::MetaverseRoom,
                metaverse: manifest.metaverse,
                dome_hosting: Some(hosting),
                manifest_blob_hash: state.current_manifest.hash.as_str().into(),
                updated_at: instance.updated_at,
                channel_id: channel_id_for_view(channel),
                audience_label: self.audience_label_for_storage(topic, channel).await,
            });
        }
        Ok(())
    }
}

/// Metadata-only projection. It is never a scene or a new active Preset. Delete
/// uses it only to publish an inactive tombstone; discovery marks it unhosted.
pub(crate) fn instance_management_manifest(
    instance: &DomeInstanceManifestV1,
) -> GameRoomManifestBlobV1 {
    GameRoomManifestBlobV1 {
        room_id: instance.instance_id.clone(),
        topic_id: instance.spatial_context.topic_id().clone(),
        channel_id: instance.spatial_context.channel_id().cloned(),
        owner_pubkey: instance.owner_pubkey.clone(),
        title: instance.title.clone(),
        description: instance.description.clone(),
        status: GameRoomStatus::Waiting,
        phase_label: Some("management_only".into()),
        participants: vec![],
        scores: vec![],
        room_kind: GameRoomKind::MetaverseRoom,
        updated_at: instance.updated_at,
        metaverse: Some(kukuri_core::MetaverseRoomStateV1 {
            world_version: kukuri_core::METAVERSE_WORLD_VERSION,
            instance_id: instance.instance_id.clone(),
            spatial_context: instance.spatial_context.clone(),
            instance_generation: instance.generation,
            instance_status: instance.status,
            relationship_detach: instance.relationship_detach.clone(),
            replacement_instance_id: instance.replacement_instance_id.clone(),
            preset_ref: instance.preset_ref.clone(),
            session_id: instance.instance_id.clone(),
            max_peers: instance.max_peers,
            dome: kukuri_core::MetaverseDomeV1::default(),
            default_spawn: instance.default_spawn.clone(),
            asset_refs: vec![],
            chat_history: instance.chat_history.clone(),
        }),
    }
}
