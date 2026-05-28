use crate::*;

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
