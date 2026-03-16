use std::collections::{BTreeSet, HashMap};
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::extract::{ConnectInfo, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use kukuri_cn_core::{
    AUTH_EVENT_KIND, ApiError, ApiResult, AuthMode, AuthRolloutConfig, CommunityNodeBootstrapNode,
    CommunityNodeResolvedUrls, DatabaseInitMode, RELAY_SERVICE_NAME, connect_postgres,
    initialize_database, initialize_database_for_runtime, load_auth_rollout, load_bootstrap_nodes,
    normalize_http_url, normalize_http_url_list, normalize_pubkey, normalize_ws_url,
    parse_raw_event, verify_raw_event,
};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::postgres::PgPool;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct RelayState {
    pool: PgPool,
    self_node: CommunityNodeBootstrapNode,
    events_tx: broadcast::Sender<Value>,
}

#[derive(Clone, Debug)]
pub struct RelayConfig {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub base_url: String,
    pub public_base_url: String,
    pub relay_ws_url: String,
    pub iroh_relay_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RelayP2pInfoResponse {
    public_base_url: String,
    relay_ws_url: String,
    iroh_relay_urls: Vec<String>,
    relay_urls: Vec<String>,
    bootstrap_nodes: Vec<String>,
}

#[derive(Default)]
struct SessionState {
    connected_at: i64,
    pubkey: Option<String>,
    challenge: Option<String>,
    challenge_issued_at: Option<i64>,
    subscriptions: HashMap<String, BTreeSet<String>>,
}

impl RelayConfig {
    pub fn from_env() -> Result<Self> {
        let bind_addr = std::env::var("COMMUNITY_NODE_RELAY_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8081".to_string())
            .parse::<SocketAddr>()
            .context("failed to parse COMMUNITY_NODE_RELAY_BIND_ADDR")?;
        let database_url = std::env::var("COMMUNITY_NODE_DATABASE_URL")
            .context("COMMUNITY_NODE_DATABASE_URL is required")?;
        let base_url = normalize_http_url(
            std::env::var("COMMUNITY_NODE_BASE_URL")
                .context("COMMUNITY_NODE_BASE_URL is required")?
                .as_str(),
        )?;
        let public_base_url = normalize_http_url(
            std::env::var("COMMUNITY_NODE_PUBLIC_BASE_URL")
                .ok()
                .as_deref()
                .unwrap_or(base_url.as_str()),
        )?;
        let relay_ws_url = normalize_ws_url(
            std::env::var("COMMUNITY_NODE_RELAY_WS_URL")
                .context("COMMUNITY_NODE_RELAY_WS_URL is required")?
                .as_str(),
        )?;
        let iroh_relay_urls =
            normalize_http_url_list(parse_csv_env("COMMUNITY_NODE_IROH_RELAY_URLS"))?;
        Ok(Self {
            bind_addr,
            database_url,
            base_url,
            public_base_url,
            relay_ws_url,
            iroh_relay_urls,
        })
    }
}

pub async fn build_state(config: &RelayConfig) -> Result<RelayState> {
    let pool = connect_postgres(config.database_url.as_str()).await?;
    initialize_database(&pool).await?;
    build_state_from_pool(config, pool).await
}

async fn build_runtime_state(config: &RelayConfig) -> Result<RelayState> {
    let pool = connect_postgres(config.database_url.as_str()).await?;
    initialize_database_for_runtime(&pool, DatabaseInitMode::from_env()?).await?;
    build_state_from_pool(config, pool).await
}

async fn build_state_from_pool(config: &RelayConfig, pool: PgPool) -> Result<RelayState> {
    let (events_tx, _) = broadcast::channel(256);
    Ok(RelayState {
        pool,
        self_node: CommunityNodeBootstrapNode {
            base_url: config.base_url.clone(),
            resolved_urls: CommunityNodeResolvedUrls::new(
                config.public_base_url.clone(),
                config.relay_ws_url.clone(),
                config.iroh_relay_urls.clone(),
            )?,
        },
        events_tx,
    })
}

pub fn app_router(state: RelayState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/p2p/info", get(p2p_info))
        .route("/relay", get(ws_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn run_from_env() -> Result<()> {
    init_tracing();

    let config = RelayConfig::from_env()?;
    let bind_addr = config.bind_addr;
    let state = build_runtime_state(&config).await?;
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind relay at {bind_addr}"))?;
    tracing::info!(bind_addr = %bind_addr, "community-node relay listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kukuri_cn_relay=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn p2p_info(State(state): State<RelayState>) -> ApiResult<Json<RelayP2pInfoResponse>> {
    let nodes = load_bootstrap_nodes(&state.pool, Some(state.self_node.clone()))
        .await
        .map_err(internal_error)?;
    Ok(Json(RelayP2pInfoResponse {
        public_base_url: state.self_node.resolved_urls.public_base_url.clone(),
        relay_ws_url: state.self_node.resolved_urls.relay_ws_url.clone(),
        iroh_relay_urls: state.self_node.resolved_urls.iroh_relay_urls.clone(),
        relay_urls: state.self_node.resolved_urls.iroh_relay_urls.clone(),
        bootstrap_nodes: nodes.into_iter().map(|node| node.base_url).collect(),
    }))
}

async fn ws_handler(
    State(state): State<RelayState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_socket(state, addr, socket))
}

async fn handle_socket(state: RelayState, _addr: SocketAddr, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let mut events_rx = state.events_tx.subscribe();
    let mut session = SessionState {
        connected_at: Utc::now().timestamp(),
        ..SessionState::default()
    };
    let mut tick = tokio::time::interval(Duration::from_secs(1));

    if let Ok(rollout) = load_auth_rollout(&state.pool, RELAY_SERVICE_NAME).await
        && rollout.mode == AuthMode::Required
    {
        let _ = send_auth_challenge(&mut sender, &mut session).await;
    }

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if let Ok(rollout) = load_auth_rollout(&state.pool, RELAY_SERVICE_NAME).await
                    && let Err(error) = enforce_auth_timeout(&mut sender, &mut session, &rollout).await
                {
                    let message = error.to_string();
                    let _ = send_notice(&mut sender, message.as_str()).await;
                    let _ = sender
                        .send(Message::Close(Some(CloseFrame {
                            code: close_code::POLICY,
                            reason: message.into(),
                        })))
                        .await;
                    let _ = sender.close().await;
                    break;
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(error) = handle_text_message(&state, &mut sender, &mut session, text.to_string()).await {
                            let _ = send_notice(&mut sender, error.to_string().as_str()).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => {
                        let _ = send_notice(&mut sender, "unsupported: binary").await;
                    }
                    Some(Err(_)) => break,
                }
            }
            received = events_rx.recv() => {
                if let Ok(event) = received {
                    if dispatch_event(&mut sender, &session, &event).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_text_message(
    state: &RelayState,
    sender: &mut SplitSink<WebSocket, Message>,
    session: &mut SessionState,
    text: String,
) -> Result<()> {
    let value: Value = serde_json::from_str(text.as_str()).context("invalid websocket payload")?;
    let arr = value
        .as_array()
        .ok_or_else(|| anyhow!("invalid websocket message"))?;
    let message_type = arr
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing message type"))?;
    let rollout = load_auth_rollout(&state.pool, RELAY_SERVICE_NAME).await?;
    if rollout.requires_auth(Utc::now().timestamp())
        && session.pubkey.is_none()
        && message_type != "AUTH"
    {
        if session.challenge.is_none() {
            send_auth_challenge(sender, session).await?;
        }
        bail!("auth-required");
    }

    match message_type {
        "AUTH" => handle_auth_message(state, sender, session, arr).await,
        "REQ" => handle_req_message(sender, session, arr).await,
        "CLOSE" => {
            if let Some(sub_id) = arr.get(1).and_then(Value::as_str) {
                session.subscriptions.remove(sub_id);
            }
            Ok(())
        }
        "EVENT" => handle_event_message(state, sender, arr).await,
        _ => bail!("unsupported: message type"),
    }
}

async fn handle_auth_message(
    state: &RelayState,
    sender: &mut SplitSink<WebSocket, Message>,
    session: &mut SessionState,
    arr: &[Value],
) -> Result<()> {
    let event_value = arr.get(1).ok_or_else(|| anyhow!("missing auth event"))?;
    let raw = parse_raw_event(event_value)?;
    verify_raw_event(&raw)?;
    if raw.kind != u32::from(AUTH_EVENT_KIND) {
        bail!("auth event kind mismatch");
    }
    if (Utc::now().timestamp() - raw.created_at).abs() > 600 {
        bail!("auth event is stale");
    }
    let challenge = raw
        .first_tag_value("challenge")
        .ok_or_else(|| anyhow!("missing challenge tag"))?;
    let relay = raw
        .first_tag_value("relay")
        .ok_or_else(|| anyhow!("missing relay tag"))?;
    if relay != state.self_node.resolved_urls.public_base_url {
        bail!("relay tag mismatch");
    }
    let Some(expected) = session.challenge.as_deref() else {
        bail!("auth challenge missing");
    };
    if challenge != expected {
        bail!("auth challenge mismatch");
    }
    let pubkey = normalize_pubkey(raw.pubkey.as_str())?;
    sqlx::query(
        "INSERT INTO cn_user.subscriber_accounts
            (subscriber_pubkey, status, last_authenticated_at)
         VALUES ($1, 'active', NOW())
         ON CONFLICT (subscriber_pubkey) DO UPDATE
         SET status = 'active',
             last_authenticated_at = NOW()",
    )
    .bind(&pubkey)
    .execute(&state.pool)
    .await?;
    session.pubkey = Some(pubkey);
    session.challenge = None;
    session.challenge_issued_at = None;
    send_notice(sender, "authenticated").await?;
    Ok(())
}

async fn handle_req_message(
    sender: &mut SplitSink<WebSocket, Message>,
    session: &mut SessionState,
    arr: &[Value],
) -> Result<()> {
    let sub_id = arr
        .get(1)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing subscription id"))?;
    let mut topics = BTreeSet::new();
    for filter in arr.iter().skip(2) {
        let Some(filter) = filter.as_object() else {
            continue;
        };
        if let Some(values) = filter.get("#t").and_then(Value::as_array) {
            for value in values {
                if let Some(topic) = value.as_str() {
                    let trimmed = topic.trim();
                    if !trimmed.is_empty() {
                        topics.insert(trimmed.to_string());
                    }
                }
            }
        }
    }
    if topics.is_empty() {
        bail!("missing required #t filter");
    }
    session.subscriptions.insert(sub_id.to_string(), topics);
    send_json(sender, json!(["EOSE", sub_id])).await?;
    Ok(())
}

async fn handle_event_message(
    state: &RelayState,
    sender: &mut SplitSink<WebSocket, Message>,
    arr: &[Value],
) -> Result<()> {
    let event_value = arr.get(1).ok_or_else(|| anyhow!("missing event payload"))?;
    let raw = parse_raw_event(event_value)?;
    verify_raw_event(&raw)?;
    let topics = raw.tag_values("t");
    if topics.is_empty() {
        bail!("event must include at least one t tag");
    }
    let payload = serde_json::to_value(&raw)?;
    let _ = state.events_tx.send(payload);
    send_json(sender, json!(["OK", raw.id, true, "accepted"])).await?;
    Ok(())
}

async fn dispatch_event(
    sender: &mut SplitSink<WebSocket, Message>,
    session: &SessionState,
    event: &Value,
) -> Result<()> {
    let raw = parse_raw_event(event)?;
    let topics = raw.tag_values("t");
    if topics.is_empty() {
        return Ok(());
    }
    for (sub_id, filter_topics) in &session.subscriptions {
        if topics.iter().any(|topic| filter_topics.contains(*topic)) {
            send_json(sender, json!(["EVENT", sub_id, event])).await?;
        }
    }
    Ok(())
}

async fn enforce_auth_timeout(
    sender: &mut SplitSink<WebSocket, Message>,
    session: &mut SessionState,
    rollout: &AuthRolloutConfig,
) -> Result<()> {
    let now = Utc::now().timestamp();
    if rollout.mode != AuthMode::Required || session.pubkey.is_some() {
        return Ok(());
    }
    if session.challenge.is_none() {
        send_auth_challenge(sender, session).await?;
    }
    let requires_auth = rollout.requires_auth(now);
    if let Some(deadline) = rollout.disconnect_deadline_for_connection(session.connected_at)
        && requires_auth
        && now >= deadline
    {
        bail!("auth-required: grace deadline reached");
    }
    if let Some(issued_at) = session.challenge_issued_at
        && requires_auth
        && now - issued_at.max(rollout.enforce_at.unwrap_or(issued_at))
            >= rollout.ws_auth_timeout_seconds.max(1)
    {
        bail!("auth-required: timeout");
    }
    Ok(())
}

async fn send_auth_challenge(
    sender: &mut SplitSink<WebSocket, Message>,
    session: &mut SessionState,
) -> Result<()> {
    let challenge = uuid::Uuid::new_v4().to_string();
    session.challenge = Some(challenge.clone());
    session.challenge_issued_at = Some(Utc::now().timestamp());
    send_json(sender, json!(["AUTH", challenge])).await
}

async fn send_notice(sender: &mut SplitSink<WebSocket, Message>, message: &str) -> Result<()> {
    send_json(sender, json!(["NOTICE", message])).await
}

async fn send_json(sender: &mut SplitSink<WebSocket, Message>, value: Value) -> Result<()> {
    sender
        .send(Message::Text(value.to_string().into()))
        .await
        .context("failed to send websocket response")
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        error.to_string(),
    )
}

fn parse_csv_env(var_name: &str) -> Vec<String> {
    std::env::var(var_name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|item| {
                    let trimmed = item.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
