use crate::service::*;
use kukuri_core::MetaverseRoomEventV1;

const METAVERSE_CHAT_HISTORY_LIMIT: usize = 100;

impl AppService {
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
        let muted_author_pubkeys = self.current_muted_author_pubkeys().await?;
        let allowed = self.allowed_channel_ids_for_scope(topic_id, &scope).await?;
        let mut rows = filter_channel_rows(
            self.projection_store
                .list_topic_game_rooms(topic_id)
                .await?,
            &allowed,
            |row| row.channel_id.as_str(),
        )
        .into_iter()
        .filter(|row| !muted_author_pubkeys.contains(row.host_pubkey.as_str()))
        .collect::<Vec<_>>();
        if rows.is_empty() {
            self.hydrate_scope_projection(topic_id, &scope).await?;
            rows = filter_channel_rows(
                self.projection_store
                    .list_topic_game_rooms(topic_id)
                    .await?,
                &allowed,
                |row| row.channel_id.as_str(),
            )
            .into_iter()
            .filter(|row| !muted_author_pubkeys.contains(row.host_pubkey.as_str()))
            .collect();
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
                room_kind: row.room_kind,
                metaverse: row.metaverse,
                manifest_blob_hash: row.manifest_blob_hash.as_str().to_string(),
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
            room_kind: GameRoomKind::ScoreGame,
            metaverse: None,
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

    pub async fn create_metaverse_room(
        &self,
        topic_id: &str,
        input: CreateMetaverseRoomInput,
    ) -> Result<String> {
        self.create_metaverse_room_in_channel(topic_id, ChannelRef::Public, input)
            .await
    }

    pub async fn create_metaverse_room_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        input: CreateMetaverseRoomInput,
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
        let now = Utc::now().timestamp_millis();
        let title = input.title.trim();
        if title.is_empty() {
            anyhow::bail!("metaverse room title is required");
        }
        let owner_pubkey = Pubkey::from(self.current_author_pubkey());
        let room_id = format!("meta-{}-{}", now, short_id_suffix(owner_pubkey.as_str()));
        let metaverse = MetaverseRoomStateV1 {
            world_version: 1,
            max_peers: input.max_peers,
            scene: MetaverseRoomSceneV1 {
                ground: "default".to_string(),
                shared_object: SharedRoomObjectV1 {
                    object_id: "mvp-object-1".to_string(),
                    asset_ref: None,
                    primitive_fallback: MetaversePrimitive::Cube,
                    position: [0, 50, -240],
                    rotation: [0, 0, 0],
                    scale: [100, 100, 100],
                    updated_by: owner_pubkey.clone(),
                    updated_at: now,
                },
            },
            default_spawn: MetaverseRoomSpawnV1 {
                position: [0, 0, 260],
                rotation: [0, 180, 0],
            },
            asset_refs: Vec::new(),
            chat_history: Vec::new(),
        };
        let manifest = GameRoomManifestBlobV1 {
            room_id: room_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: channel_id.clone(),
            owner_pubkey,
            title: title.to_string(),
            description: input.description.trim().to_string(),
            status: GameRoomStatus::Waiting,
            phase_label: Some("metaverse-mvp".to_string()),
            participants: Vec::new(),
            scores: Vec::new(),
            room_kind: GameRoomKind::MetaverseRoom,
            metaverse: Some(metaverse),
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
                "room_kind": "metaverse_room",
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

    pub async fn update_metaverse_room(
        &self,
        topic_id: &str,
        room_id: &str,
        input: UpdateMetaverseRoomInput,
    ) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let (source_replica_id, state, mut manifest) = self
            .fetch_game_room_state_and_manifest(topic_id, room_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("metaverse room not found"))?;
        if manifest.room_kind != GameRoomKind::MetaverseRoom {
            anyhow::bail!("game room is not a metaverse room");
        }
        let actor = self.current_author_pubkey();
        let is_owner = state.owner_pubkey.as_str() == actor;
        if !is_owner && input.status != manifest.status {
            anyhow::bail!("only the metaverse room owner can change the room status");
        }
        validate_game_room_transition(&manifest.status, &input.status)?;
        let now = Utc::now().timestamp_millis();
        let Some(metaverse) = manifest.metaverse.as_mut() else {
            anyhow::bail!("metaverse room state is missing");
        };
        metaverse.scene.shared_object.position = input.shared_object_position;
        metaverse.scene.shared_object.rotation = input.shared_object_rotation;
        metaverse.scene.shared_object.scale = input.shared_object_scale;
        metaverse.scene.shared_object.updated_by = Pubkey::from(actor);
        metaverse.scene.shared_object.updated_at = now;
        manifest.status = input.status;
        manifest.updated_at = now;
        let envelope = build_game_session_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            room_id,
            &serde_json::json!({
                "room_id": room_id,
                "topic_id": topic_id,
                "channel_id": state.channel_id.as_ref().map(|value| value.as_str()),
                "status": format!("{:?}", manifest.status).to_lowercase(),
                "room_kind": "metaverse_room",
                "world_version": metaverse.world_version,
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

    pub async fn publish_metaverse_room_event(
        &self,
        topic_id: &str,
        input: PublishMetaverseRoomEventInput,
    ) -> Result<MetaverseRoomEventView> {
        self.ensure_topic_subscription(topic_id).await?;
        let (source_replica_id, state, mut manifest) = self
            .fetch_game_room_state_and_manifest(topic_id, input.room_id.as_str())
            .await?
            .ok_or_else(|| anyhow::anyhow!("metaverse room not found"))?;
        if manifest.room_kind != GameRoomKind::MetaverseRoom {
            anyhow::bail!("game room is not a metaverse room");
        }
        if manifest.status == GameRoomStatus::Ended {
            anyhow::bail!("cannot publish events to an ended metaverse room");
        }
        validate_metaverse_room_event_identity(
            input.room_id.as_str(),
            input.peer_id.as_str(),
            &input.event,
        )?;
        let now = Utc::now().timestamp_millis();
        let event_id = format!(
            "mre-{}-{}-{}",
            now,
            input.seq,
            short_id_suffix(self.current_author_pubkey().as_str())
        );
        let content = MetaverseRoomEventEnvelopeContentV1 {
            event_id,
            topic_id: TopicId::new(topic_id),
            channel_id: state.channel_id.clone(),
            room_id: input.room_id,
            peer_id: input.peer_id,
            seq: input.seq,
            sent_at: now,
            event: input.event,
        };
        let envelope = build_metaverse_room_event_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            content.room_id.as_str(),
            &content,
        )?;
        let view = parse_metaverse_room_event_envelope(envelope.clone(), now, "local".to_string())?
            .ok_or_else(|| anyhow::anyhow!("failed to build metaverse room event"))?;
        push_metaverse_room_event_buffer(&self.metaverse_room_events, view.clone()).await;
        if let MetaverseRoomEventV1::ChatMessage { message } = &content.event {
            let Some(metaverse) = manifest.metaverse.as_mut() else {
                anyhow::bail!("metaverse room state is missing");
            };
            if !metaverse
                .chat_history
                .iter()
                .any(|existing| existing.message_id == message.message_id)
            {
                metaverse.chat_history.push(message.clone());
                if metaverse.chat_history.len() > METAVERSE_CHAT_HISTORY_LIMIT {
                    let overflow = metaverse
                        .chat_history
                        .len()
                        .saturating_sub(METAVERSE_CHAT_HISTORY_LIMIT);
                    metaverse.chat_history.drain(0..overflow);
                }
                manifest.updated_at = now;
                let persisted = self
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
                        &persisted,
                        &manifest,
                        topic_id,
                        &source_replica_id,
                    ))
                    .await?;
            }
        }
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, state.channel_id.as_ref()),
                GossipHint::MetaverseRoomEvent {
                    topic_id: TopicId::new(topic_id),
                    room_id: content.room_id,
                    event: Box::new(envelope),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(now);
        Ok(view)
    }

    pub async fn import_metaverse_room_asset(
        &self,
        topic_id: &str,
        input: ImportMetaverseRoomAssetInput,
    ) -> Result<MetaverseAssetRefView> {
        self.ensure_topic_subscription(topic_id).await?;
        let (_, _, manifest) = self
            .fetch_game_room_state_and_manifest(topic_id, input.room_id.as_str())
            .await?
            .ok_or_else(|| anyhow::anyhow!("metaverse room not found"))?;
        if manifest.room_kind != GameRoomKind::MetaverseRoom {
            anyhow::bail!("game room is not a metaverse room");
        }
        if input.bytes.is_empty() {
            anyhow::bail!("metaverse asset bytes are required");
        }
        let stored = self
            .blob_service
            .put_blob(input.bytes, input.mime_type.as_str())
            .await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(MetaverseAssetRef {
            kind: input.kind,
            blob_hash: stored.hash.as_str().to_string(),
            mime_type: Some(stored.mime),
            size_bytes: Some(stored.bytes),
            name: input.name,
        })
    }

    pub async fn list_metaverse_room_events(
        &self,
        topic_id: &str,
        room_id: &str,
        after_envelope_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<MetaverseRoomEventView>> {
        let key = metaverse_room_event_buffer_key(topic_id, room_id);
        let guard = self.metaverse_room_events.lock().await;
        let Some(queue) = guard.get(key.as_str()) else {
            return Ok(Vec::new());
        };
        let mut include = after_envelope_id.is_none();
        let mut items = Vec::new();
        for event in queue {
            if !include {
                include = after_envelope_id == Some(event.envelope_id.as_str());
                continue;
            }
            items.push(event.clone());
        }
        if let Some(limit) = limit
            && items.len() > limit
        {
            items = items.split_off(items.len() - limit);
        }
        Ok(items)
    }
}

fn validate_metaverse_room_event_identity(
    room_id: &str,
    peer_id: &str,
    event: &MetaverseRoomEventV1,
) -> Result<()> {
    match event {
        MetaverseRoomEventV1::PresenceJoin { presence } => {
            if presence.room_id != room_id || presence.peer_id != peer_id {
                anyhow::bail!("metaverse presence event identity does not match request");
            }
        }
        MetaverseRoomEventV1::PresenceLeave {
            room_id: event_room_id,
            peer_id: event_peer_id,
            ..
        } => {
            if event_room_id != room_id || event_peer_id != peer_id {
                anyhow::bail!("metaverse presence event identity does not match request");
            }
        }
        MetaverseRoomEventV1::AvatarTransform { transform } => {
            if transform.room_id != room_id || transform.peer_id != peer_id {
                anyhow::bail!("metaverse transform event identity does not match request");
            }
        }
        MetaverseRoomEventV1::ChatMessage { message } => {
            if message.room_id != room_id || message.author_peer_id != peer_id {
                anyhow::bail!("metaverse chat event identity does not match request");
            }
        }
        MetaverseRoomEventV1::ObjectUpdate { object } => {
            if object.updated_by.as_str() != peer_id {
                anyhow::bail!("metaverse object event identity does not match request");
            }
        }
    }
    Ok(())
}
