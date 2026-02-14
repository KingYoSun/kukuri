use anyhow::{anyhow, Result};
use base64::prelude::*;
use cn_core::{metrics, topic};
use futures_util::StreamExt;
use iroh::{protocol::Router, Endpoint, SecretKey};
use iroh_gossip::{api::Event, Gossip, TopicId};
use sqlx::postgres::PgListener;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, RwLock};

use crate::config::RelayRuntimeConfig;
use crate::ingest::{ingest_event, IngestContext, IngestSource};
use crate::{AppState, RelayConfig};

const DEFAULT_BOOTSTRAP_HINT_NOTIFY_CHANNEL: &str = "cn_bootstrap_hint";

#[derive(Debug, serde::Deserialize)]
struct BootstrapHintPayload {
    #[serde(default)]
    changed_topic_ids: Vec<String>,
}

pub async fn start_gossip(state: AppState, config: RelayConfig) -> Result<()> {
    let endpoint = build_endpoint(&config).await?;
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let _router = Router::builder(endpoint)
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    let senders = Arc::clone(&state.gossip_senders);
    let node_topics = Arc::clone(&state.node_topics);
    let tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let poll_interval = Duration::from_secs(config.topic_poll_seconds);

    let sync_state = state.clone();
    tokio::spawn(async move {
        loop {
            if let Err(err) =
                sync_topics(&sync_state, &gossip, &senders, &tasks, &node_topics).await
            {
                tracing::warn!(error = %err, "gossip topic sync failed");
            }
            tokio::time::sleep(poll_interval).await;
        }
    });

    let _bootstrap_hint_ready = spawn_bootstrap_hint_bridge(state.clone());

    Ok(())
}

async fn build_endpoint(config: &RelayConfig) -> Result<Endpoint> {
    let mut builder = Endpoint::builder();
    builder = apply_bind(builder, config.p2p_bind_addr);
    if let Some(secret) = &config.p2p_secret_key {
        let decoded = BASE64_STANDARD
            .decode(secret.trim())
            .map_err(|e| anyhow!("invalid relay p2p secret key: {e}"))?;
        if decoded.len() != 32 {
            return Err(anyhow!("relay p2p secret key must be 32 bytes"));
        }
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&decoded);
        builder = builder.secret_key(SecretKey::from_bytes(&buf));
    }
    let endpoint = builder.bind().await?;
    Ok(endpoint)
}

fn apply_bind(builder: iroh::endpoint::Builder, addr: SocketAddr) -> iroh::endpoint::Builder {
    match addr {
        SocketAddr::V4(v4) => builder.bind_addr_v4(v4),
        SocketAddr::V6(v6) => builder.bind_addr_v6(v6),
    }
}

async fn sync_topics(
    state: &AppState,
    gossip: &Gossip,
    senders: &Arc<RwLock<HashMap<String, iroh_gossip::api::GossipSender>>>,
    tasks: &Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    node_topics: &Arc<RwLock<HashSet<String>>>,
) -> Result<()> {
    let desired = load_node_topics(&state.pool).await?;
    {
        let mut guard = node_topics.write().await;
        *guard = desired.clone();
    }

    let mut current = {
        let guard = senders.read().await;
        guard.keys().cloned().collect::<HashSet<_>>()
    };

    let to_add: Vec<String> = desired.difference(&current).cloned().collect();
    for topic_id in to_add {
        let sender_handle = {
            let topic_bytes = topic::topic_id_to_gossip_bytes(&topic_id)?;
            let topic = gossip.subscribe(TopicId::from(topic_bytes), vec![]).await?;
            let (sender, mut receiver) = topic.split();
            let ingest_state = state.clone();
            let topic_clone = topic_id.clone();
            let handle = tokio::spawn(async move {
                while let Some(result) = receiver.next().await {
                    match result {
                        Ok(Event::Received(message)) => {
                            metrics::inc_gossip_received(super::SERVICE_NAME);
                            let runtime = ingest_state.config.get().await;
                            let runtime = RelayRuntimeConfig::from_json(&runtime.config_json);
                            if runtime.rate_limit.enabled {
                                let key = format!("peer:{}", message.delivered_from);
                                let outcome = ingest_state
                                    .rate_limiter
                                    .check(
                                        &key,
                                        runtime.rate_limit.gossip_msgs_per_minute,
                                        Duration::from_secs(60),
                                    )
                                    .await;
                                if !outcome.allowed {
                                    metrics::inc_ingest_rejected(super::SERVICE_NAME, "ratelimit");
                                    continue;
                                }
                            }
                            if let Ok(value) =
                                serde_json::from_slice::<serde_json::Value>(&message.content)
                            {
                                if let Ok(raw) = cn_core::nostr::parse_event(&value) {
                                    let context = IngestContext {
                                        auth_pubkey: None,
                                        source_topic: Some(topic_clone.clone()),
                                        peer_id: Some(message.delivered_from.to_string()),
                                    };
                                    if let Ok(crate::ingest::IngestOutcome::Accepted {
                                        event,
                                        duplicate,
                                        ..
                                    }) = ingest_event(
                                        &ingest_state,
                                        raw,
                                        IngestSource::Gossip,
                                        context,
                                    )
                                    .await
                                    {
                                        if !duplicate {
                                            let _ = ingest_state.realtime_tx.send(event);
                                        }
                                    }
                                }
                            }
                        }
                        Ok(Event::Lagged) => {
                            tracing::warn!(topic = %topic_clone, "gossip receiver lagged");
                        }
                        Ok(_) => {}
                        Err(err) => {
                            tracing::warn!(topic = %topic_clone, error = %err, "gossip receive error");
                            break;
                        }
                    }
                }
            });
            (sender, handle)
        };

        {
            let mut sender_guard = senders.write().await;
            sender_guard.insert(topic_id.clone(), sender_handle.0);
        }
        {
            let mut task_guard = tasks.write().await;
            task_guard.insert(topic_id.clone(), sender_handle.1);
        }
        current.insert(topic_id);
    }

    for topic_id in current.difference(&desired).cloned().collect::<Vec<_>>() {
        if let Some(handle) = tasks.write().await.remove(&topic_id) {
            handle.abort();
        }
        senders.write().await.remove(&topic_id);
    }

    Ok(())
}

fn bootstrap_hint_notify_channel() -> String {
    std::env::var("RELAY_BOOTSTRAP_HINT_CHANNEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_BOOTSTRAP_HINT_NOTIFY_CHANNEL.to_string())
}

pub(crate) fn spawn_bootstrap_hint_bridge(state: AppState) -> oneshot::Receiver<()> {
    let (ready_tx, ready_rx) = oneshot::channel();
    tokio::spawn(async move {
        let channel = bootstrap_hint_notify_channel();
        let mut listener = match PgListener::connect_with(&state.pool).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::warn!(error = %err, "bootstrap hint bridge failed to connect listener");
                return;
            }
        };
        if let Err(err) = listener.listen(&channel).await {
            tracing::warn!(error = %err, channel = %channel, "bootstrap hint bridge failed to listen");
            return;
        }
        let _ = ready_tx.send(());

        loop {
            let notification = match listener.recv().await {
                Ok(notification) => notification,
                Err(err) => {
                    tracing::warn!(error = %err, "bootstrap hint bridge receive error");
                    continue;
                }
            };

            let hint: BootstrapHintPayload = match serde_json::from_str(notification.payload()) {
                Ok(payload) => payload,
                Err(err) => {
                    tracing::debug!(error = %err, payload = notification.payload(), "skip invalid bootstrap hint payload");
                    continue;
                }
            };

            for topic_id in hint.changed_topic_ids {
                if let Err(err) = publish_bootstrap_events_to_topic(&state, &topic_id).await {
                    tracing::warn!(error = %err, topic_id = %topic_id, "bootstrap hint bridge publish failed");
                }
            }
        }
    });
    ready_rx
}

async fn publish_bootstrap_events_to_topic(state: &AppState, topic_id: &str) -> Result<()> {
    let sender = {
        let guard = state.gossip_senders.read().await;
        guard.get(topic_id).cloned()
    };
    let Some(sender) = sender else {
        return Ok(());
    };

    let rows = sqlx::query(
        "SELECT event_json FROM cn_bootstrap.events
         WHERE is_active = TRUE
           AND expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT
           AND (
               (kind = 39000 AND d_tag = 'descriptor')
               OR (kind = 39001 AND topic_id = $1)
           )",
    )
    .bind(topic_id)
    .fetch_all(&state.pool)
    .await?;

    for row in rows {
        let value: serde_json::Value = row.try_get("event_json")?;
        let payload = serde_json::to_vec(&value)?;
        if send_with_retry(&sender, payload).await {
            metrics::inc_gossip_sent(super::SERVICE_NAME);
        }
    }

    Ok(())
}

async fn send_with_retry(sender: &iroh_gossip::api::GossipSender, payload: Vec<u8>) -> bool {
    const RETRIES: usize = 3;
    let mut attempt = 0;
    loop {
        match sender.broadcast(payload.clone().into()).await {
            Ok(_) => return true,
            Err(err) => {
                attempt += 1;
                if attempt >= RETRIES {
                    tracing::debug!(error = %err, "bootstrap hint bridge broadcast retries exhausted");
                    return false;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
}

async fn load_node_topics(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<HashSet<String>> {
    let rows = sqlx::query("SELECT topic_id FROM cn_admin.node_subscriptions WHERE enabled = TRUE")
        .fetch_all(pool)
        .await?;
    let mut topics = HashSet::new();
    for row in rows {
        let topic_id: String = row.try_get("topic_id")?;
        topics.insert(topic_id);
    }
    Ok(topics)
}
