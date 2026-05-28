use super::*;
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
