use super::*;
use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    routing::{get, post},
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use futures_util::StreamExt;
use image::{
    AnimationDecoder, Delay, DynamicImage, Frame, GenericImageView, ImageDecoder, ImageFormat,
    Rgba, RgbaImage,
};
use iroh::address_lookup::{AddrFilter, AddressLookup};
use iroh_mainline_address_lookup::DhtAddressLookup;
use kukuri_app_api::{GameScoreView, JoinedPrivateChannelView, SyncStatus, TimelineView};
use kukuri_cn_core::{
    BootstrapHeartbeatResponse, CommunityNodeConsentStatus, CommunityNodeResolvedUrls,
    CommunityNodeSeedPeer,
};
use kukuri_core::{
    AssetRole, ChannelAudienceKind, ChannelRef, GameRoomStatus, KukuriKeys, TimelineScope,
};
use kukuri_docs_sync::{DocQuery, DocsSync};
use kukuri_transport::{
    ConnectMode, DhtDiscoveryOptions, DiscoveryMode, SeedPeer, TransportNetworkConfig,
};
use n0_mainline::{DhtBuilder, Testnet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::{Duration, sleep, timeout};

use crate::attachments::{normalize_custom_reaction_gif, normalize_custom_reaction_static};
use crate::community_node::{
    BootstrapNodesResponse, StoredCommunityNodeToken, default_preview_community_node_config,
    load_community_node_config_from_file, normalize_community_node_config,
    persist_community_node_token, relay_config_from_community_node_config,
    save_community_node_config,
};
use crate::discovery::resolve_discovery_config_from_env;
use crate::identity::IdentityStorageMode;
use crate::paths::{community_node_config_path, discovery_config_path};

fn social_graph_propagation_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(30)
    }
}

fn seeded_dht_runtime_ready_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(120)
    } else {
        Duration::from_secs(20)
    }
}

fn runtime_replication_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(30)
    }
}

fn runtime_shutdown_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(15)
    }
}

async fn acquire_async_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().await
}

fn png_source_bytes() -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(320, 180, Rgba([0, 179, 164, 255])));
    let mut out = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut out, ImageFormat::Png)
        .expect("encode png");
    out.into_inner()
}

fn animated_gif_source_bytes() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut out);
        let frames = vec![
            Frame::from_parts(
                RgbaImage::from_pixel(4, 2, Rgba([255, 0, 0, 255])),
                0,
                0,
                Delay::from_numer_denom_ms(40, 1),
            ),
            Frame::from_parts(
                RgbaImage::from_pixel(4, 2, Rgba([0, 0, 255, 255])),
                0,
                0,
                Delay::from_numer_denom_ms(40, 1),
            ),
        ];
        encoder.encode_frames(frames).expect("encode gif");
    }
    out.into_inner()
}

fn format_sync_snapshot(status: &SyncStatus, topic: &str) -> String {
    let topic_status = status
            .topic_diagnostics
            .iter()
            .find(|entry| entry.topic == topic)
            .map(|entry| {
                format!(
                    "topic_peers={}, connected_peers={:?}, docs_assist_peer_ids={:?}, configured_peer_ids={:?}, missing_peer_ids={:?}, delivery_state={:?}, status_detail={}",
                    entry.peer_count,
                    entry.connected_peers,
                    entry.docs_assist_peer_ids,
                    entry.configured_peer_ids,
                    entry.missing_peer_ids,
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

async fn wait_for_connected_topic_peer_count(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    timeout_label: &str,
) {
    match timeout(runtime_replication_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await.expect("sync status");
            let ready = topic_has_direct_peer(&status, topic, expected);
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return;
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let status = runtime.get_sync_status().await.expect("sync status");
            panic!("{timeout_label}: {}", format_sync_snapshot(&status, topic));
        }
    }
}

async fn wait_for_topic_delivery(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    timeout_label: &str,
) {
    match timeout(runtime_replication_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await.expect("sync status");
            let ready = topic_has_delivery(&status, topic, expected);
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return;
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let status = runtime.get_sync_status().await.expect("sync status");
            panic!("{timeout_label}: {}", format_sync_snapshot(&status, topic));
        }
    }
}

async fn wait_for_topic_delivery_result(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await.context("sync status")?;
            let ready = topic_has_delivery(&status, topic, expected);
            if ready {
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
            let status = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            bail!("topic delivery timeout; {status}");
        }
    }
}

fn topic_has_direct_peer(status: &SyncStatus, topic: &str, expected: usize) -> bool {
    status.topic_diagnostics.iter().any(|topic_status| {
        topic_status.topic == topic
            && topic_status.connected_peers.len() >= expected.min(1)
            && topic_status.peer_count >= expected
            && (topic_status.joined
                || matches!(
                    topic_status.delivery_state,
                    kukuri_app_api::DeliveryState::Live
                ))
    })
}

fn topic_has_delivery(status: &SyncStatus, topic: &str, expected: usize) -> bool {
    topic_has_direct_peer(status, topic, expected) || topic_has_durable_delivery(status, topic)
}

fn should_swap_shared_identity_public_replication_direction(
    publisher_status: &SyncStatus,
    subscriber_status: &SyncStatus,
    topic: &str,
    expected: usize,
) -> bool {
    !topic_has_direct_peer(publisher_status, topic, expected)
        && topic_has_direct_peer(subscriber_status, topic, expected)
}

async fn wait_for_direct_topic_peer_count_result(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await.context("sync status")?;
            let ready = topic_has_direct_peer(&status, topic, expected);
            if ready {
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
            let status = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            bail!("direct topic readiness timeout; {status}");
        }
    }
}

async fn wait_for_mutual_author_view(runtime: &DesktopRuntime, author_pubkey: &str, topic: &str) {
    match timeout(social_graph_propagation_timeout(), async {
        loop {
            let view = runtime
                .get_author_social_view(AuthorRequest {
                    pubkey: author_pubkey.to_string(),
                })
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
            let social_view = runtime
                    .get_author_social_view(AuthorRequest {
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
            let status = runtime.get_sync_status().await.expect("sync status");
            panic!(
                "mutual author view timeout for {author_pubkey}; {social_view}; {}",
                format_sync_snapshot(&status, topic)
            );
        }
    }
}

async fn warm_author_social_view(
    runtime: &DesktopRuntime,
    author_pubkey: &str,
    timeout_label: &str,
) {
    match timeout(social_graph_propagation_timeout(), async {
        loop {
            if runtime
                .get_author_social_view(AuthorRequest {
                    pubkey: author_pubkey.to_string(),
                })
                .await
                .is_ok()
            {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let status = runtime.get_sync_status().await.expect("sync status");
            panic!("{timeout_label}: {}", format_sync_snapshot(&status, ""));
        }
    }
}

fn is_retryable_friend_plus_share_import_error(message: &str) -> bool {
    message.contains("mutual relationship")
        || message.contains("sponsor is not an active participant")
        || message.contains("timed out waiting for friend-plus sponsor participant sync")
        || message.contains("timed out waiting for friend-plus channel replica sync")
}

fn is_retryable_friend_only_grant_import_error(message: &str) -> bool {
    message.contains("mutual relationship")
        || message.contains("friend-only grant epoch does not match the current policy")
        || message.contains("friend-only grant owner is not an active participant")
        || message.contains("timed out waiting for friend-only channel replica sync")
}

async fn wait_for_friend_only_grant_import(
    runtime: &DesktopRuntime,
    token: &str,
    step_timeout: Duration,
    timeout_label: &str,
) -> kukuri_core::FriendOnlyGrantPreview {
    let preview = kukuri_core::parse_friend_only_grant_token(token).expect("parse grant token");
    let last_retryable_error = Arc::new(Mutex::new(None::<String>));
    let retry_error_slot = Arc::clone(&last_retryable_error);
    match timeout(step_timeout, async {
        loop {
            match runtime
                .import_friend_only_grant(ImportFriendOnlyGrantRequest {
                    token: token.to_string(),
                })
                .await
            {
                Ok(preview) => return preview,
                Err(error)
                    if is_retryable_friend_only_grant_import_error(error.to_string().as_str()) =>
                {
                    *retry_error_slot.lock().await = Some(error.to_string());
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("{timeout_label}: {error:#}"),
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
            let social_view = runtime
                .get_author_social_view(AuthorRequest {
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
            let status = runtime.get_sync_status().await.expect("sync status");
            panic!(
                "{timeout_label}: owner_pubkey={}, last_retryable_error={}, {social_view}, {}",
                preview.owner_pubkey.as_str(),
                last_error,
                format_sync_snapshot(&status, preview.topic_id.as_str())
            );
        }
    }
}

async fn wait_for_friend_plus_share_import(
    runtime: &DesktopRuntime,
    token: &str,
    step_timeout: Duration,
    timeout_label: &str,
) -> kukuri_core::FriendPlusSharePreview {
    let preview = kukuri_core::parse_friend_plus_share_token(token).expect("parse share token");
    let last_retryable_error = Arc::new(Mutex::new(None::<String>));
    let retry_error_slot = Arc::clone(&last_retryable_error);
    match timeout(step_timeout, async {
        loop {
            match runtime
                .import_friend_plus_share(ImportFriendPlusShareRequest {
                    token: token.to_string(),
                })
                .await
            {
                Ok(preview) => return preview,
                Err(error)
                    if is_retryable_friend_plus_share_import_error(error.to_string().as_str()) =>
                {
                    *retry_error_slot.lock().await = Some(error.to_string());
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("{timeout_label}: {error:#}"),
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
            let social_view = runtime
                .get_author_social_view(AuthorRequest {
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
            let status = runtime.get_sync_status().await.expect("sync status");
            panic!(
                "{timeout_label}: sponsor_pubkey={}, last_retryable_error={}, {social_view}, {}",
                preview.sponsor_pubkey.as_str(),
                last_error,
                format_sync_snapshot(&status, preview.topic_id.as_str())
            );
        }
    }
}

async fn wait_for_topic_doc_index_entry_result(
    runtime: &DesktopRuntime,
    topic: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
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
    {
        Ok(result) => result,
        Err(_) => {
            let status = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            bail!("topic docs index timeout; {status}");
        }
    }
}

async fn wait_for_timeline_post(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: &TimelineScope,
    object_id: &str,
    timeout_label: &str,
) {
    match timeout(runtime_replication_timeout(), async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("timeline");
            if timeline
                .items
                .iter()
                .any(|post| post.object_id == object_id)
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let status = runtime.get_sync_status().await.expect("sync status");
            let private_items = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .ok()
                .map(|timeline| {
                    timeline
                        .items
                        .into_iter()
                        .map(|post| post.object_id)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            panic!(
                "{timeout_label}: {}; private_items={private_items:?}",
                format_sync_snapshot(&status, topic)
            );
        }
    }
}

async fn wait_for_timeline_post_result(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: &TimelineScope,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("timeline query failed")?;
            if timeline
                .items
                .iter()
                .any(|post| post.object_id == object_id)
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let status = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            bail!("timeline visibility timeout; {status}");
        }
    }
}

fn topic_has_durable_delivery(status: &SyncStatus, topic: &str) -> bool {
    status.topic_diagnostics.iter().any(|topic_status| {
        topic_status.topic == topic
            && !topic_status.docs_assist_peer_ids.is_empty()
            && matches!(
                topic_status.delivery_state,
                kukuri_app_api::DeliveryState::DurableRecovering
                    | kukuri_app_api::DeliveryState::DurableReady
            )
    })
}

async fn refresh_public_runtime_for_retry(runtime: &DesktopRuntime, topic: &str) -> Result<()> {
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
    Ok(())
}

fn public_connectivity_reapply_interval() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(20)
    } else {
        Duration::from_secs(10)
    }
}

async fn force_public_runtime_connectivity_retry(runtime: &DesktopRuntime) -> Result<()> {
    runtime
        .reapply_community_node_connectivity()
        .await
        .context("reapply community-node connectivity during public retry")?;
    Ok(())
}

async fn wait_for_public_runtime_delivery_with_refresh(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    let refresh_interval = Duration::from_secs(5);
    let reapply_interval = public_connectivity_reapply_interval();
    match timeout(step_timeout, async {
        let mut next_refresh_at = tokio::time::Instant::now();
        let mut next_reapply_at = tokio::time::Instant::now() + reapply_interval;
        let mut stable_ready_polls = 0usize;
        loop {
            if tokio::time::Instant::now() >= next_refresh_at {
                refresh_public_runtime_for_retry(runtime, topic).await?;
                next_refresh_at = tokio::time::Instant::now() + refresh_interval;
            }
            if tokio::time::Instant::now() >= next_reapply_at {
                force_public_runtime_connectivity_retry(runtime).await?;
                next_reapply_at = tokio::time::Instant::now() + reapply_interval;
            }

            let status = runtime
                .get_sync_status()
                .await
                .context("runtime sync status")?;
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

            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let status = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read runtime sync status".to_string());
            bail!("public runtime delivery timeout; {status}");
        }
    }
}

async fn wait_for_public_pair_delivery_with_refresh(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    let refresh_interval = Duration::from_secs(5);
    let reapply_interval = public_connectivity_reapply_interval();
    match timeout(step_timeout, async {
        let mut next_refresh_at = tokio::time::Instant::now();
        let mut next_reapply_at = tokio::time::Instant::now() + reapply_interval;
        let mut stable_ready_polls = 0usize;
        loop {
            if tokio::time::Instant::now() >= next_refresh_at {
                refresh_public_runtime_for_retry(runtime_a, topic).await?;
                refresh_public_runtime_for_retry(runtime_b, topic).await?;
                next_refresh_at = tokio::time::Instant::now() + refresh_interval;
            }
            if tokio::time::Instant::now() >= next_reapply_at {
                force_public_runtime_connectivity_retry(runtime_a).await?;
                force_public_runtime_connectivity_retry(runtime_b).await?;
                next_reapply_at = tokio::time::Instant::now() + reapply_interval;
            }

            let status_a = runtime_a
                .get_sync_status()
                .await
                .context("runtime a sync status")?;
            let status_b = runtime_b
                .get_sync_status()
                .await
                .context("runtime b sync status")?;
            let ready_a = topic_has_direct_peer(&status_a, topic, expected)
                || topic_has_durable_delivery(&status_a, topic);
            let ready_b = topic_has_direct_peer(&status_b, topic, expected)
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
            let status_a = runtime_a
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read runtime a sync status".to_string());
            let status_b = runtime_b
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read runtime b sync status".to_string());
            bail!("public pair delivery timeout; runtime_a=({status_a}); runtime_b=({status_b})");
        }
    }
}

async fn apply_relay_backed_community_node_seed_peers(
    runtime: &DesktopRuntime,
    base_url: &str,
    relay_url: &str,
    seed_peers: Vec<CommunityNodeSeedPeer>,
) {
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.to_string(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url, vec![relay_url.to_string()], seed_peers)
                    .expect("resolved urls"),
            ),
        }],
    };
    timeout(
        Duration::from_secs(30),
        runtime.apply_runtime_connectivity_assist(),
    )
    .await
    .expect("apply assist timeout")
    .expect("apply assist");
    timeout(
        Duration::from_secs(15),
        runtime.apply_effective_seed_peers(),
    )
    .await
    .expect("apply seed peers timeout")
    .expect("apply seed peers");
}

async fn wait_for_joined_private_channel_epoch_result(
    runtime: &DesktopRuntime,
    topic: &str,
    channel_id: &str,
    expected_epoch_id: &str,
    min_participant_count: usize,
    step_timeout: Duration,
) -> Result<JoinedPrivateChannelView> {
    match timeout(step_timeout, async {
        let private_scope = TimelineScope::Channel {
            channel_id: kukuri_core::ChannelId::new(channel_id.to_string()),
        };
        loop {
            let _ = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await;
            let joined = runtime
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.into(),
                })
                .await
                .context("joined channels query failed")?;
            let Some(entry) = joined.iter().find(|item| item.channel_id == channel_id) else {
                sleep(Duration::from_millis(50)).await;
                continue;
            };
            let _ = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await;
            if entry.current_epoch_id == expected_epoch_id
                && entry.participant_count >= min_participant_count
            {
                return Ok::<JoinedPrivateChannelView, anyhow::Error>(entry.clone());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let status = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|value| format_sync_snapshot(&value, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            let joined = runtime
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.into(),
                })
                .await
                .unwrap_or_default();
            bail!("joined private channel epoch timeout; {status}; joined={joined:?}");
        }
    }
}

async fn joined_private_channel_epoch_result(
    runtime: &DesktopRuntime,
    topic: &str,
    channel_id: &str,
) -> Result<Option<JoinedPrivateChannelView>> {
    let joined = runtime
        .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
            topic: topic.into(),
        })
        .await
        .context("joined channels query failed")?;
    Ok(joined
        .into_iter()
        .find(|entry| entry.channel_id == channel_id))
}

async fn wait_for_profile_timeline_posts_result(
    runtime: &DesktopRuntime,
    author_pubkey: &str,
    object_ids: &[String],
    timeout_label: &str,
) -> Result<TimelineView> {
    match timeout(runtime_replication_timeout(), async {
        loop {
            let timeline = runtime
                .list_profile_timeline(ListProfileTimelineRequest {
                    pubkey: author_pubkey.to_string(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("profile timeline");
            if object_ids.iter().all(|object_id| {
                timeline
                    .items
                    .iter()
                    .any(|post| post.object_id == *object_id)
            }) {
                return timeline;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(timeline) => Ok(timeline),
        Err(_) => {
            let status = runtime.get_sync_status().await.expect("sync status");
            let visible_items = runtime
                .list_profile_timeline(ListProfileTimelineRequest {
                    pubkey: author_pubkey.to_string(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .ok()
                .map(|timeline| {
                    timeline
                        .items
                        .into_iter()
                        .map(|post| format!("{}@{:?}", post.object_id, post.origin_topic_id))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            bail!(
                "{timeout_label}: {}; visible_items={visible_items:?}",
                format_sync_snapshot(&status, "")
            );
        }
    }
}

fn public_replication_retry_schedule(
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

async fn topic_timeline_doc_index_rows(runtime: &DesktopRuntime, topic: &str) -> Vec<String> {
    let replica = kukuri_docs_sync::topic_replica_id(topic);
    let current = runtime.iroh_stack.current.lock().await;
    let docs_sync = current.as_ref().expect("current stack").docs_sync.clone();
    drop(current);
    docs_sync
        .query_replica(&replica, DocQuery::Prefix("indexes/timeline/".into()))
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.key)
        .collect()
}

async fn replicate_public_post_with_retry(
    publisher: &DesktopRuntime,
    subscriber: &DesktopRuntime,
    topic: &str,
    content_prefix: &str,
    timeout_label: &str,
) -> String {
    replicate_public_post_with_retry_inner(
        publisher,
        subscriber,
        topic,
        content_prefix,
        timeout_label,
        true,
    )
    .await
}

async fn replicate_public_post_with_retry_inner(
    publisher: &DesktopRuntime,
    subscriber: &DesktopRuntime,
    topic: &str,
    content_prefix: &str,
    timeout_label: &str,
    allow_shared_identity_swap: bool,
) -> String {
    let same_author_shared_identity = publisher
        .get_sync_status()
        .await
        .ok()
        .zip(subscriber.get_sync_status().await.ok())
        .is_some_and(|(publisher_status, subscriber_status)| {
            publisher_status.local_author_pubkey == subscriber_status.local_author_pubkey
        });
    let (attempts, attempt_timeout) = public_replication_retry_schedule(
        runtime_replication_timeout(),
        same_author_shared_identity,
    );
    let scope = TimelineScope::Public;
    let mut last_error = None;

    for attempt in 1..=attempts {
        let attempt_result = async {
            let _ = publisher
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("failed to resubscribe publisher to public topic")?;
            let _ = subscriber
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("failed to resubscribe subscriber to public topic")?;
            let publisher_status = publisher
                .get_sync_status()
                .await
                .context("publisher sync status")?;
            let subscriber_status = subscriber
                .get_sync_status()
                .await
                .context("subscriber sync status")?;
            let publish_from_subscriber = allow_shared_identity_swap
                && same_author_shared_identity
                && should_swap_shared_identity_public_replication_direction(
                    &publisher_status,
                    &subscriber_status,
                    topic,
                    1,
                );
            let (active_publisher, active_subscriber) = if publish_from_subscriber {
                (subscriber, publisher)
            } else {
                (publisher, subscriber)
            };
            if publish_from_subscriber {
                wait_for_direct_topic_peer_count_result(
                    active_publisher,
                    topic,
                    1,
                    attempt_timeout,
                )
                .await
                .context("publishing runtime did not observe direct public topic connectivity")?;
            }
            let object_id = active_publisher
                .create_post(CreatePostRequest {
                    topic: topic.to_string(),
                    content: format!("{content_prefix} #{attempt}"),
                    reply_to: None,
                    channel_ref: ChannelRef::Public,
                    attachments: Vec::new(),
                })
                .await
                .context("failed to create public post")?;
            wait_for_topic_doc_index_entry_result(
                active_publisher,
                topic,
                object_id.as_str(),
                attempt_timeout,
            )
            .await
            .context("publisher did not persist public post into docs index")?;
            wait_for_timeline_post_result(
                active_subscriber,
                topic,
                &scope,
                object_id.as_str(),
                attempt_timeout,
            )
            .await
            .context("subscriber did not observe replicated public post")?;
            Ok::<String, anyhow::Error>(object_id)
        }
        .await;

        match attempt_result {
            Ok(object_id) => return object_id,
            Err(error) if attempt < attempts => {
                last_error = Some(format!("{error:#}"));
                if let Err(refresh_error) = wait_for_public_pair_delivery_with_refresh(
                    publisher,
                    subscriber,
                    topic,
                    1,
                    attempt_timeout,
                )
                .await
                {
                    last_error = Some(format!(
                        "{:#}; public topic refresh failed after replication timeout: {refresh_error:#}",
                        error
                    ));
                    break;
                }
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
        .expect("publisher sync status");
    let subscriber_status = subscriber
        .get_sync_status()
        .await
        .expect("subscriber sync status");
    let publisher_docs_rows = topic_timeline_doc_index_rows(publisher, topic).await;
    let subscriber_docs_rows = topic_timeline_doc_index_rows(subscriber, topic).await;
    panic!(
        "{timeout_label}; last_error={last_error:?}; publisher=({}); subscriber=({}); publisher_docs_rows={publisher_docs_rows:?}; subscriber_docs_rows={subscriber_docs_rows:?}",
        format_sync_snapshot(&publisher_status, topic),
        format_sync_snapshot(&subscriber_status, topic),
    );
}

async fn replicate_private_post_with_retry(
    publisher: &DesktopRuntime,
    subscribers: &[&DesktopRuntime],
    topic: &str,
    scope: &TimelineScope,
    channel_ref: &ChannelRef,
    content_prefix: &str,
    timeout_label: &str,
) -> String {
    let (attempts, attempt_timeout) =
        public_replication_retry_schedule(runtime_replication_timeout(), false);
    let channel_id = match scope {
        TimelineScope::Channel { channel_id } => channel_id.as_str().to_string(),
        TimelineScope::Public | TimelineScope::AllJoined => {
            panic!("replicate_private_post_with_retry requires a private channel scope")
        }
    };
    let mut last_error = None;

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
            let _ = publisher
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("failed to resubscribe publisher to private topic")?;
            let _ = publisher
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.to_string(),
                })
                .await
                .context("failed to refresh publisher joined private channels")?;
            wait_for_topic_delivery_result(publisher, topic, 1, attempt_timeout)
                .await
                .context("publisher did not observe private topic delivery readiness")?;
            for subscriber in subscribers {
                let _ = subscriber
                    .list_timeline(ListTimelineRequest {
                        topic: topic.to_string(),
                        scope: TimelineScope::Public,
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .context("failed to resubscribe subscriber to public topic")?;
                let _ = subscriber
                    .list_timeline(ListTimelineRequest {
                        topic: topic.to_string(),
                        scope: scope.clone(),
                        cursor: None,
                        limit: Some(20),
                    })
                    .await
                    .context("failed to resubscribe subscriber to private topic")?;
                let _ = subscriber
                    .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                        topic: topic.to_string(),
                    })
                    .await
                    .context("failed to refresh subscriber joined private channels")?;
                wait_for_topic_delivery_result(subscriber, topic, 1, attempt_timeout)
                    .await
                    .context("subscriber did not observe private topic delivery readiness")?;
            }
            let pre_write_epoch =
                joined_private_channel_epoch_result(publisher, topic, channel_id.as_str())
                    .await
                    .context("failed to read publisher private channel state before write")?
                    .map(|entry| entry.current_epoch_id);
            let object_id = publisher
                .create_post(CreatePostRequest {
                    topic: topic.to_string(),
                    content: format!("{content_prefix} #{attempt}"),
                    reply_to: None,
                    channel_ref: channel_ref.clone(),
                    attachments: Vec::new(),
                })
                .await
                .context("failed to create private post")?;
            wait_for_timeline_post_result(
                publisher,
                topic,
                scope,
                object_id.as_str(),
                attempt_timeout,
            )
            .await
            .context("publisher did not observe private post locally")?;
            let post_write_epoch =
                joined_private_channel_epoch_result(publisher, topic, channel_id.as_str())
                    .await
                    .context("failed to read publisher private channel state after write")?
                    .ok_or_else(|| anyhow::anyhow!("publisher lost private channel after write"))?
                    .current_epoch_id;
            if pre_write_epoch.as_deref() != Some(post_write_epoch.as_str()) {
                let mut runtimes = Vec::with_capacity(subscribers.len() + 1);
                runtimes.push(publisher);
                runtimes.extend(subscribers.iter().copied());
                refresh_runtime_peer_tickets(&runtimes)
                    .await
                    .context("failed to refresh peer tickets after private channel rotation")?;
                for runtime in &runtimes {
                    let _ = runtime
                        .list_timeline(ListTimelineRequest {
                            topic: topic.to_string(),
                            scope: TimelineScope::Public,
                            cursor: None,
                            limit: Some(20),
                        })
                        .await
                        .context("failed to refresh public topic after private channel rotation")?;
                    let _ = runtime
                        .list_timeline(ListTimelineRequest {
                            topic: topic.to_string(),
                            scope: scope.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await
                        .context(
                            "failed to refresh private topic after private channel rotation",
                        )?;
                    let _ = runtime
                        .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                            topic: topic.to_string(),
                        })
                        .await
                        .context(
                            "failed to refresh joined private channels after private channel rotation",
                        )?;
                }
                for subscriber in subscribers {
                    wait_for_joined_private_channel_epoch_result(
                        subscriber,
                        topic,
                        channel_id.as_str(),
                        post_write_epoch.as_str(),
                        1,
                        attempt_timeout,
                    )
                    .await
                    .context("subscriber did not redeem private channel rotation after write")?;
                }
            }
            for subscriber in subscribers {
                wait_for_timeline_post_result(
                    subscriber,
                    topic,
                    scope,
                    object_id.as_str(),
                    attempt_timeout,
                )
                .await
                .context("subscriber did not observe replicated private post")?;
            }
            Ok::<String, anyhow::Error>(object_id)
        }
        .await;

        match attempt_result {
            Ok(object_id) => return object_id,
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
        .expect("publisher sync status");
    let mut subscriber_details = Vec::with_capacity(subscribers.len());
    for (index, subscriber) in subscribers.iter().enumerate() {
        let status = subscriber
            .get_sync_status()
            .await
            .expect("subscriber sync status");
        let visible_items = subscriber
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .ok()
            .map(|timeline| {
                timeline
                    .items
                    .into_iter()
                    .map(|post| post.object_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        subscriber_details.push(format!(
            "subscriber[{index}] {} visible_items={visible_items:?}",
            format_sync_snapshot(&status, topic)
        ));
    }
    panic!(
        "{timeout_label}; last_error={last_error:?}; publisher=({}); {}",
        format_sync_snapshot(&publisher_status, topic),
        subscriber_details.join(" | "),
    );
}

async fn refresh_runtime_peer_tickets(runtimes: &[&DesktopRuntime]) -> Result<()> {
    let mut tickets = Vec::with_capacity(runtimes.len());
    for (index, runtime) in runtimes.iter().enumerate() {
        let ticket = runtime
            .local_peer_ticket()
            .await
            .with_context(|| format!("failed to load local peer ticket for runtime[{index}]"))?
            .ok_or_else(|| {
                anyhow::anyhow!("runtime[{index}] did not expose a local peer ticket")
            })?;
        tickets.push(ticket);
    }
    for (runtime_index, runtime) in runtimes.iter().enumerate() {
        for (ticket_index, ticket) in tickets.iter().enumerate() {
            if runtime_index == ticket_index {
                continue;
            }
            runtime
                .import_peer_ticket(ImportPeerTicketRequest {
                    ticket: ticket.clone(),
                })
                .await
                .with_context(|| {
                    format!(
                        "failed to import peer ticket from runtime[{ticket_index}] into runtime[{runtime_index}]"
                    )
                })?;
        }
    }
    Ok(())
}

fn sync_status_with_topic(
    topic: &str,
    connected_peers: &[&str],
    docs_assist_peer_ids: &[&str],
) -> SyncStatus {
    let connected = !connected_peers.is_empty();
    let delivery_state = if connected {
        kukuri_app_api::DeliveryState::Live
    } else if !docs_assist_peer_ids.is_empty() {
        kukuri_app_api::DeliveryState::DurableRecovering
    } else {
        kukuri_app_api::DeliveryState::Offline
    };
    SyncStatus {
        connected,
        delivery_state,
        last_sync_ts: None,
        peer_count: connected_peers.len(),
        pending_events: 0,
        status_detail: "test".to_string(),
        last_error: None,
        configured_peers: Vec::new(),
        subscribed_topics: vec![topic.to_string()],
        active_path: Default::default(),
        fallback_peer_ids: Vec::new(),
        topic_diagnostics: vec![kukuri_app_api::TopicSyncStatus {
            topic: topic.to_string(),
            joined: connected,
            delivery_state,
            peer_count: connected_peers.len(),
            connected_peers: connected_peers
                .iter()
                .map(|peer| peer.to_string())
                .collect(),
            docs_assist_peer_ids: docs_assist_peer_ids
                .iter()
                .map(|peer| peer.to_string())
                .collect(),
            configured_peer_ids: Vec::new(),
            missing_peer_ids: Vec::new(),
            active_path: Default::default(),
            rendezvous_peer_ids: Vec::new(),
            fallback_peer_ids: Vec::new(),
            last_received_at: None,
            last_docs_activity_at: None,
            status_detail: "test".to_string(),
            last_error: None,
        }],
        local_author_pubkey: "author".to_string(),
        discovery: Default::default(),
        gossip_disabled_topics: Vec::new(),
        gossip_disabled_channels: Vec::new(),
    }
}

async fn wait_for_joined_private_channel_epoch(
    runtime: &DesktopRuntime,
    topic: &str,
    channel_id: &str,
    expected_epoch_id: &str,
    min_participant_count: usize,
    timeout_label: &str,
) -> JoinedPrivateChannelView {
    match wait_for_joined_private_channel_epoch_result(
        runtime,
        topic,
        channel_id,
        expected_epoch_id,
        min_participant_count,
        runtime_replication_timeout(),
    )
    .await
    {
        Ok(entry) => entry,
        Err(error) => panic!("{timeout_label}: {error:#}"),
    }
}

async fn wait_for_seeded_dht_topic_ready(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    topic: &str,
) {
    match timeout(seeded_dht_runtime_ready_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status_a = runtime_a.get_sync_status().await.expect("status a");
            let status_b = runtime_b.get_sync_status().await.expect("status b");
            let ready_a = topic_has_direct_peer(&status_a, topic, 1)
                || topic_has_durable_delivery(&status_a, topic);
            let ready_b = topic_has_direct_peer(&status_b, topic, 1)
                || topic_has_durable_delivery(&status_b, topic);
            if ready_a && ready_b {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return;
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(()) => {}
        Err(_) => {
            let status_a = runtime_a.get_sync_status().await.expect("status a");
            let status_b = runtime_b.get_sync_status().await.expect("status b");
            panic!(
                "seeded dht topic readiness timeout for `{topic}`: status_a={status_a:?} status_b={status_b:?}"
            );
        }
    }
}

fn image_attachment_request(name: &str, mime: &str, bytes: &[u8]) -> CreateAttachmentRequest {
    CreateAttachmentRequest {
        file_name: Some(name.to_string()),
        mime: mime.to_string(),
        byte_size: bytes.len() as u64,
        data_base64: BASE64_STANDARD.encode(bytes),
        role: Some("image_original".to_string()),
    }
}

fn profile_avatar_attachment_request(
    name: &str,
    mime: &str,
    bytes: &[u8],
) -> CreateAttachmentRequest {
    CreateAttachmentRequest {
        file_name: Some(name.to_string()),
        mime: mime.to_string(),
        byte_size: bytes.len() as u64,
        data_base64: BASE64_STANDARD.encode(bytes),
        role: Some("profile_avatar".to_string()),
    }
}

fn video_attachment_request(
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

fn delete_sqlite_artifacts(db_path: &Path) {
    for path in [
        db_path.to_path_buf(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
    ] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("delete sqlite artifact {}: {error}", path.display()),
        }
    }
}

async fn wait_for_runtime_endpoint_in_testnet(runtime: &DesktopRuntime, testnet: &Testnet) {
    let endpoint = runtime.iroh_stack.endpoint().await;
    let mut builder = DhtBuilder::default();
    builder.bootstrap(&testnet.bootstrap);
    let lookup = DhtAddressLookup::builder()
        .dht_builder(builder)
        .no_publish()
        .addr_filter(AddrFilter::unfiltered())
        .build()
        .expect("dht lookup");
    timeout(Duration::from_secs(30), async {
        loop {
            if let Some(mut resolved) = lookup.resolve(endpoint.id())
                && let Some(Ok(item)) = resolved.next().await
                && item.endpoint_info().endpoint_id == endpoint.id()
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("resolve published endpoint info");
}

fn seeded_dht_config(seed_peers: Vec<SeedPeer>) -> DiscoveryConfig {
    DiscoveryConfig {
        mode: DiscoveryMode::SeededDht,
        connect_mode: ConnectMode::DirectOnly,
        env_locked: false,
        seed_peers,
    }
}

#[derive(Clone)]
struct MockCommunityNodeState {
    base_url: String,
    seed_peers: Arc<Mutex<Vec<CommunityNodeSeedPeer>>>,
    heartbeat_seed_peers: Arc<Mutex<Option<Vec<CommunityNodeSeedPeer>>>>,
    heartbeat_hits: Arc<AtomicUsize>,
    bootstrap_hits: Arc<AtomicUsize>,
}

#[derive(Clone)]
struct MockHeartbeatEchoCommunityNodeState {
    base_url: String,
    connectivity_urls: Vec<String>,
    seed_peers: Arc<Mutex<Vec<CommunityNodeSeedPeer>>>,
    heartbeat_hits: Arc<AtomicUsize>,
    bootstrap_hits: Arc<AtomicUsize>,
}

async fn mock_bootstrap_heartbeat(
    State(state): State<Arc<MockCommunityNodeState>>,
    Json(_request): Json<serde_json::Value>,
) -> Json<BootstrapHeartbeatResponse> {
    state.heartbeat_hits.fetch_add(1, Ordering::SeqCst);
    if let Some(seed_peers) = state.heartbeat_seed_peers.lock().await.take() {
        *state.seed_peers.lock().await = seed_peers;
    }
    Json(BootstrapHeartbeatResponse {
        expires_at: Utc::now().timestamp() + 300,
    })
}

async fn mock_bootstrap_nodes(
    State(state): State<Arc<MockCommunityNodeState>>,
) -> Json<BootstrapNodesResponse> {
    state.bootstrap_hits.fetch_add(1, Ordering::SeqCst);
    let seed_peers = state.seed_peers.lock().await.clone();
    Json(BootstrapNodesResponse {
        nodes: vec![kukuri_cn_core::CommunityNodeBootstrapNode {
            base_url: state.base_url.clone(),
            resolved_urls: CommunityNodeResolvedUrls::new(
                state.base_url.clone(),
                Vec::new(),
                seed_peers,
            )
            .expect("resolved urls"),
        }],
    })
}

async fn mock_bootstrap_consent_status() -> Json<CommunityNodeConsentStatus> {
    Json(managed_community_node_consent_status(true))
}

async fn mock_heartbeat_echo_bootstrap_heartbeat(
    State(state): State<Arc<MockHeartbeatEchoCommunityNodeState>>,
    Json(request): Json<serde_json::Value>,
) -> Json<BootstrapHeartbeatResponse> {
    state.heartbeat_hits.fetch_add(1, Ordering::SeqCst);
    let endpoint_id = request
        .get("endpoint_id")
        .and_then(serde_json::Value::as_str)
        .expect("heartbeat endpoint id")
        .to_string();
    let addr_hint = request
        .get("addr_hint")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    *state.seed_peers.lock().await =
        vec![CommunityNodeSeedPeer::new(endpoint_id, addr_hint).expect("heartbeat seed peer")];
    Json(BootstrapHeartbeatResponse {
        expires_at: Utc::now().timestamp() + 300,
    })
}

async fn mock_heartbeat_echo_bootstrap_nodes(
    State(state): State<Arc<MockHeartbeatEchoCommunityNodeState>>,
) -> Json<BootstrapNodesResponse> {
    state.bootstrap_hits.fetch_add(1, Ordering::SeqCst);
    let seed_peers = state.seed_peers.lock().await.clone();
    Json(BootstrapNodesResponse {
        nodes: vec![kukuri_cn_core::CommunityNodeBootstrapNode {
            base_url: state.base_url.clone(),
            resolved_urls: CommunityNodeResolvedUrls::new(
                state.base_url.clone(),
                state.connectivity_urls.clone(),
                seed_peers,
            )
            .expect("resolved urls"),
        }],
    })
}

#[derive(Clone)]
struct MockManagedCommunityNodeState {
    base_url: String,
    seed_peers: Vec<CommunityNodeSeedPeer>,
    consent_accepted: Arc<AtomicBool>,
    current_token: Arc<Mutex<String>>,
    challenge_hits: Arc<AtomicUsize>,
    verify_hits: Arc<AtomicUsize>,
    consent_status_hits: Arc<AtomicUsize>,
    consent_accept_hits: Arc<AtomicUsize>,
    heartbeat_hits: Arc<AtomicUsize>,
    bootstrap_hits: Arc<AtomicUsize>,
}

fn managed_community_node_consent_status(accepted: bool) -> CommunityNodeConsentStatus {
    CommunityNodeConsentStatus {
        all_required_accepted: accepted,
        items: vec![kukuri_cn_core::CommunityNodeConsentItem {
            policy_slug: "builder-preview".into(),
            policy_version: 1,
            title: "Builder Preview".into(),
            required: true,
            accepted_at: accepted.then(|| Utc::now().timestamp()),
        }],
    }
}

async fn authorize_managed_community_node_request(
    headers: &HeaderMap,
    state: &MockManagedCommunityNodeState,
) -> std::result::Result<(), StatusCode> {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Ok(value) = value.to_str() else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let current_token = state.current_token.lock().await.clone();
    if token == current_token {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn mock_managed_auth_challenge(
    State(state): State<Arc<MockManagedCommunityNodeState>>,
    Json(_request): Json<serde_json::Value>,
) -> Json<kukuri_cn_core::AuthChallengeResponse> {
    state.challenge_hits.fetch_add(1, Ordering::SeqCst);
    Json(kukuri_cn_core::AuthChallengeResponse {
        challenge: format!("challenge-{}", state.challenge_hits.load(Ordering::SeqCst)),
        expires_at: Utc::now().timestamp() + 300,
    })
}

async fn mock_managed_auth_verify(
    State(state): State<Arc<MockManagedCommunityNodeState>>,
    Json(_request): Json<serde_json::Value>,
) -> Json<kukuri_cn_core::AuthVerifyResponse> {
    let next = state.verify_hits.fetch_add(1, Ordering::SeqCst) + 1;
    let token = format!("managed-token-{next}");
    *state.current_token.lock().await = token.clone();
    Json(kukuri_cn_core::AuthVerifyResponse {
        access_token: token,
        token_type: "Bearer".into(),
        expires_at: Utc::now().timestamp() + 3600,
        pubkey: "f".repeat(64),
    })
}

async fn mock_managed_consent_status(
    State(state): State<Arc<MockManagedCommunityNodeState>>,
    headers: HeaderMap,
) -> std::result::Result<Json<CommunityNodeConsentStatus>, StatusCode> {
    authorize_managed_community_node_request(&headers, state.as_ref()).await?;
    state.consent_status_hits.fetch_add(1, Ordering::SeqCst);
    Ok(Json(managed_community_node_consent_status(
        state.consent_accepted.load(Ordering::SeqCst),
    )))
}

async fn mock_managed_accept_consents(
    State(state): State<Arc<MockManagedCommunityNodeState>>,
    headers: HeaderMap,
    Json(_request): Json<serde_json::Value>,
) -> std::result::Result<Json<CommunityNodeConsentStatus>, StatusCode> {
    authorize_managed_community_node_request(&headers, state.as_ref()).await?;
    state.consent_accept_hits.fetch_add(1, Ordering::SeqCst);
    state.consent_accepted.store(true, Ordering::SeqCst);
    Ok(Json(managed_community_node_consent_status(true)))
}

async fn mock_managed_bootstrap_heartbeat(
    State(state): State<Arc<MockManagedCommunityNodeState>>,
    headers: HeaderMap,
    Json(_request): Json<serde_json::Value>,
) -> std::result::Result<Json<BootstrapHeartbeatResponse>, StatusCode> {
    authorize_managed_community_node_request(&headers, state.as_ref()).await?;
    if !state.consent_accepted.load(Ordering::SeqCst) {
        return Err(StatusCode::FORBIDDEN);
    }
    state.heartbeat_hits.fetch_add(1, Ordering::SeqCst);
    Ok(Json(BootstrapHeartbeatResponse {
        expires_at: Utc::now().timestamp() + 300,
    }))
}

async fn mock_managed_bootstrap_nodes(
    State(state): State<Arc<MockManagedCommunityNodeState>>,
    headers: HeaderMap,
) -> std::result::Result<Json<BootstrapNodesResponse>, StatusCode> {
    authorize_managed_community_node_request(&headers, state.as_ref()).await?;
    if !state.consent_accepted.load(Ordering::SeqCst) {
        return Err(StatusCode::FORBIDDEN);
    }
    state.bootstrap_hits.fetch_add(1, Ordering::SeqCst);
    Ok(Json(BootstrapNodesResponse {
        nodes: vec![kukuri_cn_core::CommunityNodeBootstrapNode {
            base_url: state.base_url.clone(),
            resolved_urls: CommunityNodeResolvedUrls::new(
                state.base_url.clone(),
                Vec::new(),
                state.seed_peers.clone(),
            )
            .expect("resolved urls"),
        }],
    }))
}

async fn new_seeded_dht_runtime_with_config(
    db_path: &Path,
    testnet: &Testnet,
    discovery_config: DiscoveryConfig,
) -> DesktopRuntime {
    let runtime = DesktopRuntime::new_with_config_and_identity_and_discovery(
        db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
        discovery_config,
        DhtDiscoveryOptions::with_bootstrap(&testnet.bootstrap),
        false,
    )
    .await
    .expect("seeded dht runtime");
    wait_for_runtime_endpoint_in_testnet(&runtime, testnet).await;
    runtime
}

async fn new_seeded_dht_runtime(db_path: &Path, testnet: &Testnet) -> DesktopRuntime {
    new_seeded_dht_runtime_with_config(db_path, testnet, seeded_dht_config(Vec::new())).await
}

mod attachments;
mod community_node;
mod identity_restart;
mod media_blob_restore;
mod private_channels;
mod replication_heuristics;
mod seeded_dht;
mod static_peer;
