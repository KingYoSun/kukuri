use crate::*;

pub(crate) fn format_sync_snapshot(status: &SyncStatus, topic: &str) -> String {
    let topic_status = status
        .topic_diagnostics
        .iter()
        .find(|entry| entry.topic == topic)
        .map(|entry| {
            format!(
                "topic_peers={}, connected_peers={:?}, docs_assist_peer_ids={:?}, configured_peer_ids={:?}, delivery_state={:?}, status_detail={}",
                entry.peer_count,
                entry.connected_peers,
                entry.docs_assist_peer_ids,
                entry.configured_peer_ids,
                entry.delivery_state,
                entry.status_detail
            )
        })
        .unwrap_or_else(|| "topic_status=missing".to_string());
    format!(
        "connected={}, peer_count={}, status_detail={}, last_error={:?}, discovery_connected_peers={:?}, {}",
        status.connected,
        status.peer_count,
        status.status_detail,
        status.last_error,
        status.discovery.connected_peer_ids,
        topic_status
    )
}

pub(crate) async fn wait_for_timeline_object(
    runtime: &DesktopRuntime,
    topic: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    let _ = wait_for_timeline_object_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        object_id,
        step_timeout,
    )
    .await?;
    Ok(())
}

pub(crate) async fn wait_for_timeline_object_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    object_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::PostView> {
    timeout(step_timeout, async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(50),
                })
                .await?;
            if let Some(item) = timeline
                .items
                .into_iter()
                .find(|item| item.object_id == object_id)
            {
                return Ok::<kukuri_app_api::PostView, anyhow::Error>(item);
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("timeline assertion timeout")?
}

pub(crate) async fn wait_for_topic_doc_index_entry(
    runtime: &DesktopRuntime,
    topic: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            if runtime
                .has_topic_timeline_doc_index_entry(topic, object_id)
                .await
                .context("failed to query topic docs index")?
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .context("topic docs index assertion timeout")?
}

pub(crate) async fn assert_timeline_scope_excludes_object(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    object_id: &str,
    duration: Duration,
) -> Result<()> {
    let result = timeout(duration, async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(50),
                })
                .await?;
            if timeline
                .items
                .iter()
                .any(|item| item.object_id == object_id)
            {
                anyhow::bail!("object leaked into filtered timeline scope");
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    match result {
        Err(_) => Ok(()),
        Ok(inner) => inner,
    }
}

pub(crate) async fn wait_for_thread_object(
    runtime: &DesktopRuntime,
    topic: &str,
    thread_id: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let thread = runtime
                .list_thread(ListThreadRequest {
                    topic: topic.to_string(),
                    thread_id: thread_id.to_string(),
                    cursor: None,
                    limit: Some(50),
                })
                .await?;
            if thread.items.iter().any(|item| item.object_id == object_id) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("thread assertion timeout")?
}

pub(crate) async fn wait_for_topic_peer_count(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await?;
            let ready = status.topic_diagnostics.iter().any(|entry| {
                entry.topic == topic
                    && entry.joined
                    && entry.peer_count >= expected
                    && entry.connected_peers.len() >= expected.min(1)
            });
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return Ok::<(), anyhow::Error>(());
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!("topic connected-peer assertion timeout; {snapshot}");
        }
    }
}

pub(crate) async fn wait_for_topic_delivery(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await?;
            let ready = topic_has_direct_peer(&status, topic, expected)
                || topic_has_durable_delivery(&status, topic);
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return Ok::<(), anyhow::Error>(());
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!("topic delivery assertion timeout; {snapshot}");
        }
    }
}

pub(crate) fn ci_timeout_floor(step_timeout: Duration, floor: Duration) -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        step_timeout.max(floor)
    } else {
        step_timeout
    }
}

pub(crate) fn topic_has_direct_peer(status: &SyncStatus, topic: &str, expected: usize) -> bool {
    status.topic_diagnostics.iter().any(|entry| {
        entry.topic == topic
            && entry.peer_count >= expected
            && entry.connected_peers.len() >= expected.min(1)
            && (entry.joined || matches!(entry.delivery_state, kukuri_app_api::DeliveryState::Live))
    })
}

pub(crate) fn topic_has_durable_delivery(status: &SyncStatus, topic: &str) -> bool {
    status.topic_diagnostics.iter().any(|entry| {
        entry.topic == topic
            && !entry.docs_assist_peer_ids.is_empty()
            && matches!(
                entry.delivery_state,
                kukuri_app_api::DeliveryState::DurableRecovering
                    | kukuri_app_api::DeliveryState::DurableReady
            )
    })
}

#[cfg(test)]
pub(crate) fn topic_has_direct_peer_without_pending_join(
    status: &SyncStatus,
    topic: &str,
    expected: usize,
) -> bool {
    topic_has_direct_peer(status, topic, expected)
        && status
            .last_error
            .as_deref()
            .is_none_or(|error| !error.contains("topic join pending"))
}

pub(crate) fn should_publish_from_direct_connected_subscriber(
    publisher_status: &SyncStatus,
    subscriber_status: &SyncStatus,
    topic: &str,
    expected: usize,
    direction: PublicReplicationDirection,
) -> bool {
    matches!(
        direction,
        PublicReplicationDirection::PreferDirectConnectedSubscriber
    ) && !topic_has_direct_peer(publisher_status, topic, expected)
        && topic_has_direct_peer(subscriber_status, topic, expected)
}

pub(crate) fn should_retry_public_replication_from_subscriber(
    publisher_status: &SyncStatus,
    subscriber_status: &SyncStatus,
    topic: &str,
    expected: usize,
    direction: PublicReplicationDirection,
    attempt: usize,
) -> bool {
    if should_publish_from_direct_connected_subscriber(
        publisher_status,
        subscriber_status,
        topic,
        expected,
        direction,
    ) {
        return true;
    }
    matches!(
        direction,
        PublicReplicationDirection::PreferDirectConnectedSubscriber
    ) && attempt.is_multiple_of(2)
        && !topic_has_direct_peer(publisher_status, topic, expected)
        && !topic_has_direct_peer(subscriber_status, topic, expected)
}

pub(crate) struct PublicFeatureSelection {
    pub(crate) select_subscriber: bool,
    pub(crate) require_direct_subscriber: bool,
}

pub(crate) fn select_public_feature_strategy(
    publisher_status: &SyncStatus,
    subscriber_status: &SyncStatus,
    topic: &str,
    expected: usize,
    attempt: usize,
) -> PublicFeatureSelection {
    let select_subscriber = should_retry_public_replication_from_subscriber(
        publisher_status,
        subscriber_status,
        topic,
        expected,
        PublicReplicationDirection::PreferDirectConnectedSubscriber,
        attempt,
    );
    PublicFeatureSelection {
        select_subscriber,
        require_direct_subscriber: select_subscriber
            && topic_has_direct_peer(subscriber_status, topic, expected),
    }
}

pub(crate) async fn wait_for_direct_topic_peer_count(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await?;
            let ready = topic_has_direct_peer(&status, topic, expected);
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return Ok::<(), anyhow::Error>(());
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!("direct topic connected-peer assertion timeout; {snapshot}");
        }
    }
}

pub(crate) async fn wait_for_author_social_view(
    runtime: &DesktopRuntime,
    author_pubkey: &str,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        loop {
            if runtime
                .get_author_social_view(kukuri_desktop_runtime::AuthorRequest {
                    pubkey: author_pubkey.to_string(),
                })
                .await
                .is_ok()
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!("author social view warmup timeout for {author_pubkey}"),
    }
}

pub(crate) async fn wait_for_mutual_author_view_result(
    runtime: &DesktopRuntime,
    author_pubkey: &str,
    topic: &str,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        loop {
            let view = runtime
                .get_author_social_view(kukuri_desktop_runtime::AuthorRequest {
                    pubkey: author_pubkey.to_string(),
                })
                .await
                .context("author social view")?;
            if view.mutual {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let social_view = runtime
                .get_author_social_view(kukuri_desktop_runtime::AuthorRequest {
                    pubkey: author_pubkey.to_string(),
                })
                .await
                .ok()
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|| "social_view=unavailable".to_string());
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!(
                "mutual relationship timeout for {author_pubkey}; {social_view}, {snapshot}"
            );
        }
    }
}

pub(crate) async fn wait_for_direct_message_result(
    runtime: &DesktopRuntime,
    peer_pubkey: &str,
    message_id: &str,
    step_timeout: Duration,
) -> Result<DirectMessageMessageView> {
    match timeout(step_timeout, async {
        loop {
            let timeline = runtime
                .list_direct_message_messages(ListDirectMessageMessagesRequest {
                    pubkey: peer_pubkey.to_string(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("list direct message timeline")?;
            if let Some(message) = timeline
                .items
                .into_iter()
                .find(|item| item.message_id == message_id)
            {
                return Ok::<DirectMessageMessageView, anyhow::Error>(message);
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!("direct message delivery timeout for {message_id}"),
    }
}

pub(crate) async fn wait_for_direct_message_result_with_sender_refresh(
    sender_runtime: &DesktopRuntime,
    sender_peer_pubkey: &str,
    receiver_runtime: &DesktopRuntime,
    receiver_peer_pubkey: &str,
    message_id: &str,
    step_timeout: Duration,
) -> Result<DirectMessageMessageView> {
    match timeout(step_timeout, async {
        loop {
            let _ = sender_runtime
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: sender_peer_pubkey.to_string(),
                })
                .await
                .context("sender direct message status")?;
            let timeline = receiver_runtime
                .list_direct_message_messages(ListDirectMessageMessagesRequest {
                    pubkey: receiver_peer_pubkey.to_string(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("list direct message timeline")?;
            if let Some(message) = timeline
                .items
                .into_iter()
                .find(|item| item.message_id == message_id)
            {
                return Ok::<DirectMessageMessageView, anyhow::Error>(message);
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!("direct message delivery timeout for {message_id}"),
    }
}

pub(crate) struct DirectMessagePairRefreshContext<'a> {
    pub(crate) sender_runtime: &'a DesktopRuntime,
    pub(crate) sender_ticket: &'a str,
    pub(crate) sender_peer_pubkey: &'a str,
    pub(crate) receiver_runtime: &'a DesktopRuntime,
    pub(crate) receiver_ticket: &'a str,
    pub(crate) receiver_peer_pubkey: &'a str,
}

pub(crate) async fn wait_for_direct_message_result_with_pair_refresh(
    pair: DirectMessagePairRefreshContext<'_>,
    message_id: &str,
    step_timeout: Duration,
) -> Result<DirectMessageMessageView> {
    let refresh_interval = Duration::from_secs(5);
    match timeout(step_timeout, async {
        let mut next_refresh_at = Instant::now() + refresh_interval;
        loop {
            let _ = pair
                .sender_runtime
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: pair.sender_peer_pubkey.to_string(),
                })
                .await
                .context("sender direct message status")?;
            let timeline = pair
                .receiver_runtime
                .list_direct_message_messages(ListDirectMessageMessagesRequest {
                    pubkey: pair.receiver_peer_pubkey.to_string(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("list direct message timeline")?;
            if let Some(message) = timeline
                .items
                .into_iter()
                .find(|item| item.message_id == message_id)
            {
                return Ok::<DirectMessageMessageView, anyhow::Error>(message);
            }
            if Instant::now() >= next_refresh_at {
                refresh_direct_message_pair(
                    pair.sender_runtime,
                    pair.receiver_runtime,
                    pair.sender_ticket,
                    pair.receiver_ticket,
                    pair.sender_peer_pubkey,
                    pair.receiver_peer_pubkey,
                )
                .await
                .context("refresh direct message pair")?;
                next_refresh_at = Instant::now() + refresh_interval;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!("direct message delivery timeout for {message_id}"),
    }
}

pub(crate) async fn wait_for_direct_message_conversation_result(
    runtime: &DesktopRuntime,
    peer_pubkey: &str,
    message_id: &str,
    step_timeout: Duration,
) -> Result<DirectMessageConversationView> {
    match timeout(step_timeout, async {
        loop {
            let conversations = runtime
                .list_direct_messages()
                .await
                .context("list direct messages")?;
            if let Some(conversation) = conversations.into_iter().find(|item| {
                item.peer_pubkey == peer_pubkey
                    && item.last_message_id.as_deref() == Some(message_id)
            }) {
                return Ok::<DirectMessageConversationView, anyhow::Error>(conversation);
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!("direct message conversation timeout for {message_id}"),
    }
}

pub(crate) async fn wait_for_direct_message_absence(
    runtime: &DesktopRuntime,
    peer_pubkey: &str,
    message_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        loop {
            let timeline = runtime
                .list_direct_message_messages(ListDirectMessageMessagesRequest {
                    pubkey: peer_pubkey.to_string(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("list direct message timeline")?;
            if timeline
                .items
                .iter()
                .all(|item| item.message_id != message_id)
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!("direct message delete timeout for {message_id}"),
    }
}

pub(crate) async fn wait_for_direct_message_outbox_count(
    runtime: &DesktopRuntime,
    peer_pubkey: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<DirectMessageStatusView> {
    match timeout(step_timeout, async {
        loop {
            let status = runtime
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: peer_pubkey.to_string(),
                })
                .await
                .context("direct message status")?;
            if status.pending_outbox_count == expected {
                return Ok::<DirectMessageStatusView, anyhow::Error>(status);
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "direct message outbox count timeout for {peer_pubkey}; expected={expected}"
        ),
    }
}

pub(crate) async fn wait_for_direct_message_outbox_count_with_pair_refresh(
    pair: DirectMessagePairRefreshContext<'_>,
    expected: usize,
    step_timeout: Duration,
) -> Result<DirectMessageStatusView> {
    let refresh_interval = Duration::from_secs(5);
    match timeout(step_timeout, async {
        let mut next_refresh_at = Instant::now() + refresh_interval;
        loop {
            let status = pair
                .sender_runtime
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: pair.sender_peer_pubkey.to_string(),
                })
                .await
                .context("direct message status")?;
            if status.pending_outbox_count == expected {
                return Ok::<DirectMessageStatusView, anyhow::Error>(status);
            }
            if Instant::now() >= next_refresh_at {
                refresh_direct_message_pair(
                    pair.sender_runtime,
                    pair.receiver_runtime,
                    pair.sender_ticket,
                    pair.receiver_ticket,
                    pair.sender_peer_pubkey,
                    pair.receiver_peer_pubkey,
                )
                .await
                .context("refresh direct message pair")?;
                next_refresh_at = Instant::now() + refresh_interval;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "direct message outbox count timeout for {}; expected={expected}",
            pair.sender_peer_pubkey
        ),
    }
}

pub(crate) fn image_attachment_request(
    name: &str,
    mime: &str,
    bytes: &[u8],
) -> CreateAttachmentRequest {
    CreateAttachmentRequest {
        file_name: Some(name.to_string()),
        mime: mime.to_string(),
        byte_size: bytes.len() as u64,
        data_base64: BASE64_STANDARD.encode(bytes),
        role: Some("image_original".to_string()),
    }
}

pub(crate) fn video_attachment_request(
    name: &str,
    mime: &str,
    bytes: &[u8],
    role: &str,
) -> CreateAttachmentRequest {
    CreateAttachmentRequest {
        file_name: Some(name.to_string()),
        mime: mime.to_string(),
        byte_size: bytes.len() as u64,
        data_base64: BASE64_STANDARD.encode(bytes),
        role: Some(role.to_string()),
    }
}

#[cfg(test)]
fn is_retryable_friend_only_grant_import_error(message: &str) -> bool {
    message.contains("mutual relationship")
        || message.contains("friend-only grant epoch does not match the current policy")
        || message.contains("friend-only grant owner is not an active participant")
        || message.contains("timed out waiting for friend-only channel replica sync")
}

#[cfg(test)]
pub(crate) async fn wait_for_friend_only_grant_import(
    runtime: &DesktopRuntime,
    token: String,
    step_timeout: Duration,
) -> Result<kukuri_core::FriendOnlyGrantPreview> {
    let preview = kukuri_core::parse_friend_only_grant_token(token.as_str())?;
    match timeout(step_timeout, async {
        loop {
            match runtime
                .import_friend_only_grant(kukuri_desktop_runtime::ImportFriendOnlyGrantRequest {
                    token: token.clone(),
                })
                .await
            {
                Ok(preview) => return Ok::<_, anyhow::Error>(preview),
                Err(error)
                    if is_retryable_friend_only_grant_import_error(error.to_string().as_str()) =>
                {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => return Err(error),
            }
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let social_view = runtime
                .get_author_social_view(kukuri_desktop_runtime::AuthorRequest {
                    pubkey: preview.owner_pubkey.as_str().to_string(),
                })
                .await
                .ok()
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|| "social_view=unavailable".to_string());
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!(
                "friend-only grant import assertion timeout; owner_pubkey={}, {social_view}, {snapshot}",
                preview.owner_pubkey.as_str()
            );
        }
    }
}

#[cfg(test)]
pub(crate) async fn wait_for_friend_plus_share_import(
    runtime: &DesktopRuntime,
    token: String,
    step_timeout: Duration,
) -> Result<kukuri_core::FriendPlusSharePreview> {
    let preview = kukuri_core::parse_friend_plus_share_token(token.as_str())?;
    match timeout(step_timeout, async {
        loop {
            match runtime
                .import_friend_plus_share(kukuri_desktop_runtime::ImportFriendPlusShareRequest {
                    token: token.clone(),
                })
                .await
            {
                Ok(preview) => return Ok::<_, anyhow::Error>(preview),
                Err(error) if error.to_string().contains("mutual relationship") => {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => return Err(error),
            }
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let social_view = runtime
                .get_author_social_view(kukuri_desktop_runtime::AuthorRequest {
                    pubkey: preview.sponsor_pubkey.as_str().to_string(),
                })
                .await
                .ok()
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|| "social_view=unavailable".to_string());
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!(
                "friend-plus share import assertion timeout; sponsor_pubkey={}, {social_view}, {snapshot}",
                preview.sponsor_pubkey.as_str()
            );
        }
    }
}

pub(crate) async fn wait_for_joined_private_channel(
    runtime: &DesktopRuntime,
    topic: &str,
    channel_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let joined = runtime
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.to_string(),
                })
                .await?;
            if joined.iter().any(|entry| entry.channel_id == channel_id) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("joined private-channel assertion timeout")?
}

pub(crate) fn private_replication_retry_schedule(step_timeout: Duration) -> (usize, Duration) {
    let attempts = if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        3
    } else {
        1
    };
    let per_attempt_timeout = if attempts > 1 {
        Duration::from_millis(
            (step_timeout.as_millis() / attempts as u128)
                .max(1)
                .try_into()
                .expect("private replication timeout fits in u64"),
        )
    } else {
        step_timeout
    };
    (attempts, per_attempt_timeout)
}

pub(crate) fn public_replication_retry_schedule(
    step_timeout: Duration,
    same_author_shared_identity: bool,
) -> (usize, Duration) {
    let attempts = if std::env::var_os("GITHUB_ACTIONS").is_some() || same_author_shared_identity {
        3
    } else {
        1
    };
    let per_attempt_timeout = if attempts > 1 {
        Duration::from_millis(
            (step_timeout.as_millis() / attempts as u128)
                .max(1)
                .try_into()
                .expect("public replication timeout fits in u64"),
        )
    } else {
        step_timeout
    };
    (attempts, per_attempt_timeout)
}

pub(crate) struct PublicReplicationLabels<'a> {
    pub(crate) failure: &'a str,
    pub(crate) publisher: &'a str,
    pub(crate) subscriber: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublicReplicationDirection {
    PreferOriginalPublisher,
    PreferDirectConnectedSubscriber,
}

pub(crate) async fn replicate_public_post_with_retry(
    publisher: &DesktopRuntime,
    subscriber: &DesktopRuntime,
    topic: &str,
    content_prefix: &str,
    step_timeout: Duration,
    direction: PublicReplicationDirection,
    labels: PublicReplicationLabels<'_>,
) -> Result<String> {
    let same_author_shared_identity = publisher
        .get_sync_status()
        .await
        .ok()
        .zip(subscriber.get_sync_status().await.ok())
        .is_some_and(|(publisher_status, subscriber_status)| {
            publisher_status.local_author_pubkey == subscriber_status.local_author_pubkey
        });
    let (attempts, attempt_timeout) =
        public_replication_retry_schedule(step_timeout, same_author_shared_identity);
    let mut last_error = None;
    let mut last_direction = None;

    for attempt in 1..=attempts {
        let attempt_result = async {
            let _ = publisher
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("failed to resubscribe publisher to public topic")?;
            let _ = subscriber
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("failed to resubscribe subscriber to public topic")?;
            wait_for_topic_delivery(publisher, topic, 1, attempt_timeout)
                .await
                .context("publisher did not observe public topic delivery readiness")?;
            wait_for_topic_delivery(subscriber, topic, 1, attempt_timeout)
                .await
                .context("subscriber did not observe public topic delivery readiness")?;
            let publisher_status = publisher
                .get_sync_status()
                .await
                .context("publisher sync status")?;
            let subscriber_status = subscriber
                .get_sync_status()
                .await
                .context("subscriber sync status")?;
            let publish_from_subscriber = should_retry_public_replication_from_subscriber(
                &publisher_status,
                &subscriber_status,
                topic,
                1,
                direction,
                attempt,
            );
            let (active_publisher, active_subscriber, publisher_label, subscriber_label) =
                if publish_from_subscriber {
                    (subscriber, publisher, labels.subscriber, labels.publisher)
                } else {
                    (publisher, subscriber, labels.publisher, labels.subscriber)
                };
            if publish_from_subscriber {
                wait_for_direct_topic_peer_count(active_publisher, topic, 1, attempt_timeout)
                    .await
                    .with_context(|| {
                        format!(
                            "{} did not observe direct public topic connectivity",
                            publisher_label
                        )
                    })?;
            }
            last_direction = Some(format!("publish {publisher_label}->{subscriber_label}"));
            let post_id = active_publisher
                .create_post(CreatePostRequest {
                    topic: topic.to_string(),
                    content: format!("{content_prefix} #{attempt}"),
                    reply_to: None,
                    channel_ref: ChannelRef::Public,
                    attachments: Vec::new(),
                })
                .await
                .with_context(|| format!("failed to create public post on {publisher_label}"))?;
            wait_for_topic_doc_index_entry(
                active_publisher,
                topic,
                post_id.as_str(),
                attempt_timeout,
            )
            .await
            .context("publisher did not persist public post into docs index")?;
            wait_for_timeline_object(active_subscriber, topic, post_id.as_str(), attempt_timeout)
                .await
                .context("timeline assertion timeout")?;
            Ok::<String, anyhow::Error>(post_id)
        }
        .await;

        match attempt_result {
            Ok(post_id) => return Ok(post_id),
            Err(error) if attempt < attempts => {
                last_error = Some(format!("{error:#}"));
                sleep(Duration::from_millis(250)).await;
            }
            Err(error) => {
                last_error = Some(format!("{error:#}"));
                break;
            }
        }
    }

    let publisher_status = publisher
        .get_sync_status()
        .await
        .ok()
        .map(|status| format_sync_snapshot(&status, topic))
        .unwrap_or_else(|| format!("failed to read {} sync status", labels.publisher));
    let subscriber_status = subscriber
        .get_sync_status()
        .await
        .ok()
        .map(|status| format_sync_snapshot(&status, topic))
        .unwrap_or_else(|| format!("failed to read {} sync status", labels.subscriber));
    Err(anyhow::anyhow!(
        "{}",
        last_error
            .unwrap_or_else(|| { format!("unknown replication failure for {}", labels.failure) })
    )
    .context(format!(
        "{} did not receive the {}; direction={}; {}=({publisher_status}); {}=({subscriber_status})",
        labels.subscriber,
        labels.failure,
        last_direction.unwrap_or_else(|| "unknown".to_string()),
        labels.publisher,
        labels.subscriber
    )))
}

pub(crate) async fn refresh_private_channel_pair(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    ticket_a: &str,
    ticket_b: &str,
    topic: &str,
    private_scope: &TimelineScope,
) -> Result<()> {
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.to_string(),
        })
        .await?;
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.to_string(),
        })
        .await?;
    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await;
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await;
    Ok(())
}

pub(crate) async fn refresh_public_pair(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    topic: &str,
    step_timeout: Duration,
) -> Result<()> {
    async fn refresh_public_runtime(runtime: &DesktopRuntime, topic: &str) {
        let _ = runtime
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await;
        let _ = runtime
            .list_live_sessions(ListLiveSessionsRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
            })
            .await;
        let _ = runtime
            .list_game_rooms(ListGameRoomsRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
            })
            .await;
        if let Ok(statuses) = runtime.get_community_node_statuses().await {
            for node in statuses {
                if node.auth_state.authenticated {
                    let _ = runtime
                        .refresh_community_node_metadata(CommunityNodeTargetRequest {
                            base_url: node.base_url,
                        })
                        .await;
                    }
            }
        }
    }

    fn public_connectivity_reapply_interval() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(20)
        } else {
            Duration::from_secs(10)
        }
    }

    async fn force_public_runtime_connectivity(runtime: &DesktopRuntime) {
        let _ = runtime.reapply_community_node_connectivity().await;
    }

    let refresh_interval = Duration::from_secs(5);
    let reapply_interval = public_connectivity_reapply_interval();
    match timeout(step_timeout, async {
        let mut next_refresh_at = Instant::now();
        let mut next_reapply_at = Instant::now() + reapply_interval;
        let mut stable_ready_polls = 0usize;
        loop {
            if Instant::now() >= next_refresh_at {
                refresh_public_runtime(runtime_a, topic).await;
                refresh_public_runtime(runtime_b, topic).await;
                next_refresh_at = Instant::now() + refresh_interval;
            }
            if Instant::now() >= next_reapply_at {
                force_public_runtime_connectivity(runtime_a).await;
                force_public_runtime_connectivity(runtime_b).await;
                next_reapply_at = Instant::now() + reapply_interval;
            }

            let status_a = runtime_a
                .get_sync_status()
                .await
                .context("desktop a public sync status")?;
            let status_b = runtime_b
                .get_sync_status()
                .await
                .context("desktop b public sync status")?;
            let ready_a = topic_has_direct_peer(&status_a, topic, 1)
                || topic_has_durable_delivery(&status_a, topic);
            let ready_b = topic_has_direct_peer(&status_b, topic, 1)
                || topic_has_durable_delivery(&status_b, topic);
            if ready_a && ready_b {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return Ok::<(), anyhow::Error>(());
                }
            } else {
                stable_ready_polls = 0;
            }

            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let snapshot_a = runtime_a
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read desktop a sync status".to_string());
            let snapshot_b = runtime_b
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read desktop b sync status".to_string());
            anyhow::bail!(
                "public pair refresh timeout; desktop_a=({snapshot_a}); desktop_b=({snapshot_b})"
            );
        }
    }
}

pub(crate) async fn refresh_direct_message_pair(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    ticket_a: &str,
    ticket_b: &str,
    a_pubkey: &str,
    b_pubkey: &str,
) -> Result<()> {
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.to_string(),
        })
        .await?;
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.to_string(),
        })
        .await?;
    match runtime_a
        .open_direct_message(DirectMessageRequest {
            pubkey: b_pubkey.to_string(),
        })
        .await
    {
        Ok(_) => {}
        Err(error)
            if is_retryable_direct_message_pair_refresh_error(error.to_string().as_str()) => {}
        Err(error) => return Err(error),
    }
    match runtime_b
        .open_direct_message(DirectMessageRequest {
            pubkey: a_pubkey.to_string(),
        })
        .await
    {
        Ok(_) => {}
        Err(error)
            if is_retryable_direct_message_pair_refresh_error(error.to_string().as_str()) => {}
        Err(error) => return Err(error),
    }
    Ok(())
}

pub(crate) fn is_retryable_direct_message_pair_refresh_error(message: &str) -> bool {
    message.contains("mutual relationship")
}

pub(crate) async fn wait_for_direct_message_pair_ready_with_refresh(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    ticket_a: &str,
    ticket_b: &str,
    a_pubkey: &str,
    b_pubkey: &str,
    step_timeout: Duration,
) -> Result<()> {
    let refresh_interval = Duration::from_secs(5);
    match timeout(step_timeout, async {
        let mut next_refresh_at = Instant::now() + refresh_interval;
        loop {
            let status_a = runtime_a
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: b_pubkey.to_string(),
                })
                .await
                .context("desktop a direct message status")?;
            let topic_a = runtime_a
                .get_direct_message_topic_status(DirectMessageRequest {
                    pubkey: b_pubkey.to_string(),
                })
                .await
                .context("desktop a direct message topic status")?;
            let status_b = runtime_b
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: a_pubkey.to_string(),
                })
                .await
                .context("desktop b direct message status")?;
            let topic_b = runtime_b
                .get_direct_message_topic_status(DirectMessageRequest {
                    pubkey: a_pubkey.to_string(),
                })
                .await
                .context("desktop b direct message topic status")?;
            let ready_a = status_a.send_enabled
                && topic_a.as_ref().is_some_and(|topic_status| {
                    topic_status.joined && topic_status.peer_count >= 1
                });
            let ready_b = status_b.send_enabled
                && topic_b.as_ref().is_some_and(|topic_status| {
                    topic_status.joined && topic_status.peer_count >= 1
                });
            if ready_a && ready_b {
                return Ok::<(), anyhow::Error>(());
            }
            if Instant::now() >= next_refresh_at {
                refresh_direct_message_pair(
                    runtime_a, runtime_b, ticket_a, ticket_b, a_pubkey, b_pubkey,
                )
                .await
                .context("refresh direct message pair")?;
                next_refresh_at = Instant::now() + refresh_interval;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let snapshot_a = runtime_a
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: b_pubkey.to_string(),
                })
                .await
                .ok()
                .map(|status| {
                    format!(
                        "send_enabled={}, mutual={}, peer_count={}, pending_outbox_count={}",
                        status.send_enabled,
                        status.mutual,
                        status.peer_count,
                        status.pending_outbox_count
                    )
                })
                .unwrap_or_else(|| "direct_message_status=unavailable".to_string());
            let topic_snapshot_a = runtime_a
                .get_direct_message_topic_status(DirectMessageRequest {
                    pubkey: b_pubkey.to_string(),
                })
                .await
                .ok()
                .flatten()
                .map(|topic_status| {
                    format!(
                        "topic={}, joined={}, topic_peer_count={}, connected_peers={:?}, status_detail={}, last_error={:?}",
                        topic_status.topic,
                        topic_status.joined,
                        topic_status.peer_count,
                        topic_status.connected_peers,
                        topic_status.status_detail,
                        topic_status.last_error
                    )
                })
                .unwrap_or_else(|| "direct_message_topic=unavailable".to_string());
            let snapshot_b = runtime_b
                .get_direct_message_status(DirectMessageRequest {
                    pubkey: a_pubkey.to_string(),
                })
                .await
                .ok()
                .map(|status| {
                    format!(
                        "send_enabled={}, mutual={}, peer_count={}, pending_outbox_count={}",
                        status.send_enabled,
                        status.mutual,
                        status.peer_count,
                        status.pending_outbox_count
                    )
                })
                .unwrap_or_else(|| "direct_message_status=unavailable".to_string());
            let topic_snapshot_b = runtime_b
                .get_direct_message_topic_status(DirectMessageRequest {
                    pubkey: a_pubkey.to_string(),
                })
                .await
                .ok()
                .flatten()
                .map(|topic_status| {
                    format!(
                        "topic={}, joined={}, topic_peer_count={}, connected_peers={:?}, status_detail={}, last_error={:?}",
                        topic_status.topic,
                        topic_status.joined,
                        topic_status.peer_count,
                        topic_status.connected_peers,
                        topic_status.status_detail,
                        topic_status.last_error
                    )
                })
                .unwrap_or_else(|| "direct_message_topic=unavailable".to_string());
            anyhow::bail!(
                "direct message pair readiness timeout; desktop_a=({snapshot_a}; {topic_snapshot_a}); desktop_b=({snapshot_b}; {topic_snapshot_b})"
            );
        }
    }
}

pub(crate) async fn select_public_feature_pair<'a>(
    runtime_a: &'a DesktopRuntime,
    runtime_b: &'a DesktopRuntime,
    topic: &str,
    step_timeout: Duration,
    attempt: usize,
) -> Result<(
    &'a DesktopRuntime,
    &'a DesktopRuntime,
    &'static str,
    &'static str,
)> {
    refresh_public_pair(runtime_a, runtime_b, topic, step_timeout).await?;
    let direct_pair_timeout = ci_timeout_floor(step_timeout, Duration::from_secs(60));
    let _ =
        timeout(direct_pair_timeout, async {
            loop {
                let publisher_status = runtime_a.get_sync_status().await.context(
                    "desktop a sync status while waiting for public feature connectivity",
                )?;
                let subscriber_status = runtime_b.get_sync_status().await.context(
                    "desktop b sync status while waiting for public feature connectivity",
                )?;
                if topic_has_direct_peer(&publisher_status, topic, 1)
                    && topic_has_direct_peer(&subscriber_status, topic, 1)
                {
                    return Ok::<(), anyhow::Error>(());
                }
                refresh_public_pair(runtime_a, runtime_b, topic, direct_pair_timeout).await?;
                sleep(Duration::from_millis(250)).await;
            }
        })
        .await;
    let publisher_status = runtime_a
        .get_sync_status()
        .await
        .context("desktop a sync status for public feature selection")?;
    let subscriber_status = runtime_b
        .get_sync_status()
        .await
        .context("desktop b sync status for public feature selection")?;
    let strategy =
        select_public_feature_strategy(&publisher_status, &subscriber_status, topic, 1, attempt);
    if strategy.select_subscriber {
        if strategy.require_direct_subscriber {
            wait_for_direct_topic_peer_count(runtime_b, topic, 1, step_timeout)
                .await
                .context("desktop b did not observe direct public topic connectivity")?;
        }
        Ok((runtime_b, runtime_a, "desktop b", "desktop a"))
    } else {
        if topic_has_direct_peer(&publisher_status, topic, 1) {
            wait_for_direct_topic_peer_count(runtime_a, topic, 1, step_timeout)
                .await
                .context("desktop a did not observe direct public topic connectivity")?;
        }
        Ok((runtime_a, runtime_b, "desktop a", "desktop b"))
    }
}

pub(crate) async fn wait_for_live_session(
    runtime: &DesktopRuntime,
    topic: &str,
    session_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    let _ = wait_for_live_session_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        session_id,
        step_timeout,
    )
    .await?;
    Ok(())
}

pub(crate) async fn wait_for_live_session_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::LiveSessionView> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if let Some(session) = sessions
                .into_iter()
                .find(|session| session.session_id == session_id)
            {
                return Ok::<kukuri_app_api::LiveSessionView, anyhow::Error>(session);
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("live-session assertion timeout")?
}

pub(crate) async fn wait_for_live_viewer_count(
    runtime: &DesktopRuntime,
    topic: &str,
    session_id: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    wait_for_live_viewer_count_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        session_id,
        expected,
        step_timeout,
    )
    .await
}

pub(crate) async fn wait_for_live_viewer_count_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if sessions
                .iter()
                .any(|session| session.session_id == session_id && session.viewer_count == expected)
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("live-session viewer assertion timeout")?
}

pub(crate) async fn wait_for_live_ended(
    runtime: &DesktopRuntime,
    topic: &str,
    session_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    wait_for_live_ended_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        session_id,
        step_timeout,
    )
    .await
}

pub(crate) async fn wait_for_live_ended_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if sessions.iter().any(|session| {
                session.session_id == session_id
                    && session.status == kukuri_core::LiveSessionStatus::Ended
            }) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("live-session ended assertion timeout")?
}

pub(crate) async fn assert_live_session_absent_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    duration: Duration,
) -> Result<()> {
    let result = timeout(duration, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if sessions
                .iter()
                .any(|session| session.session_id == session_id)
            {
                anyhow::bail!("live session leaked into filtered scope");
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    match result {
        Err(_) => Ok(()),
        Ok(inner) => inner,
    }
}

pub(crate) async fn wait_for_game_room(
    runtime: &DesktopRuntime,
    topic: &str,
    room_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::GameRoomView> {
    wait_for_game_room_in_scope(runtime, topic, TimelineScope::Public, room_id, step_timeout).await
}

pub(crate) async fn wait_for_game_room_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    room_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::GameRoomView> {
    timeout(step_timeout, async {
        loop {
            let rooms = runtime
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if let Some(room) = rooms.into_iter().find(|room| room.room_id == room_id) {
                return Ok::<kukuri_app_api::GameRoomView, anyhow::Error>(room);
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("game-room assertion timeout")?
}

pub(crate) async fn wait_for_game_score(
    runtime: &DesktopRuntime,
    topic: &str,
    room_id: &str,
    label: &str,
    expected: i64,
    step_timeout: Duration,
) -> Result<()> {
    wait_for_game_score_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        room_id,
        label,
        expected,
        step_timeout,
    )
    .await
}

pub(crate) async fn wait_for_game_score_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    room_id: &str,
    label: &str,
    expected: i64,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let rooms = runtime
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if rooms.iter().any(|room| {
                room.room_id == room_id
                    && room
                        .scores
                        .iter()
                        .any(|score| score.label == label && score.score == expected)
            }) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("game-score assertion timeout")?
}

pub(crate) async fn assert_game_room_absent_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    room_id: &str,
    duration: Duration,
) -> Result<()> {
    let result = timeout(duration, async {
        loop {
            let rooms = runtime
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if rooms.iter().any(|room| room.room_id == room_id) {
                anyhow::bail!("game room leaked into filtered scope");
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    match result {
        Err(_) => Ok(()),
        Ok(inner) => inner,
    }
}

pub(crate) fn step_name(step: &ScenarioStep) -> &'static str {
    match step {
        ScenarioStep::LaunchDesktop => "launch_desktop",
        ScenarioStep::SelectTopic { .. } => "select_topic",
        ScenarioStep::SelectPublicTimeline => "select_public_timeline",
        ScenarioStep::CreatePrivateChannel { .. } => "create_private_channel",
        ScenarioStep::SelectPrivateChannel { .. } => "select_private_channel",
        ScenarioStep::CreatePost { .. } => "create_post",
        ScenarioStep::AssertTimelineContains { .. } => "assert_timeline_contains",
        ScenarioStep::BookmarkPost { .. } => "bookmark_post",
        ScenarioStep::AssertBookmarkListContains { .. } => "assert_bookmark_list_contains",
        ScenarioStep::AssertBookmarkListMissing { .. } => "assert_bookmark_list_missing",
        ScenarioStep::RemoveBookmark { .. } => "remove_bookmark",
        ScenarioStep::CreateLiveSession { .. } => "create_live_session",
        ScenarioStep::JoinLiveSession { .. } => "join_live_session",
        ScenarioStep::AssertLiveViewerCount { .. } => "assert_live_viewer_count",
        ScenarioStep::EndLiveSession { .. } => "end_live_session",
        ScenarioStep::CreateGameRoom { .. } => "create_game_room",
        ScenarioStep::UpdateGameRoom { .. } => "update_game_room",
        ScenarioStep::AssertGameScore { .. } => "assert_game_score",
        ScenarioStep::RestartDesktop => "restart_desktop",
    }
}

pub(crate) fn parse_game_status(value: &str) -> Result<GameRoomStatus> {
    match value {
        "Open" | "Waiting" => Ok(GameRoomStatus::Waiting),
        "InProgress" | "Running" => Ok(GameRoomStatus::Running),
        "Paused" => Ok(GameRoomStatus::Paused),
        "Finished" | "Ended" => Ok(GameRoomStatus::Ended),
        _ => anyhow::bail!("unsupported game room status: {value}"),
    }
}
