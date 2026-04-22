use super::*;

impl AppService {
    pub(crate) async fn stop_live_presence_task(
        &self,
        topic_id: &str,
        channel_id: &str,
        session_id: &str,
    ) {
        let key = live_presence_task_key(topic_id, channel_id, session_id);
        let handle = self.live_presence_tasks.lock().await.remove(key.as_str());
        if let Some(handle) = handle {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
    }

    pub(crate) async fn cleanup_ended_live_presence_tasks(
        &self,
        rows: &[LiveSessionProjectionRow],
    ) {
        for row in rows {
            if row.status == LiveSessionStatus::Ended {
                self.stop_live_presence_task(
                    row.topic_id.as_str(),
                    row.channel_id.as_str(),
                    row.session_id.as_str(),
                )
                .await;
            }
        }
    }

    pub(crate) async fn apply_live_presence(
        &self,
        topic_id: &str,
        channel_id: Option<&ChannelId>,
        session_id: &str,
        ttl_ms: u32,
    ) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let author = self.current_author_pubkey();
        self.projection_store
            .upsert_live_presence(
                topic_id,
                channel_storage_id(channel_id).as_str(),
                session_id,
                author.as_str(),
                now + i64::from(ttl_ms),
                now,
            )
            .await?;
        self.projection_store
            .clear_expired_live_presence(now)
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, channel_id),
                GossipHint::LivePresence {
                    topic_id: TopicId::new(topic_id),
                    session_id: session_id.to_string(),
                    author: Pubkey::from(author),
                    ttl_ms,
                },
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn persist_live_session_manifest(
        &self,
        replica: &ReplicaId,
        topic_id: &str,
        manifest: LiveSessionManifestBlobV1,
        created_at: i64,
        last_envelope_id: EnvelopeId,
    ) -> Result<LiveSessionStateDocV1> {
        let now = Utc::now().timestamp_millis();
        let stored =
            store_manifest_blob(self.blob_service.as_ref(), &manifest, LIVE_MANIFEST_MIME).await?;
        let state = LiveSessionStateDocV1 {
            session_id: manifest.session_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: manifest.channel_id.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            created_at,
            updated_at: now,
            status: manifest.status.clone(),
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            last_envelope_id,
        };
        persist_live_session_state(self.docs_sync.as_ref(), replica, &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    pub(crate) async fn persist_game_room_manifest(
        &self,
        replica: &ReplicaId,
        topic_id: &str,
        manifest: GameRoomManifestBlobV1,
        created_at: i64,
        last_envelope_id: EnvelopeId,
    ) -> Result<GameRoomStateDocV1> {
        let now = Utc::now().timestamp_millis();
        let stored =
            store_manifest_blob(self.blob_service.as_ref(), &manifest, GAME_MANIFEST_MIME).await?;
        let state = GameRoomStateDocV1 {
            room_id: manifest.room_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: manifest.channel_id.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            created_at,
            updated_at: now,
            status: manifest.status.clone(),
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            last_envelope_id,
        };
        persist_game_room_state(self.docs_sync.as_ref(), replica, &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    pub(crate) async fn fetch_live_session_state_and_manifest(
        &self,
        topic_id: &str,
        session_id: &str,
    ) -> Result<Option<(ReplicaId, LiveSessionStateDocV1, LiveSessionManifestBlobV1)>> {
        for replica in subscription_replicas_for_topic(
            topic_id,
            self.joined_private_channel_states_for_topic(topic_id).await,
        ) {
            let Some(state) = fetch_live_session_state_from_replica(
                self.docs_sync.as_ref(),
                &replica,
                session_id,
            )
            .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<LiveSessionManifestBlobV1>(
                self.blob_service.as_ref(),
                &state.current_manifest,
            )
            .await?
            else {
                continue;
            };
            return Ok(Some((replica, state, manifest)));
        }
        Ok(None)
    }

    pub(crate) async fn fetch_game_room_state_and_manifest(
        &self,
        topic_id: &str,
        room_id: &str,
    ) -> Result<Option<(ReplicaId, GameRoomStateDocV1, GameRoomManifestBlobV1)>> {
        for replica in subscription_replicas_for_topic(
            topic_id,
            self.joined_private_channel_states_for_topic(topic_id).await,
        ) {
            let Some(state) =
                fetch_game_room_state_from_replica(self.docs_sync.as_ref(), &replica, room_id)
                    .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<GameRoomManifestBlobV1>(
                self.blob_service.as_ref(),
                &state.current_manifest,
            )
            .await?
            else {
                continue;
            };
            return Ok(Some((replica, state, manifest)));
        }
        Ok(None)
    }
}
