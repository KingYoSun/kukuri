use super::super::*;

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

pub(crate) async fn wait_for_topic_delivery(app: &AppService, topic: &str, expected: usize) {
    match timeout(social_graph_propagation_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = app.get_sync_status().await.expect("sync status");
            let ready = status.topic_diagnostics.iter().any(|entry| {
                let live_ready = entry.peer_count >= expected
                    && entry.connected_peers.len() >= expected.min(1)
                    && (entry.joined || matches!(entry.delivery_state, DeliveryState::Live));
                let durable_ready = !entry.docs_assist_peer_ids.is_empty()
                    && matches!(
                        entry.delivery_state,
                        DeliveryState::DurableRecovering | DeliveryState::DurableReady
                    );
                entry.topic == topic && (live_ready || durable_ready)
            });
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return;
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!("topic delivery timeout for {topic}; {snapshot}");
        }
    }
}

pub(crate) async fn warm_author_social_view(app: &AppService, author_pubkey: &str, topic: &str) {
    match timeout(social_graph_propagation_timeout(), async {
        loop {
            if app.get_author_social_view(author_pubkey).await.is_ok() {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!("author social view warmup timeout for {author_pubkey}; {snapshot}");
        }
    }
}

pub(crate) async fn wait_for_mutual_author_view(
    app: &AppService,
    author_pubkey: &str,
    topic: &str,
) {
    match timeout(social_graph_propagation_timeout(), async {
        loop {
            let view = app
                .get_author_social_view(author_pubkey)
                .await
                .expect("author social view");
            if view.mutual {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let social_view = app
                .get_author_social_view(author_pubkey)
                .await
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
                .unwrap_or_else(|_| "social_view=unavailable".to_string());
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!("mutual relationship timeout for {author_pubkey}; {social_view}, {snapshot}");
        }
    }
}

pub(crate) fn is_retryable_friend_only_grant_import_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<PrivateChannelImportError>()
        .is_some_and(PrivateChannelImportError::is_retryable_friend_only)
}

pub(crate) async fn wait_for_friend_only_grant_import(
    app: &AppService,
    token: &str,
    step_timeout: Duration,
) -> kukuri_core::FriendOnlyGrantPreview {
    match timeout(step_timeout, async {
        loop {
            match app.import_friend_only_grant(token).await {
                Ok(preview) => return preview,
                Err(error) if is_retryable_friend_only_grant_import_error(&error) => {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("friend-only grant import failed: {error:#}"),
            }
        }
    })
    .await
    {
        Ok(preview) => preview,
        Err(_) => {
            let preview =
                kukuri_core::parse_friend_only_grant_token(token).expect("parse grant token");
            let social_view = app
                .get_author_social_view(preview.owner_pubkey.as_str())
                .await
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
                .unwrap_or_else(|_| "social_view=unavailable".to_string());
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!(
                "friend-only grant import timeout for {}; {social_view}, {snapshot}",
                preview.owner_pubkey.as_str()
            );
        }
    }
}

pub(crate) fn is_retryable_friend_plus_share_import_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<PrivateChannelImportError>()
        .is_some_and(PrivateChannelImportError::is_retryable_friend_plus)
}

pub(crate) async fn wait_for_friend_plus_share_import(
    app: &AppService,
    token: &str,
    step_timeout: Duration,
) -> kukuri_core::FriendPlusSharePreview {
    let preview = kukuri_core::parse_friend_plus_share_token(token).expect("parse share token");
    let last_retryable_error = Arc::new(TokioMutex::new(None::<String>));
    let retry_error_slot = Arc::clone(&last_retryable_error);
    match timeout(step_timeout, async {
        loop {
            match app.import_friend_plus_share(token).await {
                Ok(preview) => return preview,
                Err(error) if is_retryable_friend_plus_share_import_error(&error) => {
                    *retry_error_slot.lock().await = Some(error.to_string());
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("friend-plus share import failed: {error:#}"),
            }
        }
    })
    .await
    {
        Ok(preview) => preview,
        Err(_) => {
            let last_error = last_retryable_error
                .lock()
                .await
                .clone()
                .unwrap_or_else(|| "none".to_string());
            let social_view = app
                .get_author_social_view(preview.sponsor_pubkey.as_str())
                .await
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
                .unwrap_or_else(|_| "social_view=unavailable".to_string());
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!(
                "friend-plus share import timeout; sponsor_pubkey={}, last_retryable_error={}, {social_view}, {snapshot}",
                preview.sponsor_pubkey.as_str(),
                last_error
            );
        }
    }
}

pub(crate) async fn wait_for_friend_plus_share_rejection(
    app: &AppService,
    token: &str,
    step_timeout: Duration,
) -> String {
    let preview = kukuri_core::parse_friend_plus_share_token(token).expect("parse share token");
    let last_retryable_error = Arc::new(TokioMutex::new(None::<String>));
    let retry_error_slot = Arc::clone(&last_retryable_error);
    match timeout(step_timeout, async {
        loop {
            let error = app
                .import_friend_plus_share(token)
                .await
                .expect_err("friend-plus share should be rejected");
            match error.downcast_ref::<PrivateChannelImportError>() {
                Some(PrivateChannelImportError::SharingClosed {
                    kind: PrivateChannelImportKind::FriendPlus,
                }) => return error.to_string(),
                Some(PrivateChannelImportError::SnapshotTimeout {
                    kind: PrivateChannelImportKind::FriendPlus,
                }) => {
                    *retry_error_slot.lock().await = Some(error.to_string());
                    sleep(Duration::from_millis(100)).await;
                    continue;
                }
                _ => panic!("unexpected friend-plus share rejection error: {error}"),
            }
        }
    })
    .await
    {
        Ok(message) => message,
        Err(_) => {
            let last_error = last_retryable_error
                .lock()
                .await
                .clone()
                .unwrap_or_else(|| "none".to_string());
            let social_view = app
                .get_author_social_view(preview.sponsor_pubkey.as_str())
                .await
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
                .unwrap_or_else(|_| "social_view=unavailable".to_string());
            let snapshot = app
                .get_sync_status()
                .await
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|_| "failed to read sync status".to_string());
            panic!(
                "friend-plus share rejection timeout; sponsor_pubkey={}, last_retryable_error={}, {social_view}, {snapshot}",
                preview.sponsor_pubkey.as_str(),
                last_error
            );
        }
    }
}

#[cfg(test)]
mod error_contract_tests {
    use crate::service::{PrivateChannelImportError, PrivateChannelImportKind};

    use super::{
        is_retryable_friend_only_grant_import_error, is_retryable_friend_plus_share_import_error,
    };

    #[test]
    fn friend_only_import_retry_contract_is_exactly_characterized() {
        for error in [
            PrivateChannelImportError::MutualRelationshipRequired {
                kind: PrivateChannelImportKind::FriendOnly,
            },
            PrivateChannelImportError::EpochMismatch {
                kind: PrivateChannelImportKind::FriendOnly,
            },
            PrivateChannelImportError::OwnerInactive {
                kind: PrivateChannelImportKind::FriendOnly,
            },
            PrivateChannelImportError::SnapshotTimeout {
                kind: PrivateChannelImportKind::FriendOnly,
            },
        ] {
            let message = error.to_string();
            assert!(
                is_retryable_friend_only_grant_import_error(&error.into()),
                "expected retryable: {message}"
            );
        }
        for error in [
            PrivateChannelImportError::Expired {
                kind: PrivateChannelImportKind::FriendOnly,
            },
            PrivateChannelImportError::AudienceMismatch {
                kind: PrivateChannelImportKind::FriendOnly,
            },
            PrivateChannelImportError::SharingClosed {
                kind: PrivateChannelImportKind::FriendOnly,
            },
        ] {
            let message = error.to_string();
            assert!(
                !is_retryable_friend_only_grant_import_error(&error.into()),
                "expected terminal: {message}"
            );
        }
        assert!(!is_retryable_friend_only_grant_import_error(
            &anyhow::anyhow!("unrecognized private channel access token")
        ));
    }

    #[test]
    fn friend_plus_import_retry_contract_is_exactly_characterized() {
        for error in [
            PrivateChannelImportError::MutualRelationshipRequired {
                kind: PrivateChannelImportKind::FriendPlus,
            },
            PrivateChannelImportError::SponsorInactive,
            PrivateChannelImportError::SponsorSnapshotTimeout,
            PrivateChannelImportError::SnapshotTimeout {
                kind: PrivateChannelImportKind::FriendPlus,
            },
        ] {
            let message = error.to_string();
            assert!(
                is_retryable_friend_plus_share_import_error(&error.into()),
                "expected retryable: {message}"
            );
        }
        for error in [
            PrivateChannelImportError::Expired {
                kind: PrivateChannelImportKind::FriendPlus,
            },
            PrivateChannelImportError::AudienceMismatch {
                kind: PrivateChannelImportKind::FriendPlus,
            },
            PrivateChannelImportError::SharingClosed {
                kind: PrivateChannelImportKind::FriendPlus,
            },
            PrivateChannelImportError::EpochMismatch {
                kind: PrivateChannelImportKind::FriendPlus,
            },
        ] {
            let message = error.to_string();
            assert!(
                !is_retryable_friend_plus_share_import_error(&error.into()),
                "expected terminal: {message}"
            );
        }
    }
}
