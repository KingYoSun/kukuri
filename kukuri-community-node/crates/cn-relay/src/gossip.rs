use anyhow::{anyhow, Result};
use base64::prelude::*;
use cn_core::{metrics, topic};
use futures_util::StreamExt;
use iroh::{
    address_lookup::MemoryLookup,
    endpoint::QuicTransportConfig, protocol::Router, Endpoint, EndpointAddr, EndpointId, RelayMode,
    RelayUrl, SecretKey,
};
use iroh_gossip::{
    api::{Event, GossipTopic},
    Gossip, TopicId,
};
use sqlx::postgres::PgListener;
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, RwLock};

use crate::config::RelayRuntimeConfig;
use crate::ingest::{ingest_event, IngestContext, IngestSource};
use crate::{AppState, RelayConfig};

const DEFAULT_BOOTSTRAP_HINT_NOTIFY_CHANNEL: &str = "cn_bootstrap_hint";
const TOPIC_SUBSCRIBE_MAX_RETRIES: usize = 3;
const GOSSIP_JOIN_RESULT_SUCCESS: &str = "success";
const GOSSIP_JOIN_RESULT_FAILURE: &str = "failure";
const GOSSIP_JOIN_REASON_OK: &str = "ok";
const GOSSIP_JOIN_REASON_SUBSCRIBE_FAILED: &str = "subscribe_failed";
const GOSSIP_JOIN_REASON_SUBSCRIBE_RETRY: &str = "subscribe_retry";
const GOSSIP_JOIN_REASON_SEED_RESOLUTION_FAILED: &str = "seed_resolution_failed";
const RELAY_IROH_TRANSPORT_PROFILE_ENV: &str = "RELAY_IROH_TRANSPORT_PROFILE";
const ENDPOINT_ONLINE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndpointTransportProfile {
    Default,
    RelayOnly,
}

impl EndpointTransportProfile {
    fn from_env_value(raw: Option<&str>) -> Result<Self> {
        let Some(raw) = raw else {
            return Ok(Self::Default);
        };

        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "default" => Ok(Self::Default),
            "relay-only" | "relay_only" => Ok(Self::RelayOnly),
            other => Err(anyhow!(
                "invalid {RELAY_IROH_TRANSPORT_PROFILE_ENV} value `{other}`"
            )),
        }
    }

    fn apply_to_builder(self, builder: iroh::endpoint::Builder) -> iroh::endpoint::Builder {
        match self {
            // Relay-only mode hides direct address hints and disables direct discovery,
            // but keeps local sockets bound so the endpoint can fully initialize on the relay.
            Self::Default | Self::RelayOnly => builder,
        }
    }

    fn quic_transport_config(self) -> Option<QuicTransportConfig> {
        match self {
            Self::Default => None,
            Self::RelayOnly => Some(
                QuicTransportConfig::builder()
                    .enable_segmentation_offload(false)
                    .send_observed_address_reports(false)
                    .receive_observed_address_reports(false)
                    .build(),
            ),
        }
    }

    fn allows_direct_ip_bind(self) -> bool {
        true
    }
}

#[derive(Debug, serde::Deserialize)]
struct BootstrapHintPayload {
    #[serde(default)]
    changed_topic_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct ResolvedSeedPeer {
    node_id: EndpointId,
    node_addr: Option<EndpointAddr>,
}

pub async fn start_gossip(state: AppState, config: RelayConfig) -> Result<()> {
    let endpoint = build_endpoint(&state, &config).await?;
    wait_for_endpoint_online(&endpoint).await;
    let node_id = endpoint.id().to_string();
    {
        let mut guard = state.p2p_node_id.write().await;
        *guard = Some(node_id.clone());
    }
    tracing::info!(
        node_id = %node_id,
        bind_addr = %config.p2p_bind_addr,
        "relay p2p endpoint initialized"
    );
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let router = Arc::new(
        Router::builder(endpoint.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .spawn(),
    );
    {
        let mut guard = state.p2p_router.write().await;
        *guard = Some(router);
    }

    let senders = Arc::clone(&state.gossip_senders);
    let node_topics = Arc::clone(&state.node_topics);
    let tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let poll_interval = Duration::from_secs(config.topic_poll_seconds);

    let sync_state = state.clone();
    let sync_endpoint = endpoint.clone();
    tokio::spawn(async move {
        loop {
            if let Err(err) = sync_topics(
                &sync_state,
                &sync_endpoint,
                &gossip,
                &senders,
                &tasks,
                &node_topics,
            )
            .await
            {
                tracing::warn!(error = %err, "gossip topic sync failed");
            }
            tokio::time::sleep(poll_interval).await;
        }
    });

    let _bootstrap_hint_ready = spawn_bootstrap_hint_bridge(state.clone());

    Ok(())
}

async fn wait_for_endpoint_online(endpoint: &Endpoint) {
    match tokio::time::timeout(ENDPOINT_ONLINE_TIMEOUT, endpoint.online()).await {
        Ok(()) => {
            let addr = endpoint.addr();
            tracing::info!(
                endpoint_id = %endpoint.id(),
                relay_urls = ?addr.relay_urls().collect::<Vec<_>>(),
                ip_addrs = ?addr.ip_addrs().collect::<Vec<_>>(),
                "relay p2p endpoint reported online"
            );
        }
        Err(_) => {
            tracing::warn!(
                endpoint_id = %endpoint.id(),
                timeout_secs = ENDPOINT_ONLINE_TIMEOUT.as_secs(),
                "timed out waiting for relay p2p endpoint to report online"
            );
        }
    }
}

async fn build_endpoint(state: &AppState, config: &RelayConfig) -> Result<Endpoint> {
    let relay_mode = resolve_relay_mode(config)?;
    let transport_profile = resolve_transport_profile()?;
    let mut builder = transport_profile
        .apply_to_builder(Endpoint::empty_builder(relay_mode))
        .address_lookup(state.p2p_address_lookup.as_ref().clone());
    if let Some(transport_config) = transport_profile.quic_transport_config() {
        builder = builder.transport_config(transport_config);
    }
    if transport_profile.allows_direct_ip_bind() {
        builder = apply_bind(builder, config.p2p_bind_addr)?;
    }
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

fn resolve_relay_mode(config: &RelayConfig) -> Result<RelayMode> {
    if config.p2p_relay_mode_default || config.p2p_relay_urls.is_empty() {
        return Ok(RelayMode::Default);
    }

    let mut relay_urls = Vec::new();
    for raw in &config.p2p_relay_urls {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let relay_url = RelayUrl::from_str(trimmed)
            .map_err(|err| anyhow!("invalid RELAY_IROH_RELAY_URLS entry `{trimmed}`: {err}"))?;
        relay_urls.push(relay_url);
    }

    if relay_urls.is_empty() {
        return Ok(RelayMode::Default);
    }

    Ok(RelayMode::custom(relay_urls))
}

fn resolve_transport_profile() -> Result<EndpointTransportProfile> {
    EndpointTransportProfile::from_env_value(
        std::env::var(RELAY_IROH_TRANSPORT_PROFILE_ENV)
            .ok()
            .as_deref(),
    )
}

fn apply_bind(
    builder: iroh::endpoint::Builder,
    addr: SocketAddr,
) -> Result<iroh::endpoint::Builder> {
    match addr {
        SocketAddr::V4(v4) => builder.bind_addr(v4).map_err(|e| anyhow!(e)),
        SocketAddr::V6(v6) => builder.bind_addr(v6).map_err(|e| anyhow!(e)),
    }
}

async fn sync_topics(
    state: &AppState,
    endpoint: &Endpoint,
    gossip: &Gossip,
    senders: &Arc<RwLock<HashMap<String, iroh_gossip::api::GossipSender>>>,
    tasks: &Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    node_topics: &Arc<RwLock<HashSet<String>>>,
) -> Result<()> {
    let runtime_snapshot = state.config.get().await;
    let runtime = RelayRuntimeConfig::from_json(&runtime_snapshot.config_json);
    let desired =
        load_node_topics(&state.pool, runtime.node_subscription.max_concurrent_topics).await?;
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
        let seed_peers = match resolve_seed_peers_for_topic(state, &topic_id).await {
            Ok(seed_peers) => seed_peers,
            Err(err) => {
                metrics::inc_gossip_join_total(
                    super::SERVICE_NAME,
                    GOSSIP_JOIN_RESULT_FAILURE,
                    GOSSIP_JOIN_REASON_SEED_RESOLUTION_FAILED,
                );
                tracing::warn!(
                    error = %err,
                    topic = %topic_id,
                    "failed to resolve seed peers for gossip join; proceeding without seeds"
                );
                Vec::new()
            }
        };
        let seed_with_addr = seed_peers
            .iter()
            .filter(|seed_peer| seed_peer.node_addr.is_some())
            .count();
        tracing::debug!(
            topic = %topic_id,
            seed_count = seed_peers.len(),
            seed_with_addr = seed_with_addr,
            "resolved gossip join seed peers"
        );
        let registered_seed_addrs =
            register_seed_peer_addrs(state.p2p_address_lookup.as_ref(), &seed_peers);
        tracing::debug!(
            topic = %topic_id,
            registered_seed_addrs = registered_seed_addrs,
            "registered gossip join seed peer addresses"
        );
        let preconnected_seed_peers = preconnect_seed_peers(endpoint, &topic_id, &seed_peers).await;
        tracing::debug!(
            topic = %topic_id,
            preconnected_seed_peers = preconnected_seed_peers,
            "preconnected gossip join seed peers"
        );
        let seed_peer_ids = seed_peers
            .iter()
            .map(|peer| peer.node_id.clone())
            .collect::<Vec<_>>();

        let sender_handle = {
            let topic = subscribe_topic_with_retry(gossip, &topic_id, &seed_peer_ids).await?;
            let (sender, mut receiver) = topic.split();
            if !seed_peer_ids.is_empty() {
                sender
                    .join_peers(seed_peer_ids.clone())
                    .await
                    .map_err(|err| {
                        anyhow!(
                            "failed to join gossip seed peers for topic `{topic_id}`: {err}"
                        )
                    })?;
                tracing::debug!(
                    topic = %topic_id,
                    seed_peer_count = seed_peer_ids.len(),
                    "issued gossip join_peers for resolved seed peers"
                );
            }
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
        remove_topic_runtime(&topic_id, senders, tasks).await;
    }

    Ok(())
}

fn register_seed_peer_addrs(
    address_lookup: &MemoryLookup,
    seed_peers: &[ResolvedSeedPeer],
) -> usize {
    let mut registered = 0usize;
    for seed_peer in seed_peers {
        if let Some(node_addr) = seed_peer.node_addr.clone() {
            address_lookup.add_endpoint_info(node_addr);
            registered += 1;
        }
    }
    registered
}

async fn preconnect_seed_peers(
    endpoint: &Endpoint,
    topic_id: &str,
    seed_peers: &[ResolvedSeedPeer],
) -> usize {
    let mut connected = 0usize;
    let mut seen = HashSet::new();

    for seed_peer in seed_peers {
        let Some(node_addr) = seed_peer.node_addr.clone() else {
            continue;
        };
        if !seen.insert(node_addr.id) {
            continue;
        }

        match tokio::time::timeout(
            Duration::from_secs(5),
            endpoint.connect(node_addr.clone(), iroh_gossip::ALPN),
        )
        .await
        {
            Ok(Ok(_connection)) => {
                connected += 1;
                tracing::debug!(
                    topic = %topic_id,
                    peer = %node_addr.id,
                    "preconnected gossip seed peer"
                );
            }
            Ok(Err(err)) => {
                tracing::warn!(
                    topic = %topic_id,
                    peer = %node_addr.id,
                    error = %err,
                    "failed to preconnect gossip seed peer"
                );
            }
            Err(_) => {
                tracing::warn!(
                    topic = %topic_id,
                    peer = %node_addr.id,
                    "timed out preconnecting gossip seed peer"
                );
            }
        }
    }

    connected
}

async fn remove_topic_runtime(
    topic_id: &str,
    senders: &Arc<RwLock<HashMap<String, iroh_gossip::api::GossipSender>>>,
    tasks: &Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
) {
    if let Some(handle) = tasks.write().await.remove(topic_id) {
        handle.abort();
    }
    senders.write().await.remove(topic_id);
}

async fn resolve_seed_peers_for_topic(
    state: &AppState,
    topic_id: &str,
) -> Result<Vec<ResolvedSeedPeer>> {
    let rows = sqlx::query(
        "SELECT event_json
         FROM cn_bootstrap.events
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

    let local_node_id = state
        .p2p_node_id
        .read()
        .await
        .as_ref()
        .and_then(|value| EndpointId::from_str(value).ok());

    let mut seen_node_ids = HashSet::new();
    let mut seed_peers = Vec::new();
    for row in rows {
        let event_json: serde_json::Value = row.try_get("event_json")?;
        for hint in collect_p2p_hints_from_bootstrap_event(&event_json) {
            match parse_seed_peer_hint(&hint) {
                Ok(seed_peer) => {
                    if local_node_id.as_ref() == Some(&seed_peer.node_id) {
                        continue;
                    }
                    let node_key = seed_peer.node_id.to_string();
                    if seen_node_ids.insert(node_key) {
                        seed_peers.push(seed_peer);
                    }
                }
                Err(err) => {
                    tracing::debug!(
                        topic = %topic_id,
                        hint = %hint,
                        error = %err,
                        "skip invalid bootstrap seed hint"
                    );
                }
            }
        }
    }

    Ok(seed_peers)
}

fn collect_p2p_hints_from_bootstrap_event(event_json: &serde_json::Value) -> Vec<String> {
    let mut hints = Vec::new();
    let Some(content_raw) = event_json.get("content").and_then(|value| value.as_str()) else {
        return hints;
    };
    let Ok(content) = serde_json::from_str::<serde_json::Value>(content_raw.trim()) else {
        return hints;
    };

    let p2p = content
        .pointer("/endpoints/p2p")
        .or_else(|| content.get("p2p"));
    match p2p {
        Some(serde_json::Value::String(value)) => {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                hints.push(trimmed.to_string());
            }
        }
        Some(serde_json::Value::Array(values)) => {
            for value in values {
                if let Some(raw) = value.as_str() {
                    let trimmed = raw.trim();
                    if !trimmed.is_empty() {
                        hints.push(trimmed.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    hints
}

async fn subscribe_topic_with_retry(
    gossip: &Gossip,
    topic_id: &str,
    bootstrap_peers: &[EndpointId],
) -> Result<GossipTopic> {
    let started_at = Instant::now();
    let mut attempt = 0usize;

    loop {
        attempt += 1;
        let topic_bytes = topic::topic_id_to_gossip_bytes(topic_id)?;
        let subscribe_result = gossip
            .subscribe(TopicId::from(topic_bytes), bootstrap_peers.to_vec())
            .await;

        match subscribe_result {
            Ok(topic) => {
                tracing::debug!(
                    topic = %topic_id,
                    bootstrap_peer_count = bootstrap_peers.len(),
                    attempt = attempt,
                    "subscribed gossip topic without blocking on immediate neighbor establishment"
                );
                metrics::inc_gossip_join_total(
                    super::SERVICE_NAME,
                    GOSSIP_JOIN_RESULT_SUCCESS,
                    GOSSIP_JOIN_REASON_OK,
                );
                metrics::observe_gossip_join_convergence(
                    super::SERVICE_NAME,
                    GOSSIP_JOIN_RESULT_SUCCESS,
                    started_at.elapsed(),
                );
                return Ok(topic);
            }
            Err(err) => {
                if attempt >= TOPIC_SUBSCRIBE_MAX_RETRIES {
                    metrics::inc_gossip_join_total(
                        super::SERVICE_NAME,
                        GOSSIP_JOIN_RESULT_FAILURE,
                        GOSSIP_JOIN_REASON_SUBSCRIBE_FAILED,
                    );
                    metrics::observe_gossip_join_convergence(
                        super::SERVICE_NAME,
                        GOSSIP_JOIN_RESULT_FAILURE,
                        started_at.elapsed(),
                    );
                    return Err(anyhow!(
                        "failed to subscribe gossip topic `{topic_id}` after {attempt} attempts: {err}"
                    ));
                }

                metrics::inc_gossip_join_retry(
                    super::SERVICE_NAME,
                    GOSSIP_JOIN_REASON_SUBSCRIBE_RETRY,
                );
                tracing::warn!(
                    topic = %topic_id,
                    attempt = attempt,
                    error = %err,
                    "gossip subscribe failed; retrying"
                );
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
}

fn parse_seed_peer_hint(value: &str) -> Result<ResolvedSeedPeer> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("peer hint is empty"));
    }

    if !trimmed.contains('|') {
        if let Some((node_part, addr_part)) = trimmed.split_once('@') {
            let node_id = EndpointId::from_str(node_part.trim())
                .map_err(|err| anyhow!("invalid node id `{node_part}`: {err}"))?;
            let socket_addrs = resolve_socket_addrs(addr_part)?;
            let node_addr = build_endpoint_addr(node_id.clone(), socket_addrs, Vec::new());
            return Ok(ResolvedSeedPeer { node_id, node_addr });
        }

        let node_id = EndpointId::from_str(trimmed)
            .map_err(|err| anyhow!("invalid node id `{trimmed}`: {err}"))?;
        return Ok(ResolvedSeedPeer {
            node_id,
            node_addr: None,
        });
    }

    let mut segments = trimmed
        .split('|')
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty());
    let first = segments
        .next()
        .ok_or_else(|| anyhow!("peer hint is missing node id"))?;

    let (node_id, initial_addr) = if let Some((node_part, addr_part)) = first.split_once('@') {
        let node_id = EndpointId::from_str(node_part.trim())
            .map_err(|err| anyhow!("invalid node id `{node_part}`: {err}"))?;
        (node_id, Some(addr_part.trim()))
    } else if first.contains('=') {
        return Err(anyhow!("peer hint is missing node id before attributes"));
    } else {
        let node_id = EndpointId::from_str(first.trim())
            .map_err(|err| anyhow!("invalid node id `{first}`: {err}"))?;
        (node_id, None)
    };

    let mut socket_addrs = Vec::new();
    if let Some(addr_part) = initial_addr {
        socket_addrs.extend(resolve_socket_addrs(addr_part)?);
    }
    let mut relay_urls = Vec::new();
    for segment in segments {
        let (raw_key, raw_value) = segment
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid hint segment `{segment}`"))?;
        let key = raw_key.trim().to_ascii_lowercase();
        let value = raw_value.trim();
        if value.is_empty() {
            return Err(anyhow!("empty value in hint segment `{segment}`"));
        }

        match key.as_str() {
            "addr" | "ip" => {
                socket_addrs.extend(resolve_socket_addrs(value)?);
            }
            "relay" | "relay_url" => {
                let relay_url = RelayUrl::from_str(value)
                    .map_err(|err| anyhow!("invalid relay url `{value}`: {err}"))?;
                if !relay_urls.contains(&relay_url) {
                    relay_urls.push(relay_url);
                }
            }
            "node" | "node_id" => {
                let hinted_node_id = EndpointId::from_str(value)
                    .map_err(|err| anyhow!("invalid node id in hint `{value}`: {err}"))?;
                if hinted_node_id != node_id {
                    return Err(anyhow!("conflicting node ids in hint `{trimmed}`"));
                }
            }
            _ => {
                return Err(anyhow!("unsupported hint key `{key}`"));
            }
        }
    }

    let node_addr = build_endpoint_addr(node_id.clone(), socket_addrs, relay_urls);
    Ok(ResolvedSeedPeer { node_id, node_addr })
}

fn build_endpoint_addr(
    node_id: EndpointId,
    socket_addrs: Vec<SocketAddr>,
    relay_urls: Vec<RelayUrl>,
) -> Option<EndpointAddr> {
    if socket_addrs.is_empty() && relay_urls.is_empty() {
        return None;
    }

    let mut endpoint_addr = EndpointAddr::new(node_id);
    for relay_url in relay_urls {
        endpoint_addr = endpoint_addr.with_relay_url(relay_url);
    }
    for socket_addr in socket_addrs {
        endpoint_addr = endpoint_addr.with_ip_addr(socket_addr);
    }
    Some(endpoint_addr)
}

fn resolve_socket_addrs(raw: &str) -> Result<Vec<SocketAddr>> {
    let trimmed = raw.trim();
    if let Ok(socket_addr) = trimmed.parse::<SocketAddr>() {
        return Ok(vec![socket_addr]);
    }

    let (host, port_raw) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("invalid socket address `{raw}`"))?;
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return Err(anyhow!("invalid host in socket address `{raw}`"));
    }
    let port: u16 = port_raw
        .trim()
        .parse()
        .map_err(|err| anyhow!("invalid port `{port_raw}`: {err}"))?;

    if host.eq_ignore_ascii_case("localhost") {
        return Ok(vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)]);
    }

    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|err| anyhow!("failed to resolve host `{host}`: {err}"))?
        .collect::<Vec<_>>();
    let prioritized = prioritize_socket_addrs(addrs);
    if prioritized.is_empty() {
        return Err(anyhow!(
            "resolved host `{host}` but no socket addresses were returned"
        ));
    }

    Ok(prioritized)
}

fn prioritize_socket_addrs(addrs: Vec<SocketAddr>) -> Vec<SocketAddr> {
    let mut unique = Vec::new();
    for addr in addrs {
        if !unique.contains(&addr) {
            unique.push(addr);
        }
    }

    let mut ipv4 = Vec::new();
    let mut other = Vec::new();
    for addr in unique {
        if addr.is_ipv4() {
            ipv4.push(addr);
        } else {
            other.push(addr);
        }
    }
    ipv4.extend(other);
    ipv4
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

async fn load_node_topics(
    pool: &sqlx::Pool<sqlx::Postgres>,
    max_concurrent_topics: i64,
) -> Result<HashSet<String>> {
    let rows = sqlx::query(
        "SELECT topic_id FROM cn_admin.node_subscriptions WHERE enabled = TRUE ORDER BY updated_at DESC, topic_id ASC LIMIT $1",
    )
    .bind(max_concurrent_topics)
    .fetch_all(pool)
    .await?;
    let mut topics = HashSet::new();
    for row in rows {
        let topic_id: String = row.try_get("topic_id")?;
        topics.insert(topic_id);
    }
    Ok(topics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cn_core::rate_limit::RateLimiter;
    use cn_core::service_config;
    use iroh::address_lookup::MemoryLookup;
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use std::collections::{HashMap, HashSet};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
    use tokio::sync::{broadcast, RwLock};
    use tokio::time::timeout;

    fn test_state() -> AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool");
        let config = service_config::static_handle(json!({
            "auth": {
                "mode": "off",
                "enforce_at": null,
                "grace_seconds": 900,
                "ws_auth_timeout_seconds": 10
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            }
        }));
        let (realtime_tx, _) = broadcast::channel(8);
        AppState {
            pool,
            config,
            rate_limiter: Arc::new(RateLimiter::new()),
            realtime_tx,
            gossip_senders: Arc::new(RwLock::new(HashMap::new())),
            node_topics: Arc::new(RwLock::new(HashSet::new())),
            relay_public_url: None,
            p2p_public_host: None,
            p2p_public_port: None,
            p2p_node_id: Arc::new(RwLock::new(None)),
            p2p_address_lookup: Arc::new(MemoryLookup::new()),
            p2p_bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            p2p_relay_urls: Arc::new(Vec::new()),
            p2p_advertised_relay_urls: Arc::new(Vec::new()),
            p2p_router: Arc::new(RwLock::new(None)),
            bootstrap_hint_rejoin_requests: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    #[tokio::test]
    async fn start_gossip_keeps_router_alive_in_state() {
        let state = test_state();
        let config = RelayConfig {
            addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8082)),
            database_url: "postgres://postgres:postgres@localhost/postgres".to_string(),
            p2p_bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            p2p_public_host: None,
            p2p_public_port: None,
            p2p_secret_key: None,
            p2p_relay_urls: Vec::new(),
            p2p_advertised_relay_urls: Vec::new(),
            p2p_relay_mode_default: false,
            topic_poll_seconds: 60,
            config_poll_seconds: 60,
            relay_public_url: Some("ws://localhost:8082/relay".to_string()),
        };

        start_gossip(state.clone(), config)
            .await
            .expect("start gossip");

        let node_id = state.p2p_node_id.read().await.clone();
        assert!(node_id.is_some());
        let router_is_present = state.p2p_router.read().await.is_some();
        assert!(router_is_present);
    }

    #[test]
    fn parse_seed_peer_hint_accepts_extended_relay_hint() {
        let parsed = parse_seed_peer_hint(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example|addr=127.0.0.1:11223",
        )
        .expect("parse relay hint");

        assert_eq!(
            parsed.node_id.to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        let node_addr = parsed.node_addr.expect("node addr");
        assert_eq!(node_addr.ip_addrs().count(), 1);
        assert_eq!(node_addr.relay_urls().count(), 1);
    }

    #[test]
    fn resolve_relay_mode_returns_custom_when_urls_present() {
        let config = RelayConfig {
            addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8082)),
            database_url: "postgres://postgres:postgres@localhost/postgres".to_string(),
            p2p_bind_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
            p2p_public_host: None,
            p2p_public_port: None,
            p2p_secret_key: None,
            p2p_relay_urls: vec!["https://relay.example".to_string()],
            p2p_advertised_relay_urls: vec!["https://public-relay.example".to_string()],
            p2p_relay_mode_default: false,
            topic_poll_seconds: 60,
            config_poll_seconds: 60,
            relay_public_url: None,
        };
        let mode = resolve_relay_mode(&config).expect("relay mode");
        assert!(matches!(mode, RelayMode::Custom(_)));
    }

    #[test]
    fn transport_profile_supports_relay_only() {
        let profile =
            EndpointTransportProfile::from_env_value(Some("relay-only")).expect("profile parse");
        let config = profile
            .quic_transport_config()
            .expect("relay-only config should be present");

        let debug = format!("{config:?}");
        assert!(debug.contains("enable_segmentation_offload: false"));
        assert!(!profile.allows_direct_ip_bind());
    }

    #[test]
    fn transport_profile_rejects_unknown_values() {
        let err = EndpointTransportProfile::from_env_value(Some("invalid-profile"))
            .expect_err("invalid transport profile should fail");
        assert!(err
            .to_string()
            .contains("invalid RELAY_IROH_TRANSPORT_PROFILE value"));
    }

    #[tokio::test]
    async fn subscribe_topic_with_retry_does_not_block_on_unreachable_bootstrap_peer() {
        let endpoint = Endpoint::empty_builder(RelayMode::Disabled)
            .bind_addr(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("bind addr")
            .bind()
            .await
            .expect("endpoint");
        let gossip = Gossip::builder().spawn(endpoint.clone());
        let router = Router::builder(endpoint)
            .accept(iroh_gossip::ALPN, gossip.clone())
            .spawn();

        let unreachable_endpoint = Endpoint::empty_builder(RelayMode::Disabled)
            .bind_addr(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("bind addr b")
            .bind()
            .await
            .expect("unreachable endpoint");
        let topic_id = format!(
            "kukuri:relay-subscribe-retry-test:{}",
            uuid::Uuid::new_v4()
        );

        let topic = timeout(
            Duration::from_secs(2),
            subscribe_topic_with_retry(&gossip, &topic_id, &[unreachable_endpoint.id()]),
        )
        .await
        .expect("subscribe should not block on unreachable bootstrap peer")
        .expect("subscribe topic");

        drop(topic);
        let _ = timeout(Duration::from_secs(2), router.shutdown()).await;
    }
}
