use anyhow::{anyhow, Result};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use cn_core::{auth, metrics, nostr};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::{interval, sleep, Instant};
use uuid::Uuid;

use crate::config::RelayRuntimeConfig;
use crate::filters::{matches_filter, parse_filters, RelayFilter};
use crate::ingest::{ingest_event, IngestContext, IngestOutcome, IngestSource, RelayEvent};
use crate::AppState;

const AUTH_EVENT_KIND: u32 = 22242;
const GOSSIP_BROADCAST_MAX_RETRIES: usize = 3;
const GOSSIP_BROADCAST_RETRY_BASE_DELAY_MS: u64 = 50;

struct AuthSessionState {
    pubkey: Option<String>,
    challenge: Option<String>,
    is_unauthenticated_connection: bool,
}

impl Default for AuthSessionState {
    fn default() -> Self {
        Self {
            pubkey: None,
            challenge: None,
            is_unauthenticated_connection: true,
        }
    }
}

pub async fn ws_handler(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let config_snapshot = state.config.get().await;
    let runtime = RelayRuntimeConfig::from_json(&config_snapshot.config_json);
    if runtime.rate_limit.enabled {
        let key = format!("conn:{}", addr.ip());
        let outcome = state
            .rate_limiter
            .check(
                &key,
                runtime.rate_limit.ws_conns_per_minute,
                Duration::from_secs(60),
            )
            .await;
        if !outcome.allowed {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "ratelimit");
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        }
    }

    ws.on_upgrade(move |socket| handle_socket(state, addr, socket))
}

async fn handle_socket(state: AppState, addr: SocketAddr, socket: WebSocket) {
    metrics::inc_ws_connections(super::SERVICE_NAME);
    metrics::inc_ws_unauthenticated_connections(super::SERVICE_NAME);
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = state.realtime_tx.subscribe();

    let mut subscriptions: HashMap<String, Vec<RelayFilter>> = HashMap::new();
    let mut auth_state = AuthSessionState::default();
    let mut auth_deadline: Option<Instant> = None;
    let mut auth_timeout_applies = false;

    let mut tick = interval(Duration::from_secs(5));

    if let Ok(runtime) = current_runtime(&state).await {
        let now = auth::unix_seconds().unwrap_or(0) as i64;
        if runtime.auth.requires_auth(now) {
            auth_timeout_applies = true;
            auth_deadline = Some(
                Instant::now() + ws_auth_timeout_duration(runtime.auth.ws_auth_timeout_seconds),
            );
            let _ = send_auth_challenge(&mut sender, &mut auth_state).await;
        }
    }

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if let Ok(runtime) = current_runtime(&state).await {
                    let now = auth::unix_seconds().unwrap_or(0) as i64;
                    let auth_required = runtime.auth.requires_auth(now);
                    if !auth_required {
                        auth_state.challenge = None;
                        auth_deadline = None;
                        auth_timeout_applies = false;
                    } else if auth_state.pubkey.is_none() {
                        let disconnect_deadline = runtime.auth.disconnect_deadline();
                        let should_apply_timeout =
                            auth_timeout_applies || disconnect_deadline.is_none();
                        if should_apply_timeout {
                            auth_timeout_applies = true;
                            if auth_deadline.is_none() {
                                auth_deadline = Some(
                                    Instant::now()
                                        + ws_auth_timeout_duration(
                                            runtime.auth.ws_auth_timeout_seconds,
                                        ),
                                );
                            }
                        } else {
                            auth_deadline = None;
                        }
                        if auth_state.challenge.is_none() {
                            let _ = send_auth_challenge(&mut sender, &mut auth_state).await;
                        }
                        if let Some(deadline) = disconnect_deadline {
                            if now >= deadline {
                                metrics::inc_ingest_rejected(super::SERVICE_NAME, "auth");
                                metrics::inc_ws_auth_disconnect(super::SERVICE_NAME, "deadline");
                                let _ = send_json(&mut sender, json!(["NOTICE", "auth-required: deadline reached"])).await;
                                break;
                            }
                        }
                    } else {
                        auth_deadline = None;
                    }
                }
                if let Some(deadline) = auth_deadline {
                    if Instant::now() >= deadline && auth_state.pubkey.is_none() {
                        metrics::inc_ingest_rejected(super::SERVICE_NAME, "auth");
                        metrics::inc_ws_auth_disconnect(super::SERVICE_NAME, "timeout");
                        let _ = send_json(&mut sender, json!(["NOTICE", "auth-required: timeout"])).await;
                        break;
                    }
                }
            }
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(err) = handle_text_message(
                            &state,
                            &addr,
                            &mut sender,
                            &mut subscriptions,
                            &mut auth_state,
                            text.to_string(),
                        ).await {
                            let _ = send_json(&mut sender, json!(["NOTICE", err.to_string()])).await;
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                    Ok(Message::Binary(_)) => {
                        let _ = send_json(&mut sender, json!(["NOTICE", "unsupported: binary message"])).await;
                    }
                    Err(_) => break,
                }
            }
            recv = broadcast_rx.recv() => {
                if let Ok(event) = recv {
                    if let Err(err) = dispatch_event(&mut sender, &subscriptions, auth_state.pubkey.as_deref(), &event).await {
                        let _ = send_json(&mut sender, json!(["NOTICE", err.to_string()])).await;
                    }
                }
            }
        }
    }

    if auth_state.is_unauthenticated_connection {
        metrics::dec_ws_unauthenticated_connections(super::SERVICE_NAME);
    }
    metrics::dec_ws_connections(super::SERVICE_NAME);
}

fn ws_auth_timeout_duration(seconds: i64) -> Duration {
    Duration::from_secs(seconds.max(1) as u64)
}

async fn send_auth_challenge(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    auth_state: &mut AuthSessionState,
) -> Result<()> {
    let challenge = Uuid::new_v4().to_string();
    auth_state.challenge = Some(challenge.clone());
    send_json(sender, json!(["AUTH", challenge])).await
}

async fn handle_text_message(
    state: &AppState,
    addr: &SocketAddr,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    subscriptions: &mut HashMap<String, Vec<RelayFilter>>,
    auth_state: &mut AuthSessionState,
    text: String,
) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let arr = value.as_array().ok_or_else(|| anyhow!("invalid message"))?;
    let msg_type = arr
        .first()
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("invalid message"))?;

    match msg_type {
        "EVENT" => {
            metrics::inc_ws_event_total(super::SERVICE_NAME);
            handle_event_message(state, addr, sender, &mut auth_state.pubkey, arr).await?;
        }
        "REQ" => {
            metrics::inc_ws_req_total(super::SERVICE_NAME);
            handle_req_message(
                state,
                addr,
                sender,
                subscriptions,
                auth_state.pubkey.as_deref(),
                arr,
            )
            .await?;
        }
        "CLOSE" => {
            if let Some(sub_id) = arr.get(1).and_then(|v| v.as_str()) {
                subscriptions.remove(sub_id);
            }
        }
        "AUTH" => {
            handle_auth_message(state, sender, auth_state, arr).await?;
        }
        _ => {
            let _ = send_json(sender, json!(["NOTICE", "unsupported: message type"])).await;
        }
    }

    Ok(())
}

async fn handle_event_message(
    state: &AppState,
    addr: &SocketAddr,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    auth_pubkey: &mut Option<String>,
    arr: &[serde_json::Value],
) -> Result<()> {
    let event_value = arr.get(1).ok_or_else(|| anyhow!("missing event"))?;
    let raw = nostr::parse_event(event_value)?;

    let runtime = current_runtime(state).await?;
    if runtime.rate_limit.enabled {
        let key = rate_limit_key(addr, auth_pubkey.as_deref());
        let outcome = state
            .rate_limiter
            .check(
                &key,
                runtime.rate_limit.ws_events_per_minute,
                Duration::from_secs(60),
            )
            .await;
        if !outcome.allowed {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "ratelimit");
            send_ok(sender, &raw.id, false, "rate-limited").await?;
            return Ok(());
        }
    }

    let context = IngestContext {
        auth_pubkey: auth_pubkey.clone(),
        source_topic: None,
        peer_id: None,
    };
    match ingest_event(state, raw.clone(), IngestSource::Ws, context).await? {
        IngestOutcome::Accepted {
            event,
            duplicate,
            broadcast_gossip,
        } => {
            if !duplicate {
                let _ = state.realtime_tx.send(event.clone());
                if broadcast_gossip {
                    broadcast_to_gossip(state, &event).await;
                }
            }
            send_ok(
                sender,
                &raw.id,
                true,
                if duplicate { "duplicate" } else { "" },
            )
            .await?;
        }
        IngestOutcome::Rejected { reason } => {
            send_ok(sender, &raw.id, false, &reason).await?;
        }
    }
    Ok(())
}

async fn handle_req_message(
    state: &AppState,
    addr: &SocketAddr,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    subscriptions: &mut HashMap<String, Vec<RelayFilter>>,
    auth_pubkey: Option<&str>,
    arr: &[serde_json::Value],
) -> Result<()> {
    let sub_id = arr
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing subscription id"))?
        .to_string();
    let mut filters = parse_filters(&arr[2..])?;
    let allowed_topics = state.node_topics.read().await;
    for filter in &mut filters {
        if let Some(values) = filter.tags.get_mut("t") {
            for value in values.iter_mut() {
                *value = cn_core::topic::normalize_topic_id(value)?;
                if !allowed_topics.contains(value) {
                    send_closed(sender, &sub_id, "restricted: topic not enabled").await?;
                    return Ok(());
                }
            }
        }
    }

    let runtime = current_runtime(state).await?;
    let now = auth::unix_seconds().unwrap_or(0) as i64;
    if runtime.auth.requires_auth(now) && auth_pubkey.is_none() {
        metrics::inc_ingest_rejected(super::SERVICE_NAME, "auth");
        send_closed(sender, &sub_id, "auth-required: missing auth").await?;
        return Ok(());
    }
    if runtime.rate_limit.enabled {
        let key = rate_limit_key(addr, auth_pubkey);
        let outcome = state
            .rate_limiter
            .check(
                &key,
                runtime.rate_limit.ws_reqs_per_minute,
                Duration::from_secs(60),
            )
            .await;
        if !outcome.allowed {
            metrics::inc_ingest_rejected(super::SERVICE_NAME, "ratelimit");
            send_closed(sender, &sub_id, "rate-limited").await?;
            return Ok(());
        }
    }

    subscriptions.insert(sub_id.clone(), filters.clone());
    let mut seen = std::collections::HashSet::new();
    let mut backfill_events = Vec::new();
    for filter in &filters {
        let events = fetch_events(state, filter).await?;
        for raw in events {
            if !seen.insert(raw.id.clone()) {
                continue;
            }
            if !is_allowed_event(auth_pubkey, &raw).await? {
                continue;
            }
            backfill_events.push(raw);
        }
    }
    backfill_events.sort_unstable_by(compare_backfill_events);
    for raw in &backfill_events {
        send_event(sender, &sub_id, raw).await?;
    }
    send_eose(sender, &sub_id).await?;
    Ok(())
}

async fn handle_auth_message(
    state: &AppState,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    auth_state: &mut AuthSessionState,
    arr: &[serde_json::Value],
) -> Result<()> {
    let event_value = arr.get(1).ok_or_else(|| anyhow!("missing auth event"))?;
    let raw = nostr::parse_event(event_value)?;
    if raw.kind != AUTH_EVENT_KIND {
        send_ok(sender, &raw.id, false, "invalid: kind").await?;
        metrics::inc_auth_failure(super::SERVICE_NAME);
        return Ok(());
    }
    nostr::verify_event(&raw)?;
    let now = auth::unix_seconds().unwrap_or(0) as i64;
    if (now - raw.created_at).abs() > 600 {
        send_ok(sender, &raw.id, false, "invalid: stale auth").await?;
        metrics::inc_auth_failure(super::SERVICE_NAME);
        return Ok(());
    }
    let challenge = raw.first_tag_value("challenge");
    let relay_tag = raw.first_tag_value("relay");
    if auth_state.challenge.as_deref() != challenge.as_deref() {
        send_ok(sender, &raw.id, false, "auth-required: challenge mismatch").await?;
        metrics::inc_auth_failure(super::SERVICE_NAME);
        return Ok(());
    }
    if let Some(expected) = &state.relay_public_url {
        if relay_tag.as_deref() != Some(expected.as_str()) {
            send_ok(sender, &raw.id, false, "auth-required: relay mismatch").await?;
            metrics::inc_auth_failure(super::SERVICE_NAME);
            return Ok(());
        }
    }

    if !has_current_consents(state, &raw.pubkey).await? {
        send_ok(sender, &raw.id, false, "consent-required").await?;
        metrics::inc_consent_required(super::SERVICE_NAME);
        return Ok(());
    }

    auth_state.pubkey = Some(raw.pubkey.clone());
    auth_state.challenge = None;
    if auth_state.is_unauthenticated_connection {
        metrics::dec_ws_unauthenticated_connections(super::SERVICE_NAME);
        auth_state.is_unauthenticated_connection = false;
    }
    send_ok(sender, &raw.id, true, "").await?;
    metrics::inc_auth_success(super::SERVICE_NAME);
    Ok(())
}

async fn current_runtime(state: &AppState) -> Result<RelayRuntimeConfig> {
    let config_snapshot = state.config.get().await;
    Ok(RelayRuntimeConfig::from_json(&config_snapshot.config_json))
}

async fn fetch_events(state: &AppState, filter: &RelayFilter) -> Result<Vec<nostr::RawEvent>> {
    let topics = filter.topic_ids().cloned().unwrap_or_default();
    let now = auth::unix_seconds().unwrap_or(0) as i64;

    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT e.raw_json FROM cn_relay.events e \
         JOIN cn_relay.event_topics t ON t.event_id = e.event_id \
         WHERE t.topic_id = ANY(",
    );
    builder.push_bind(&topics);
    builder.push(
        ") AND e.is_deleted = FALSE AND e.is_current = TRUE \
        AND (e.expires_at IS NULL OR e.expires_at > ",
    );
    builder.push_bind(now);
    builder.push(")");

    if let Some(ids) = &filter.ids {
        builder.push(" AND e.event_id = ANY(");
        builder.push_bind(ids);
        builder.push(")");
    }
    if let Some(authors) = &filter.authors {
        builder.push(" AND e.pubkey = ANY(");
        builder.push_bind(authors);
        builder.push(")");
    }
    if let Some(kinds) = &filter.kinds {
        let kinds: Vec<i32> = kinds.iter().map(|value| *value as i32).collect();
        builder.push(" AND e.kind = ANY(");
        builder.push_bind(kinds);
        builder.push(")");
    }
    if let Some(since) = filter.since {
        builder.push(" AND e.created_at >= ");
        builder.push_bind(since);
    }
    if let Some(until) = filter.until {
        builder.push(" AND e.created_at <= ");
        builder.push_bind(until);
    }
    builder.push(" ORDER BY e.created_at DESC, e.event_id ASC");
    if let Some(limit) = filter.limit {
        builder.push(" LIMIT ");
        builder.push(limit.to_string());
    }

    let rows = builder.build().fetch_all(&state.pool).await?;
    let mut events = Vec::new();
    for row in rows {
        let raw_json: serde_json::Value = row.try_get("raw_json")?;
        let raw = nostr::parse_event(&raw_json)?;
        events.push(raw);
    }
    Ok(events)
}

fn compare_backfill_events(a: &nostr::RawEvent, b: &nostr::RawEvent) -> std::cmp::Ordering {
    b.created_at
        .cmp(&a.created_at)
        .then_with(|| a.id.cmp(&b.id))
}

async fn dispatch_event(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    subscriptions: &HashMap<String, Vec<RelayFilter>>,
    auth_pubkey: Option<&str>,
    event: &RelayEvent,
) -> Result<()> {
    if !is_allowed_event(auth_pubkey, &event.raw).await? {
        return Ok(());
    }
    for (sub_id, filters) in subscriptions {
        if filters
            .iter()
            .any(|filter| matches_filter(filter, &event.raw))
        {
            send_event(sender, sub_id, &event.raw).await?;
        }
    }
    Ok(())
}

async fn is_allowed_event(auth_pubkey: Option<&str>, event: &nostr::RawEvent) -> Result<bool> {
    let scope = event
        .first_tag_value("scope")
        .unwrap_or_else(|| "public".into());
    if scope == "public" {
        return Ok(true);
    }
    if auth_pubkey.is_none() {
        return Ok(false);
    };
    let Some(epoch_value) = event.first_tag_value("epoch") else {
        return Ok(false);
    };
    let epoch = epoch_value.parse::<i64>().unwrap_or(0);
    if epoch <= 0 {
        return Ok(false);
    }
    Ok(true)
}

async fn has_current_consents(state: &AppState, pubkey: &str) -> Result<bool> {
    crate::ingest::has_current_consents(&state.pool, pubkey).await
}

fn rate_limit_key(addr: &SocketAddr, pubkey: Option<&str>) -> String {
    if let Some(pubkey) = pubkey {
        format!("pubkey:{pubkey}")
    } else {
        format!("ip:{}", addr.ip())
    }
}

async fn send_event(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    sub_id: &str,
    raw: &nostr::RawEvent,
) -> Result<()> {
    send_json(sender, json!(["EVENT", sub_id, raw])).await
}

async fn send_eose(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    sub_id: &str,
) -> Result<()> {
    send_json(sender, json!(["EOSE", sub_id])).await
}

async fn send_closed(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    sub_id: &str,
    reason: &str,
) -> Result<()> {
    send_json(sender, json!(["CLOSED", sub_id, reason])).await
}

async fn send_ok(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    event_id: &str,
    ok: bool,
    message: &str,
) -> Result<()> {
    send_json(sender, json!(["OK", event_id, ok, message])).await
}

async fn send_json(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    value: serde_json::Value,
) -> Result<()> {
    let text = serde_json::to_string(&value)?;
    sender.send(Message::Text(text.into())).await?;
    Ok(())
}

async fn broadcast_to_gossip(state: &AppState, event: &RelayEvent) {
    let payload = match serde_json::to_vec(&event.raw) {
        Ok(payload) => payload,
        Err(_) => return,
    };
    let senders = {
        let guard = state.gossip_senders.read().await;
        event
            .topic_ids
            .iter()
            .filter_map(|topic_id| guard.get(topic_id).cloned())
            .collect::<Vec<_>>()
    };
    for sender in senders {
        if broadcast_with_retry(&sender, payload.clone()).await {
            metrics::inc_gossip_sent(super::SERVICE_NAME);
        }
    }
}

async fn broadcast_with_retry(sender: &iroh_gossip::api::GossipSender, payload: Vec<u8>) -> bool {
    for attempt in 0..GOSSIP_BROADCAST_MAX_RETRIES {
        if sender.broadcast(payload.clone().into()).await.is_ok() {
            return true;
        }
        if attempt + 1 < GOSSIP_BROADCAST_MAX_RETRIES {
            let delay_ms = GOSSIP_BROADCAST_RETRY_BASE_DELAY_MS * (attempt as u64 + 1);
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }
    false
}
