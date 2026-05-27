use super::*;

pub(crate) async fn persist_post_object(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    object: CanonicalPostHeader,
    envelope: KukuriEnvelope,
) -> Result<()> {
    let sort_key = timeline_sort_key(object.created_at, &object.object_id);
    let object_json = serde_json::to_value(&object)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("objects", &format!("{}/state", object.object_id.as_str())),
                value: object_json,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "objects",
                    &format!("{}/envelope", object.object_id.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "indexes/timeline",
                    &format!("{sort_key}/{}", object.object_id.as_str()),
                ),
                value: serde_json::json!({
                    "object_id": object.object_id,
                    "created_at": object.created_at,
                    "object_kind": object.object_kind,
                }),
            },
        )
        .await?;
    let root_id = object
        .root
        .clone()
        .unwrap_or_else(|| object.object_id.clone());
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "indexes/thread",
                    &format!(
                        "{}/{sort_key}/{}",
                        root_id.as_str(),
                        object.object_id.as_str()
                    ),
                ),
                value: serde_json::json!({
                    "object_id": object.object_id,
                    "root_id": root_id,
                    "reply_to": object.reply_to,
                }),
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_media_manifest(
    replica: &ReplicaId,
    envelope: &KukuriEnvelope,
    manifest: &KukuriMediaManifestV1,
    docs_sync: &dyn DocsSync,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "manifests/media",
                    &format!("{}/state", manifest.manifest_id),
                ),
                value: serde_json::to_value(manifest)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "manifests/media",
                    &format!("{}/envelope", manifest.manifest_id),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_live_session_state(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    state: &LiveSessionStateDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_game_room_state(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    state: &GameRoomStateDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_private_channel_metadata(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    metadata: &PrivateChannelMetadataDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "metadata"),
                value: serde_json::to_value(metadata)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "topic"),
                value: serde_json::json!({ "topic_id": metadata.topic_id }),
            },
        )
        .await
}

pub(crate) async fn persist_private_channel_policy(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    policy: &PrivateChannelPolicyDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_policy_envelope(keys, policy)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "policy/envelope"),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_private_channel_participant(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    participant: &PrivateChannelParticipantDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_participant_envelope(keys, participant)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "channels/participants",
                    &format!("{}/envelope", participant.participant_pubkey.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_private_channel_rotation_grant(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    grant: &PrivateChannelEpochHandoffGrantDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_epoch_handoff_grant_envelope(keys, grant)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "channels/rotation-grants",
                    &format!("{}/envelope", grant.recipient_pubkey.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn fetch_private_channel_metadata_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Option<PrivateChannelMetadataDocV1>> {
    fetch_private_channel_metadata_from_replica_with_policy(
        docs_sync,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn fetch_private_channel_metadata_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<Option<PrivateChannelMetadataDocV1>> {
    let Some(record) = query_replica_with_fetch_policy(
        docs_sync,
        replica,
        DocQuery::Exact(stable_key("channels", "metadata")),
        policy,
    )
    .await?
    .into_iter()
    .next() else {
        return Ok(None);
    };
    let mut metadata: PrivateChannelMetadataDocV1 = serde_json::from_slice(&record.value)?;
    if metadata.owner_pubkey.as_str().trim().is_empty() {
        metadata.owner_pubkey = metadata.creator_pubkey.clone();
    }
    Ok(Some(metadata))
}

pub(crate) async fn fetch_private_channel_policy_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Option<PrivateChannelPolicyDocV1>> {
    fetch_private_channel_policy_from_replica_with_policy(
        docs_sync,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn fetch_private_channel_policy_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<Option<PrivateChannelPolicyDocV1>> {
    let Some(record) = query_replica_with_fetch_policy(
        docs_sync,
        replica,
        DocQuery::Exact(stable_key("channels", "policy/envelope")),
        policy,
    )
    .await?
    .into_iter()
    .next() else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
    envelope.verify()?;
    parse_private_channel_policy(&envelope)
}

pub(crate) async fn fetch_private_channel_participants_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Vec<PrivateChannelParticipantDocV1>> {
    fetch_private_channel_participants_from_replica_with_policy(
        docs_sync,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn fetch_private_channel_participants_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<Vec<PrivateChannelParticipantDocV1>> {
    let records = query_replica_with_fetch_policy(
        docs_sync,
        replica,
        DocQuery::Prefix(stable_key("channels/participants", "")),
        policy,
    )
    .await?;
    let mut items = Vec::new();
    for record in records {
        if !record.key.ends_with("/envelope") {
            continue;
        }
        let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
        envelope.verify()?;
        if let Some(participant) = parse_private_channel_participant(&envelope)? {
            items.push(participant);
        }
    }
    items.sort_by(|left, right| left.participant_pubkey.cmp(&right.participant_pubkey));
    Ok(items)
}

pub(crate) async fn fetch_private_channel_rotation_grant_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    recipient_pubkey: &str,
) -> Result<Option<PrivateChannelEpochHandoffGrantDocV1>> {
    fetch_private_channel_rotation_grant_from_replica_with_policy(
        docs_sync,
        replica,
        recipient_pubkey,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn fetch_private_channel_rotation_grant_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    recipient_pubkey: &str,
    policy: DocFetchPolicy,
) -> Result<Option<PrivateChannelEpochHandoffGrantDocV1>> {
    let Some(record) = query_replica_with_fetch_policy(
        docs_sync,
        replica,
        DocQuery::Exact(stable_key(
            "channels/rotation-grants",
            &format!("{recipient_pubkey}/envelope"),
        )),
        policy,
    )
    .await?
    .into_iter()
    .next() else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
    envelope.verify()?;
    parse_private_channel_epoch_handoff_grant(&envelope)
}

pub(crate) async fn wait_for_private_channel_epoch_snapshot(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    timeout_label: &str,
) -> Result<(
    PrivateChannelMetadataDocV1,
    PrivateChannelPolicyDocV1,
    Vec<PrivateChannelParticipantDocV1>,
)> {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let metadata = fetch_private_channel_metadata_from_replica(docs_sync, replica).await?;
            let policy = fetch_private_channel_policy_from_replica(docs_sync, replica).await?;
            let participants =
                fetch_private_channel_participants_from_replica(docs_sync, replica).await?;
            let owner_participant_visible = policy.as_ref().is_some_and(|policy| {
                participants.iter().any(|participant| {
                    participant.participant_pubkey == policy.owner_pubkey
                        && participant.epoch_id == policy.epoch_id
                        && participant.is_owner
                        && participant.left_at.is_none()
                })
            });
            if let (Some(metadata), Some(policy)) = (metadata, policy)
                && owner_participant_visible
            {
                return Ok::<_, anyhow::Error>((metadata, policy, participants));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("timed out waiting for {timeout_label}"))?
}

pub(crate) async fn private_channel_rotation_is_pending(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    state: &JoinedPrivateChannelState,
) -> Result<bool> {
    let replica = current_private_channel_replica_id(state);
    let Some(policy) = fetch_private_channel_policy_from_replica(docs_sync, &replica).await? else {
        return Ok(false);
    };
    if policy.sharing_state != ChannelSharingState::Frozen || policy.rotated_at.is_none() {
        return Ok(false);
    }
    let local_author = keys.public_key_hex();
    let Some(grant) = fetch_private_channel_rotation_grant_from_replica(
        docs_sync,
        &replica,
        local_author.as_str(),
    )
    .await?
    else {
        return Ok(false);
    };
    let payload = decrypt_private_channel_epoch_handoff_grant(keys, &grant)?;
    Ok(payload.new_epoch_id != state.current_epoch_id)
}

pub(crate) async fn store_manifest_blob<T: Serialize>(
    blob_service: &dyn BlobService,
    manifest: &T,
    mime: &str,
) -> Result<StoredBlob> {
    let payload = serde_json::to_vec(manifest)?;
    blob_service.put_blob(payload, mime).await
}

pub(crate) async fn fetch_manifest_blob<T: DeserializeOwned>(
    blob_service: &dyn BlobService,
    blob_ref: &ManifestBlobRef,
) -> Result<Option<T>> {
    let Some(bytes) = (match tokio::time::timeout(
        projection_blob_fetch_timeout(),
        blob_service.fetch_blob(&blob_ref.hash),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) | Err(_) => None,
    }) else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

pub(crate) fn projection_blob_fetch_timeout() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_secs(5)
    } else {
        tokio::time::Duration::from_secs(2)
    }
}

pub(crate) fn projection_blob_status_timeout() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_secs(1)
    } else {
        tokio::time::Duration::from_millis(250)
    }
}

pub(crate) fn session_projection_retry_attempts() -> usize {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        20
    } else {
        10
    }
}

pub(crate) fn session_projection_retry_delay() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_millis(500)
    } else {
        tokio::time::Duration::from_millis(250)
    }
}

pub(crate) async fn fetch_projection_blob_text(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> Option<String> {
    match tokio::time::timeout(
        projection_blob_fetch_timeout(),
        blob_service.fetch_blob(hash),
    )
    .await
    {
        Ok(Ok(Some(bytes))) => Some(String::from_utf8_lossy(&bytes).to_string()),
        Ok(Ok(None)) | Ok(Err(_)) | Err(_) => None,
    }
}

pub(crate) async fn best_effort_blob_cache_status(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> BlobCacheStatus {
    match tokio::time::timeout(
        projection_blob_status_timeout(),
        blob_service.blob_status(hash),
    )
    .await
    {
        Ok(Ok(status)) => blob_status(status),
        Ok(Err(_)) | Err(_) => BlobCacheStatus::Missing,
    }
}

pub(crate) async fn best_effort_blob_view_status(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> BlobViewStatus {
    match tokio::time::timeout(
        projection_blob_status_timeout(),
        blob_service.blob_status(hash),
    )
    .await
    {
        Ok(Ok(status)) => blob_view_status(status),
        Ok(Err(_)) | Err(_) => BlobViewStatus::Missing,
    }
}

pub(crate) async fn fetch_live_session_state_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    session_id: &str,
) -> Result<Option<LiveSessionStateDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("sessions/live", &format!("{session_id}/state"))),
        )
        .await?;
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&record.value)?))
}

pub(crate) async fn fetch_game_room_state_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    room_id: &str,
) -> Result<Option<GameRoomStateDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("sessions/game", &format!("{room_id}/state"))),
        )
        .await?;
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&record.value)?))
}

pub(crate) fn live_projection_row_from_state(
    state: &LiveSessionStateDocV1,
    manifest: &LiveSessionManifestBlobV1,
    topic_id: &str,
    source_replica_id: &ReplicaId,
) -> LiveSessionProjectionRow {
    LiveSessionProjectionRow {
        session_id: state.session_id.clone(),
        topic_id: topic_id.to_string(),
        channel_id: channel_storage_id(state.channel_id.as_ref()),
        host_pubkey: state.owner_pubkey.as_str().to_string(),
        title: manifest.title.clone(),
        description: manifest.description.clone(),
        status: state.status.clone(),
        started_at: manifest.started_at,
        ended_at: manifest.ended_at,
        updated_at: state.updated_at,
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
        viewer_count: 0,
    }
}

pub(crate) fn game_projection_row_from_state(
    state: &GameRoomStateDocV1,
    manifest: &GameRoomManifestBlobV1,
    topic_id: &str,
    source_replica_id: &ReplicaId,
) -> GameRoomProjectionRow {
    GameRoomProjectionRow {
        room_id: state.room_id.clone(),
        topic_id: topic_id.to_string(),
        channel_id: channel_storage_id(state.channel_id.as_ref()),
        host_pubkey: state.owner_pubkey.as_str().to_string(),
        title: manifest.title.clone(),
        description: manifest.description.clone(),
        status: state.status.clone(),
        phase_label: manifest.phase_label.clone(),
        scores: manifest.scores.clone(),
        room_kind: manifest.room_kind.clone(),
        metaverse: manifest.metaverse.clone(),
        updated_at: state.updated_at,
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

pub(crate) fn projection_row_from_header(
    header: &CanonicalPostHeader,
    content: Option<String>,
    source_replica_id: &ReplicaId,
) -> ObjectProjectionRow {
    let source_blob_hash = match &header.payload_ref {
        PayloadRef::BlobText { hash, .. } => Some(hash.clone()),
        PayloadRef::InlineText { .. } => None,
    };
    ObjectProjectionRow {
        object_id: header.object_id.clone(),
        topic_id: header.topic_id.as_str().to_string(),
        channel_id: channel_storage_id(header.channel_id.as_ref()),
        author_pubkey: header.author.as_str().to_string(),
        created_at: header.created_at,
        object_kind: header.object_kind.clone(),
        root_object_id: header.root.clone(),
        reply_to_object_id: header.reply_to.clone(),
        payload_ref: header.payload_ref.clone(),
        content,
        attachments: header.attachments.clone(),
        repost_of: header.repost_of.clone(),
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("objects", &format!("{}/state", header.object_id.as_str())),
        source_envelope_id: header.envelope_id.clone(),
        source_blob_hash,
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 2,
    }
}

pub(crate) fn reaction_projection_row_from_doc(
    reaction: &ReactionDocV1,
    source_replica_id: &ReplicaId,
) -> ReactionProjectionRow {
    ReactionProjectionRow {
        source_replica_id: source_replica_id.clone(),
        target_object_id: reaction.target_object_id.clone(),
        reaction_id: reaction.reaction_id.clone(),
        author_pubkey: reaction.author_pubkey.as_str().to_string(),
        created_at: reaction.created_at,
        updated_at: reaction.updated_at,
        reaction_key_kind: reaction.reaction_key_kind.clone(),
        normalized_reaction_key: reaction.normalized_reaction_key.clone(),
        emoji: reaction.emoji.clone(),
        custom_asset_id: reaction.custom_asset_id.clone(),
        custom_asset_snapshot: reaction.custom_asset_snapshot.clone(),
        status: reaction.status.clone(),
        source_key: stable_key(
            "reactions",
            &format!(
                "{}/{}/state",
                reaction.target_object_id.as_str(),
                reaction.reaction_id.as_str()
            ),
        ),
        source_envelope_id: reaction.envelope_id.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

pub(crate) fn custom_reaction_asset_view_from_snapshot(
    snapshot: &CustomReactionAssetSnapshotV1,
) -> CustomReactionAssetView {
    CustomReactionAssetView {
        asset_id: snapshot.asset_id.clone(),
        owner_pubkey: snapshot.owner_pubkey.as_str().to_string(),
        blob_hash: snapshot.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(
            snapshot.search_key.as_str(),
            snapshot.asset_id.as_str(),
        ),
        mime: snapshot.mime.clone(),
        bytes: snapshot.bytes,
        width: snapshot.width,
        height: snapshot.height,
    }
}

pub(crate) fn custom_reaction_asset_view_from_doc(
    asset: &CustomReactionAssetDocV1,
) -> CustomReactionAssetView {
    CustomReactionAssetView {
        asset_id: asset.asset_id.clone(),
        owner_pubkey: asset.author_pubkey.as_str().to_string(),
        blob_hash: asset.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(asset.search_key.as_str(), asset.asset_id.as_str()),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        width: asset.width,
        height: asset.height,
    }
}

pub(crate) fn bookmarked_custom_reaction_view_from_row(
    row: BookmarkedCustomReactionRow,
) -> BookmarkedCustomReactionView {
    let asset_id = row.asset_id;
    BookmarkedCustomReactionView {
        asset_id: asset_id.clone(),
        owner_pubkey: row.owner_pubkey,
        blob_hash: row.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(row.search_key.as_str(), asset_id.as_str()),
        mime: row.mime,
        bytes: row.bytes,
        width: row.width,
        height: row.height,
    }
}

pub(crate) fn recent_reaction_view_from_projection(
    row: &ReactionProjectionRow,
) -> RecentReactionView {
    RecentReactionView {
        reaction_key_kind: reaction_key_kind_label(&row.reaction_key_kind).to_string(),
        normalized_reaction_key: row.normalized_reaction_key.clone(),
        emoji: row.emoji.clone(),
        custom_asset: row
            .custom_asset_snapshot
            .as_ref()
            .map(custom_reaction_asset_view_from_snapshot),
        updated_at: row.updated_at,
    }
}

pub(crate) fn reaction_key_kind_label(kind: &ReactionKeyKind) -> &'static str {
    match kind {
        ReactionKeyKind::Emoji => "emoji",
        ReactionKeyKind::CustomAsset => "custom_asset",
    }
}

pub(crate) fn reaction_key_view_from_projection(row: &ReactionProjectionRow) -> ReactionKeyView {
    ReactionKeyView {
        reaction_key_kind: reaction_key_kind_label(&row.reaction_key_kind).to_string(),
        normalized_reaction_key: row.normalized_reaction_key.clone(),
        emoji: row.emoji.clone(),
        custom_asset: row
            .custom_asset_snapshot
            .as_ref()
            .map(custom_reaction_asset_view_from_snapshot),
    }
}

pub(crate) fn reaction_cache_key(
    source_replica_id: &ReplicaId,
    target_object_id: &EnvelopeId,
) -> String {
    format!(
        "{}:{}",
        source_replica_id.as_str(),
        target_object_id.as_str()
    )
}

pub(crate) fn reaction_state_view_from_rows(
    source_replica_id: &ReplicaId,
    target_object_id: &EnvelopeId,
    rows: Vec<ReactionProjectionRow>,
    current_author: &str,
) -> ReactionStateView {
    let mut summary = BTreeMap::<String, ReactionSummaryView>::new();
    let mut my_reactions = Vec::new();
    for row in rows {
        let key_view = reaction_key_view_from_projection(&row);
        if row.status == ObjectStatus::Active {
            summary
                .entry(row.normalized_reaction_key.clone())
                .and_modify(|value| value.count += 1)
                .or_insert_with(|| ReactionSummaryView {
                    reaction_key_kind: key_view.reaction_key_kind.clone(),
                    normalized_reaction_key: key_view.normalized_reaction_key.clone(),
                    emoji: key_view.emoji.clone(),
                    custom_asset: key_view.custom_asset.clone(),
                    count: 1,
                });
            if row.author_pubkey == current_author {
                my_reactions.push(key_view);
            }
        }
    }
    ReactionStateView {
        target_object_id: target_object_id.as_str().to_string(),
        source_replica_id: source_replica_id.as_str().to_string(),
        reaction_summary: summary.into_values().collect(),
        my_reactions,
    }
}

pub(crate) fn search_key_or_asset_id(search_key: &str, asset_id: &str) -> String {
    let normalized = search_key.trim();
    if normalized.is_empty() {
        return asset_id.to_string();
    }
    normalized.to_string()
}
