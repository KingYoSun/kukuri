use crate::service::*;

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
}
