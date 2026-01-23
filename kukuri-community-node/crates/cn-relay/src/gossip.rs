use anyhow::{anyhow, Result};
use base64::prelude::*;
use cn_core::{metrics, topic};
use futures_util::StreamExt;
use iroh::{protocol::Router, Endpoint, SecretKey};
use iroh_gossip::{api::Event, Gossip, TopicId};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::config::RelayRuntimeConfig;
use crate::ingest::{ingest_event, IngestContext, IngestSource};
use crate::{AppState, RelayConfig};

pub async fn start_gossip(state: AppState, config: RelayConfig) -> Result<()> {
    let endpoint = build_endpoint(&config).await?;
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let _router = Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();

    let senders = Arc::clone(&state.gossip_senders);
    let node_topics = Arc::clone(&state.node_topics);
    let tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let poll_interval = Duration::from_secs(config.topic_poll_seconds);

    tokio::spawn(async move {
        loop {
            if let Err(err) = sync_topics(&state, &gossip, &senders, &tasks, &node_topics).await {
                tracing::warn!(error = %err, "gossip topic sync failed");
            }
            tokio::time::sleep(poll_interval).await;
        }
    });

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

fn apply_bind(mut builder: iroh::endpoint::Builder, addr: SocketAddr) -> iroh::endpoint::Builder {
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

    for topic_id in desired.difference(&current) {
        let topic_id = topic_id.to_string();
        let sender_handle = {
            let topic_bytes = topic::topic_id_to_gossip_bytes(&topic_id)?;
            let mut topic = gossip.subscribe(TopicId::from(topic_bytes), vec![]).await?;
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
                            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(
                                &message.content,
                            ) {
                                if let Ok(raw) = cn_core::nostr::parse_event(&value) {
                                    let context = IngestContext {
                                        auth_pubkey: None,
                                        source_topic: Some(topic_clone.clone()),
                                        peer_id: Some(message.delivered_from.to_string()),
                                    };
                                    if let Ok(ingest) = ingest_event(
                                        &ingest_state,
                                        raw,
                                        IngestSource::Gossip,
                                        context,
                                    )
                                    .await
                                    {
                                        if let crate::ingest::IngestOutcome::Accepted { event, duplicate, .. } = ingest
                                        {
                                            if !duplicate {
                                                let _ = ingest_state.realtime_tx.send(event);
                                            }
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

async fn load_node_topics(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<HashSet<String>> {
    let rows = sqlx::query(
        "SELECT topic_id FROM cn_admin.node_subscriptions WHERE enabled = TRUE",
    )
    .fetch_all(pool)
    .await?;
    let mut topics = HashSet::new();
    for row in rows {
        let topic_id: String = row.try_get("topic_id")?;
        topics.insert(topic_id);
    }
    Ok(topics)
}
