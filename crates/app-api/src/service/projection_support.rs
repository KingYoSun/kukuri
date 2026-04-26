use super::*;

pub(crate) async fn filtered_timeline_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channels: &BTreeSet<String>,
    muted_author_pubkeys: &BTreeSet<String>,
) -> Result<Page<ObjectProjectionRow>> {
    if limit == 0 {
        return Ok(Page {
            items: Vec::new(),
            next_cursor: cursor,
        });
    }
    let mut current_cursor = cursor;
    let mut items = Vec::new();
    let page_size = limit.max(20);
    loop {
        let page = ProjectionStore::list_topic_timeline_filtered(
            projection_store,
            topic_id,
            allowed_channels,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if !object_projection_row_is_muted(&row, muted_author_pubkeys) {
                items.push(row);
                if items.len() >= limit {
                    return Ok(Page { items, next_cursor });
                }
            }
        }
        if next_cursor.is_none() {
            return Ok(Page { items, next_cursor });
        }
        current_cursor = next_cursor;
    }
}

pub(crate) async fn filtered_thread_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    thread_root_object_id: &EnvelopeId,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channel: Option<&str>,
    muted_author_pubkeys: &BTreeSet<String>,
) -> Result<Page<ObjectProjectionRow>> {
    if limit == 0 {
        return Ok(Page {
            items: Vec::new(),
            next_cursor: cursor,
        });
    }
    let mut current_cursor = cursor;
    let mut items = Vec::new();
    let page_size = limit.max(20);
    loop {
        let page = ProjectionStore::list_thread_filtered(
            projection_store,
            topic_id,
            thread_root_object_id,
            allowed_channel,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if !object_projection_row_is_muted(&row, muted_author_pubkeys) {
                items.push(row);
                if items.len() >= limit {
                    return Ok(Page { items, next_cursor });
                }
            }
        }
        if next_cursor.is_none() {
            return Ok(Page { items, next_cursor });
        }
        current_cursor = next_cursor;
    }
}

pub(crate) fn filter_channel_rows<T>(
    rows: Vec<T>,
    allowed_channels: &BTreeSet<String>,
    channel_id: impl Fn(&T) -> &str,
) -> Vec<T> {
    rows.into_iter()
        .filter(|row| allowed_channels.contains(channel_id(row)))
        .collect()
}

pub(crate) fn object_projection_row_is_muted(
    row: &ObjectProjectionRow,
    muted_author_pubkeys: &BTreeSet<String>,
) -> bool {
    muted_author_pubkeys.contains(row.author_pubkey.as_str())
        || row.repost_of.as_ref().is_some_and(|snapshot| {
            muted_author_pubkeys.contains(snapshot.source_author_pubkey.as_str())
        })
}

pub(crate) fn bookmarked_post_row_is_muted(
    row: &BookmarkedPostRow,
    muted_author_pubkeys: &BTreeSet<String>,
) -> bool {
    muted_author_pubkeys.contains(row.author_pubkey.as_str())
        || row.repost_of.as_ref().is_some_and(|snapshot| {
            muted_author_pubkeys.contains(snapshot.source_author_pubkey.as_str())
        })
}

pub(crate) fn profile_timeline_item_is_muted(
    item: &ProfileTimelineItem,
    muted_author_pubkeys: &BTreeSet<String>,
) -> bool {
    match item {
        ProfileTimelineItem::Post(post) => {
            muted_author_pubkeys.contains(post.author_pubkey.as_str())
        }
        ProfileTimelineItem::Repost(repost) => {
            muted_author_pubkeys.contains(repost.author_pubkey.as_str())
                || muted_author_pubkeys.contains(repost.repost_of.source_author_pubkey.as_str())
        }
    }
}

pub(crate) async fn fetch_post_object_for_projection(
    docs_sync: &dyn DocsSync,
    replica_id: &ReplicaId,
    source_key: &str,
) -> Result<Option<CanonicalPostHeader>> {
    let Ok(records) = query_replica_local_only(
        docs_sync,
        replica_id,
        DocQuery::Exact(source_key.to_string()),
    )
    .await
    else {
        return Ok(None);
    };
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    let header = serde_json::from_slice(&record.value)?;
    Ok(Some(header))
}

pub(crate) fn legacy_epoch_id() -> &'static str {
    "legacy"
}

pub(crate) fn private_channel_is_epoch_aware(audience_kind: &ChannelAudienceKind) -> bool {
    let _ = audience_kind;
    true
}

pub(crate) fn initial_private_channel_epoch_id(
    audience_kind: &ChannelAudienceKind,
    now_ms: i64,
    owner_pubkey: &str,
) -> String {
    let _ = audience_kind;
    format!("epoch-{now_ms}-{}", short_id_suffix(owner_pubkey))
}

pub(crate) fn next_private_channel_epoch_id(owner_pubkey: &str) -> String {
    format!(
        "epoch-{}-{}",
        Utc::now().timestamp_millis(),
        short_id_suffix(owner_pubkey)
    )
}

pub(crate) fn private_channel_replica_for_epoch(channel_id: &str, epoch_id: &str) -> ReplicaId {
    if epoch_id == legacy_epoch_id() {
        return private_channel_replica_id(channel_id);
    }
    private_channel_epoch_replica_id(channel_id, epoch_id)
}

pub(crate) fn current_private_channel_replica_id(state: &JoinedPrivateChannelState) -> ReplicaId {
    private_channel_replica_for_epoch(state.channel_id.as_str(), state.current_epoch_id.as_str())
}

pub(crate) fn private_channel_epoch_capabilities(
    state: &JoinedPrivateChannelState,
) -> Vec<PrivateChannelEpochCapability> {
    let mut items = vec![PrivateChannelEpochCapability {
        epoch_id: state.current_epoch_id.clone(),
        namespace_secret_hex: state.current_epoch_secret_hex.clone(),
    }];
    for epoch in &state.archived_epochs {
        if items.iter().any(|item| item.epoch_id == epoch.epoch_id) {
            continue;
        }
        items.push(epoch.clone());
    }
    items
}

pub(crate) fn joined_private_channel_state_from_capability(
    capability: PrivateChannelCapability,
) -> Result<JoinedPrivateChannelState> {
    let current_epoch_id = if capability.current_epoch_id.trim().is_empty() {
        legacy_epoch_id().to_string()
    } else {
        capability.current_epoch_id
    };
    let current_epoch_secret_hex = if capability.current_epoch_secret_hex.trim().is_empty() {
        capability.namespace_secret_hex.clone()
    } else {
        capability.current_epoch_secret_hex
    };
    if current_epoch_secret_hex.trim().is_empty() {
        anyhow::bail!("private channel capability is missing current epoch secret");
    }
    let owner_pubkey = if capability.owner_pubkey.trim().is_empty() {
        capability.creator_pubkey.clone()
    } else {
        capability.owner_pubkey
    };
    Ok(JoinedPrivateChannelState {
        topic_id: capability.topic_id,
        channel_id: ChannelId::new(capability.channel_id),
        label: capability.label.trim().to_string(),
        creator_pubkey: capability.creator_pubkey,
        owner_pubkey,
        joined_via_pubkey: capability.joined_via_pubkey,
        audience_kind: capability.audience_kind,
        current_epoch_id,
        current_epoch_secret_hex,
        archived_epochs: capability.archived_epochs,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn merged_private_channel_state_from_epoch_join(
    existing: Option<JoinedPrivateChannelState>,
    topic_id: &str,
    channel_id: ChannelId,
    label: &str,
    creator_pubkey: &str,
    owner_pubkey: &str,
    joined_via_pubkey: Option<&str>,
    audience_kind: ChannelAudienceKind,
    epoch_id: &str,
    namespace_secret_hex: &str,
) -> JoinedPrivateChannelState {
    let mut archived_epochs = existing
        .as_ref()
        .map(|state| state.archived_epochs.clone())
        .unwrap_or_default();
    archived_epochs.retain(|epoch| epoch.epoch_id != epoch_id);
    if let Some(existing_state) = existing.as_ref()
        && existing_state.current_epoch_id != epoch_id
        && !archived_epochs
            .iter()
            .any(|epoch| epoch.epoch_id == existing_state.current_epoch_id)
    {
        archived_epochs.push(PrivateChannelEpochCapability {
            epoch_id: existing_state.current_epoch_id.clone(),
            namespace_secret_hex: existing_state.current_epoch_secret_hex.clone(),
        });
    }
    JoinedPrivateChannelState {
        topic_id: topic_id.to_string(),
        channel_id,
        label: label.to_string(),
        creator_pubkey: creator_pubkey.to_string(),
        owner_pubkey: owner_pubkey.to_string(),
        joined_via_pubkey: joined_via_pubkey.map(str::to_string),
        audience_kind,
        current_epoch_id: epoch_id.to_string(),
        current_epoch_secret_hex: namespace_secret_hex.to_string(),
        archived_epochs,
    }
}

pub(crate) fn archive_private_channel_epoch(
    state: &mut JoinedPrivateChannelState,
    epoch_id: &str,
    namespace_secret_hex: &str,
) {
    if state
        .archived_epochs
        .iter()
        .any(|epoch| epoch.epoch_id == epoch_id)
    {
        return;
    }
    state.archived_epochs.push(PrivateChannelEpochCapability {
        epoch_id: epoch_id.to_string(),
        namespace_secret_hex: namespace_secret_hex.to_string(),
    });
}

pub(crate) fn active_private_channel_participants(
    participants: &[PrivateChannelParticipantDocV1],
    epoch_id: &str,
) -> Vec<PrivateChannelParticipantDocV1> {
    participants
        .iter()
        .filter(|participant| participant.epoch_id == epoch_id && participant.left_at.is_none())
        .cloned()
        .collect()
}
