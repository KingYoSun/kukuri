use super::*;

pub(crate) async fn hydrate_object_projection_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
) -> Result<usize> {
    hydrate_object_projection_from_replica_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn hydrate_object_projection_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<usize> {
    let records =
        query_replica_with_fetch_policy(docs_sync, replica, DocQuery::Prefix("objects/".into()), policy)
            .await?;
    let mut hydrated = 0usize;
    let mut blob_statuses = Vec::new();
    let mut projections = Vec::new();
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
        let content = match &header.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => {
                let payload = fetch_projection_blob_text(blob_service, hash).await;
                blob_statuses.push((
                    hash.clone(),
                    match payload {
                        Some(_) => BlobCacheStatus::Available,
                        None => BlobCacheStatus::Missing,
                    },
                ));
                payload
            }
        };
        for attachment in &header.attachments {
            let status = best_effort_blob_cache_status(blob_service, &attachment.hash).await;
            blob_statuses.push((attachment.hash.clone(), status));
        }
        projections.push(projection_row_from_header(&header, content, replica));
        hydrated += 1;
    }
    projection_store.mark_blob_statuses(blob_statuses).await?;
    projection_store.put_object_projections(projections).await?;
    Ok(hydrated)
}

pub(crate) async fn hydrate_object_projection_from_record(
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
    let content = match &header.payload_ref {
        PayloadRef::InlineText { text } => Some(text.clone()),
        PayloadRef::BlobText { hash, .. } => {
            let payload = fetch_projection_blob_text(blob_service, hash).await;
            projection_store
                .mark_blob_status(
                    hash,
                    match payload {
                        Some(_) => BlobCacheStatus::Available,
                        None => BlobCacheStatus::Missing,
                    },
                )
                .await?;
            payload
        }
    };
    for attachment in &header.attachments {
        let status = best_effort_blob_cache_status(blob_service, &attachment.hash).await;
        projection_store
            .mark_blob_status(&attachment.hash, status)
            .await?;
    }
    projection_store
        .put_object_projection(projection_row_from_header(&header, content, replica))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_object_projection_from_key(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_object_projection_from_record(blob_service, projection_store, replica, record).await
}

pub(crate) async fn hydrate_reaction_cache_from_replica(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
) -> Result<usize> {
    hydrate_reaction_cache_from_replica_with_policy(
        docs_sync,
        projection_store,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn hydrate_reaction_cache_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<usize> {
    let records = query_replica_with_fetch_policy(
        docs_sync,
        replica,
        DocQuery::Prefix("reactions/".into()),
        policy,
    )
    .await?;
    let mut hydrated = 0usize;
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        let reaction: ReactionDocV1 = serde_json::from_slice(record.value.as_slice())?;
        projection_store
            .upsert_reaction_cache(reaction_projection_row_from_doc(&reaction, replica))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_reaction_cache_from_record(
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let reaction: ReactionDocV1 = serde_json::from_slice(record.value.as_slice())?;
    projection_store
        .upsert_reaction_cache(reaction_projection_row_from_doc(&reaction, replica))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_reaction_cache_from_key(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_reaction_cache_from_record(projection_store, replica, record).await
}

pub(crate) async fn hydrate_reaction_cache_for_target(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    target_object_id: &str,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Prefix(stable_key("reactions", &format!("{target_object_id}/"))),
        )
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        hydrated +=
            hydrate_reaction_cache_from_record(projection_store, replica, record).await? as usize;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_topic_state_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
) -> Result<usize> {
    hydrate_topic_state_with_services_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn hydrate_topic_state_with_services_with_policy(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    policy: DocFetchPolicy,
) -> Result<usize> {
    hydrate_subscription_state_with_services_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        &topic_replica_id(topic_id),
        policy,
    )
    .await
}

pub(crate) async fn hydrate_subscription_state_with_services_with_policy(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<usize> {
    let post_count = hydrate_object_projection_from_replica_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        replica,
        policy,
    )
    .await
    ?;
    let reaction_count =
        hydrate_reaction_cache_from_replica_with_policy(docs_sync, projection_store, replica, policy)
            .await?;
    let live_count = hydrate_live_sessions_from_replica_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
        policy,
    )
    .await?;
    let game_count = hydrate_game_rooms_from_replica_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
        policy,
    )
    .await?;
    Ok(post_count + reaction_count + live_count + game_count)
}

pub(crate) async fn hydrate_subscription_state_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    hydrate_subscription_state_with_services_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn hydrate_live_sessions_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    hydrate_live_sessions_from_replica_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn hydrate_live_sessions_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<usize> {
    let records = query_replica_with_fetch_policy(
        docs_sync,
        replica,
        DocQuery::Prefix("sessions/live/".into()),
        policy,
    )
    .await?;
    let mut hydrated = 0usize;
    for record in records {
        let state: LiveSessionStateDocV1 = serde_json::from_slice(&record.value)?;
        projection_store
            .mark_blob_status(
                &state.current_manifest.hash,
                blob_status(
                    blob_service
                        .blob_status(&state.current_manifest.hash)
                        .await?,
                ),
            )
            .await?;
        let Some(manifest) =
            fetch_manifest_blob::<LiveSessionManifestBlobV1>(blob_service, &state.current_manifest)
                .await?
        else {
            continue;
        };
        projection_store
            .upsert_live_session_cache(live_projection_row_from_state(
                &state, &manifest, topic_id, replica,
            ))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_live_session_from_record(
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let state: LiveSessionStateDocV1 = serde_json::from_slice(&record.value)?;
    projection_store
        .mark_blob_status(
            &state.current_manifest.hash,
            blob_status(
                blob_service
                    .blob_status(&state.current_manifest.hash)
                    .await?,
            ),
        )
        .await?;
    let Some(manifest) =
        fetch_manifest_blob::<LiveSessionManifestBlobV1>(blob_service, &state.current_manifest)
            .await?
    else {
        return Ok(false);
    };
    projection_store
        .upsert_live_session_cache(live_projection_row_from_state(
            &state, &manifest, topic_id, replica,
        ))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_live_session_from_key(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_live_session_from_record(blob_service, projection_store, topic_id, replica, record)
        .await
}

pub(crate) async fn hydrate_live_session_from_key_with_retry(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<usize> {
    for attempt in 0..session_projection_retry_attempts() {
        if hydrate_live_session_from_key(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await?
        {
            return Ok(1);
        }
        if attempt + 1 < session_projection_retry_attempts() {
            tokio::time::sleep(session_projection_retry_delay()).await;
        }
    }
    Ok(0)
}

pub(crate) async fn hydrate_game_rooms_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    hydrate_game_rooms_from_replica_with_policy(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
        DocFetchPolicy::LocalThenRemote,
    )
    .await
}

pub(crate) async fn hydrate_game_rooms_from_replica_with_policy(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    policy: DocFetchPolicy,
) -> Result<usize> {
    let records = query_replica_with_fetch_policy(
        docs_sync,
        replica,
        DocQuery::Prefix("sessions/game/".into()),
        policy,
    )
    .await?;
    let mut hydrated = 0usize;
    for record in records {
        let state: GameRoomStateDocV1 = serde_json::from_slice(&record.value)?;
        projection_store
            .mark_blob_status(
                &state.current_manifest.hash,
                blob_status(
                    blob_service
                        .blob_status(&state.current_manifest.hash)
                        .await?,
                ),
            )
            .await?;
        let Some(manifest) =
            fetch_manifest_blob::<GameRoomManifestBlobV1>(blob_service, &state.current_manifest)
                .await?
        else {
            continue;
        };
        projection_store
            .upsert_game_room_cache(game_projection_row_from_state(
                &state, &manifest, topic_id, replica,
            ))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_game_room_from_record(
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let state: GameRoomStateDocV1 = serde_json::from_slice(&record.value)?;
    projection_store
        .mark_blob_status(
            &state.current_manifest.hash,
            blob_status(
                blob_service
                    .blob_status(&state.current_manifest.hash)
                    .await?,
            ),
        )
        .await?;
    let Some(manifest) =
        fetch_manifest_blob::<GameRoomManifestBlobV1>(blob_service, &state.current_manifest)
            .await?
    else {
        return Ok(false);
    };
    projection_store
        .upsert_game_room_cache(game_projection_row_from_state(
            &state, &manifest, topic_id, replica,
        ))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_game_room_from_key(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_game_room_from_record(blob_service, projection_store, topic_id, replica, record).await
}

pub(crate) async fn hydrate_game_room_from_key_with_retry(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<usize> {
    for attempt in 0..session_projection_retry_attempts() {
        if hydrate_game_room_from_key(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await?
        {
            return Ok(1);
        }
        if attempt + 1 < session_projection_retry_attempts() {
            tokio::time::sleep(session_projection_retry_delay()).await;
        }
    }
    Ok(0)
}

pub(crate) async fn hydrate_subscription_event_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<usize> {
    if key.starts_with("objects/") && key.ends_with("/state") {
        return Ok(hydrate_object_projection_from_key(
            docs_sync,
            blob_service,
            projection_store,
            replica,
            key,
        )
        .await? as usize);
    }
    if key.starts_with("reactions/") && key.ends_with("/state") {
        return Ok(
            hydrate_reaction_cache_from_key(docs_sync, projection_store, replica, key).await?
                as usize,
        );
    }
    if key.starts_with("sessions/live/") && key.ends_with("/state") {
        return hydrate_live_session_from_key_with_retry(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await;
    }
    if key.starts_with("sessions/game/") && key.ends_with("/state") {
        return hydrate_game_room_from_key_with_retry(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await;
    }
    Ok(0)
}

pub(crate) async fn hydrate_subscription_hint_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    hint: &GossipHint,
) -> Result<usize> {
    match hint {
        GossipHint::TopicObjectsChanged { objects, .. } => {
            let mut hydrated = 0usize;
            for object in objects {
                if object.object_kind == "reaction" {
                    hydrated += hydrate_reaction_cache_for_target(
                        docs_sync,
                        projection_store,
                        replica,
                        object.object_id.as_str(),
                    )
                    .await?;
                    continue;
                }
                hydrated += hydrate_object_projection_from_key(
                    docs_sync,
                    blob_service,
                    projection_store,
                    replica,
                    stable_key("objects", &format!("{}/state", object.object_id)).as_str(),
                )
                .await? as usize;
            }
            Ok(hydrated)
        }
        GossipHint::ThreadUpdated { object_ids, .. } => {
            let mut hydrated = 0usize;
            for object_id in object_ids {
                hydrated += hydrate_object_projection_from_key(
                    docs_sync,
                    blob_service,
                    projection_store,
                    replica,
                    stable_key("objects", &format!("{}/state", object_id.as_str())).as_str(),
                )
                .await? as usize;
            }
            Ok(hydrated)
        }
        GossipHint::SessionChanged {
            session_id,
            object_kind,
            ..
        } => match object_kind.as_str() {
            "live-session" => {
                hydrate_live_session_from_key_with_retry(
                    docs_sync,
                    blob_service,
                    projection_store,
                    topic_id,
                    replica,
                    stable_key("sessions/live", &format!("{session_id}/state")).as_str(),
                )
                .await
            }
            "game-session" => {
                hydrate_game_room_from_key_with_retry(
                    docs_sync,
                    blob_service,
                    projection_store,
                    topic_id,
                    replica,
                    stable_key("sessions/game", &format!("{session_id}/state")).as_str(),
                )
                .await
            }
            _ => Ok(0),
        },
        GossipHint::ProfileUpdated { .. }
        | GossipHint::Presence { .. }
        | GossipHint::Typing { .. }
        | GossipHint::LivePresence { .. }
        | GossipHint::DirectMessageFrame { .. }
        | GossipHint::DirectMessageAck { .. } => Ok(0),
    }
}

pub(crate) fn hint_targets_topic(hint: &GossipHint, topic: &str) -> bool {
    match hint {
        GossipHint::TopicObjectsChanged { topic_id, .. }
        | GossipHint::Presence { topic_id, .. }
        | GossipHint::Typing { topic_id, .. }
        | GossipHint::SessionChanged { topic_id, .. }
        | GossipHint::LivePresence { topic_id, .. }
        | GossipHint::DirectMessageFrame { topic_id, .. }
        | GossipHint::DirectMessageAck { topic_id, .. } => topic_id.as_str() == topic,
        GossipHint::ThreadUpdated { .. } | GossipHint::ProfileUpdated { .. } => true,
    }
}

pub(crate) fn projection_page_needs_hydration(page: &Page<ObjectProjectionRow>) -> bool {
    page.items.iter().any(|item| item.content.is_none())
}

pub(crate) fn profile_timeline_page(
    posts: Vec<ProfileTimelineItem>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<ProfileTimelineItem> {
    if limit == 0 {
        return Page {
            items: Vec::new(),
            next_cursor: cursor,
        };
    }

    let mut items = Vec::new();
    let mut next_cursor = None;
    for post in posts {
        let include = cursor.as_ref().is_none_or(|current| {
            post.created_at() < current.created_at
                || (post.created_at() == current.created_at
                    && post.object_id() < &current.object_id)
        });
        if !include {
            continue;
        }
        if items.len() >= limit {
            next_cursor = Some(TimelineCursor {
                created_at: post.created_at(),
                object_id: post.object_id().clone(),
            });
            break;
        }
        items.push(post);
    }

    Page { items, next_cursor }
}
