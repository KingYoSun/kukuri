use super::*;

impl AppService {
    pub(crate) async fn persist_dome_preset_manifest(
        &self,
        manifest: DomePresetManifestV1,
    ) -> Result<DomePresetRefV1> {
        let envelope = build_dome_preset_envelope(self.services.keys.as_ref(), &manifest)?;
        let stored = store_manifest_blob(
            self.services.blob_service.as_ref(),
            &manifest,
            DOME_PRESET_MANIFEST_MIME,
        )
        .await?;
        let state = DomePresetStateDocV1 {
            preset_id: manifest.preset_id.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            updated_at: manifest.updated_at,
            last_envelope_id: envelope.id.clone(),
        };
        let replica = author_replica_id(manifest.owner_pubkey.as_str());
        self.services.docs_sync.open_replica(&replica).await?;
        self.services
            .docs_sync
            .apply_doc_op(
                &replica,
                DocOp::SetJson {
                    key: stable_key("envelopes", envelope.id.as_str()),
                    value: serde_json::to_value(&envelope)?,
                },
            )
            .await?;
        self.services
            .docs_sync
            .apply_doc_op(
                &replica,
                DocOp::SetJson {
                    key: stable_key(
                        "metaverse/dome-presets",
                        &format!("{}/state", manifest.preset_id),
                    ),
                    value: serde_json::to_value(&state)?,
                },
            )
            .await?;
        self.services
            .projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(DomePresetRefV1 {
            preset_id: manifest.preset_id,
            owner_pubkey: manifest.owner_pubkey,
            manifest_blob_hash: stored.hash.as_str().to_string(),
            manifest_mime: stored.mime,
            manifest_bytes: stored.bytes,
        })
    }

    pub(crate) async fn fetch_dome_preset_manifest(
        &self,
        preset_ref: &DomePresetRefV1,
    ) -> Result<Option<DomePresetManifestV1>> {
        let replica = author_replica_id(preset_ref.owner_pubkey.as_str());
        let state_records = self
            .services
            .docs_sync
            .query_replica(
                &replica,
                DocQuery::Exact(stable_key(
                    "metaverse/dome-presets",
                    &format!("{}/state", preset_ref.preset_id),
                )),
            )
            .await?;
        let Some(state_record) = state_records.into_iter().next() else {
            return Ok(None);
        };
        let state: DomePresetStateDocV1 = serde_json::from_slice(&state_record.value)?;
        if state.preset_id != preset_ref.preset_id
            || state.owner_pubkey != preset_ref.owner_pubkey
            || state.current_manifest.hash.as_str() != preset_ref.manifest_blob_hash
            || state.current_manifest.mime != preset_ref.manifest_mime
            || state.current_manifest.bytes != preset_ref.manifest_bytes
        {
            anyhow::bail!("Dome Preset state does not match its reference");
        }
        let manifest = fetch_manifest_blob::<DomePresetManifestV1>(
            self.services.blob_service.as_ref(),
            &ManifestBlobRef {
                hash: kukuri_core::BlobHash::new(preset_ref.manifest_blob_hash.clone()),
                mime: preset_ref.manifest_mime.clone(),
                bytes: preset_ref.manifest_bytes,
            },
        )
        .await?;
        let Some(manifest) = manifest else {
            return Ok(None);
        };
        validate_dome_preset_manifest(&manifest)?;
        if manifest.preset_id != preset_ref.preset_id
            || manifest.owner_pubkey != preset_ref.owner_pubkey
        {
            anyhow::bail!("Dome Preset reference does not match its manifest");
        }
        let signed: DomePresetManifestV1 = fetch_verified_dome_envelope(
            self.services.docs_sync.as_ref(),
            &replica,
            &state.last_envelope_id,
            "dome-preset",
            &manifest.owner_pubkey,
        )
        .await?;
        if signed != manifest {
            anyhow::bail!("signed Dome Preset content does not match its manifest blob");
        }
        Ok(Some(manifest))
    }

    pub(crate) async fn persist_dome_instance_manifest(
        &self,
        replica: &ReplicaId,
        manifest: &DomeInstanceManifestV1,
        created_at: i64,
    ) -> Result<DomeInstanceStateDocV1> {
        let envelope = build_dome_instance_envelope(self.services.keys.as_ref(), manifest)?;
        let stored = store_manifest_blob(
            self.services.blob_service.as_ref(),
            manifest,
            DOME_INSTANCE_MANIFEST_MIME,
        )
        .await?;
        let state = DomeInstanceStateDocV1 {
            instance_id: manifest.instance_id.clone(),
            spatial_context: manifest.spatial_context.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            generation: manifest.generation,
            status: manifest.status,
            created_at,
            updated_at: manifest.updated_at,
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            last_envelope_id: envelope.id.clone(),
        };
        self.services.docs_sync.open_replica(replica).await?;
        self.services
            .docs_sync
            .apply_doc_op(
                replica,
                DocOp::SetJson {
                    key: stable_key("envelopes", envelope.id.as_str()),
                    value: serde_json::to_value(&envelope)?,
                },
            )
            .await?;
        self.services
            .docs_sync
            .apply_doc_op(
                replica,
                DocOp::SetJson {
                    key: stable_key(
                        "metaverse/dome-instances",
                        &format!("{}/state", manifest.owner_pubkey.as_str()),
                    ),
                    value: serde_json::to_value(&state)?,
                },
            )
            .await?;
        self.services
            .projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    pub(crate) async fn fetch_dome_instance_manifest(
        &self,
        replica: &ReplicaId,
        owner_pubkey: &Pubkey,
    ) -> Result<Option<(DomeInstanceStateDocV1, DomeInstanceManifestV1)>> {
        let records = self
            .services
            .docs_sync
            .query_replica(
                replica,
                DocQuery::Exact(stable_key(
                    "metaverse/dome-instances",
                    &format!("{}/state", owner_pubkey.as_str()),
                )),
            )
            .await?;
        let Some(record) = records.into_iter().next() else {
            return Ok(None);
        };
        let state: DomeInstanceStateDocV1 = serde_json::from_slice(&record.value)?;
        let manifest = fetch_manifest_blob::<DomeInstanceManifestV1>(
            self.services.blob_service.as_ref(),
            &state.current_manifest,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("Dome Instance manifest is unavailable"))?;
        kukuri_core::validate_dome_instance_manifest(&manifest)?;
        if state.instance_id != manifest.instance_id
            || state.owner_pubkey != manifest.owner_pubkey
            || state.spatial_context != manifest.spatial_context
            || state.generation != manifest.generation
            || state.status != manifest.status
        {
            anyhow::bail!("Dome Instance state does not match its manifest");
        }
        let signed: DomeInstanceManifestV1 = fetch_verified_dome_envelope(
            self.services.docs_sync.as_ref(),
            replica,
            &state.last_envelope_id,
            "dome-instance",
            &manifest.owner_pubkey,
        )
        .await?;
        if signed != manifest {
            anyhow::bail!("signed Dome Instance content does not match its manifest blob");
        }
        Ok(Some((state, manifest)))
    }

    pub(crate) async fn persist_dome_move_record(&self, record: &DomeMoveRecordV1) -> Result<()> {
        let envelope = build_dome_move_envelope(self.services.keys.as_ref(), record)?;
        let replica = author_replica_id(record.owner_pubkey.as_str());
        self.services.docs_sync.open_replica(&replica).await?;
        self.services
            .docs_sync
            .apply_doc_op(
                &replica,
                DocOp::SetJson {
                    key: stable_key("envelopes", envelope.id.as_str()),
                    value: serde_json::to_value(&envelope)?,
                },
            )
            .await?;
        self.services
            .docs_sync
            .apply_doc_op(
                &replica,
                DocOp::SetJson {
                    key: stable_key("metaverse/dome-moves", &format!("{}/state", record.move_id)),
                    value: serde_json::to_value(DomeMoveStateDocV1 {
                        record: record.clone(),
                        last_envelope_id: envelope.id,
                    })?,
                },
            )
            .await
    }

    pub(crate) async fn fetch_dome_move_record(
        &self,
        owner_pubkey: &str,
        move_id: &str,
    ) -> Result<Option<DomeMoveRecordV1>> {
        let records = self
            .services
            .docs_sync
            .query_replica(
                &author_replica_id(owner_pubkey),
                DocQuery::Exact(stable_key(
                    "metaverse/dome-moves",
                    &format!("{move_id}/state"),
                )),
            )
            .await?;
        let Some(state_record) = records.into_iter().next() else {
            return Ok(None);
        };
        let state: DomeMoveStateDocV1 = serde_json::from_slice(&state_record.value)?;
        let signed: DomeMoveRecordV1 = fetch_verified_dome_envelope(
            self.services.docs_sync.as_ref(),
            &author_replica_id(owner_pubkey),
            &state.last_envelope_id,
            "dome-move",
            &state.record.owner_pubkey,
        )
        .await?;
        if signed != state.record {
            anyhow::bail!("signed Dome move content does not match its state record");
        }
        Ok(Some(state.record))
    }

    pub(crate) async fn stop_live_presence_task(
        &self,
        topic_id: &str,
        channel_id: &str,
        session_id: &str,
    ) {
        let key = live_presence_task_key(topic_id, channel_id, session_id);
        let handle = self
            .subscription_registry
            .live_presence_tasks
            .lock()
            .await
            .remove(key.as_str());
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
        self.services
            .projection_store
            .upsert_live_presence(
                topic_id,
                channel_storage_id(channel_id).as_str(),
                session_id,
                author.as_str(),
                now + i64::from(ttl_ms),
                now,
            )
            .await?;
        self.services
            .projection_store
            .clear_expired_live_presence(now)
            .await?;
        self.services
            .hint_transport
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
        let stored = store_manifest_blob(
            self.services.blob_service.as_ref(),
            &manifest,
            LIVE_MANIFEST_MIME,
        )
        .await?;
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
        persist_live_session_state(self.services.docs_sync.as_ref(), replica, &state).await?;
        self.services
            .projection_store
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
        let stored = store_manifest_blob(
            self.services.blob_service.as_ref(),
            &manifest,
            GAME_MANIFEST_MIME,
        )
        .await?;
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
        persist_game_room_state(self.services.docs_sync.as_ref(), replica, &state).await?;
        self.services
            .projection_store
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
                self.services.docs_sync.as_ref(),
                &replica,
                session_id,
            )
            .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<LiveSessionManifestBlobV1>(
                self.services.blob_service.as_ref(),
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
            let Some(state) = fetch_game_room_state_from_replica(
                self.services.docs_sync.as_ref(),
                &replica,
                room_id,
            )
            .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<GameRoomManifestBlobV1>(
                self.services.blob_service.as_ref(),
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

async fn fetch_verified_dome_envelope<T: DeserializeOwned>(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    envelope_id: &EnvelopeId,
    expected_kind: &str,
    expected_owner: &Pubkey,
) -> Result<T> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("envelopes", envelope_id.as_str())),
        )
        .await?;
    let envelope: KukuriEnvelope = records
        .into_iter()
        .next()
        .map(|record| serde_json::from_slice(&record.value))
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("signed Dome envelope is unavailable"))?;
    envelope.verify()?;
    if envelope.id != *envelope_id
        || envelope.kind != expected_kind
        || envelope.pubkey != *expected_owner
    {
        anyhow::bail!("signed Dome envelope identity does not match state");
    }
    serde_json::from_str(envelope.content.as_str()).map_err(Into::into)
}
