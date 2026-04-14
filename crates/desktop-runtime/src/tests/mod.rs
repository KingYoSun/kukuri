use super::*;
use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use image::{
    AnimationDecoder, Delay, DynamicImage, Frame, GenericImageView, ImageDecoder, ImageFormat,
    Rgba, RgbaImage,
};
use iroh::address_lookup::EndpointInfo;
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
use pkarr::errors::{ConcurrencyError, PublishError};
use pkarr::{Client as PkarrClient, SignedPacket, Timestamp, mainline::Testnet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::{Duration, sleep, timeout};

use crate::attachments::{normalize_custom_reaction_gif, normalize_custom_reaction_static};
use crate::community_node::{
    BootstrapNodesResponse, StoredCommunityNodeToken, load_community_node_config_from_file,
    normalize_community_node_config, persist_community_node_token,
    relay_config_from_community_node_config, save_community_node_config,
};
use crate::discovery::resolve_discovery_config_from_env;
use crate::identity::IdentityStorageMode;
use crate::paths::discovery_config_path;

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

#[test]
fn normalize_custom_reaction_static_resizes_png_to_square() {
    let normalized = normalize_custom_reaction_static(
        png_source_bytes(),
        &CustomReactionCropRect {
            x: 70,
            y: 0,
            size: 180,
        },
    )
    .expect("normalize png");
    let image = image::load_from_memory(normalized.bytes.as_slice()).expect("decode png");

    assert_eq!(normalized.mime, "image/png");
    assert_eq!(image.dimensions(), (128, 128));
}

#[test]
fn animated_gif_custom_reaction_preserves_gif_mime_after_normalization() {
    let normalized = normalize_custom_reaction_gif(
        animated_gif_source_bytes(),
        &CustomReactionCropRect {
            x: 1,
            y: 0,
            size: 2,
        },
    )
    .expect("normalize gif");
    let decoder =
        image::codecs::gif::GifDecoder::new(std::io::Cursor::new(normalized.bytes.clone()))
            .expect("decode normalized gif");
    let dimensions = decoder.dimensions();
    let frame_count = decoder
        .into_frames()
        .collect_frames()
        .expect("collect normalized gif frames")
        .len();

    assert_eq!(normalized.mime, "image/gif");
    assert_eq!(dimensions, (128, 128));
    assert_eq!(frame_count, 2);
}

fn format_sync_snapshot(status: &SyncStatus, topic: &str) -> String {
    let topic_status = status
            .topic_diagnostics
            .iter()
            .find(|entry| entry.topic == topic)
            .map(|entry| {
                format!(
                    "topic_peers={}, connected_peers={:?}, assist_peer_ids={:?}, configured_peer_ids={:?}, missing_peer_ids={:?}, status_detail={}",
                    entry.peer_count,
                    entry.connected_peers,
                    entry.assist_peer_ids,
                    entry.configured_peer_ids,
                    entry.missing_peer_ids,
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
            let ready = status.connected
                && status.peer_count >= expected
                && status.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && (topic_status.connected_peers.len() >= expected.min(1)
                            || topic_status.assist_peer_ids.len() >= expected.min(1))
                        && topic_status.peer_count >= expected
                });
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

async fn wait_for_connected_topic_peer_count_result(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    match timeout(step_timeout, async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await.context("sync status")?;
            let ready = status.connected
                && status.peer_count >= expected
                && status.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && (topic_status.connected_peers.len() >= expected.min(1)
                            || topic_status.assist_peer_ids.len() >= expected.min(1))
                        && topic_status.peer_count >= expected
                });
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
            bail!("topic readiness timeout; {status}");
        }
    }
}

fn topic_has_direct_peer(status: &SyncStatus, topic: &str, expected: usize) -> bool {
    status.connected
        && status.peer_count >= expected
        && status.topic_diagnostics.iter().any(|topic_status| {
            topic_status.topic == topic
                && topic_status.joined
                && topic_status.connected_peers.len() >= expected.min(1)
                && topic_status.peer_count >= expected
        })
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

async fn wait_for_connected_peer_count(
    runtime: &DesktopRuntime,
    expected: usize,
    timeout_label: &str,
) {
    match timeout(social_graph_propagation_timeout(), async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await.expect("sync status");
            let ready = status.connected && status.peer_count >= expected;
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
            panic!("{timeout_label}: {}", format_sync_snapshot(&status, ""));
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

async fn replicate_public_post_from_original_publisher_with_retry(
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
        false,
    )
    .await
}

async fn refresh_public_pair_result(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    topic: &str,
    step_timeout: Duration,
) -> Result<()> {
    let scope = TimelineScope::Public;
    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await;
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await;
    let _ = runtime_a
        .list_live_sessions(ListLiveSessionsRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
        })
        .await;
    let _ = runtime_b
        .list_live_sessions(ListLiveSessionsRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
        })
        .await;
    let _ = runtime_a
        .list_game_rooms(ListGameRoomsRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
        })
        .await;
    let _ = runtime_b
        .list_game_rooms(ListGameRoomsRequest {
            topic: topic.to_string(),
            scope,
        })
        .await;
    wait_for_connected_topic_peer_count_result(runtime_a, topic, 1, step_timeout)
        .await
        .context("runtime a did not observe public topic connectivity")?;
    wait_for_connected_topic_peer_count_result(runtime_b, topic, 1, step_timeout)
        .await
        .context("runtime b did not observe public topic connectivity")?;
    Ok(())
}

async fn wait_for_direct_public_pair_with_refresh_result(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    topic: &str,
    step_timeout: Duration,
    same_author_shared_identity: bool,
) -> Result<()> {
    let (attempts, attempt_timeout) =
        public_replication_retry_schedule(step_timeout, same_author_shared_identity);
    let mut last_error = None;

    for attempt in 1..=attempts {
        let attempt_result = async {
            refresh_public_pair_result(runtime_a, runtime_b, topic, attempt_timeout)
                .await
                .context("failed to refresh public pair before waiting for direct connectivity")?;
            wait_for_direct_topic_peer_count_result(runtime_a, topic, 1, attempt_timeout)
                .await
                .context("runtime a did not observe direct public topic connectivity")?;
            wait_for_direct_topic_peer_count_result(runtime_b, topic, 1, attempt_timeout)
                .await
                .context("runtime b did not observe direct public topic connectivity")?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        match attempt_result {
            Ok(()) => return Ok(()),
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

    bail!(
        "public pair direct topic readiness timeout; {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    );
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
            wait_for_connected_topic_peer_count_result(publisher, topic, 1, attempt_timeout)
                .await
                .context("publisher did not observe public topic connectivity")?;
            wait_for_connected_topic_peer_count_result(subscriber, topic, 1, attempt_timeout)
                .await
                .context("subscriber did not observe public topic connectivity")?;
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
            wait_for_connected_topic_peer_count_result(publisher, topic, 1, attempt_timeout)
                .await
                .context("publisher did not observe private topic connectivity")?;
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
                wait_for_connected_topic_peer_count_result(subscriber, topic, 1, attempt_timeout)
                    .await
                    .context("subscriber did not observe private topic connectivity")?;
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

fn sync_status_with_topic(
    topic: &str,
    connected_peers: &[&str],
    assist_peer_ids: &[&str],
) -> SyncStatus {
    SyncStatus {
        connected: true,
        last_sync_ts: None,
        peer_count: connected_peers.len().max(assist_peer_ids.len()),
        pending_events: 0,
        status_detail: "test".to_string(),
        last_error: None,
        configured_peers: Vec::new(),
        subscribed_topics: vec![topic.to_string()],
        topic_diagnostics: vec![kukuri_app_api::TopicSyncStatus {
            topic: topic.to_string(),
            joined: true,
            peer_count: connected_peers.len().max(assist_peer_ids.len()),
            connected_peers: connected_peers
                .iter()
                .map(|peer| peer.to_string())
                .collect(),
            assist_peer_ids: assist_peer_ids
                .iter()
                .map(|peer| peer.to_string())
                .collect(),
            configured_peer_ids: Vec::new(),
            missing_peer_ids: Vec::new(),
            last_received_at: None,
            status_detail: "test".to_string(),
            last_error: None,
        }],
        local_author_pubkey: "author".to_string(),
        discovery: Default::default(),
    }
}

#[test]
fn shared_identity_public_replication_prefers_direct_connected_runtime() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer"]);
    let subscriber_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);

    assert!(should_swap_shared_identity_public_replication_direction(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
    ));
}

#[test]
fn shared_identity_public_replication_keeps_original_publisher_when_it_is_direct() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);
    let subscriber_status = sync_status_with_topic(topic, &[], &["assist-peer"]);

    assert!(!should_swap_shared_identity_public_replication_direction(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
    ));
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
            let ready_a = status_a.topic_diagnostics.iter().any(|topic_status| {
                topic_status.topic == topic
                    && topic_status.joined
                    && (!topic_status.connected_peers.is_empty()
                        || !topic_status.assist_peer_ids.is_empty())
                    && topic_status.peer_count > 0
            });
            let ready_b = status_b.topic_diagnostics.iter().any(|topic_status| {
                topic_status.topic == topic
                    && topic_status.joined
                    && (!topic_status.connected_peers.is_empty()
                        || !topic_status.assist_peer_ids.is_empty())
                    && topic_status.peer_count > 0
            });
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

fn dht_test_client(testnet: &Testnet) -> PkarrClient {
    let mut builder = PkarrClient::builder();
    builder.no_default_network().bootstrap(&testnet.bootstrap);
    builder.build().expect("pkarr client")
}

fn build_endpoint_signed_packet_with_timestamp(
    endpoint_info: &EndpointInfo,
    secret_key: &iroh::SecretKey,
    ttl: u32,
    timestamp: Timestamp,
) -> SignedPacket {
    use pkarr::dns::{self, rdata};

    let keypair = pkarr::Keypair::from_secret_key(&secret_key.to_bytes());
    let mut builder = SignedPacket::builder().timestamp(timestamp);
    let name = dns::Name::new("_iroh").expect("iroh txt name");
    for entry in endpoint_info.to_txt_strings() {
        let mut txt = rdata::TXT::new();
        txt.add_string(&entry)
            .expect("valid endpoint info txt entry");
        builder = builder.txt(name.clone(), txt.into_owned(), ttl);
    }
    builder.sign(&keypair).expect("sign endpoint info packet")
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

async fn publish_runtime_endpoint_to_testnet(runtime: &DesktopRuntime, testnet: &Testnet) {
    let endpoint = runtime.iroh_stack.endpoint().await;
    let client = dht_test_client(testnet);
    let public_key =
        pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
    let expected_info = EndpointInfo::from(endpoint.addr());
    for _ in 0..20 {
        let previous_timestamp = client
            .resolve_most_recent(&public_key)
            .await
            .map(|packet| packet.timestamp());
        let now = Timestamp::now();
        let timestamp = match previous_timestamp {
            Some(previous) if previous >= now => previous + 1,
            _ => now,
        };
        let signed_packet = build_endpoint_signed_packet_with_timestamp(
            &expected_info,
            endpoint.secret_key(),
            1,
            timestamp,
        );
        match client.publish(&signed_packet, previous_timestamp).await {
            Ok(()) => break,
            Err(PublishError::Concurrency(
                ConcurrencyError::ConflictRisk
                | ConcurrencyError::NotMostRecent
                | ConcurrencyError::CasFailed,
            )) => sleep(Duration::from_millis(50)).await,
            Err(error) => panic!("publish endpoint info: {error}"),
        }
    }
    timeout(Duration::from_secs(5), async {
        loop {
            if client
                .resolve_most_recent(&public_key)
                .await
                .as_ref()
                .and_then(|packet| EndpointInfo::from_pkarr_signed_packet(packet).ok())
                .is_some_and(|packet_info| {
                    packet_info.to_txt_strings() == expected_info.to_txt_strings()
                })
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
        DhtDiscoveryOptions::with_client(dht_test_client(testnet)),
    )
    .await
    .expect("seeded dht runtime");
    publish_runtime_endpoint_to_testnet(&runtime, testnet).await;
    runtime
}

async fn new_seeded_dht_runtime(db_path: &Path, testnet: &Testnet) -> DesktopRuntime {
    new_seeded_dht_runtime_with_config(db_path, testnet, seeded_dht_config(Vec::new())).await
}

#[test]
fn resolve_db_path_ignores_legacy_runtime_artifacts() {
    let dir = tempdir().expect("tempdir");
    let legacy_db_path = dir.path().join("kukuri-next.db");
    let legacy_data_dir = dir.path().join("kukuri-next.iroh-data");
    fs::write(&legacy_db_path, b"sqlite").expect("legacy db");
    fs::create_dir_all(&legacy_data_dir).expect("legacy data dir");
    fs::write(legacy_data_dir.join("blob.bin"), b"blob").expect("legacy blob");

    let resolved = resolve_db_path_from_env(dir.path()).expect("resolved db path");

    assert_eq!(resolved, dir.path().join("kukuri.db"));
    assert!(!resolved.exists());
    assert!(!resolved.with_extension("iroh-data").exists());
    assert!(legacy_db_path.exists());
    assert!(legacy_data_dir.join("blob.bin").exists());
}

#[tokio::test]
async fn desktop_runtime_persists_posts_and_author_identity_after_restart() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let runtime = timeout(
        Duration::from_secs(15),
        DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        ),
    )
    .await
    .expect("runtime creation timeout")
    .expect("runtime");
    let object_id = runtime
        .create_post(CreatePostRequest {
            topic: "kukuri:topic:runtime".into(),
            content: "persist me".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("create post");
    timeout(Duration::from_secs(15), runtime.shutdown())
        .await
        .expect("runtime shutdown timeout");
    drop(runtime);

    let restarted = timeout(
        Duration::from_secs(15),
        DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        ),
    )
    .await
    .expect("runtime restart timeout")
    .expect("runtime restart");
    let restarted_object_id = restarted
        .create_post(CreatePostRequest {
            topic: "kukuri:topic:runtime".into(),
            content: "persist me again".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("create post after restart");
    let timeline = restarted
        .list_timeline(ListTimelineRequest {
            topic: "kukuri:topic:runtime".into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline");

    assert!(
        timeline
            .items
            .iter()
            .any(|post| post.object_id == object_id)
    );
    assert!(
        timeline
            .items
            .iter()
            .any(|post| post.object_id == restarted_object_id)
    );
    let original_post = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("original post");
    let restarted_post = timeline
        .items
        .iter()
        .find(|post| post.object_id == restarted_object_id)
        .expect("restarted post");
    assert_eq!(original_post.author_pubkey, restarted_post.author_pubkey);
    assert_eq!(restarted.db_path(), db_path.as_path());
    timeout(Duration::from_secs(15), restarted.shutdown())
        .await
        .expect("restarted shutdown timeout");
}

#[tokio::test]
async fn desktop_runtime_restores_profile_avatar_blob_after_restart() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("profile-avatar-restart.db");
    let avatar_bytes = b"runtime-profile-avatar".to_vec();
    let expected_payload = BASE64_STANDARD.encode(&avatar_bytes);
    let runtime = timeout(
        Duration::from_secs(15),
        DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        ),
    )
    .await
    .expect("runtime creation timeout")
    .expect("runtime");

    let updated = runtime
        .set_my_profile(SetMyProfileRequest {
            name: Some("runtime-avatar-owner".into()),
            display_name: Some("Runtime Avatar Owner".into()),
            about: Some("profile avatar restart".into()),
            picture: None,
            picture_upload: Some(profile_avatar_attachment_request(
                "avatar.png",
                "image/png",
                &avatar_bytes,
            )),
            clear_picture: false,
        })
        .await
        .expect("set profile");
    let asset = updated.picture_asset.clone().expect("profile avatar");
    let author_pubkey = updated.pubkey.as_str().to_string();
    let payload_before_restart = runtime
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: asset.hash.as_str().to_string(),
            mime: asset.mime.clone(),
        })
        .await
        .expect("avatar payload before restart")
        .expect("avatar payload before restart value");
    let author_before_restart = runtime
        .get_author_social_view(AuthorRequest {
            pubkey: author_pubkey.clone(),
        })
        .await
        .expect("author social view before restart");

    assert_eq!(payload_before_restart.mime, "image/png");
    assert_eq!(payload_before_restart.bytes_base64, expected_payload);
    assert_eq!(
        author_before_restart
            .picture_asset
            .as_ref()
            .map(|value| value.hash.as_str()),
        Some(asset.hash.as_str())
    );
    assert_eq!(
        author_before_restart
            .picture_asset
            .as_ref()
            .map(|value| value.role.as_str()),
        Some("profile_avatar")
    );

    timeout(Duration::from_secs(15), runtime.shutdown())
        .await
        .expect("runtime shutdown timeout");
    drop(runtime);

    let restarted = timeout(
        Duration::from_secs(15),
        DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        ),
    )
    .await
    .expect("runtime restart timeout")
    .expect("runtime restart");
    let my_profile = restarted.get_my_profile().await.expect("my profile");
    let author_after_restart = restarted
        .get_author_social_view(AuthorRequest {
            pubkey: author_pubkey,
        })
        .await
        .expect("author social view after restart");
    let payload_after_restart = restarted
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: asset.hash.as_str().to_string(),
            mime: asset.mime.clone(),
        })
        .await
        .expect("avatar payload after restart")
        .expect("avatar payload after restart value");

    assert_eq!(
        my_profile
            .picture_asset
            .as_ref()
            .map(|value| value.hash.as_str()),
        Some(asset.hash.as_str())
    );
    assert_eq!(
        my_profile
            .picture_asset
            .as_ref()
            .map(|value| value.role.clone()),
        Some(AssetRole::ProfileAvatar)
    );
    assert_eq!(
        author_after_restart
            .picture_asset
            .as_ref()
            .map(|value| value.hash.as_str()),
        Some(asset.hash.as_str())
    );
    assert_eq!(
        author_after_restart
            .picture_asset
            .as_ref()
            .map(|value| value.role.as_str()),
        Some("profile_avatar")
    );
    assert_eq!(payload_after_restart.mime, "image/png");
    assert_eq!(payload_after_restart.bytes_base64, expected_payload);

    timeout(Duration::from_secs(15), restarted.shutdown())
        .await
        .expect("restarted shutdown timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_runtime_imports_peer_ticket_and_tracks_local_posts() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("a.db");
    let db_b = dir.path().join("b.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = runtime_b
        .local_peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    let endpoint_a = runtime_a
        .get_sync_status()
        .await
        .expect("status a before import")
        .discovery
        .local_endpoint_id;
    let endpoint_b = runtime_b
        .get_sync_status()
        .await
        .expect("status b before import")
        .discovery
        .local_endpoint_id;

    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("import b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("import a");

    let status_a = runtime_a
        .get_sync_status()
        .await
        .expect("status a after import");
    let status_b = runtime_b
        .get_sync_status()
        .await
        .expect("status b after import");
    assert_eq!(status_a.discovery.manual_ticket_peer_ids, vec![endpoint_b]);
    assert_eq!(status_b.discovery.manual_ticket_peer_ids, vec![endpoint_a]);

    let topic = "kukuri:topic:desktop-runtime";
    let object_id = runtime_a
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "hello desktop runtime".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("create post");

    let timeline = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline a");
    let post = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("local post");
    assert_eq!(post.content, "hello desktop runtime");
    let status = runtime_a.get_sync_status().await.expect("sync status");
    assert!(status.last_sync_ts.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn profile_timeline_reads_author_public_posts_across_untracked_topics() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("profile-runtime-a.db");
    let db_b = dir.path().join("profile-runtime-b.db");
    let shared_keys = KukuriKeys::generate();
    let shared_secret = shared_keys.export_secret_hex();
    fs::write(
        db_a.with_extension("identity-key"),
        shared_secret.as_bytes(),
    )
    .expect("persist shared identity key a");
    fs::write(db_a.with_extension("identity-store"), b"file")
        .expect("persist shared identity backend a");
    fs::write(
        db_b.with_extension("identity-key"),
        shared_secret.as_bytes(),
    )
    .expect("persist shared identity key b");
    fs::write(db_b.with_extension("identity-store"), b"file")
        .expect("persist shared identity backend b");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = runtime_b
        .local_peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");

    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("import b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("import a");
    wait_for_connected_peer_count(&runtime_a, 1, "profile topic owner peer readiness timeout")
        .await;
    wait_for_connected_peer_count(&runtime_b, 1, "profile topic viewer peer readiness timeout")
        .await;

    let author_pubkey = runtime_a
        .get_sync_status()
        .await
        .expect("status a")
        .local_author_pubkey;
    assert_eq!(
        author_pubkey,
        runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .local_author_pubkey
    );
    let tracked_topic = "kukuri:topic:desktop-profile-demo";
    let untracked_topic = "kukuri:topic:desktop-profile-relay";
    let public_scope = TimelineScope::Public;

    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: tracked_topic.into(),
            scope: public_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe a tracked topic");
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: tracked_topic.into(),
            scope: public_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe b tracked topic");
    wait_for_connected_topic_peer_count(
        &runtime_a,
        tracked_topic,
        1,
        "profile tracked topic readiness timeout a",
    )
    .await;
    wait_for_connected_topic_peer_count(
        &runtime_b,
        tracked_topic,
        1,
        "profile tracked topic readiness timeout b",
    )
    .await;

    let tracked_object_id = replicate_public_post_with_retry(
        &runtime_a,
        &runtime_b,
        tracked_topic,
        "tracked profile post",
        "tracked topic visibility timeout",
    )
    .await;
    let untracked_object_id = runtime_a
        .create_post(CreatePostRequest {
            topic: untracked_topic.into(),
            content: "untracked profile post".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("untracked public post");
    let before_profile = runtime_b
        .get_sync_status()
        .await
        .expect("status before profile");
    assert!(
        before_profile
            .subscribed_topics
            .iter()
            .any(|topic| topic == tracked_topic)
    );
    assert!(
        before_profile
            .subscribed_topics
            .iter()
            .all(|topic| topic != untracked_topic)
    );

    let profile_object_ids = vec![tracked_object_id.clone(), untracked_object_id.clone()];
    let (runtime_b, profile_timeline) = match wait_for_profile_timeline_posts_result(
        &runtime_b,
        author_pubkey.as_str(),
        &profile_object_ids,
        "profile timeline visibility timeout",
    )
    .await
    {
        Ok(timeline) => (runtime_b, timeline),
        Err(first_error) => {
            timeout(runtime_shutdown_timeout(), runtime_b.shutdown())
                .await
                .expect("profile viewer restart shutdown timeout");
            drop(runtime_b);

            let restarted_b = DesktopRuntime::new_with_config_and_identity(
                &db_b,
                TransportNetworkConfig::loopback(),
                IdentityStorageMode::FileOnly,
            )
            .await
            .expect("restart runtime b");
            let restarted_ticket_b = restarted_b
                .local_peer_ticket()
                .await
                .expect("restarted ticket b")
                .expect("restarted ticket b value");
            runtime_a
                .import_peer_ticket(ImportPeerTicketRequest {
                    ticket: restarted_ticket_b,
                })
                .await
                .expect("import restarted b");
            restarted_b
                .import_peer_ticket(ImportPeerTicketRequest {
                    ticket: ticket_a.clone(),
                })
                .await
                .expect("import a after restart");
            wait_for_connected_peer_count(
                &restarted_b,
                1,
                "profile viewer peer restart readiness timeout",
            )
            .await;
            let _ = restarted_b
                .list_timeline(ListTimelineRequest {
                    topic: tracked_topic.into(),
                    scope: public_scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("resubscribe restarted b tracked topic");
            wait_for_connected_topic_peer_count(
                &restarted_b,
                tracked_topic,
                1,
                "profile tracked topic restart readiness timeout b",
            )
            .await;
            let timeline = wait_for_profile_timeline_posts_result(
                    &restarted_b,
                    author_pubkey.as_str(),
                    &profile_object_ids,
                    "profile timeline visibility timeout after restart",
                )
                .await
                .unwrap_or_else(|second_error| {
                    panic!(
                        "profile timeline visibility timeout after viewer restart: first_error={first_error:#}; second_error={second_error:#}"
                    )
                });
            (restarted_b, timeline)
        }
    };
    assert!(
        profile_timeline
            .items
            .iter()
            .any(|post| post.object_id == tracked_object_id
                && post.origin_topic_id.as_deref() == Some(tracked_topic))
    );
    assert!(
        profile_timeline
            .items
            .iter()
            .any(|post| post.object_id == untracked_object_id
                && post.origin_topic_id.as_deref() == Some(untracked_topic))
    );

    let after_profile = runtime_b
        .get_sync_status()
        .await
        .expect("status after profile");
    assert!(
        after_profile
            .subscribed_topics
            .iter()
            .all(|topic| topic != untracked_topic)
    );

    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: untracked_topic.into(),
            scope: public_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("open original topic");
    wait_for_timeline_post(
        &runtime_b,
        untracked_topic,
        &public_scope,
        untracked_object_id.as_str(),
        "origin topic visibility timeout",
    )
    .await;

    let after_origin = runtime_b
        .get_sync_status()
        .await
        .expect("status after origin");
    assert!(
        after_origin
            .subscribed_topics
            .iter()
            .any(|topic| topic == untracked_topic)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn private_channel_invite_restores_after_restart_without_reimport() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("private-runtime-a.db");
    let db_b = dir.path().join("private-runtime-b.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = runtime_b
        .local_peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");

    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_b })
        .await
        .expect("import b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
        .await
        .expect("import a");
    wait_for_connected_peer_count(&runtime_a, 1, "friend-only owner peer readiness timeout").await;
    wait_for_connected_peer_count(&runtime_b, 1, "friend-only invitee peer readiness timeout")
        .await;

    let topic = "kukuri:topic:desktop-private-channel";
    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe a");
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe b");
    let channel = runtime_a
        .create_private_channel(CreatePrivateChannelRequest {
            topic: topic.into(),
            label: "core".into(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");
    let invite = runtime_a
        .export_private_channel_invite(ExportPrivateChannelInviteRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export invite");
    let preview = runtime_b
        .import_private_channel_invite(ImportPrivateChannelInviteRequest { token: invite })
        .await
        .expect("import invite");
    assert_eq!(preview.topic_id.as_str(), topic);
    assert_eq!(preview.channel_id.as_str(), channel.channel_id);

    let private_channel_id = kukuri_core::ChannelId::new(channel.channel_id.clone());
    let private_channel_ref = ChannelRef::PrivateChannel {
        channel_id: private_channel_id.clone(),
    };
    let private_scope = TimelineScope::Channel {
        channel_id: private_channel_id.clone(),
    };
    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe private a");
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe private b");

    let private_post_id = runtime_b
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "private hello from b".into(),
            reply_to: None,
            channel_ref: private_channel_ref.clone(),
            attachments: vec![],
        })
        .await
        .expect("create private post");

    let private_post = timeout(Duration::from_secs(10), async {
        loop {
            let public_timeline = runtime_b
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("public timeline");
            assert!(
                public_timeline
                    .items
                    .iter()
                    .all(|post| post.object_id != private_post_id),
                "private post leaked into public timeline"
            );
            let private_timeline = runtime_b
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("private timeline");
            if let Some(post) = private_timeline
                .items
                .iter()
                .find(|post| post.object_id == private_post_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("private post timeout");
    assert_eq!(
        private_post.channel_id.as_deref(),
        Some(channel.channel_id.as_str())
    );
    assert_eq!(private_post.audience_label, "core");
    let _ = runtime_b
        .list_thread(ListThreadRequest {
            topic: topic.into(),
            thread_id: private_post_id.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe private thread");

    let private_reply_id = runtime_b
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "private reply".into(),
            reply_to: Some(private_post_id.clone()),
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("create private reply");
    let private_thread = timeout(Duration::from_secs(10), async {
        loop {
            let thread = runtime_b
                .list_thread(ListThreadRequest {
                    topic: topic.into(),
                    thread_id: private_post_id.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("thread");
            if thread
                .items
                .iter()
                .any(|post| post.object_id == private_reply_id)
            {
                return thread;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("private thread timeout");
    let reply = private_thread
        .items
        .iter()
        .find(|post| post.object_id == private_reply_id)
        .expect("reply");
    assert_eq!(
        reply.channel_id.as_deref(),
        Some(channel.channel_id.as_str())
    );

    let session_id = runtime_b
        .create_live_session(CreateLiveSessionRequest {
            topic: topic.into(),
            channel_ref: private_channel_ref.clone(),
            title: "core live".into(),
            description: "private stream".into(),
        })
        .await
        .expect("create private live session");
    let _private_session = timeout(Duration::from_secs(10), async {
        loop {
            let sessions = runtime_b
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                })
                .await
                .expect("list private live sessions");
            if let Some(session) = sessions
                .iter()
                .find(|session| session.session_id == session_id)
            {
                return session.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("private live timeout");
    runtime_b
        .end_live_session(LiveSessionCommandRequest {
            topic: topic.into(),
            session_id: session_id.clone(),
        })
        .await
        .expect("end live session");
    timeout(Duration::from_secs(10), async {
        loop {
            let sessions = runtime_b
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                })
                .await
                .expect("list live sessions b");
            if sessions.iter().any(|session| {
                session.session_id == session_id
                    && session.status == kukuri_core::LiveSessionStatus::Ended
            }) {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("live end timeout");

    let room_id = runtime_b
        .create_game_room(CreateGameRoomRequest {
            topic: topic.into(),
            channel_ref: private_channel_ref.clone(),
            title: "core room".into(),
            description: "private set".into(),
            participants: vec!["Alice".into(), "Bob".into()],
        })
        .await
        .expect("create private game room");
    let room_before_update = timeout(Duration::from_secs(10), async {
        loop {
            let rooms = runtime_b
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                })
                .await
                .expect("list private game rooms");
            if let Some(room) = rooms.iter().find(|room| room.room_id == room_id) {
                return room.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("private game timeout");
    runtime_b
        .update_game_room(UpdateGameRoomRequest {
            topic: topic.into(),
            room_id: room_id.clone(),
            status: GameRoomStatus::Running,
            phase_label: Some("Round 2".into()),
            scores: room_before_update
                .scores
                .iter()
                .map(|score| GameScoreView {
                    participant_id: score.participant_id.clone(),
                    label: score.label.clone(),
                    score: if score.label == "Alice" { 2 } else { 1 },
                })
                .collect(),
        })
        .await
        .expect("update private game room");
    timeout(Duration::from_secs(10), async {
        loop {
            let rooms = runtime_b
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                })
                .await
                .expect("list updated game rooms");
            if rooms.iter().any(|room| {
                room.room_id == room_id
                    && room.phase_label.as_deref() == Some("Round 2")
                    && room
                        .scores
                        .iter()
                        .any(|score| score.label == "Alice" && score.score == 2)
            }) {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("game update timeout");

    let joined_before_restart = runtime_b
        .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
            topic: topic.into(),
        })
        .await
        .expect("list joined channels before restart");
    assert_eq!(joined_before_restart.len(), 1);
    assert_eq!(joined_before_restart[0].channel_id, channel.channel_id);

    timeout(Duration::from_secs(30), runtime_a.shutdown())
        .await
        .expect("runtime a shutdown timeout");
    timeout(Duration::from_secs(30), runtime_b.shutdown())
        .await
        .expect("runtime b shutdown timeout");
    drop(runtime_a);
    drop(runtime_b);
    delete_sqlite_artifacts(&db_b);

    let restarted_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart runtime b");

    let joined_after_restart = restarted_b
        .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
            topic: topic.into(),
        })
        .await
        .expect("list joined channels after restart");
    assert_eq!(joined_after_restart.len(), 1);
    assert_eq!(joined_after_restart[0].channel_id, channel.channel_id);
    assert_eq!(joined_after_restart[0].label, "core");

    let public_timeline_after_restart = restarted_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("public timeline after restart");
    assert!(
        public_timeline_after_restart
            .items
            .iter()
            .all(|post| post.object_id != private_post_id)
    );
    let private_timeline_after_restart = restarted_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("private timeline after restart");
    assert!(
        private_timeline_after_restart
            .items
            .iter()
            .any(|post| post.object_id == private_post_id)
    );
    assert!(
        private_timeline_after_restart
            .items
            .iter()
            .any(|post| post.object_id == private_reply_id)
    );

    let private_thread_after_restart = restarted_b
        .list_thread(ListThreadRequest {
            topic: topic.into(),
            thread_id: private_post_id.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("private thread after restart");
    assert!(
        private_thread_after_restart
            .items
            .iter()
            .any(|post| post.object_id == private_reply_id)
    );

    let sessions_after_restart = restarted_b
        .list_live_sessions(ListLiveSessionsRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
        })
        .await
        .expect("live sessions after restart");
    assert!(sessions_after_restart.iter().any(|session| {
        session.session_id == session_id && session.status == kukuri_core::LiveSessionStatus::Ended
    }));

    let rooms_after_restart = restarted_b
        .list_game_rooms(ListGameRoomsRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
        })
        .await
        .expect("game rooms after restart");
    assert!(rooms_after_restart.iter().any(|room| {
        room.room_id == room_id
            && room.phase_label.as_deref() == Some("Round 2")
            && room
                .scores
                .iter()
                .any(|score| score.label == "Alice" && score.score == 2)
    }));

    let fresh_invite = restarted_b
        .export_private_channel_invite(ExportPrivateChannelInviteRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export fresh invite");
    assert!(fresh_invite.contains(topic));
    assert!(fresh_invite.contains(channel.channel_id.as_str()));

    timeout(Duration::from_secs(30), restarted_b.shutdown())
        .await
        .expect("restarted runtime shutdown timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn friend_only_channel_restore_keeps_archived_epoch_history() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("friend-only-runtime-a.db");
    let db_b = dir.path().join("friend-only-runtime-b.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = runtime_b
        .local_peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");

    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_b })
        .await
        .expect("import b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
        .await
        .expect("import a");
    wait_for_connected_peer_count(&runtime_a, 1, "friend-only owner peer readiness timeout").await;
    wait_for_connected_peer_count(
        &runtime_b,
        1,
        "friend-only recipient peer readiness timeout",
    )
    .await;

    let a_pubkey = runtime_a
        .get_sync_status()
        .await
        .expect("status a")
        .local_author_pubkey;
    let b_pubkey = runtime_b
        .get_sync_status()
        .await
        .expect("status b")
        .local_author_pubkey;
    warm_author_social_view(
        &runtime_a,
        b_pubkey.as_str(),
        "friend-only owner author warm timeout",
    )
    .await;
    warm_author_social_view(
        &runtime_b,
        a_pubkey.as_str(),
        "friend-only recipient author warm timeout",
    )
    .await;
    runtime_a
        .follow_author(AuthorRequest {
            pubkey: b_pubkey.clone(),
        })
        .await
        .expect("a follows b");
    runtime_b
        .follow_author(AuthorRequest {
            pubkey: a_pubkey.clone(),
        })
        .await
        .expect("b follows a");

    timeout(social_graph_propagation_timeout(), async {
        loop {
            let a_view = runtime_a
                .get_author_social_view(AuthorRequest {
                    pubkey: b_pubkey.clone(),
                })
                .await
                .expect("a loads b");
            let b_view = runtime_b
                .get_author_social_view(AuthorRequest {
                    pubkey: a_pubkey.clone(),
                })
                .await
                .expect("b loads a");
            if a_view.mutual && b_view.mutual {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("mutual propagation timeout");

    let topic = "kukuri:topic:desktop-friend-only-restart";
    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe a");
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe b");

    let channel = runtime_a
        .create_private_channel(CreatePrivateChannelRequest {
            topic: topic.into(),
            label: "friends".into(),
            audience_kind: ChannelAudienceKind::FriendOnly,
        })
        .await
        .expect("create friend-only channel");
    let grant = runtime_a
        .export_friend_only_grant(ExportFriendOnlyGrantRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export friend-only grant");
    let preview = runtime_b
        .import_friend_only_grant(ImportFriendOnlyGrantRequest { token: grant })
        .await
        .expect("import friend-only grant");
    let original_epoch_id = preview.epoch_id.clone();
    assert_eq!(preview.topic_id.as_str(), topic);
    assert_eq!(preview.channel_id.as_str(), channel.channel_id);

    let private_channel_id = kukuri_core::ChannelId::new(channel.channel_id.clone());
    let private_channel_ref = ChannelRef::PrivateChannel {
        channel_id: private_channel_id.clone(),
    };
    let private_scope = TimelineScope::Channel {
        channel_id: private_channel_id.clone(),
    };
    let private_post_id = runtime_b
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "friends hello from b".into(),
            reply_to: None,
            channel_ref: private_channel_ref,
            attachments: vec![],
        })
        .await
        .expect("create friend-only post");

    timeout(runtime_replication_timeout(), async {
        loop {
            let public_timeline = runtime_b
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("public timeline");
            assert!(
                public_timeline
                    .items
                    .iter()
                    .all(|post| post.object_id != private_post_id),
                "friend-only post leaked into public timeline"
            );
            let private_timeline = runtime_b
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("private timeline");
            if private_timeline
                .items
                .iter()
                .any(|post| post.object_id == private_post_id)
            {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("friend-only post timeout");

    let rotated = runtime_a
        .rotate_private_channel(RotatePrivateChannelRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
        })
        .await
        .expect("rotate friend-only channel");
    assert_ne!(rotated.current_epoch_id, original_epoch_id);
    assert_eq!(rotated.archived_epoch_ids, vec![original_epoch_id.clone()]);

    let fresh_grant = runtime_a
        .export_friend_only_grant(ExportFriendOnlyGrantRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export fresh friend-only grant");
    let fresh_preview = runtime_b
        .import_friend_only_grant(ImportFriendOnlyGrantRequest { token: fresh_grant })
        .await
        .expect("import fresh friend-only grant");
    assert_eq!(fresh_preview.epoch_id, rotated.current_epoch_id);

    let joined_before_restart = vec![
        wait_for_joined_private_channel_epoch(
            &runtime_b,
            topic,
            channel.channel_id.as_str(),
            rotated.current_epoch_id.as_str(),
            2,
            "joined channel update timeout",
        )
        .await,
    ];
    assert_eq!(joined_before_restart.len(), 1);
    assert_eq!(
        joined_before_restart[0].archived_epoch_ids,
        vec![original_epoch_id.clone()]
    );

    timeout(Duration::from_secs(30), runtime_a.shutdown())
        .await
        .expect("runtime a shutdown timeout");
    timeout(Duration::from_secs(30), runtime_b.shutdown())
        .await
        .expect("runtime b shutdown timeout");
    drop(runtime_a);
    drop(runtime_b);
    delete_sqlite_artifacts(&db_b);

    let restarted_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart runtime b");

    let joined_after_restart = restarted_b
        .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
            topic: topic.into(),
        })
        .await
        .expect("list joined channels after restart");
    assert_eq!(joined_after_restart.len(), 1);
    assert_eq!(joined_after_restart[0].channel_id, channel.channel_id);
    assert_eq!(
        joined_after_restart[0].audience_kind,
        ChannelAudienceKind::FriendOnly
    );
    assert_eq!(
        joined_after_restart[0].current_epoch_id,
        rotated.current_epoch_id
    );
    assert_eq!(
        joined_after_restart[0].archived_epoch_ids,
        vec![original_epoch_id.clone()]
    );

    let private_timeline_after_restart = restarted_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("private timeline after restart");
    assert!(
        private_timeline_after_restart
            .items
            .iter()
            .any(|post| post.object_id == private_post_id)
    );

    timeout(Duration::from_secs(30), restarted_b.shutdown())
        .await
        .expect("restarted runtime shutdown timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn friend_plus_channel_restore_accepts_fresh_share_after_restart() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("friend-plus-runtime-a.db");
    let db_b = dir.path().join("friend-plus-runtime-b.db");
    let db_c = dir.path().join("friend-plus-runtime-c.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    let runtime_c = DesktopRuntime::new_with_config_and_identity(
        &db_c,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime c");

    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = runtime_b
        .local_peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    let ticket_c = runtime_c
        .local_peer_ticket()
        .await
        .expect("ticket c")
        .expect("ticket c value");

    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("a imports b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("b imports a");
    wait_for_connected_peer_count(&runtime_a, 1, "friend-plus owner peer readiness timeout").await;
    wait_for_connected_peer_count(&runtime_b, 1, "friend-plus sponsor peer readiness timeout")
        .await;

    let status_a = runtime_a.get_sync_status().await.expect("status a");
    let a_pubkey = status_a.local_author_pubkey;
    let status_b = runtime_b.get_sync_status().await.expect("status b");
    let b_pubkey = status_b.local_author_pubkey;
    let status_c = runtime_c.get_sync_status().await.expect("status c");
    let c_pubkey = status_c.local_author_pubkey;
    let topic = "kukuri:topic:desktop-friend-plus-restart";
    for runtime in [&runtime_a, &runtime_b] {
        let _ = runtime
            .list_timeline(ListTimelineRequest {
                topic: topic.into(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe runtime");
    }
    wait_for_connected_topic_peer_count(
        &runtime_a,
        topic,
        1,
        "friend-plus owner topic readiness timeout",
    )
    .await;
    wait_for_connected_topic_peer_count(
        &runtime_b,
        topic,
        1,
        "friend-plus sponsor topic readiness timeout",
    )
    .await;
    warm_author_social_view(
        &runtime_a,
        b_pubkey.as_str(),
        "friend-plus owner author warm timeout",
    )
    .await;
    warm_author_social_view(
        &runtime_b,
        a_pubkey.as_str(),
        "friend-plus sponsor owner author warm timeout",
    )
    .await;
    runtime_a
        .follow_author(AuthorRequest {
            pubkey: b_pubkey.clone(),
        })
        .await
        .expect("a follows b");
    runtime_b
        .follow_author(AuthorRequest {
            pubkey: a_pubkey.clone(),
        })
        .await
        .expect("b follows a");
    wait_for_mutual_author_view(&runtime_a, b_pubkey.as_str(), topic).await;
    wait_for_mutual_author_view(&runtime_b, a_pubkey.as_str(), topic).await;
    let channel = runtime_a
        .create_private_channel(CreatePrivateChannelRequest {
            topic: topic.into(),
            label: "friends+".into(),
            audience_kind: ChannelAudienceKind::FriendPlus,
        })
        .await
        .expect("create friend-plus channel");
    let share_ab = runtime_a
        .export_friend_plus_share(ExportFriendPlusShareRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export a->b share");
    runtime_b
        .import_friend_plus_share(ImportFriendPlusShareRequest { token: share_ab })
        .await
        .expect("b imports friend-plus share");
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_c.clone(),
        })
        .await
        .expect("a imports c");
    runtime_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("c imports a");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_c.clone(),
        })
        .await
        .expect("b imports c");
    runtime_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("c imports b");
    wait_for_connected_peer_count(&runtime_a, 2, "friend-plus owner full-mesh timeout").await;
    wait_for_connected_peer_count(&runtime_b, 2, "friend-plus sponsor full-mesh timeout").await;
    wait_for_connected_peer_count(&runtime_c, 2, "friend-plus recipient full-mesh timeout").await;
    let _ = runtime_c
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe runtime c");
    // C joins after A and B have already subscribed, so re-import the peer tickets to
    // rebuild the existing topic subscriptions against C's endpoint instead of leaving
    // them assist-only.
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_c.clone(),
        })
        .await
        .expect("a refreshes c after subscribe");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_c.clone(),
        })
        .await
        .expect("b refreshes c after subscribe");
    runtime_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("c refreshes a after subscribe");
    runtime_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("c refreshes b after subscribe");
    wait_for_connected_topic_peer_count(
        &runtime_a,
        topic,
        2,
        "friend-plus owner topic mesh timeout",
    )
    .await;
    wait_for_connected_topic_peer_count(
        &runtime_b,
        topic,
        2,
        "friend-plus sponsor topic mesh timeout",
    )
    .await;
    wait_for_connected_topic_peer_count(
        &runtime_c,
        topic,
        2,
        "friend-plus recipient topic mesh timeout",
    )
    .await;
    // Relay-assisted sync is sufficient for the downstream share import and private-channel
    // replication assertions in this test. Slower CI hosts can remain assist-only here even
    // after ticket refresh, so keep the ticket re-imports above but rely on the actual
    // friend-plus restore/share assertions below instead of requiring direct topic peers.
    warm_author_social_view(
        &runtime_b,
        c_pubkey.as_str(),
        "friend-plus sponsor recipient author warm timeout",
    )
    .await;
    warm_author_social_view(
        &runtime_c,
        b_pubkey.as_str(),
        "friend-plus recipient sponsor author warm timeout",
    )
    .await;
    runtime_b
        .follow_author(AuthorRequest {
            pubkey: c_pubkey.clone(),
        })
        .await
        .expect("b follows c");
    runtime_c
        .follow_author(AuthorRequest {
            pubkey: b_pubkey.clone(),
        })
        .await
        .expect("c follows b");
    wait_for_mutual_author_view(&runtime_b, c_pubkey.as_str(), topic).await;
    wait_for_mutual_author_view(&runtime_c, b_pubkey.as_str(), topic).await;
    let share_bc = runtime_b
        .export_friend_plus_share(ExportFriendPlusShareRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export b->c share");
    let preview_c = runtime_c
        .import_friend_plus_share(ImportFriendPlusShareRequest { token: share_bc })
        .await
        .expect("c imports friend-plus share");
    let original_epoch_id = preview_c.epoch_id.clone();
    assert_eq!(preview_c.sponsor_pubkey.as_str(), b_pubkey.as_str());
    // Importing the fresh share updates C's joined private-channel state after A and B have
    // already built their active topic/private subscriptions, so refresh the tickets once more
    // to rebuild those subscriptions against the new friend-plus epoch before the first write.
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_c.clone(),
        })
        .await
        .expect("a refreshes c after friend-plus share");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_c.clone(),
        })
        .await
        .expect("b refreshes c after friend-plus share");
    runtime_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("c refreshes a after friend-plus share");
    runtime_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("c refreshes b after friend-plus share");

    let private_scope = TimelineScope::Channel {
        channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
    };
    let private_ref = ChannelRef::PrivateChannel {
        channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
    };
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe friend-plus private b");
    let _ = runtime_c
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe friend-plus private c");
    let joined_a_before_history = wait_for_joined_private_channel_epoch(
        &runtime_a,
        topic,
        channel.channel_id.as_str(),
        original_epoch_id.as_str(),
        3,
        "friend-plus owner private readiness timeout",
    )
    .await;
    assert_eq!(joined_a_before_history.participant_count, 3);
    let joined_b_before_history = wait_for_joined_private_channel_epoch(
        &runtime_b,
        topic,
        channel.channel_id.as_str(),
        original_epoch_id.as_str(),
        3,
        "friend-plus sponsor private readiness timeout",
    )
    .await;
    assert_eq!(
        joined_b_before_history.joined_via_pubkey.as_deref(),
        Some(a_pubkey.as_str())
    );
    assert_eq!(joined_b_before_history.participant_count, 3);
    let joined_c_before_history = wait_for_joined_private_channel_epoch(
        &runtime_c,
        topic,
        channel.channel_id.as_str(),
        original_epoch_id.as_str(),
        3,
        "friend-plus recipient private readiness timeout",
    )
    .await;
    assert_eq!(
        joined_c_before_history.joined_via_pubkey.as_deref(),
        Some(b_pubkey.as_str())
    );
    assert_eq!(joined_c_before_history.participant_count, 3);
    let old_post_id = replicate_private_post_with_retry(
        &runtime_a,
        &[&runtime_b, &runtime_c],
        topic,
        &private_scope,
        &private_ref,
        "friend-plus history",
        "friend-plus history propagation timeout",
    )
    .await;

    let public_timeline_c = runtime_c
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("public timeline c");
    assert!(
        public_timeline_c
            .items
            .iter()
            .all(|post| post.object_id != old_post_id),
        "friend-plus post leaked into public timeline"
    );

    let joined_before_restart = runtime_c
        .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
            topic: topic.into(),
        })
        .await
        .expect("joined channels before restart");
    assert_eq!(joined_before_restart.len(), 1);
    assert_eq!(joined_before_restart[0].channel_id, channel.channel_id);
    let restored_epoch_id = joined_before_restart[0].current_epoch_id.clone();
    assert_ne!(restored_epoch_id, original_epoch_id);
    assert_eq!(
        joined_before_restart[0].joined_via_pubkey.as_deref(),
        Some(b_pubkey.as_str())
    );

    timeout(Duration::from_secs(30), runtime_c.shutdown())
        .await
        .expect("runtime c shutdown timeout");
    drop(runtime_c);
    delete_sqlite_artifacts(&db_c);

    let restarted_c = DesktopRuntime::new_with_config_and_identity(
        &db_c,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart runtime c");
    restarted_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("restarted c imports a");
    restarted_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("restarted c imports b");
    let restarted_ticket_c = restarted_c
        .local_peer_ticket()
        .await
        .expect("restarted ticket c")
        .expect("restarted ticket c value");
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: restarted_ticket_c.clone(),
        })
        .await
        .expect("a imports restarted c");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: restarted_ticket_c.clone(),
        })
        .await
        .expect("b imports restarted c");
    let _ = restarted_c
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe restarted c public");
    let _ = restarted_c
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe restarted c private");
    // Re-importing tickets forces existing topic subscriptions to rebuild against C's new endpoint.
    restarted_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("restarted c refreshes a");
    restarted_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("restarted c refreshes b");
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: restarted_ticket_c.clone(),
        })
        .await
        .expect("a refreshes restarted c");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: restarted_ticket_c.clone(),
        })
        .await
        .expect("b refreshes restarted c");
    let joined_after_restart = restarted_c
        .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
            topic: topic.into(),
        })
        .await
        .expect("joined channels after restart");
    assert_eq!(joined_after_restart.len(), 1);
    assert_eq!(joined_after_restart[0].channel_id, channel.channel_id);
    assert_eq!(joined_after_restart[0].current_epoch_id, restored_epoch_id);
    assert_eq!(
        joined_after_restart[0].joined_via_pubkey.as_deref(),
        Some(b_pubkey.as_str())
    );

    let private_timeline_after_restart = restarted_c
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("private timeline after restart");
    assert!(
        private_timeline_after_restart
            .items
            .iter()
            .any(|post| post.object_id == old_post_id)
    );
    let joined_restarted_before_rotate = wait_for_joined_private_channel_epoch(
        &restarted_c,
        topic,
        channel.channel_id.as_str(),
        restored_epoch_id.as_str(),
        3,
        "friend-plus restarted private readiness timeout",
    )
    .await;
    assert_eq!(
        joined_restarted_before_rotate.joined_via_pubkey.as_deref(),
        Some(b_pubkey.as_str())
    );
    assert_eq!(joined_restarted_before_rotate.participant_count, 3);

    wait_for_connected_topic_peer_count(
        &runtime_a,
        topic,
        1,
        "friend-plus owner topic readiness timeout",
    )
    .await;
    wait_for_connected_topic_peer_count(
        &runtime_b,
        topic,
        1,
        "friend-plus sponsor topic readiness timeout",
    )
    .await;

    let rotated = runtime_a
        .rotate_private_channel(RotatePrivateChannelRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
        })
        .await
        .expect("rotate friend-plus channel");
    assert_ne!(rotated.current_epoch_id, restored_epoch_id);

    let refreshed_share_ab = runtime_a
        .export_friend_plus_share(ExportFriendPlusShareRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export refreshed a->b share after rotate");
    let preview_b_after_rotate = runtime_b
        .import_friend_plus_share(ImportFriendPlusShareRequest {
            token: refreshed_share_ab,
        })
        .await
        .expect("b imports refreshed friend-plus share");
    let shared_epoch_id = preview_b_after_rotate.epoch_id.clone();
    assert_ne!(shared_epoch_id, restored_epoch_id);
    assert_eq!(
        preview_b_after_rotate.sponsor_pubkey.as_str(),
        a_pubkey.as_str()
    );
    let joined_b_after_rotate = wait_for_joined_private_channel_epoch(
        &runtime_b,
        topic,
        channel.channel_id.as_str(),
        shared_epoch_id.as_str(),
        2,
        "friend-plus sponsor refresh share redeem timeout",
    )
    .await;
    assert_eq!(
        joined_b_after_rotate.joined_via_pubkey.as_deref(),
        Some(a_pubkey.as_str())
    );
    assert!(
        joined_b_after_rotate
            .archived_epoch_ids
            .iter()
            .any(|epoch_id| epoch_id == &restored_epoch_id)
    );

    let fresh_share = runtime_b
        .export_friend_plus_share(ExportFriendPlusShareRequest {
            topic: topic.into(),
            channel_id: channel.channel_id.clone(),
            expires_at: None,
        })
        .await
        .expect("export fresh friend-plus share after restart");
    let preview_after_restart = restarted_c
        .import_friend_plus_share(ImportFriendPlusShareRequest { token: fresh_share })
        .await
        .expect("restarted c imports fresh friend-plus share");
    assert_eq!(preview_after_restart.epoch_id, shared_epoch_id);
    assert_eq!(
        preview_after_restart.sponsor_pubkey.as_str(),
        b_pubkey.as_str()
    );
    let joined_after_rotate = wait_for_joined_private_channel_epoch(
        &restarted_c,
        topic,
        channel.channel_id.as_str(),
        shared_epoch_id.as_str(),
        3,
        "friend-plus restarted share redeem timeout",
    )
    .await;
    assert_eq!(
        joined_after_rotate.joined_via_pubkey.as_deref(),
        Some(b_pubkey.as_str())
    );
    assert_eq!(joined_after_rotate.participant_count, 3);
    assert!(
        joined_after_rotate
            .archived_epoch_ids
            .iter()
            .any(|epoch_id| epoch_id == &restored_epoch_id)
    );
    wait_for_connected_topic_peer_count(
        &restarted_c,
        topic,
        1,
        "friend-plus restarted topic reconnect timeout",
    )
    .await;
    restarted_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.clone(),
        })
        .await
        .expect("restarted c refreshes a after rotate");
    restarted_c
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.clone(),
        })
        .await
        .expect("restarted c refreshes b after rotate");
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: restarted_ticket_c.clone(),
        })
        .await
        .expect("a refreshes restarted c after rotate");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: restarted_ticket_c.clone(),
        })
        .await
        .expect("b refreshes restarted c after rotate");
    let _ = restarted_c
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("resubscribe restarted c private after fresh share");

    let restarted_post_id = restarted_c
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "friend-plus restarted after rotate".into(),
            reply_to: None,
            channel_ref: private_ref.clone(),
            attachments: vec![],
        })
        .await
        .expect("restarted c creates friend-plus rotated post");
    match timeout(runtime_replication_timeout(), async {
        loop {
            let public_timeline = restarted_c
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("public timeline after rotate");
            assert!(
                public_timeline
                    .items
                    .iter()
                    .all(|post| post.object_id != restarted_post_id),
                "friend-plus rotated post leaked into public timeline"
            );
            let private_timeline = restarted_c
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("private timeline after rotate");
            if private_timeline
                .items
                .iter()
                .any(|post| post.object_id == restarted_post_id)
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
            let restarted_status = restarted_c.get_sync_status().await.expect("status c");
            let joined = restarted_c
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.into(),
                })
                .await
                .unwrap_or_default();
            let private_timeline = restarted_c
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: private_scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .unwrap_or_else(|_| TimelineView {
                    items: vec![],
                    next_cursor: None,
                });
            panic!(
                "friend-plus restarted rotated post visibility timeout: restarted={} joined={joined:?} private_items={:?}",
                format_sync_snapshot(&restarted_status, topic),
                private_timeline
                    .items
                    .iter()
                    .map(|item| item.object_id.clone())
                    .collect::<Vec<_>>()
            );
        }
    }

    timeout(Duration::from_secs(30), runtime_a.shutdown())
        .await
        .expect("runtime a shutdown timeout");
    timeout(Duration::from_secs(30), runtime_b.shutdown())
        .await
        .expect("runtime b shutdown timeout");
    timeout(Duration::from_secs(30), restarted_c.shutdown())
        .await
        .expect("restarted runtime c shutdown timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_discovery_seeds_reapplies_runtime_without_restart() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("seeded-a.db");
    let db_b = dir.path().join("seeded-b.db");
    let testnet = Testnet::new(5).expect("testnet");
    let runtime_a = new_seeded_dht_runtime(&db_a, &testnet).await;
    let runtime_b = new_seeded_dht_runtime(&db_b, &testnet).await;
    let endpoint_a = runtime_a
        .get_sync_status()
        .await
        .expect("status a")
        .discovery
        .local_endpoint_id;
    let endpoint_b = runtime_b
        .get_sync_status()
        .await
        .expect("status b")
        .discovery
        .local_endpoint_id;

    runtime_a
        .set_discovery_seeds(SetDiscoverySeedsRequest {
            seed_entries: vec![endpoint_b.clone()],
        })
        .await
        .expect("set seeds a");
    runtime_b
        .set_discovery_seeds(SetDiscoverySeedsRequest {
            seed_entries: vec![endpoint_a.clone()],
        })
        .await
        .expect("set seeds b");

    let config_a = runtime_a
        .get_discovery_config()
        .await
        .expect("discovery config a");
    let config_b = runtime_b
        .get_discovery_config()
        .await
        .expect("discovery config b");
    assert_eq!(config_a.seed_peers[0].endpoint_id, endpoint_b);
    assert_eq!(config_b.seed_peers[0].endpoint_id, endpoint_a);
    let topic = "kukuri:topic:runtime-seeded-dht";
    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe a");
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe b");
    wait_for_seeded_dht_topic_ready(&runtime_a, &runtime_b, topic).await;
    let status_a = runtime_a
        .get_sync_status()
        .await
        .expect("status a after seeds");
    let status_b = runtime_b
        .get_sync_status()
        .await
        .expect("status b after seeds");
    assert!(
        status_a
            .subscribed_topics
            .iter()
            .any(|entry| entry == topic)
    );
    assert!(
        status_b
            .subscribed_topics
            .iter()
            .any(|entry| entry == topic)
    );
    assert!(status_a.topic_diagnostics.iter().any(|entry| {
        entry.topic == topic
            && entry.joined
            && entry.peer_count > 0
            && (!entry.connected_peers.is_empty() || !entry.assist_peer_ids.is_empty())
    }));
    assert!(status_b.topic_diagnostics.iter().any(|entry| {
        entry.topic == topic
            && entry.joined
            && entry.peer_count > 0
            && (!entry.connected_peers.is_empty() || !entry.assist_peer_ids.is_empty())
    }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_restores_seeded_dht_config_and_endpoint_identity() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("restart-seeded-a.db");
    let db_b = dir.path().join("restart-seeded-b.db");
    let testnet = Testnet::new(5).expect("testnet");
    let runtime_a = new_seeded_dht_runtime(&db_a, &testnet).await;
    let runtime_b = new_seeded_dht_runtime(&db_b, &testnet).await;
    let endpoint_a = runtime_a
        .get_sync_status()
        .await
        .expect("status a")
        .discovery
        .local_endpoint_id;
    let endpoint_b = runtime_b
        .get_sync_status()
        .await
        .expect("status b")
        .discovery
        .local_endpoint_id;

    runtime_a
        .set_discovery_seeds(SetDiscoverySeedsRequest {
            seed_entries: vec![endpoint_b.clone()],
        })
        .await
        .expect("set seeds a");
    runtime_b
        .set_discovery_seeds(SetDiscoverySeedsRequest {
            seed_entries: vec![endpoint_a.clone()],
        })
        .await
        .expect("set seeds b");

    timeout(Duration::from_secs(15), runtime_a.shutdown())
        .await
        .expect("shutdown a");
    timeout(Duration::from_secs(15), runtime_b.shutdown())
        .await
        .expect("shutdown b");
    drop(runtime_a);
    drop(runtime_b);

    let restored_a = resolve_discovery_config_from_env(&db_a).expect("restored discovery config a");
    let restored_b = resolve_discovery_config_from_env(&db_b).expect("restored discovery config b");
    let restarted_a = new_seeded_dht_runtime_with_config(&db_a, &testnet, restored_a.clone()).await;
    let restarted_b = new_seeded_dht_runtime_with_config(&db_b, &testnet, restored_b.clone()).await;
    let restarted_endpoint_a = restarted_a
        .get_sync_status()
        .await
        .expect("restarted status a")
        .discovery
        .local_endpoint_id;
    let restarted_endpoint_b = restarted_b
        .get_sync_status()
        .await
        .expect("restarted status b")
        .discovery
        .local_endpoint_id;

    assert_eq!(restored_a.mode, DiscoveryMode::SeededDht);
    assert_eq!(restored_b.mode, DiscoveryMode::SeededDht);
    assert_eq!(restored_a.seed_peers[0].endpoint_id, endpoint_b);
    assert_eq!(restored_b.seed_peers[0].endpoint_id, endpoint_a);
    assert_eq!(restarted_endpoint_a, endpoint_a);
    assert_eq!(restarted_endpoint_b, endpoint_b);
    let topic = "kukuri:topic:runtime-seeded-restart";
    let _ = restarted_a
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe restarted a");
    let _ = restarted_b
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("subscribe restarted b");
    let status_a = restarted_a.get_sync_status().await.expect("sync status a");
    let status_b = restarted_b.get_sync_status().await.expect("sync status b");
    assert_eq!(status_a.discovery.mode, DiscoveryMode::SeededDht);
    assert_eq!(status_b.discovery.mode, DiscoveryMode::SeededDht);
    assert_eq!(status_a.discovery.local_endpoint_id, endpoint_a);
    assert_eq!(status_b.discovery.local_endpoint_id, endpoint_b);
    assert_eq!(
        status_a.discovery.configured_seed_peer_ids,
        vec![endpoint_b]
    );
    assert_eq!(
        status_b.discovery.configured_seed_peer_ids,
        vec![endpoint_a]
    );
    assert!(status_a.subscribed_topics.iter().any(|item| item == topic));
    assert!(status_b.subscribed_topics.iter().any(|item| item == topic));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_seed_entry_rejected_without_mutating_runtime() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("invalid-seed.db");
    let testnet = Testnet::new(5).expect("testnet");
    let runtime = new_seeded_dht_runtime(&db_path, &testnet).await;

    let error = runtime
        .set_discovery_seeds(SetDiscoverySeedsRequest {
            seed_entries: vec!["not-a-node-id".into()],
        })
        .await
        .expect_err("invalid seed should fail");
    assert!(error.to_string().contains("invalid seed endpoint id"));

    let config = runtime
        .get_discovery_config()
        .await
        .expect("discovery config");
    assert!(config.seed_peers.is_empty());
    assert!(!discovery_config_path(&db_path).exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_joiner_backfills_timeline_from_docs() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("late-a.db");
    let db_b = dir.path().join("late-b.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let topic = "kukuri:topic:late-join";
    let object_id = runtime_a
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "hello from before join".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("create post before join");
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");

    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
        .await
        .expect("import a into b");

    let received = timeout(Duration::from_secs(10), async {
        loop {
            let timeline = runtime_b
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("timeline b");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("late join timeout");

    assert_eq!(received.content, "hello from before join");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_joiner_backfills_image_post_from_docs() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("late-image-a.db");
    let db_b = dir.path().join("late-image-b.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let topic = "kukuri:topic:late-image-runtime";
    let object_id = runtime_a
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "late image".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![image_attachment_request(
                "late.png",
                "image/png",
                b"late-image-runtime",
            )],
        })
        .await
        .expect("create image post before join");
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");

    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
        .await
        .expect("import a into b");

    let received = timeout(Duration::from_secs(10), async {
        loop {
            let timeline = runtime_b
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("timeline b");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("late image timeout");

    assert_eq!(received.attachments.len(), 1);
    let preview = runtime_b
        .get_blob_preview_url(GetBlobPreviewRequest {
            hash: received.attachments[0].hash.clone(),
            mime: received.attachments[0].mime.clone(),
        })
        .await
        .expect("blob preview");
    assert!(preview.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_joiner_backfills_video_media_payload() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("late-video-a.db");
    let db_b = dir.path().join("late-video-b.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let topic = "kukuri:topic:late-video-runtime";
    let object_id = runtime_a
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "late video".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![
                video_attachment_request(
                    "late-video.mp4",
                    "video/mp4",
                    b"late-video-runtime",
                    "video_manifest",
                ),
                video_attachment_request(
                    "late-poster.jpg",
                    "image/jpeg",
                    b"late-video-poster",
                    "video_poster",
                ),
            ],
        })
        .await
        .expect("create video post before join");
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");

    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
        .await
        .expect("import a into b");

    let received = timeout(Duration::from_secs(10), async {
        loop {
            let timeline = runtime_b
                .list_timeline(ListTimelineRequest {
                    topic: topic.into(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("timeline b");
            if let Some(post) = timeline
                .items
                .iter()
                .find(|post| post.object_id == object_id)
            {
                return post.clone();
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("late video timeout");

    let poster = received
        .attachments
        .iter()
        .find(|attachment| attachment.role == "video_poster")
        .expect("video poster");
    let preview = runtime_b
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: poster.hash.clone(),
            mime: poster.mime.clone(),
        })
        .await
        .expect("video poster payload");
    assert!(preview.is_some());
    let manifest = received
        .attachments
        .iter()
        .find(|attachment| attachment.role == "video_manifest")
        .expect("video manifest");
    let playback = runtime_b
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: manifest.hash.clone(),
            mime: manifest.mime.clone(),
        })
        .await
        .expect("video playback payload");
    assert!(playback.is_some());
}

#[tokio::test]
async fn blob_media_payload_roundtrip() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("blob-media-roundtrip.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let topic = "kukuri:topic:blob-media-roundtrip";
    let object_id = runtime
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "roundtrip".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![image_attachment_request(
                "roundtrip.png",
                "image/png",
                b"blob-media-roundtrip",
            )],
        })
        .await
        .expect("create image post");
    let timeline = runtime
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline");
    let created = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("created post");

    let payload = runtime
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: created.attachments[0].hash.clone(),
            mime: created.attachments[0].mime.clone(),
        })
        .await
        .expect("blob media payload")
        .expect("blob media payload present");

    assert_eq!(payload.mime, "image/png");
    assert_eq!(
        payload.bytes_base64,
        BASE64_STANDARD.encode(b"blob-media-roundtrip")
    );
}

#[tokio::test]
async fn blank_blob_media_hash_returns_none_without_panicking() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("blank-blob-media-hash.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let payload = runtime
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: "   ".into(),
            mime: "image/png".into(),
        })
        .await
        .expect("blank hash payload");

    assert!(payload.is_none());
}

#[tokio::test]
async fn sqlite_deletion_does_not_lose_shared_state() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("delete-sqlite.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let topic = "kukuri:topic:sqlite-delete";
    let root_id = runtime
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "root".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("root post");
    let reply_id = runtime
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "reply".into(),
            reply_to: Some(root_id.clone()),
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("reply post");
    runtime.shutdown().await;
    drop(runtime);
    delete_sqlite_artifacts(&db_path);

    let restarted = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart");
    let timeline = restarted
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline");
    let thread = restarted
        .list_thread(ListThreadRequest {
            topic: topic.into(),
            thread_id: root_id.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("thread");

    assert!(timeline.items.iter().any(|post| post.object_id == root_id));
    assert!(timeline.items.iter().any(|post| post.object_id == reply_id));
    assert!(thread.items.iter().any(|post| post.object_id == root_id));
    assert!(thread.items.iter().any(|post| post.object_id == reply_id));
}

#[tokio::test]
async fn restart_restores_from_docs_blobs_without_sqlite_seed() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("restart-no-seed.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let topic = "kukuri:topic:restart-no-seed";
    let object_id = runtime
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "restored from docs".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![],
        })
        .await
        .expect("create post");
    runtime.shutdown().await;
    drop(runtime);
    delete_sqlite_artifacts(&db_path);

    let restarted = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart");
    let timeline = restarted
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline");

    let restored = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("restored post");
    assert_eq!(restored.content, "restored from docs");
}

#[tokio::test]
async fn restart_restores_image_post_preview() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("restart-image.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let topic = "kukuri:topic:restart-image";
    let object_id = runtime
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "restored image".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![image_attachment_request(
                "restored.png",
                "image/png",
                b"restart-image-preview",
            )],
        })
        .await
        .expect("create image post");
    runtime.shutdown().await;
    drop(runtime);
    delete_sqlite_artifacts(&db_path);

    let restarted = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart");
    let timeline = restarted
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline");
    let restored = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("restored image post");

    assert_eq!(restored.attachments.len(), 1);
    let preview = restarted
        .get_blob_preview_url(GetBlobPreviewRequest {
            hash: restored.attachments[0].hash.clone(),
            mime: restored.attachments[0].mime.clone(),
        })
        .await
        .expect("preview after restart");
    assert!(preview.is_some());
}

#[tokio::test]
async fn restart_restores_video_media_payload() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("restart-video.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let topic = "kukuri:topic:restart-video";
    let object_id = runtime
        .create_post(CreatePostRequest {
            topic: topic.into(),
            content: "restored video".into(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![
                video_attachment_request(
                    "clip.mp4",
                    "video/mp4",
                    b"restart-video-manifest",
                    "video_manifest",
                ),
                video_attachment_request(
                    "clip-poster.jpg",
                    "image/jpeg",
                    b"restart-video-poster",
                    "video_poster",
                ),
            ],
        })
        .await
        .expect("create video post");
    runtime.shutdown().await;
    drop(runtime);
    delete_sqlite_artifacts(&db_path);

    let restarted = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart");
    let timeline = restarted
        .list_timeline(ListTimelineRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline");
    let restored = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("restored video post");

    let poster = restored
        .attachments
        .iter()
        .find(|attachment| attachment.role == "video_poster")
        .expect("restored poster");
    let preview = restarted
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: poster.hash.clone(),
            mime: poster.mime.clone(),
        })
        .await
        .expect("video payload after restart");
    assert!(preview.is_some());
    let manifest = restored
        .attachments
        .iter()
        .find(|attachment| attachment.role == "video_manifest")
        .expect("restored video manifest");
    let playback = restarted
        .get_blob_media_payload(GetBlobMediaRequest {
            hash: manifest.hash.clone(),
            mime: manifest.mime.clone(),
        })
        .await
        .expect("video playback payload after restart");
    assert!(playback.is_some());
}

#[tokio::test]
async fn restart_restores_live_session_manifest() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("restart-live.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let topic = "kukuri:topic:restart-live";
    let session_id = runtime
        .create_live_session(CreateLiveSessionRequest {
            topic: topic.into(),
            channel_ref: ChannelRef::Public,
            title: "restart live".into(),
            description: "session".into(),
        })
        .await
        .expect("create live session");
    runtime
        .join_live_session(LiveSessionCommandRequest {
            topic: topic.into(),
            session_id: session_id.clone(),
        })
        .await
        .expect("join live session");
    runtime
        .end_live_session(LiveSessionCommandRequest {
            topic: topic.into(),
            session_id: session_id.clone(),
        })
        .await
        .expect("end live session");
    runtime.shutdown().await;
    drop(runtime);
    delete_sqlite_artifacts(&db_path);

    let restarted = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart");
    let sessions = restarted
        .list_live_sessions(ListLiveSessionsRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
        })
        .await
        .expect("list live sessions");
    let restored = sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .expect("restored live session");
    assert_eq!(restored.status, kukuri_core::LiveSessionStatus::Ended);
    assert_eq!(restored.viewer_count, 0);
    assert!(!restored.joined_by_me);
}

#[tokio::test]
async fn restart_restores_game_room_manifest() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("restart-game.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let topic = "kukuri:topic:restart-game";
    let room_id = runtime
        .create_game_room(CreateGameRoomRequest {
            topic: topic.into(),
            channel_ref: ChannelRef::Public,
            title: "restart finals".into(),
            description: "set".into(),
            participants: vec!["Alice".into(), "Bob".into()],
        })
        .await
        .expect("create game room");
    runtime
        .update_game_room(UpdateGameRoomRequest {
            topic: topic.into(),
            room_id: room_id.clone(),
            status: GameRoomStatus::Running,
            phase_label: Some("Round 3".into()),
            scores: vec![
                GameScoreView {
                    participant_id: "participant-1".into(),
                    label: "Alice".into(),
                    score: 2,
                },
                GameScoreView {
                    participant_id: "participant-2".into(),
                    label: "Bob".into(),
                    score: 1,
                },
            ],
        })
        .await
        .expect("update game room");
    runtime.shutdown().await;
    drop(runtime);
    delete_sqlite_artifacts(&db_path);

    let restarted = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restart");
    let rooms = restarted
        .list_game_rooms(ListGameRoomsRequest {
            topic: topic.into(),
            scope: TimelineScope::Public,
        })
        .await
        .expect("list game rooms");
    let restored = rooms
        .iter()
        .find(|room| room.room_id == room_id)
        .expect("restored game room");
    assert_eq!(restored.status, GameRoomStatus::Running);
    assert_eq!(restored.phase_label.as_deref(), Some("Round 3"));
    assert_eq!(
        restored
            .scores
            .iter()
            .find(|score| score.label == "Alice")
            .map(|score| score.score),
        Some(2)
    );
}

#[test]
fn community_node_config_normalizes_base_urls_and_connectivity_urls() {
    let config = normalize_community_node_config(CommunityNodeConfig {
        nodes: vec![
            CommunityNodeNodeConfig {
                base_url: "https://community.example.com/".into(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://public.example.com/",
                        vec![
                            "https://relay-b.example.com/".into(),
                            "https://relay-a.example.com/".into(),
                            "https://relay-a.example.com/".into(),
                        ],
                        vec![CommunityNodeSeedPeer::new("peer-b", None).expect("seed peer")],
                    )
                    .expect("resolved urls"),
                ),
            },
            CommunityNodeNodeConfig {
                base_url: "https://community.example.com".into(),
                resolved_urls: None,
            },
        ],
    })
    .expect("normalized config");

    assert_eq!(config.nodes.len(), 1);
    assert_eq!(config.nodes[0].base_url, "https://community.example.com");
    assert_eq!(
        config.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .connectivity_urls,
        vec![
            "https://relay-a.example.com".to_string(),
            "https://relay-b.example.com".to_string(),
        ]
    );
    assert_eq!(
        config.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![CommunityNodeSeedPeer::new("peer-b", None).expect("seed peer")]
    );
}

#[test]
fn community_node_config_preserves_public_kukuri_urls() {
    let config = normalize_community_node_config(CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: "https://api.kukuri.app/".into(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(
                    "https://api.kukuri.app/",
                    vec!["https://iroh-relay.kukuri.app/".into()],
                    Vec::new(),
                )
                .expect("resolved urls"),
            ),
        }],
    })
    .expect("normalized config");

    let resolved = config.nodes[0]
        .resolved_urls
        .as_ref()
        .expect("resolved urls");

    assert_eq!(config.nodes[0].base_url, "https://api.kukuri.app");
    assert_eq!(resolved.public_base_url, "https://api.kukuri.app");
    assert_eq!(
        resolved.connectivity_urls,
        vec!["https://iroh-relay.kukuri.app".to_string()]
    );
    assert!(
        resolved
            .connectivity_urls
            .iter()
            .all(|url| !url.contains("api.kukuri.app/relay"))
    );
}

#[test]
fn stored_community_node_config_restores_cached_connectivity_union() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-relay.db");
    save_community_node_config(
        &db_path,
        &CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: "https://community.example.com".into(),
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://public.example.com",
                        vec!["https://relay.example.com".into()],
                        vec![CommunityNodeSeedPeer::new("peer-a", None).expect("seed peer")],
                    )
                    .expect("resolved urls"),
                ),
            }],
        },
    )
    .expect("save community node config");
    let restored = load_community_node_config_from_file(&db_path)
        .expect("load community node config")
        .expect("community node config");
    let relay_config = relay_config_from_community_node_config(&restored);

    assert_eq!(relay_config.connect_mode(), ConnectMode::DirectOrRelay);
    assert_eq!(
        relay_config.iroh_relay_urls,
        vec!["https://relay.example.com".to_string()]
    );
    assert_eq!(
        restored.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![CommunityNodeSeedPeer::new("peer-a", None).expect("seed peer")]
    );
}

#[tokio::test]
async fn community_node_status_does_not_require_restart_when_connectivity_is_active() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-status.db");
    let test_timeout = Duration::from_secs(15);
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let base_url = "https://community.example.com".to_string();
    let connectivity_url = "http://127.0.0.1:9".to_string();
    let resolved_urls = CommunityNodeResolvedUrls::new(
        base_url.clone(),
        vec![connectivity_url.clone()],
        Vec::new(),
    )
    .expect("resolved urls");
    let node = CommunityNodeNodeConfig {
        base_url: base_url.clone(),
        resolved_urls: Some(resolved_urls.clone()),
    };
    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![node.clone()],
    };
    *runtime.active_connectivity_urls.lock().await = vec![connectivity_url.clone()];

    let status = timeout(
        test_timeout,
        runtime.community_node_status(
            node,
            Some(CommunityNodeConsentStatus {
                all_required_accepted: true,
                items: vec![kukuri_cn_core::CommunityNodeConsentItem {
                    policy_slug: "community-basic".to_string(),
                    policy_version: 1,
                    title: "Community Basic".to_string(),
                    required: true,
                    accepted_at: Some(Utc::now().timestamp()),
                }],
            }),
            None,
        ),
    )
    .await
    .expect("community-node status timeout")
    .expect("community-node status");
    assert!(status.auth_state.authenticated);
    assert!(
        status
            .consent_state
            .as_ref()
            .expect("consent state")
            .all_required_accepted
    );
    assert_eq!(
        status
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .connectivity_urls,
        vec![connectivity_url]
    );
    assert!(!status.restart_required);

    timeout(test_timeout, runtime.shutdown())
        .await
        .expect("runtime shutdown timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn community_node_connectivity_assist_syncs_public_timeline_without_manual_tickets() {
    let _serial = acquire_async_test_lock().await;
    let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
        .await
        .expect("relay server");
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("community-relay-a.db");
    let db_b = dir.path().join("community-relay-b.db");
    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");

    let endpoint_a = runtime_a
        .get_sync_status()
        .await
        .expect("status a")
        .discovery
        .local_endpoint_id;
    let endpoint_b = runtime_b
        .get_sync_status()
        .await
        .expect("status b")
        .discovery
        .local_endpoint_id;
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = runtime_b
        .local_peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    let addr_hint_a = ticket_a
        .split_once('@')
        .map(|(_, addr)| addr.to_string())
        .expect("addr hint a");
    let addr_hint_b = ticket_b
        .split_once('@')
        .map(|(_, addr)| addr.to_string())
        .expect("addr hint b");
    let base_url = "https://community.example.com";

    *runtime_a.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.to_string(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(
                    base_url,
                    vec![relay_url.to_string()],
                    vec![
                        CommunityNodeSeedPeer::new(endpoint_b.as_str(), Some(addr_hint_b.clone()))
                            .expect("seed peer b"),
                    ],
                )
                .expect("resolved urls a"),
            ),
        }],
    };
    *runtime_b.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.to_string(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(
                    base_url,
                    vec![relay_url.to_string()],
                    vec![
                        CommunityNodeSeedPeer::new(endpoint_a.as_str(), Some(addr_hint_a.clone()))
                            .expect("seed peer a"),
                    ],
                )
                .expect("resolved urls b"),
            ),
        }],
    };

    timeout(
        Duration::from_secs(15),
        runtime_a.apply_runtime_connectivity_assist(),
    )
    .await
    .expect("apply assist a timeout")
    .expect("apply assist a");
    timeout(
        Duration::from_secs(15),
        runtime_a.apply_effective_seed_peers(),
    )
    .await
    .expect("apply seed peers a timeout")
    .expect("apply seed peers a");
    timeout(
        Duration::from_secs(15),
        runtime_b.apply_runtime_connectivity_assist(),
    )
    .await
    .expect("apply assist b timeout")
    .expect("apply assist b");
    timeout(
        Duration::from_secs(15),
        runtime_b.apply_effective_seed_peers(),
    )
    .await
    .expect("apply seed peers b timeout")
    .expect("apply seed peers b");

    let topic = "kukuri:topic:community-node-relay-assist";
    let scope = TimelineScope::Public;
    let _ = timeout(
        Duration::from_secs(15),
        runtime_a.list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
            cursor: None,
            limit: Some(20),
        }),
    )
    .await
    .expect("subscribe a timeout")
    .expect("subscribe a");
    let _ = timeout(
        Duration::from_secs(15),
        runtime_b.list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
            cursor: None,
            limit: Some(20),
        }),
    )
    .await
    .expect("subscribe b timeout")
    .expect("subscribe b");

    wait_for_direct_public_pair_with_refresh_result(
        &runtime_a,
        &runtime_b,
        topic,
        Duration::from_secs(15),
        false,
    )
    .await
    .expect("community-node assist direct topic readiness timeout");

    let _object_id = replicate_public_post_from_original_publisher_with_retry(
        &runtime_a,
        &runtime_b,
        topic,
        "community relay hello",
        "community-node assist forward post sync timeout",
    )
    .await;
    let _reverse_object_id = replicate_public_post_from_original_publisher_with_retry(
        &runtime_b,
        &runtime_a,
        topic,
        "community relay reverse hello",
        "community-node assist reverse post sync timeout",
    )
    .await;

    timeout(runtime_shutdown_timeout(), runtime_a.shutdown())
        .await
        .expect("runtime a shutdown timeout");
    timeout(runtime_shutdown_timeout(), runtime_b.shutdown())
        .await
        .expect("runtime b shutdown timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn community_node_connectivity_assist_syncs_public_timeline_with_shared_identity() {
    let _serial = acquire_async_test_lock().await;
    let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
        .await
        .expect("relay server");
    let dir = tempdir().expect("tempdir");
    let db_a = dir.path().join("community-relay-shared-a.db");
    let db_b = dir.path().join("community-relay-shared-b.db");
    let shared_keys = KukuriKeys::generate();
    let shared_secret = shared_keys.export_secret_hex();
    fs::write(
        db_a.with_extension("identity-key"),
        shared_secret.as_bytes(),
    )
    .expect("persist shared identity key a");
    fs::write(db_a.with_extension("identity-store"), b"file")
        .expect("persist shared identity backend a");
    fs::write(
        db_b.with_extension("identity-key"),
        shared_secret.as_bytes(),
    )
    .expect("persist shared identity key b");
    fs::write(db_b.with_extension("identity-store"), b"file")
        .expect("persist shared identity backend b");

    let runtime_a = DesktopRuntime::new_with_config_and_identity(
        &db_a,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime a");
    let runtime_b = DesktopRuntime::new_with_config_and_identity(
        &db_b,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime b");

    let status_a = runtime_a.get_sync_status().await.expect("status a");
    let status_b = runtime_b.get_sync_status().await.expect("status b");
    assert_eq!(status_a.local_author_pubkey, status_b.local_author_pubkey);

    let endpoint_a = status_a.discovery.local_endpoint_id;
    let endpoint_b = status_b.discovery.local_endpoint_id;
    let ticket_a = runtime_a
        .local_peer_ticket()
        .await
        .expect("ticket a")
        .expect("ticket a value");
    let ticket_b = runtime_b
        .local_peer_ticket()
        .await
        .expect("ticket b")
        .expect("ticket b value");
    let addr_hint_a = ticket_a
        .split_once('@')
        .map(|(_, addr)| addr.to_string())
        .expect("addr hint a");
    let addr_hint_b = ticket_b
        .split_once('@')
        .map(|(_, addr)| addr.to_string())
        .expect("addr hint b");
    let base_url = "https://community.example.com";

    *runtime_a.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.to_string(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(
                    base_url,
                    vec![relay_url.to_string()],
                    vec![
                        CommunityNodeSeedPeer::new(endpoint_b.as_str(), Some(addr_hint_b.clone()))
                            .expect("seed peer b"),
                    ],
                )
                .expect("resolved urls a"),
            ),
        }],
    };
    *runtime_b.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.to_string(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(
                    base_url,
                    vec![relay_url.to_string()],
                    vec![
                        CommunityNodeSeedPeer::new(endpoint_a.as_str(), Some(addr_hint_a.clone()))
                            .expect("seed peer a"),
                    ],
                )
                .expect("resolved urls b"),
            ),
        }],
    };

    timeout(
        Duration::from_secs(15),
        runtime_a.apply_runtime_connectivity_assist(),
    )
    .await
    .expect("apply assist a timeout")
    .expect("apply assist a");
    timeout(
        Duration::from_secs(15),
        runtime_a.apply_effective_seed_peers(),
    )
    .await
    .expect("apply seed peers a timeout")
    .expect("apply seed peers a");
    timeout(
        Duration::from_secs(15),
        runtime_b.apply_runtime_connectivity_assist(),
    )
    .await
    .expect("apply assist b timeout")
    .expect("apply assist b");
    timeout(
        Duration::from_secs(15),
        runtime_b.apply_effective_seed_peers(),
    )
    .await
    .expect("apply seed peers b timeout")
    .expect("apply seed peers b");

    let topic = "kukuri:topic:community-node-relay-assist-shared";
    let scope = TimelineScope::Public;
    let _ = timeout(
        Duration::from_secs(15),
        runtime_a.list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
            cursor: None,
            limit: Some(20),
        }),
    )
    .await
    .expect("subscribe a timeout")
    .expect("subscribe a");
    let _ = timeout(
        Duration::from_secs(15),
        runtime_b.list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: scope.clone(),
            cursor: None,
            limit: Some(20),
        }),
    )
    .await
    .expect("subscribe b timeout")
    .expect("subscribe b");

    wait_for_direct_public_pair_with_refresh_result(
        &runtime_a,
        &runtime_b,
        topic,
        Duration::from_secs(15),
        true,
    )
    .await
    .expect("community-node assist shared direct topic readiness timeout");

    let _object_id = replicate_public_post_from_original_publisher_with_retry(
        &runtime_a,
        &runtime_b,
        topic,
        "community relay shared hello",
        "community-node assist shared identity forward post sync timeout",
    )
    .await;
    let _reverse_object_id = replicate_public_post_from_original_publisher_with_retry(
        &runtime_b,
        &runtime_a,
        topic,
        "community relay shared reverse hello",
        "community-node assist shared identity reverse post sync timeout",
    )
    .await;

    timeout(runtime_shutdown_timeout(), runtime_a.shutdown())
        .await
        .expect("runtime a shutdown timeout");
    timeout(runtime_shutdown_timeout(), runtime_b.shutdown())
        .await
        .expect("runtime b shutdown timeout");
}

#[tokio::test]
async fn community_node_status_refresh_updates_bootstrap_seed_peers() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-heartbeat-refresh.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(vec![
            CommunityNodeSeedPeer::new(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                None,
            )
            .expect("seed peer"),
        ])),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let statuses = runtime
        .get_community_node_statuses()
        .await
        .expect("community node statuses");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(statuses.len(), 1);
    assert_eq!(
        statuses[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        state.seed_peers.lock().await.clone()
    );
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        state.seed_peers.lock().await.clone()
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_sync_status_refresh_updates_bootstrap_seed_peers() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-sync-status-refresh.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(vec![
            CommunityNodeSeedPeer::new(
                "1111111111111111111111111111111111111111111111111111111111111111",
                None,
            )
            .expect("seed peer"),
        ])),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let _status = runtime.get_sync_status().await.expect("sync status");

    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        state.seed_peers.lock().await.clone()
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_status_retries_bootstrap_metadata_when_seed_peers_are_empty() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-metadata-retry.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let seed_peer = CommunityNodeSeedPeer::new(
        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        None,
    )
    .expect("seed peer");
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(Vec::new())),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let initial_statuses = runtime
        .get_community_node_statuses()
        .await
        .expect("initial community node statuses");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        initial_statuses[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        Vec::<CommunityNodeSeedPeer>::new()
    );
    assert!(
        runtime
            .community_node_metadata_refresh_deadlines
            .lock()
            .await
            .contains_key(base_url.as_str()),
        "empty bootstrap metadata should schedule a retry"
    );

    *state.seed_peers.lock().await = vec![seed_peer.clone()];
    runtime
        .community_node_metadata_refresh_deadlines
        .lock()
        .await
        .insert(base_url.clone(), Utc::now().timestamp() - 1);

    let refreshed_statuses = runtime
        .get_community_node_statuses()
        .await
        .expect("refreshed community node statuses");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 2);
    assert_eq!(
        refreshed_statuses[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![seed_peer]
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn refresh_community_node_metadata_refreshes_registration_before_bootstrap_sync() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-refresh-heartbeat.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let refreshed_seed_peer = CommunityNodeSeedPeer::new(
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        Some("127.0.0.1:44003".into()),
    )
    .expect("refreshed seed peer");
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(Vec::new())),
        heartbeat_seed_peers: Arc::new(Mutex::new(Some(vec![refreshed_seed_peer.clone()]))),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let status = runtime
        .refresh_community_node_metadata(CommunityNodeTargetRequest {
            base_url: base_url.clone(),
        })
        .await
        .expect("refresh metadata");

    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert!(
        state.bootstrap_hits.load(Ordering::SeqCst) >= 1,
        "metadata refresh should fetch bootstrap nodes"
    );
    assert_eq!(
        status
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![refreshed_seed_peer.clone()]
    );
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![refreshed_seed_peer]
    );

    runtime.shutdown().await;
    server.abort();
}
