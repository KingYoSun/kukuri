use super::AppState;
use crate::gossip;
use crate::ingest::ACCESS_CONTROL_P2P_ONLY_REASON;
use crate::ws;
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use cn_core::rate_limit::RateLimiter;
use cn_core::service_config;
use cn_core::{metrics, nostr};
use cn_kip_types::{
    KIND_INVITE_CAPABILITY, KIND_JOIN_REQUEST, KIND_KEY_ENVELOPE, KIP_NAMESPACE, KIP_VERSION,
    SCHEMA_INVITE_CAPABILITY, SCHEMA_JOIN_REQUEST, SCHEMA_KEY_ENVELOPE,
};
use futures_util::{SinkExt, StreamExt};
use iroh::discovery::static_provider::StaticProvider;
use iroh::endpoint::Connection;
use iroh::protocol::Router as IrohRouter;
use iroh::Endpoint;
use iroh_gossip::api::{Event as GossipEvent, GossipReceiver, GossipSender};
use iroh_gossip::{Gossip, TopicId};
use nostr_sdk::prelude::{EventBuilder, Keys, Kind, Tag, TagKind, Timestamp};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex, OnceCell, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
static INTEGRATION_TEST_LOCK: OnceCell<Arc<Mutex<()>>> = OnceCell::const_new();
const WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const AUTH_EVENT_KIND: u16 = 22242;

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:15432/cn".to_string())
}

async fn ensure_migrated(pool: &Pool<Postgres>) {
    MIGRATIONS
        .get_or_init(|| async {
            cn_core::migrations::run(pool)
                .await
                .expect("run migrations");
        })
        .await;
}

async fn acquire_integration_test_lock() -> tokio::sync::OwnedMutexGuard<()> {
    let lock = INTEGRATION_TEST_LOCK
        .get_or_init(|| async { Arc::new(Mutex::new(())) })
        .await
        .clone();
    lock.lock_owned().await
}

fn default_runtime_config() -> serde_json::Value {
    json!({
        "auth": {
            "mode": "off",
            "enforce_at": null,
            "grace_seconds": 900,
            "ws_auth_timeout_seconds": 10
        },
        "limits": {
            "max_event_bytes": 32768,
            "max_tags": 200
        },
        "rate_limit": {
            "enabled": false
        }
    })
}

fn build_state(pool: Pool<Postgres>) -> AppState {
    build_state_with_config(pool, default_runtime_config())
}

fn build_state_with_config(pool: Pool<Postgres>, config_json: serde_json::Value) -> AppState {
    let config = service_config::static_handle(config_json);
    let (realtime_tx, _) = broadcast::channel(64);
    AppState {
        pool,
        config,
        rate_limiter: Arc::new(RateLimiter::new()),
        realtime_tx,
        gossip_senders: Arc::new(RwLock::new(HashMap::new())),
        node_topics: Arc::new(RwLock::new(HashSet::new())),
        relay_public_url: None,
    }
}

async fn enable_topic(pool: &Pool<Postgres>, state: &AppState, topic_id: &str) {
    sqlx::query(
        "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count) \
         VALUES ($1, TRUE, 1) \
         ON CONFLICT (topic_id) DO UPDATE SET enabled = TRUE, updated_at = NOW()",
    )
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("insert node subscription");

    let mut topics = state.node_topics.write().await;
    topics.insert(topic_id.to_string());
}

async fn update_topic_ingest_policy(
    pool: &Pool<Postgres>,
    topic_id: &str,
    policy: serde_json::Value,
) {
    sqlx::query(
        "UPDATE cn_admin.node_subscriptions \
         SET ingest_policy = $1, updated_at = NOW() \
         WHERE topic_id = $2",
    )
    .bind(policy)
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("update topic ingest policy");
}

async fn reset_topic_health_state(pool: &Pool<Postgres>, state: &AppState) {
    sqlx::query("UPDATE cn_admin.node_subscriptions SET enabled = FALSE")
        .execute(pool)
        .await
        .expect("disable node subscriptions");

    state.node_topics.write().await.clear();
    state.gossip_senders.write().await.clear();
}

async fn spawn_relay_server(state: AppState) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/relay", get(ws::ws_handler))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );
    let server_handle = tokio::spawn(async move {
        server.await.expect("server");
    });
    (addr, server_handle)
}

async fn connect_ws(addr: SocketAddr) -> WsStream {
    timeout(
        WAIT_TIMEOUT,
        tokio_tungstenite::connect_async(format!("ws://{}/relay", addr)),
    )
    .await
    .expect("connect timeout")
    .expect("connect")
    .0
}

struct GossipHarness {
    receiver: GossipReceiver,
    _receiver_a: GossipReceiver,
    router_a: IrohRouter,
    router_b: IrohRouter,
    _discovery: StaticProvider,
    _sender_b: GossipSender,
    _conn_a: Connection,
    _conn_b: Connection,
    _gossip_a: Gossip,
    _gossip_b: Gossip,
}

async fn setup_gossip(topic_id: &str) -> (GossipSender, GossipHarness) {
    let endpoint_a = Endpoint::builder()
        .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .bind()
        .await
        .expect("endpoint a");
    let gossip_a = Gossip::builder().spawn(endpoint_a.clone());
    let router_a = IrohRouter::builder(endpoint_a.clone())
        .accept(iroh_gossip::ALPN, gossip_a.clone())
        .spawn();

    let endpoint_b = Endpoint::builder()
        .bind_addr_v4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .bind()
        .await
        .expect("endpoint b");
    let gossip_b = Gossip::builder().spawn(endpoint_b.clone());
    let router_b = IrohRouter::builder(endpoint_b.clone())
        .accept(iroh_gossip::ALPN, gossip_b.clone())
        .spawn();

    let discovery = StaticProvider::new();
    discovery.add_endpoint_info(endpoint_a.addr());
    discovery.add_endpoint_info(endpoint_b.addr());
    endpoint_a.discovery().add(discovery.clone());
    endpoint_b.discovery().add(discovery.clone());

    let topic_bytes = cn_core::topic::topic_id_to_gossip_bytes(topic_id).expect("topic bytes");
    let peer_a = endpoint_a.id();
    let peer_b = endpoint_b.id();
    let topic_a = gossip_a
        .subscribe(TopicId::from(topic_bytes.clone()), vec![peer_b])
        .await
        .expect("subscribe a");
    let (sender_a, mut receiver_a) = topic_a.split();

    let topic_b = gossip_b
        .subscribe(TopicId::from(topic_bytes), vec![peer_a])
        .await
        .expect("subscribe b");
    let (sender_b, mut receiver_b) = topic_b.split();

    let conn_b = timeout(
        WAIT_TIMEOUT,
        endpoint_b.connect(endpoint_a.addr(), iroh_gossip::ALPN),
    )
    .await
    .expect("connect b->a timeout")
    .expect("connect b->a");
    let conn_a = timeout(
        WAIT_TIMEOUT,
        endpoint_a.connect(endpoint_b.addr(), iroh_gossip::ALPN),
    )
    .await
    .expect("connect a->b timeout")
    .expect("connect a->b");
    timeout(WAIT_TIMEOUT, sender_a.join_peers(vec![endpoint_b.id()]))
        .await
        .expect("join peers a timeout")
        .expect("join peers a");
    timeout(WAIT_TIMEOUT, sender_b.join_peers(vec![endpoint_a.id()]))
        .await
        .expect("join peers b timeout")
        .expect("join peers b");

    timeout(WAIT_TIMEOUT, receiver_a.joined())
        .await
        .expect("join confirm a timeout")
        .expect("join confirm a");
    timeout(WAIT_TIMEOUT, receiver_b.joined())
        .await
        .expect("join confirm b timeout")
        .expect("join confirm b");

    (
        sender_a,
        GossipHarness {
            receiver: receiver_b,
            _receiver_a: receiver_a,
            router_a,
            router_b,
            _discovery: discovery,
            _sender_b: sender_b,
            _conn_a: conn_a,
            _conn_b: conn_b,
            _gossip_a: gossip_a,
            _gossip_b: gossip_b,
        },
    )
}

async fn wait_for_ws_json<F>(
    ws: &mut WsStream,
    wait: Duration,
    label: &str,
    predicate: F,
) -> serde_json::Value
where
    F: Fn(&serde_json::Value) -> bool,
{
    let mut last: Option<serde_json::Value> = None;
    let result = timeout(wait, async {
        while let Some(message) = ws.next().await {
            let message = message.expect("ws message");
            if let Message::Text(text) = message {
                let value: serde_json::Value = serde_json::from_str(&text).expect("ws json");
                if matches!(
                    value.get(0).and_then(|v| v.as_str()),
                    Some("NOTICE") | Some("CLOSED")
                ) {
                    panic!("websocket error ({}): {}", label, value);
                }
                if predicate(&value) {
                    return value;
                }
                last = Some(value);
            }
        }
        panic!("websocket closed");
    })
    .await;

    result.unwrap_or_else(|_| {
        panic!(
            "websocket timeout ({}): last={}",
            label,
            last.map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string())
        )
    })
}

async fn wait_for_ws_json_any<F>(
    ws: &mut WsStream,
    wait: Duration,
    label: &str,
    predicate: F,
) -> serde_json::Value
where
    F: Fn(&serde_json::Value) -> bool,
{
    let mut last: Option<serde_json::Value> = None;
    let result = timeout(wait, async {
        while let Some(message) = ws.next().await {
            let message = message.expect("ws message");
            if let Message::Text(text) = message {
                let value: serde_json::Value = serde_json::from_str(&text).expect("ws json");
                if predicate(&value) {
                    return value;
                }
                last = Some(value);
            }
        }
        panic!("websocket closed");
    })
    .await;

    result.unwrap_or_else(|_| {
        panic!(
            "websocket timeout ({}): last={}",
            label,
            last.map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string())
        )
    })
}

async fn assert_req_notice_reason(
    ws: &mut WsStream,
    req: serde_json::Value,
    expected_reason: &str,
    label: &str,
) {
    ws.send(Message::Text(req.to_string().into()))
        .await
        .expect("send req");
    let notice = wait_for_ws_json_any(ws, WAIT_TIMEOUT, label, |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("NOTICE")
            && value.get(1).and_then(|v| v.as_str()) == Some(expected_reason)
    })
    .await;
    assert_eq!(
        notice.get(1).and_then(|v| v.as_str()),
        Some(expected_reason)
    );
}

async fn wait_for_gossip_event(receiver: &mut GossipReceiver, wait: Duration, expected_id: &str) {
    let mut last_received_id: Option<String> = None;
    let result = timeout(wait, async {
        while let Some(event) = receiver.next().await {
            let event = event.expect("gossip event");
            match event {
                GossipEvent::Received(message) => {
                    let value: serde_json::Value =
                        serde_json::from_slice(&message.content).expect("gossip json");
                    let raw = nostr::parse_event(&value).expect("gossip event");
                    last_received_id = Some(raw.id.to_string());
                    if raw.id == expected_id {
                        return;
                    }
                }
                GossipEvent::Lagged => continue,
                _ => {}
            }
        }
        panic!("gossip receiver closed");
    })
    .await;

    result.unwrap_or_else(|_| {
        panic!(
            "gossip timeout: expected_id={}, last_received_id={}",
            expected_id,
            last_received_id.as_deref().unwrap_or("<none>")
        )
    });
}

async fn assert_no_ws_event(ws: &mut WsStream, wait: Duration, label: &str) {
    let _ = timeout(wait, async {
        while let Some(message) = ws.next().await {
            let message = message.expect("ws message");
            if let Message::Text(text) = message {
                let value: serde_json::Value = serde_json::from_str(&text).expect("ws json");
                if value.get(0).and_then(|v| v.as_str()) == Some("EVENT") {
                    panic!("unexpected websocket EVENT ({}): {}", label, value);
                }
            }
        }
        panic!("websocket closed");
    })
    .await;
}

async fn assert_no_gossip_received(receiver: &mut GossipReceiver, wait: Duration, label: &str) {
    let _ = timeout(wait, async {
        while let Some(event) = receiver.next().await {
            let event = event.expect("gossip event");
            if let GossipEvent::Received(message) = event {
                let payload = String::from_utf8(message.content.to_vec())
                    .unwrap_or_else(|_| "<binary>".into());
                panic!("unexpected gossip message ({}): {}", label, payload);
            }
        }
        panic!("gossip receiver closed");
    })
    .await;
}

async fn ensure_required_policies(pool: &Pool<Postgres>) {
    for policy_type in ["terms", "privacy"] {
        let current_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cn_admin.policies WHERE type = $1 AND is_current = TRUE",
        )
        .bind(policy_type)
        .fetch_one(pool)
        .await
        .expect("count current policies");
        if current_count > 0 {
            continue;
        }

        let policy_id = format!("relay-it-{}-{}", policy_type, Uuid::new_v4());
        let title = format!("Relay Integration Test {policy_type}");
        let content_hash = format!("sha256:{policy_id}");
        sqlx::query(
            "INSERT INTO cn_admin.policies \
             (policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current) \
             VALUES ($1, $2, '1.0.0', 'ja-JP', $3, '# relay integration test policy', $4, NOW(), NOW(), TRUE)",
        )
        .bind(&policy_id)
        .bind(policy_type)
        .bind(title)
        .bind(content_hash)
        .execute(pool)
        .await
        .expect("insert current policy");
    }
}

async fn ensure_consents(pool: &Pool<Postgres>, pubkey: &str) {
    for _ in 0..5 {
        let missing_policies = sqlx::query_scalar::<_, String>(
            "SELECT p.policy_id \
             FROM cn_admin.policies p \
             LEFT JOIN cn_user.policy_consents c \
               ON c.policy_id = p.policy_id AND c.accepter_pubkey = $1 \
             WHERE p.is_current = TRUE \
               AND p.type IN ('terms', 'privacy') \
               AND c.policy_id IS NULL",
        )
        .bind(pubkey)
        .fetch_all(pool)
        .await
        .expect("fetch missing policies");

        if missing_policies.is_empty() {
            return;
        }

        for policy_id in missing_policies {
            let consent_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO cn_user.policy_consents \
                 (consent_id, policy_id, accepter_pubkey) \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(consent_id)
            .bind(policy_id)
            .bind(pubkey)
            .execute(pool)
            .await
            .expect("insert consent");
        }

        tokio::task::yield_now().await;
    }
}

async fn insert_topic_subscription(pool: &Pool<Postgres>, topic_id: &str, pubkey: &str) {
    sqlx::query(
        "INSERT INTO cn_user.topic_subscriptions \
         (topic_id, subscriber_pubkey, status) \
         VALUES ($1, $2, 'active') \
         ON CONFLICT (topic_id, subscriber_pubkey) \
         DO UPDATE SET status = 'active', ended_at = NULL",
    )
    .bind(topic_id)
    .bind(pubkey)
    .execute(pool)
    .await
    .expect("insert topic subscription");
}

async fn insert_backfill_event(
    pool: &Pool<Postgres>,
    topic_id: &str,
    event_id: &str,
    created_at: i64,
    content: &str,
) {
    let tags = json!([["t", topic_id]]);
    let raw_json = json!({
        "id": event_id,
        "pubkey": "1".repeat(64),
        "created_at": created_at,
        "kind": 1,
        "tags": tags.clone(),
        "content": content,
        "sig": "2".repeat(128)
    });
    sqlx::query(
        "INSERT INTO cn_relay.events \
         (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, ingested_at, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key, expires_at) \
         VALUES ($1, $2, 1, $3, $4, $5, $6, $7, NOW(), FALSE, FALSE, TRUE, NULL, NULL, NULL)",
    )
    .bind(event_id)
    .bind("1".repeat(64))
    .bind(created_at)
    .bind(tags)
    .bind(content)
    .bind("2".repeat(128))
    .bind(raw_json)
    .execute(pool)
    .await
    .expect("insert relay event");

    sqlx::query(
        "INSERT INTO cn_relay.event_topics (event_id, topic_id) VALUES ($1, $2) \
         ON CONFLICT (event_id, topic_id) DO NOTHING",
    )
    .bind(event_id)
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("insert relay event topic");
}

async fn insert_non_current_event(
    pool: &Pool<Postgres>,
    topic_id: &str,
    event_id: &str,
    created_at: i64,
    ingested_days_ago: i64,
) {
    let tags = json!([["t", topic_id]]);
    let raw_json = json!({
        "id": event_id,
        "pubkey": "1".repeat(64),
        "created_at": created_at,
        "kind": 1,
        "tags": tags.clone(),
        "content": "stale",
        "sig": "2".repeat(128)
    });
    sqlx::query(
        "INSERT INTO cn_relay.events \
         (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, ingested_at, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key, expires_at) \
         VALUES ($1, $2, 1, $3, $4, $5, $6, $7, NOW() - ($8 * INTERVAL '1 day'), FALSE, FALSE, FALSE, NULL, NULL, NULL)",
    )
    .bind(event_id)
    .bind("1".repeat(64))
    .bind(created_at)
    .bind(tags)
    .bind("stale")
    .bind("2".repeat(128))
    .bind(raw_json)
    .bind(ingested_days_ago)
    .execute(pool)
    .await
    .expect("insert non-current relay event");

    sqlx::query(
        "INSERT INTO cn_relay.event_topics (event_id, topic_id) VALUES ($1, $2) \
         ON CONFLICT (event_id, topic_id) DO NOTHING",
    )
    .bind(event_id)
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("insert non-current relay event topic");
}

fn unique_hex_event_id(prefix: char) -> String {
    let mut normalized = prefix.to_ascii_lowercase();
    if !normalized.is_ascii_hexdigit() {
        normalized = 'a';
    }
    let mut id = String::with_capacity(64);
    id.push(normalized);
    id.push_str(&Uuid::new_v4().simple().to_string());
    id.push_str(&"0".repeat(31));
    id
}

fn build_event_at(
    keys: &Keys,
    kind: u16,
    tags: Vec<Vec<String>>,
    content: String,
    created_at: i64,
) -> nostr::RawEvent {
    let mut builder = EventBuilder::new(Kind::Custom(kind), content)
        .custom_created_at(Timestamp::from_secs(created_at.max(0) as u64));
    for tag in tags {
        if tag.is_empty() {
            continue;
        }
        let kind = TagKind::from(tag[0].as_str());
        let values = if tag.len() > 1 {
            tag[1..].to_vec()
        } else {
            Vec::new()
        };
        builder = builder.tag(Tag::custom(kind, values));
    }
    let signed = builder.sign_with_keys(keys).expect("sign event");
    let value = serde_json::to_value(&signed).expect("event json");
    nostr::parse_event(&value).expect("parse event")
}

fn build_access_control_event(keys: &Keys, kind: u32, topic_id: &str) -> nostr::RawEvent {
    match kind {
        KIND_KEY_ENVELOPE => {
            let recipient = Keys::generate().public_key().to_string();
            let tags = vec![
                vec!["k".to_string(), KIP_NAMESPACE.to_string()],
                vec!["ver".to_string(), KIP_VERSION.to_string()],
                vec!["p".to_string(), recipient.clone()],
                vec!["t".to_string(), topic_id.to_string()],
                vec!["scope".to_string(), "invite".to_string()],
                vec!["epoch".to_string(), "1".to_string()],
                vec![
                    "d".to_string(),
                    format!("keyenv:{topic_id}:invite:1:{recipient}"),
                ],
            ];
            let content = json!({
                "schema": SCHEMA_KEY_ENVELOPE,
                "topic": topic_id,
                "scope": "invite",
                "epoch": 1,
            })
            .to_string();
            nostr::build_signed_event(keys, KIND_KEY_ENVELOPE as u16, tags, content)
                .expect("build key.envelope")
        }
        KIND_INVITE_CAPABILITY => {
            let nonce = Uuid::new_v4().to_string();
            let tags = vec![
                vec!["k".to_string(), KIP_NAMESPACE.to_string()],
                vec!["ver".to_string(), KIP_VERSION.to_string()],
                vec!["t".to_string(), topic_id.to_string()],
                vec!["scope".to_string(), "invite".to_string()],
                vec!["d".to_string(), format!("invite:{nonce}")],
            ];
            let content = json!({
                "schema": SCHEMA_INVITE_CAPABILITY,
            })
            .to_string();
            nostr::build_signed_event(keys, KIND_INVITE_CAPABILITY as u16, tags, content)
                .expect("build invite.capability")
        }
        KIND_JOIN_REQUEST => {
            let nonce = Uuid::new_v4().to_string();
            let requester = keys.public_key().to_string();
            let tags = vec![
                vec!["k".to_string(), KIP_NAMESPACE.to_string()],
                vec!["ver".to_string(), KIP_VERSION.to_string()],
                vec!["t".to_string(), topic_id.to_string()],
                vec!["scope".to_string(), "invite".to_string()],
                vec![
                    "d".to_string(),
                    format!("join:{topic_id}:{nonce}:{requester}"),
                ],
            ];
            let content = json!({
                "schema": SCHEMA_JOIN_REQUEST,
            })
            .to_string();
            nostr::build_signed_event(keys, KIND_JOIN_REQUEST as u16, tags, content)
                .expect("build join.request")
        }
        _ => panic!("unsupported access control kind: {kind}"),
    }
}

async fn wait_for_auth_challenge(ws: &mut WsStream, label: &str) -> String {
    let auth = wait_for_ws_json_any(ws, WAIT_TIMEOUT, label, |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("AUTH")
    })
    .await;
    let challenge = auth
        .get(1)
        .and_then(|v| v.as_str())
        .expect("auth challenge");
    assert!(!challenge.is_empty(), "AUTH challenge should not be empty");
    challenge.to_string()
}

async fn send_auth(ws: &mut WsStream, keys: &Keys, challenge: &str) -> String {
    let auth_event = nostr::build_signed_event(
        keys,
        AUTH_EVENT_KIND,
        vec![vec!["challenge".to_string(), challenge.to_string()]],
        String::new(),
    )
    .expect("build auth event");
    let auth_event_id = auth_event.id.clone();
    ws.send(Message::Text(
        json!(["AUTH", auth_event]).to_string().into(),
    ))
    .await
    .expect("send auth");
    auth_event_id
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("json body")
}

async fn response_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    String::from_utf8_lossy(&body).to_string()
}

fn assert_metric_line(body: &str, metric_name: &str, labels: &[(&str, &str)]) {
    let found = body.lines().any(|line| {
        if !line.starts_with(metric_name) {
            return false;
        }
        labels.iter().all(|(key, value)| {
            let token = format!("{key}=\"{value}\"");
            line.contains(&token)
        })
    });

    assert!(
        found,
        "metrics body did not contain {metric_name} with labels {labels:?}: {body}"
    );
}

#[tokio::test]
async fn healthz_contract_success_shape_compatible() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;
    let state = build_state(pool.clone());
    reset_topic_health_state(&pool, &state).await;

    let response = super::healthz(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(
        payload.get("status").and_then(|value| value.as_str()),
        Some("ok")
    );
}

#[tokio::test]
async fn healthz_contract_dependency_unavailable_shape_compatible() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    pool.close().await;
    let state = build_state(pool);

    let response = super::healthz(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let payload = response_json(response).await;
    assert_eq!(
        payload.get("status").and_then(|value| value.as_str()),
        Some("unavailable")
    );
}

#[tokio::test]
async fn healthz_contract_gossip_dependency_unavailable_shape_compatible() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;
    let state = build_state(pool.clone());
    reset_topic_health_state(&pool, &state).await;

    let topic_id = format!("kukuri:relay-healthz-gossip-it-{}", Uuid::new_v4());
    enable_topic(&pool, &state, &topic_id).await;

    let response = super::healthz(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let payload = response_json(response).await;
    assert_eq!(
        payload.get("status").and_then(|value| value.as_str()),
        Some("unavailable")
    );
}

#[tokio::test]
async fn healthz_contract_topic_sync_degraded_shape_compatible() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;
    let state = build_state(pool.clone());
    reset_topic_health_state(&pool, &state).await;

    {
        let mut topics = state.node_topics.write().await;
        topics.insert(format!("kukuri:relay-healthz-stale-it-{}", Uuid::new_v4()));
    }

    let response = super::healthz(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let payload = response_json(response).await;
    assert_eq!(
        payload.get("status").and_then(|value| value.as_str()),
        Some("degraded")
    );
}

#[tokio::test]
async fn metrics_contract_required_metrics_shape_compatible() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    metrics::inc_ws_connections(super::SERVICE_NAME);
    metrics::dec_ws_connections(super::SERVICE_NAME);
    metrics::inc_ws_unauthenticated_connections(super::SERVICE_NAME);
    metrics::dec_ws_unauthenticated_connections(super::SERVICE_NAME);
    metrics::inc_ws_req_total(super::SERVICE_NAME);
    metrics::inc_ws_event_total(super::SERVICE_NAME);
    metrics::inc_ws_auth_disconnect(super::SERVICE_NAME, "timeout");
    metrics::inc_ingest_received(super::SERVICE_NAME, "contract");
    metrics::inc_ingest_rejected(super::SERVICE_NAME, "contract");
    metrics::inc_gossip_received(super::SERVICE_NAME);
    metrics::inc_gossip_sent(super::SERVICE_NAME);
    metrics::inc_dedupe_hit(super::SERVICE_NAME);
    metrics::inc_dedupe_miss(super::SERVICE_NAME);
    let route = "/metrics-contract";
    metrics::record_http_request(
        super::SERVICE_NAME,
        "GET",
        route,
        200,
        std::time::Duration::from_millis(5),
    );

    let state = build_state(pool);
    let response = super::metrics_endpoint(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    assert_eq!(content_type, Some("text/plain; version=0.0.4"));

    let body = response_text(response).await;
    assert_metric_line(&body, "ws_connections", &[("service", super::SERVICE_NAME)]);
    assert_metric_line(
        &body,
        "ws_unauthenticated_connections",
        &[("service", super::SERVICE_NAME)],
    );
    assert_metric_line(&body, "ws_req_total", &[("service", super::SERVICE_NAME)]);
    assert_metric_line(&body, "ws_event_total", &[("service", super::SERVICE_NAME)]);
    assert_metric_line(
        &body,
        "ws_auth_disconnect_total",
        &[("service", super::SERVICE_NAME), ("reason", "timeout")],
    );
    assert_metric_line(
        &body,
        "ingest_received_total",
        &[("service", super::SERVICE_NAME), ("source", "contract")],
    );
    assert_metric_line(
        &body,
        "ingest_rejected_total",
        &[("service", super::SERVICE_NAME), ("reason", "contract")],
    );
    assert_metric_line(
        &body,
        "gossip_received_total",
        &[("service", super::SERVICE_NAME)],
    );
    assert_metric_line(
        &body,
        "gossip_sent_total",
        &[("service", super::SERVICE_NAME)],
    );
    assert_metric_line(
        &body,
        "dedupe_hits_total",
        &[("service", super::SERVICE_NAME)],
    );
    assert_metric_line(
        &body,
        "dedupe_misses_total",
        &[("service", super::SERVICE_NAME)],
    );
    assert_metric_line(
        &body,
        "http_requests_total",
        &[
            ("service", super::SERVICE_NAME),
            ("route", route),
            ("method", "GET"),
            ("status", "200"),
        ],
    );
    assert_metric_line(
        &body,
        "http_request_duration_seconds_bucket",
        &[
            ("service", super::SERVICE_NAME),
            ("route", route),
            ("method", "GET"),
            ("status", "200"),
        ],
    );
}

#[tokio::test]
async fn ingest_outbox_ws_gossip_integration() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;

    let (gossip_sender, mut gossip) = setup_gossip(&topic_id).await;
    {
        let mut senders = state.gossip_senders.write().await;
        senders.insert(topic_id.clone(), gossip_sender);
    }

    let (addr, server_handle) = spawn_relay_server(state).await;

    let mut subscriber = connect_ws(addr).await;
    let sub_id = "sub-1";
    let req = json!(["REQ", sub_id, { "kinds": [1], "#t": [topic_id.clone()] }]);
    subscriber
        .send(Message::Text(req.to_string().into()))
        .await
        .expect("send req");
    let _ = wait_for_ws_json(&mut subscriber, WAIT_TIMEOUT, "subscriber eose", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("EOSE")
            && value.get(1).and_then(|v| v.as_str()) == Some(sub_id)
    })
    .await;

    let mut publisher = connect_ws(addr).await;
    let keys = Keys::generate();
    let raw = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "integration-test".to_string(),
    )
    .expect("build event");
    let event_msg = json!(["EVENT", raw.clone()]);
    publisher
        .send(Message::Text(event_msg.to_string().into()))
        .await
        .expect("send event");

    let _ = wait_for_ws_json(&mut publisher, WAIT_TIMEOUT, "publisher ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(raw.id.as_str())
    })
    .await;

    let _ = wait_for_ws_json(&mut subscriber, WAIT_TIMEOUT, "subscriber event", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("EVENT")
            && value
                .get(2)
                .and_then(|event| event.get("id"))
                .and_then(|id| id.as_str())
                == Some(raw.id.as_str())
    })
    .await;

    let row = sqlx::query("SELECT op, topic_id FROM cn_relay.events_outbox WHERE event_id = $1")
        .bind(&raw.id)
        .fetch_one(&pool)
        .await
        .expect("outbox row");
    let op: String = row.try_get("op").expect("op");
    let outbox_topic: String = row.try_get("topic_id").expect("topic_id");
    assert_eq!(op, "upsert");
    assert_eq!(outbox_topic, topic_id);

    wait_for_gossip_event(&mut gossip.receiver, WAIT_TIMEOUT, &raw.id).await;

    server_handle.abort();
    let _ = timeout(WAIT_TIMEOUT, gossip.router_a.shutdown()).await;
    let _ = timeout(WAIT_TIMEOUT, gossip.router_b.shutdown()).await;
}

#[tokio::test]
async fn replaceable_and_addressable_tiebreak_prefers_lexicographically_smaller_event_id() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-tiebreak-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;
    let (addr, server_handle) = spawn_relay_server(state).await;

    let mut ws = connect_ws(addr).await;
    let keys = Keys::generate();
    let pubkey = keys.public_key().to_string();
    let created_at = cn_core::auth::unix_seconds().expect("unix seconds") as i64;

    let replaceable_a = build_event_at(
        &keys,
        10002,
        vec![vec!["t".to_string(), topic_id.clone()]],
        format!("replaceable-a-{}", Uuid::new_v4()),
        created_at,
    );
    let replaceable_b = build_event_at(
        &keys,
        10002,
        vec![vec!["t".to_string(), topic_id.clone()]],
        format!("replaceable-b-{}", Uuid::new_v4()),
        created_at,
    );
    let (replaceable_first, replaceable_second) = if replaceable_a.id > replaceable_b.id {
        (replaceable_a, replaceable_b)
    } else {
        (replaceable_b, replaceable_a)
    };

    ws.send(Message::Text(
        json!(["EVENT", replaceable_first.clone()])
            .to_string()
            .into(),
    ))
    .await
    .expect("send first replaceable");
    let first_replaceable_ok =
        wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "first replaceable ok", |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(replaceable_first.id.as_str())
        })
        .await;
    assert_eq!(
        first_replaceable_ok.get(2).and_then(|v| v.as_bool()),
        Some(true)
    );

    ws.send(Message::Text(
        json!(["EVENT", replaceable_second.clone()])
            .to_string()
            .into(),
    ))
    .await
    .expect("send second replaceable");
    let second_replaceable_ok =
        wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "second replaceable ok", |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(replaceable_second.id.as_str())
        })
        .await;
    assert_eq!(
        second_replaceable_ok.get(2).and_then(|v| v.as_bool()),
        Some(true)
    );

    let replaceable_key = format!("{pubkey}:10002");
    let replaceable_current: String = sqlx::query_scalar(
        "SELECT event_id FROM cn_relay.replaceable_current WHERE replaceable_key = $1",
    )
    .bind(&replaceable_key)
    .fetch_one(&pool)
    .await
    .expect("replaceable current event");
    assert_eq!(replaceable_current, replaceable_second.id);
    let replaceable_first_current: bool =
        sqlx::query_scalar("SELECT is_current FROM cn_relay.events WHERE event_id = $1")
            .bind(&replaceable_first.id)
            .fetch_one(&pool)
            .await
            .expect("first replaceable current flag");
    assert!(!replaceable_first_current);
    let replaceable_second_current: bool =
        sqlx::query_scalar("SELECT is_current FROM cn_relay.events WHERE event_id = $1")
            .bind(&replaceable_second.id)
            .fetch_one(&pool)
            .await
            .expect("second replaceable current flag");
    assert!(replaceable_second_current);

    let d_tag = "profile";
    let addressable_a = build_event_at(
        &keys,
        30023,
        vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["d".to_string(), d_tag.to_string()],
        ],
        format!("addressable-a-{}", Uuid::new_v4()),
        created_at,
    );
    let addressable_b = build_event_at(
        &keys,
        30023,
        vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["d".to_string(), d_tag.to_string()],
        ],
        format!("addressable-b-{}", Uuid::new_v4()),
        created_at,
    );
    let (addressable_first, addressable_second) = if addressable_a.id > addressable_b.id {
        (addressable_a, addressable_b)
    } else {
        (addressable_b, addressable_a)
    };

    ws.send(Message::Text(
        json!(["EVENT", addressable_first.clone()])
            .to_string()
            .into(),
    ))
    .await
    .expect("send first addressable");
    let first_addressable_ok =
        wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "first addressable ok", |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(addressable_first.id.as_str())
        })
        .await;
    assert_eq!(
        first_addressable_ok.get(2).and_then(|v| v.as_bool()),
        Some(true)
    );

    ws.send(Message::Text(
        json!(["EVENT", addressable_second.clone()])
            .to_string()
            .into(),
    ))
    .await
    .expect("send second addressable");
    let second_addressable_ok =
        wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "second addressable ok", |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(addressable_second.id.as_str())
        })
        .await;
    assert_eq!(
        second_addressable_ok.get(2).and_then(|v| v.as_bool()),
        Some(true)
    );

    let addressable_key = format!("30023:{pubkey}:{d_tag}");
    let addressable_current: String = sqlx::query_scalar(
        "SELECT event_id FROM cn_relay.addressable_current WHERE addressable_key = $1",
    )
    .bind(&addressable_key)
    .fetch_one(&pool)
    .await
    .expect("addressable current event");
    assert_eq!(addressable_current, addressable_second.id);
    let addressable_first_current: bool =
        sqlx::query_scalar("SELECT is_current FROM cn_relay.events WHERE event_id = $1")
            .bind(&addressable_first.id)
            .fetch_one(&pool)
            .await
            .expect("first addressable current flag");
    assert!(!addressable_first_current);
    let addressable_second_current: bool =
        sqlx::query_scalar("SELECT is_current FROM cn_relay.events WHERE event_id = $1")
            .bind(&addressable_second.id)
            .fetch_one(&pool)
            .await
            .expect("second addressable current flag");
    assert!(addressable_second_current);

    server_handle.abort();
}

#[tokio::test]
async fn kind5_deletion_applies_e_and_a_targets() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-kind5-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;
    let (addr, server_handle) = spawn_relay_server(state).await;

    let mut ws = connect_ws(addr).await;
    let keys = Keys::generate();
    let pubkey = keys.public_key().to_string();

    let regular = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "kind5-target-e".to_string(),
    )
    .expect("build regular target");
    ws.send(Message::Text(
        json!(["EVENT", regular.clone()]).to_string().into(),
    ))
    .await
    .expect("send regular target");
    let regular_ok = wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "regular target ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(regular.id.as_str())
    })
    .await;
    assert_eq!(regular_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let d_tag = "profile";
    let addressable = nostr::build_signed_event(
        &keys,
        30023,
        vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["d".to_string(), d_tag.to_string()],
        ],
        "kind5-target-a".to_string(),
    )
    .expect("build addressable target");
    ws.send(Message::Text(
        json!(["EVENT", addressable.clone()]).to_string().into(),
    ))
    .await
    .expect("send addressable target");
    let addressable_ok =
        wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "addressable target ok", |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(addressable.id.as_str())
        })
        .await;
    assert_eq!(addressable_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let deletion = nostr::build_signed_event(
        &keys,
        5,
        vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["e".to_string(), regular.id.clone()],
            vec!["a".to_string(), format!("30023:{pubkey}:{d_tag}")],
        ],
        "kind5-delete".to_string(),
    )
    .expect("build deletion event");
    ws.send(Message::Text(
        json!(["EVENT", deletion.clone()]).to_string().into(),
    ))
    .await
    .expect("send deletion event");
    let deletion_ok = wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "deletion ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(deletion.id.as_str())
    })
    .await;
    assert_eq!(deletion_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let deletion_event_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
            .bind(&deletion.id)
            .fetch_one(&pool)
            .await
            .expect("count deletion event");
    assert_eq!(deletion_event_count, 1);

    let regular_deleted: bool =
        sqlx::query_scalar("SELECT is_deleted FROM cn_relay.events WHERE event_id = $1")
            .bind(&regular.id)
            .fetch_one(&pool)
            .await
            .expect("regular deleted flag");
    assert!(regular_deleted);
    let addressable_deleted: bool =
        sqlx::query_scalar("SELECT is_deleted FROM cn_relay.events WHERE event_id = $1")
            .bind(&addressable.id)
            .fetch_one(&pool)
            .await
            .expect("addressable deleted flag");
    assert!(addressable_deleted);

    let addressable_current_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_relay.addressable_current WHERE addressable_key = $1",
    )
    .bind(format!("30023:{pubkey}:{d_tag}"))
    .fetch_one(&pool)
    .await
    .expect("count addressable current rows");
    assert_eq!(addressable_current_count, 0);

    let regular_delete_reason: String = sqlx::query_scalar(
        "SELECT reason FROM cn_relay.events_outbox WHERE event_id = $1 AND op = 'delete' ORDER BY seq DESC LIMIT 1",
    )
    .bind(&regular.id)
    .fetch_one(&pool)
    .await
    .expect("regular delete outbox reason");
    assert_eq!(regular_delete_reason, "nip09");

    let addressable_delete_reason: String = sqlx::query_scalar(
        "SELECT reason FROM cn_relay.events_outbox WHERE event_id = $1 AND op = 'delete' ORDER BY seq DESC LIMIT 1",
    )
    .bind(&addressable.id)
    .fetch_one(&pool)
    .await
    .expect("addressable delete outbox reason");
    assert_eq!(addressable_delete_reason, "nip09");

    server_handle.abort();
}

#[tokio::test]
async fn expiration_reaches_and_enqueues_delete_outbox_notification() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-expiration-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;
    let cleanup_state = state.clone();
    let (addr, server_handle) = spawn_relay_server(state).await;

    let mut ws = connect_ws(addr).await;
    let keys = Keys::generate();
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let expires_at = now + 2;
    let raw = build_event_at(
        &keys,
        1,
        vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["expiration".to_string(), expires_at.to_string()],
        ],
        "expires-soon".to_string(),
        now,
    );
    ws.send(Message::Text(
        json!(["EVENT", raw.clone()]).to_string().into(),
    ))
    .await
    .expect("send expiration event");
    let publish_ok = wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "expiration event ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(raw.id.as_str())
    })
    .await;
    assert_eq!(publish_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    tokio::time::sleep(Duration::from_secs(3)).await;

    crate::retention::cleanup_once(
        &cleanup_state,
        &crate::config::RelayRetention {
            events_days: 0,
            tombstone_days: 0,
            dedupe_days: 0,
            outbox_days: 0,
            cleanup_interval_seconds: 60,
        },
    )
    .await
    .expect("run relay cleanup once");

    let deleted: bool =
        sqlx::query_scalar("SELECT is_deleted FROM cn_relay.events WHERE event_id = $1")
            .bind(&raw.id)
            .fetch_one(&pool)
            .await
            .expect("expired event deleted flag");
    assert!(deleted);
    let delete_reason: String = sqlx::query_scalar(
        "SELECT reason FROM cn_relay.events_outbox WHERE event_id = $1 AND op = 'delete' ORDER BY seq DESC LIMIT 1",
    )
    .bind(&raw.id)
    .fetch_one(&pool)
    .await
    .expect("expiration delete reason");
    assert_eq!(delete_reason, "expiration");

    server_handle.abort();
}

#[tokio::test]
async fn ephemeral_event_is_not_persisted_but_is_delivered_in_realtime() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-ephemeral-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;

    let (gossip_sender, mut gossip) = setup_gossip(&topic_id).await;
    {
        let mut senders = state.gossip_senders.write().await;
        senders.insert(topic_id.clone(), gossip_sender);
    }

    let (addr, server_handle) = spawn_relay_server(state).await;

    let mut subscriber = connect_ws(addr).await;
    let sub_id = "sub-ephemeral";
    subscriber
        .send(Message::Text(
            json!(["REQ", sub_id, { "kinds": [20001], "#t": [topic_id.clone()] }])
                .to_string()
                .into(),
        ))
        .await
        .expect("send ephemeral req");
    let _ = wait_for_ws_json(&mut subscriber, WAIT_TIMEOUT, "ephemeral eose", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("EOSE")
            && value.get(1).and_then(|v| v.as_str()) == Some(sub_id)
    })
    .await;

    let mut publisher = connect_ws(addr).await;
    let keys = Keys::generate();
    let raw = nostr::build_signed_event(
        &keys,
        20001,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "ephemeral".to_string(),
    )
    .expect("build ephemeral event");
    publisher
        .send(Message::Text(
            json!(["EVENT", raw.clone()]).to_string().into(),
        ))
        .await
        .expect("send ephemeral event");

    let publish_ok = wait_for_ws_json(&mut publisher, WAIT_TIMEOUT, "ephemeral ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(raw.id.as_str())
    })
    .await;
    assert_eq!(publish_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let _ = wait_for_ws_json(&mut subscriber, WAIT_TIMEOUT, "ephemeral event", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("EVENT")
            && value
                .get(2)
                .and_then(|event| event.get("id"))
                .and_then(|id| id.as_str())
                == Some(raw.id.as_str())
    })
    .await;
    wait_for_gossip_event(&mut gossip.receiver, WAIT_TIMEOUT, &raw.id).await;

    let event_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
            .bind(&raw.id)
            .fetch_one(&pool)
            .await
            .expect("count relay events");
    assert_eq!(event_count, 0);
    let topic_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_topics WHERE event_id = $1")
            .bind(&raw.id)
            .fetch_one(&pool)
            .await
            .expect("count relay event topics");
    assert_eq!(topic_count, 0);
    let outbox_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events_outbox WHERE event_id = $1")
            .bind(&raw.id)
            .fetch_one(&pool)
            .await
            .expect("count relay outbox");
    assert_eq!(outbox_count, 0);
    let dedupe_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_dedupe WHERE event_id = $1")
            .bind(&raw.id)
            .fetch_one(&pool)
            .await
            .expect("count relay dedupe");
    assert_eq!(dedupe_count, 0);

    server_handle.abort();
    let _ = timeout(WAIT_TIMEOUT, gossip.router_a.shutdown()).await;
    let _ = timeout(WAIT_TIMEOUT, gossip.router_b.shutdown()).await;
}

#[tokio::test]
async fn access_control_events_are_rejected_and_not_distributed() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-access-control-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;

    let (gossip_sender, mut gossip) = setup_gossip(&topic_id).await;
    {
        let mut senders = state.gossip_senders.write().await;
        senders.insert(topic_id.clone(), gossip_sender);
    }

    let (addr, server_handle) = spawn_relay_server(state).await;

    let mut subscriber = connect_ws(addr).await;
    let sub_id = "sub-access-control";
    subscriber
        .send(Message::Text(
            json!(["REQ", sub_id, { "kinds": [KIND_KEY_ENVELOPE, KIND_INVITE_CAPABILITY, KIND_JOIN_REQUEST], "#t": [topic_id.clone()] }])
                .to_string()
                .into(),
        ))
        .await
        .expect("send subscriber req");
    let _ = wait_for_ws_json(&mut subscriber, WAIT_TIMEOUT, "subscriber eose", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("EOSE")
            && value.get(1).and_then(|v| v.as_str()) == Some(sub_id)
    })
    .await;

    let mut publisher = connect_ws(addr).await;
    let keys = Keys::generate();
    for kind in [KIND_KEY_ENVELOPE, KIND_INVITE_CAPABILITY, KIND_JOIN_REQUEST] {
        let raw = build_access_control_event(&keys, kind, &topic_id);
        publisher
            .send(Message::Text(
                json!(["EVENT", raw.clone()]).to_string().into(),
            ))
            .await
            .expect("send access control event");

        let rejected =
            wait_for_ws_json(&mut publisher, WAIT_TIMEOUT, "publisher reject", |value| {
                value.get(0).and_then(|v| v.as_str()) == Some("OK")
                    && value.get(1).and_then(|v| v.as_str()) == Some(raw.id.as_str())
            })
            .await;
        assert_eq!(rejected.get(2).and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            rejected.get(3).and_then(|v| v.as_str()),
            Some(ACCESS_CONTROL_P2P_ONLY_REASON)
        );

        let event_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
                .bind(&raw.id)
                .fetch_one(&pool)
                .await
                .expect("count relay events");
        assert_eq!(event_count, 0, "event should not be stored for kind={kind}");

        let topic_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_topics WHERE event_id = $1")
                .bind(&raw.id)
                .fetch_one(&pool)
                .await
                .expect("count relay event topics");
        assert_eq!(
            topic_count, 0,
            "event_topics should remain empty for kind={kind}"
        );

        let outbox_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events_outbox WHERE event_id = $1")
                .bind(&raw.id)
                .fetch_one(&pool)
                .await
                .expect("count relay outbox events");
        assert_eq!(
            outbox_count, 0,
            "outbox should remain empty for kind={kind}"
        );

        assert_no_ws_event(
            &mut subscriber,
            Duration::from_millis(500),
            &format!("access control kind={kind}"),
        )
        .await;
        assert_no_gossip_received(
            &mut gossip.receiver,
            Duration::from_millis(500),
            &format!("access control kind={kind}"),
        )
        .await;
    }

    server_handle.abort();
    let _ = timeout(WAIT_TIMEOUT, gossip.router_a.shutdown()).await;
    let _ = timeout(WAIT_TIMEOUT, gossip.router_b.shutdown()).await;
}

#[tokio::test]
async fn ws_backfill_orders_desc_applies_limit_and_transitions_to_realtime_after_eose() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-backfill-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;

    let newest_id = unique_hex_event_id('a');
    let tie_low_id = unique_hex_event_id('1');
    let tie_high_id = unique_hex_event_id('9');
    let oldest_id = unique_hex_event_id('0');

    insert_backfill_event(&pool, &topic_id, &newest_id, 1_700_000_300, "newest").await;
    insert_backfill_event(&pool, &topic_id, &tie_high_id, 1_700_000_200, "tie-high").await;
    insert_backfill_event(&pool, &topic_id, &tie_low_id, 1_700_000_200, "tie-low").await;
    insert_backfill_event(&pool, &topic_id, &oldest_id, 1_700_000_100, "oldest").await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut subscriber = connect_ws(addr).await;
    let sub_id = "sub-backfill-limit";
    subscriber
        .send(Message::Text(
            json!(["REQ", sub_id, { "kinds": [1], "#t": [topic_id.clone()], "limit": 2 }])
                .to_string()
                .into(),
        ))
        .await
        .expect("send req with limit");

    let first = wait_for_ws_json(
        &mut subscriber,
        WAIT_TIMEOUT,
        "first backfill message",
        |_| true,
    )
    .await;
    assert_eq!(first.get(0).and_then(|v| v.as_str()), Some("EVENT"));
    assert_eq!(first.get(1).and_then(|v| v.as_str()), Some(sub_id));
    assert_eq!(
        first
            .get(2)
            .and_then(|event| event.get("id"))
            .and_then(|id| id.as_str()),
        Some(newest_id.as_str())
    );

    let second = wait_for_ws_json(
        &mut subscriber,
        WAIT_TIMEOUT,
        "second backfill message",
        |_| true,
    )
    .await;
    assert_eq!(second.get(0).and_then(|v| v.as_str()), Some("EVENT"));
    assert_eq!(second.get(1).and_then(|v| v.as_str()), Some(sub_id));
    assert_eq!(
        second
            .get(2)
            .and_then(|event| event.get("id"))
            .and_then(|id| id.as_str()),
        Some(tie_low_id.as_str())
    );

    let third = wait_for_ws_json(&mut subscriber, WAIT_TIMEOUT, "backfill eose", |_| true).await;
    assert_eq!(third.get(0).and_then(|v| v.as_str()), Some("EOSE"));
    assert_eq!(third.get(1).and_then(|v| v.as_str()), Some(sub_id));

    let mut publisher = connect_ws(addr).await;
    let keys = Keys::generate();
    let realtime = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "realtime-after-eose".to_string(),
    )
    .expect("build realtime event");
    publisher
        .send(Message::Text(
            json!(["EVENT", realtime.clone()]).to_string().into(),
        ))
        .await
        .expect("send realtime event");

    let publish_ok = wait_for_ws_json(&mut publisher, WAIT_TIMEOUT, "publisher ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(realtime.id.as_str())
    })
    .await;
    assert_eq!(publish_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let delivered = wait_for_ws_json(
        &mut subscriber,
        WAIT_TIMEOUT,
        "realtime event after eose",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("EVENT")
                && value
                    .get(2)
                    .and_then(|event| event.get("id"))
                    .and_then(|id| id.as_str())
                    == Some(realtime.id.as_str())
        },
    )
    .await;
    assert_eq!(delivered.get(1).and_then(|v| v.as_str()), Some(sub_id));

    server_handle.abort();
}

#[tokio::test]
async fn req_filter_time_range_rejections_use_stable_notice_reasons() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-req-time-range-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut ws = connect_ws(addr).await;
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;

    assert_req_notice_reason(
        &mut ws,
        json!(["REQ", "sub-invalid-order", {
            "#t": [topic_id.clone()],
            "since": now,
            "until": now - 1
        }]),
        "invalid since/until range",
        "reject since > until",
    )
    .await;

    assert_req_notice_reason(
        &mut ws,
        json!(["REQ", "sub-over-lookback", {
            "#t": [topic_id.clone()],
            "since": 0
        }]),
        "lookback too large",
        "reject over lookback",
    )
    .await;

    assert_req_notice_reason(
        &mut ws,
        json!(["REQ", "sub-over-window", {
            "#t": [topic_id],
            "since": now - 10,
            "until": now + 31_536_000
        }]),
        "time window too large",
        "reject over window",
    )
    .await;

    server_handle.abort();
}

#[tokio::test]
async fn ws_backfill_skips_history_when_topic_policy_disables_backfill() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-backfill-policy-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;
    update_topic_ingest_policy(
        &pool,
        &topic_id,
        json!({
            "allow_backfill": false
        }),
    )
    .await;
    let historical_event_id = unique_hex_event_id('b');
    insert_backfill_event(
        &pool,
        &topic_id,
        &historical_event_id,
        1_700_001_000,
        "historical",
    )
    .await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut subscriber = connect_ws(addr).await;
    let sub_id = "sub-no-backfill";
    subscriber
        .send(Message::Text(
            json!(["REQ", sub_id, { "kinds": [1], "#t": [topic_id.clone()], "limit": 10 }])
                .to_string()
                .into(),
        ))
        .await
        .expect("send req");

    let first = wait_for_ws_json(
        &mut subscriber,
        WAIT_TIMEOUT,
        "eose without backfill",
        |_| true,
    )
    .await;
    assert_eq!(first.get(0).and_then(|v| v.as_str()), Some("EOSE"));
    assert_eq!(first.get(1).and_then(|v| v.as_str()), Some(sub_id));

    let mut publisher = connect_ws(addr).await;
    let keys = Keys::generate();
    let realtime = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "realtime-only".to_string(),
    )
    .expect("build realtime event");
    publisher
        .send(Message::Text(
            json!(["EVENT", realtime.clone()]).to_string().into(),
        ))
        .await
        .expect("send realtime event");
    let publish_ok = wait_for_ws_json(&mut publisher, WAIT_TIMEOUT, "publisher ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(realtime.id.as_str())
    })
    .await;
    assert_eq!(publish_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let delivered = wait_for_ws_json(
        &mut subscriber,
        WAIT_TIMEOUT,
        "realtime after no-backfill eose",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("EVENT")
                && value
                    .get(2)
                    .and_then(|event| event.get("id"))
                    .and_then(|id| id.as_str())
                    == Some(realtime.id.as_str())
        },
    )
    .await;
    assert_eq!(delivered.get(1).and_then(|v| v.as_str()), Some(sub_id));

    server_handle.abort();
}

#[tokio::test]
async fn ingest_rejects_when_topic_max_events_capacity_is_reached() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-capacity-events-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;
    update_topic_ingest_policy(
        &pool,
        &topic_id,
        json!({
            "max_events": 1
        }),
    )
    .await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut publisher = connect_ws(addr).await;
    let keys = Keys::generate();

    let first = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "capacity-first".to_string(),
    )
    .expect("build first event");
    publisher
        .send(Message::Text(
            json!(["EVENT", first.clone()]).to_string().into(),
        ))
        .await
        .expect("send first event");
    let first_ok = wait_for_ws_json(&mut publisher, WAIT_TIMEOUT, "first event ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(first.id.as_str())
    })
    .await;
    assert_eq!(first_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let second = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "capacity-second".to_string(),
    )
    .expect("build second event");
    publisher
        .send(Message::Text(
            json!(["EVENT", second.clone()]).to_string().into(),
        ))
        .await
        .expect("send second event");
    let second_rejected = wait_for_ws_json(
        &mut publisher,
        WAIT_TIMEOUT,
        "second event capacity reject",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(second.id.as_str())
        },
    )
    .await;
    assert_eq!(
        second_rejected.get(2).and_then(|v| v.as_bool()),
        Some(false)
    );
    let reason = second_rejected
        .get(3)
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        reason.starts_with("restricted: ingest capacity exceeded"),
        "unexpected capacity reject reason: {reason}"
    );

    let second_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
            .bind(&second.id)
            .fetch_one(&pool)
            .await
            .expect("count second event");
    assert_eq!(second_count, 0);

    server_handle.abort();
}

#[tokio::test]
async fn cleanup_applies_topic_retention_override_from_ingest_policy() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let override_topic = format!("kukuri:relay-retention-override-it-{}", Uuid::new_v4());
    let default_topic = format!("kukuri:relay-retention-default-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &override_topic).await;
    enable_topic(&pool, &state, &default_topic).await;
    update_topic_ingest_policy(
        &pool,
        &override_topic,
        json!({
            "retention_days": 1
        }),
    )
    .await;

    let override_event_id = format!("retention-override-{}", Uuid::new_v4().simple());
    let default_event_id = format!("retention-default-{}", Uuid::new_v4().simple());
    insert_non_current_event(&pool, &override_topic, &override_event_id, 1_700_002_000, 2).await;
    insert_non_current_event(&pool, &default_topic, &default_event_id, 1_700_002_100, 2).await;

    crate::retention::cleanup_once(
        &state,
        &crate::config::RelayRetention {
            events_days: 30,
            tombstone_days: 0,
            dedupe_days: 0,
            outbox_days: 0,
            cleanup_interval_seconds: 60,
        },
    )
    .await
    .expect("run retention cleanup");

    let override_event_topics: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_topics WHERE event_id = $1")
            .bind(&override_event_id)
            .fetch_one(&pool)
            .await
            .expect("count override event_topics");
    let override_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
            .bind(&override_event_id)
            .fetch_one(&pool)
            .await
            .expect("count override events");
    assert_eq!(override_event_topics, 0);
    assert_eq!(override_events, 0);

    let default_event_topics: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_topics WHERE event_id = $1")
            .bind(&default_event_id)
            .fetch_one(&pool)
            .await
            .expect("count default event_topics");
    let default_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
            .bind(&default_event_id)
            .fetch_one(&pool)
            .await
            .expect("count default events");
    assert_eq!(default_event_topics, 1);
    assert_eq!(default_events, 1);
}

#[tokio::test]
async fn cleanup_cleans_all_retention_targets_and_preserves_policy_backcompat() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let fallback_topic = format!("kukuri:relay-retention-fallback-it-{}", Uuid::new_v4());
    let override_topic = format!("kukuri:relay-retention-override2-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &fallback_topic).await;
    enable_topic(&pool, &state, &override_topic).await;

    update_topic_ingest_policy(
        &pool,
        &fallback_topic,
        json!({
            "retention_days": 0
        }),
    )
    .await;
    update_topic_ingest_policy(
        &pool,
        &override_topic,
        json!({
            "retention_days": 5
        }),
    )
    .await;

    let fallback_event_id = unique_hex_event_id('a');
    let override_event_id = unique_hex_event_id('b');
    insert_non_current_event(&pool, &fallback_topic, &fallback_event_id, 1_700_003_000, 2).await;
    insert_non_current_event(&pool, &override_topic, &override_event_id, 1_700_003_100, 2).await;

    let dedupe_old_event_id = unique_hex_event_id('c');
    let dedupe_recent_event_id = unique_hex_event_id('d');
    sqlx::query(
        "INSERT INTO cn_relay.event_dedupe (event_id, first_seen_at, last_seen_at, seen_count) \
         VALUES ($1, NOW() - (2 * INTERVAL '1 day'), NOW() - (2 * INTERVAL '1 day'), 1)",
    )
    .bind(&dedupe_old_event_id)
    .execute(&pool)
    .await
    .expect("insert old dedupe row");
    sqlx::query(
        "INSERT INTO cn_relay.event_dedupe (event_id, first_seen_at, last_seen_at, seen_count) \
         VALUES ($1, NOW() - INTERVAL '12 hours', NOW() - INTERVAL '12 hours', 1)",
    )
    .bind(&dedupe_recent_event_id)
    .execute(&pool)
    .await
    .expect("insert recent dedupe row");

    let outbox_old_event_id = unique_hex_event_id('e');
    let outbox_recent_event_id = unique_hex_event_id('f');
    sqlx::query(
        "INSERT INTO cn_relay.events_outbox \
         (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason) \
         VALUES ('upsert', $1, $2, 1, $3, NOW() - (2 * INTERVAL '1 day'), NULL, 'retention-it')",
    )
    .bind(&outbox_old_event_id)
    .bind(&fallback_topic)
    .bind(1_700_003_200_i64)
    .execute(&pool)
    .await
    .expect("insert old outbox row");
    sqlx::query(
        "INSERT INTO cn_relay.events_outbox \
         (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason) \
         VALUES ('upsert', $1, $2, 1, $3, NOW(), NULL, 'retention-it')",
    )
    .bind(&outbox_recent_event_id)
    .bind(&fallback_topic)
    .bind(1_700_003_300_i64)
    .execute(&pool)
    .await
    .expect("insert recent outbox row");

    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let tombstone_old_id = unique_hex_event_id('1');
    let tombstone_recent_id = unique_hex_event_id('2');
    sqlx::query(
        "INSERT INTO cn_relay.deletion_tombstones (target_event_id, deletion_event_id, requested_at) \
         VALUES ($1, $2, $3)",
    )
    .bind(&fallback_event_id)
    .bind(&tombstone_old_id)
    .bind(now.saturating_sub(2 * 86_400))
    .execute(&pool)
    .await
    .expect("insert old tombstone row");
    sqlx::query(
        "INSERT INTO cn_relay.deletion_tombstones (target_event_id, deletion_event_id, requested_at) \
         VALUES ($1, $2, $3)",
    )
    .bind(&override_event_id)
    .bind(&tombstone_recent_id)
    .bind(now.saturating_sub(60))
    .execute(&pool)
    .await
    .expect("insert recent tombstone row");

    crate::retention::cleanup_once(
        &state,
        &crate::config::RelayRetention {
            events_days: 1,
            tombstone_days: 1,
            dedupe_days: 1,
            outbox_days: 1,
            cleanup_interval_seconds: 60,
        },
    )
    .await
    .expect("run retention cleanup");

    let fallback_event_topics: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_topics WHERE event_id = $1")
            .bind(&fallback_event_id)
            .fetch_one(&pool)
            .await
            .expect("count fallback event_topics");
    let fallback_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
            .bind(&fallback_event_id)
            .fetch_one(&pool)
            .await
            .expect("count fallback events");
    assert_eq!(fallback_event_topics, 0);
    assert_eq!(fallback_events, 0);

    let override_event_topics: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_topics WHERE event_id = $1")
            .bind(&override_event_id)
            .fetch_one(&pool)
            .await
            .expect("count override event_topics");
    let override_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events WHERE event_id = $1")
            .bind(&override_event_id)
            .fetch_one(&pool)
            .await
            .expect("count override events");
    assert_eq!(override_event_topics, 1);
    assert_eq!(override_events, 1);

    let dedupe_old_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_dedupe WHERE event_id = $1")
            .bind(&dedupe_old_event_id)
            .fetch_one(&pool)
            .await
            .expect("count old dedupe row");
    let dedupe_recent_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.event_dedupe WHERE event_id = $1")
            .bind(&dedupe_recent_event_id)
            .fetch_one(&pool)
            .await
            .expect("count recent dedupe row");
    assert_eq!(dedupe_old_count, 0);
    assert_eq!(dedupe_recent_count, 1);

    let outbox_old_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events_outbox WHERE event_id = $1")
            .bind(&outbox_old_event_id)
            .fetch_one(&pool)
            .await
            .expect("count old outbox row");
    let outbox_recent_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_relay.events_outbox WHERE event_id = $1")
            .bind(&outbox_recent_event_id)
            .fetch_one(&pool)
            .await
            .expect("count recent outbox row");
    assert_eq!(outbox_old_count, 0);
    assert_eq!(outbox_recent_count, 1);

    let tombstone_old_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_relay.deletion_tombstones WHERE deletion_event_id = $1",
    )
    .bind(&tombstone_old_id)
    .fetch_one(&pool)
    .await
    .expect("count old tombstone row");
    let tombstone_recent_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_relay.deletion_tombstones WHERE deletion_event_id = $1",
    )
    .bind(&tombstone_recent_id)
    .fetch_one(&pool)
    .await
    .expect("count recent tombstone row");
    assert_eq!(tombstone_old_count, 0);
    assert_eq!(tombstone_recent_count, 1);
}

#[tokio::test]
async fn auth_enforce_existing_connection_disconnects_after_grace_period() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-auth-it-{}", Uuid::new_v4());
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let state = build_state_with_config(
        pool.clone(),
        json!({
            "auth": {
                "mode": "required",
                // Keep enough room for one pre-enforce tick and one post-enforce grace window.
                "enforce_at": now + 2,
                "grace_seconds": 7,
                "ws_auth_timeout_seconds": 1
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            },
            "rate_limit": {
                "enabled": false
            }
        }),
    );
    enable_topic(&pool, &state, &topic_id).await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut ws = connect_ws(addr).await;

    let sub_before_auth = "sub-before-auth";
    ws.send(Message::Text(
        json!(["REQ", sub_before_auth, { "kinds": [1], "#t": [topic_id.clone()] }])
            .to_string()
            .into(),
    ))
    .await
    .expect("send req before auth");
    let _ = wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "pre-enforce eose", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("EOSE")
            && value.get(1).and_then(|v| v.as_str()) == Some(sub_before_auth)
    })
    .await;

    let _challenge = wait_for_auth_challenge(&mut ws, "auth challenge").await;

    let sub_after_auth = "sub-after-auth";
    ws.send(Message::Text(
        json!(["REQ", sub_after_auth, { "kinds": [1], "#t": [topic_id.clone()] }])
            .to_string()
            .into(),
    ))
    .await
    .expect("send req after auth enforced");
    let closed = wait_for_ws_json_any(&mut ws, WAIT_TIMEOUT, "post-enforce closed", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("CLOSED")
            && value.get(1).and_then(|v| v.as_str()) == Some(sub_after_auth)
    })
    .await;
    assert_eq!(
        closed.get(2).and_then(|v| v.as_str()),
        Some("auth-required: missing auth")
    );

    // Existing pre-enforce connections must not be disconnected by ws_auth_timeout_seconds.
    match timeout(Duration::from_secs(3), ws.next()).await {
        Err(_) => {}
        Ok(None) => panic!("connection closed before grace deadline"),
        Ok(Some(Ok(Message::Ping(_)))) | Ok(Some(Ok(Message::Pong(_)))) => {}
        Ok(Some(Ok(Message::Text(text)))) => {
            let value: serde_json::Value =
                serde_json::from_str(&text).expect("parse early ws message");
            panic!("unexpected ws message before grace deadline: {value}");
        }
        Ok(Some(Ok(Message::Close(frame)))) => {
            panic!("connection closed before grace deadline: {frame:?}");
        }
        Ok(Some(Ok(other))) => panic!("unexpected ws frame before grace deadline: {other:?}"),
        Ok(Some(Err(err))) => panic!("websocket error before grace deadline: {err:?}"),
    }

    let notice = wait_for_ws_json_any(
        &mut ws,
        Duration::from_secs(20),
        "auth grace deadline notice",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("NOTICE")
                && value.get(1).and_then(|v| v.as_str()) == Some("auth-required: deadline reached")
        },
    )
    .await;
    assert_eq!(
        notice.get(1).and_then(|v| v.as_str()),
        Some("auth-required: deadline reached")
    );

    let close = timeout(Duration::from_secs(10), ws.next())
        .await
        .expect("connection close timeout");
    assert!(
        close.is_none()
            || matches!(
                close,
                Some(Ok(Message::Close(_)))
                    | Some(Err(tokio_tungstenite::tungstenite::Error::Protocol(_)))
            ),
        "expected websocket termination after grace deadline, got: {close:?}"
    );

    server_handle.abort();
}

#[tokio::test]
async fn auth_required_new_connection_times_out_without_auth() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-auth-timeout-it-{}", Uuid::new_v4());
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let state = build_state_with_config(
        pool.clone(),
        json!({
            "auth": {
                "mode": "required",
                "enforce_at": now - 1,
                "grace_seconds": 600,
                "ws_auth_timeout_seconds": 1
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            },
            "rate_limit": {
                "enabled": false
            }
        }),
    );
    enable_topic(&pool, &state, &topic_id).await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut ws = connect_ws(addr).await;

    let _challenge = wait_for_auth_challenge(&mut ws, "auth challenge").await;

    let sub_without_auth = "sub-without-auth";
    ws.send(Message::Text(
        json!(["REQ", sub_without_auth, { "kinds": [1], "#t": [topic_id.clone()] }])
            .to_string()
            .into(),
    ))
    .await
    .expect("send req without auth");
    let closed = wait_for_ws_json_any(&mut ws, WAIT_TIMEOUT, "missing auth closed", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("CLOSED")
            && value.get(1).and_then(|v| v.as_str()) == Some(sub_without_auth)
    })
    .await;
    assert_eq!(
        closed.get(2).and_then(|v| v.as_str()),
        Some("auth-required: missing auth")
    );

    let notice = wait_for_ws_json_any(
        &mut ws,
        Duration::from_secs(20),
        "auth timeout notice",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("NOTICE")
                && value.get(1).and_then(|v| v.as_str()) == Some("auth-required: timeout")
        },
    )
    .await;
    assert_eq!(
        notice.get(1).and_then(|v| v.as_str()),
        Some("auth-required: timeout")
    );

    let close = timeout(Duration::from_secs(10), ws.next())
        .await
        .expect("connection close timeout");
    assert!(
        close.is_none()
            || matches!(
                close,
                Some(Ok(Message::Close(_)))
                    | Some(Err(tokio_tungstenite::tungstenite::Error::Protocol(_)))
            ),
        "expected websocket termination after timeout, got: {close:?}"
    );

    server_handle.abort();
}

#[tokio::test]
async fn auth_required_enforces_consent_and_subscription() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;
    ensure_required_policies(&pool).await;

    let topic_id = format!("kukuri:relay-authz-it-{}", Uuid::new_v4());
    let now = cn_core::auth::unix_seconds().expect("unix seconds") as i64;
    let state = build_state_with_config(
        pool.clone(),
        json!({
            "auth": {
                "mode": "required",
                "enforce_at": now - 1,
                "grace_seconds": 600,
                "ws_auth_timeout_seconds": 10
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            },
            "rate_limit": {
                "enabled": false
            }
        }),
    );
    enable_topic(&pool, &state, &topic_id).await;
    let (addr, server_handle) = spawn_relay_server(state).await;

    // AUTH succeeds structurally, but consent is missing.
    let mut ws_missing_consent = connect_ws(addr).await;
    let challenge_missing_consent =
        wait_for_auth_challenge(&mut ws_missing_consent, "missing consent auth challenge").await;
    let keys_missing_consent = Keys::generate();
    let auth_missing_consent_id = send_auth(
        &mut ws_missing_consent,
        &keys_missing_consent,
        &challenge_missing_consent,
    )
    .await;
    let auth_missing_consent = wait_for_ws_json_any(
        &mut ws_missing_consent,
        WAIT_TIMEOUT,
        "missing consent auth response",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(auth_missing_consent_id.as_str())
        },
    )
    .await;
    assert_eq!(
        auth_missing_consent.get(2).and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        auth_missing_consent.get(3).and_then(|v| v.as_str()),
        Some("consent-required")
    );

    // Consent accepted, but no active topic subscription.
    let mut ws_missing_subscription = connect_ws(addr).await;
    let challenge_missing_subscription = wait_for_auth_challenge(
        &mut ws_missing_subscription,
        "missing subscription auth challenge",
    )
    .await;
    let keys_missing_subscription = Keys::generate();
    let pubkey_missing_subscription = keys_missing_subscription.public_key().to_string();
    ensure_consents(&pool, &pubkey_missing_subscription).await;
    let auth_missing_subscription_id = send_auth(
        &mut ws_missing_subscription,
        &keys_missing_subscription,
        &challenge_missing_subscription,
    )
    .await;
    let auth_missing_subscription = wait_for_ws_json_any(
        &mut ws_missing_subscription,
        WAIT_TIMEOUT,
        "missing subscription auth response",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str())
                    == Some(auth_missing_subscription_id.as_str())
        },
    )
    .await;
    assert_eq!(
        auth_missing_subscription.get(2).and_then(|v| v.as_bool()),
        Some(true)
    );

    let event_missing_subscription = nostr::build_signed_event(
        &keys_missing_subscription,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "missing-subscription".to_string(),
    )
    .expect("build event without subscription");
    ws_missing_subscription
        .send(Message::Text(
            json!(["EVENT", event_missing_subscription.clone()])
                .to_string()
                .into(),
        ))
        .await
        .expect("send event without subscription");
    let event_rejected = wait_for_ws_json(
        &mut ws_missing_subscription,
        WAIT_TIMEOUT,
        "missing subscription reject",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str())
                    == Some(event_missing_subscription.id.as_str())
        },
    )
    .await;
    assert_eq!(event_rejected.get(2).and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        event_rejected.get(3).and_then(|v| v.as_str()),
        Some("restricted: subscription required")
    );

    // Consent accepted + active topic subscription.
    let mut ws_subscribed = connect_ws(addr).await;
    let challenge_subscribed =
        wait_for_auth_challenge(&mut ws_subscribed, "subscribed auth challenge").await;
    let keys_subscribed = Keys::generate();
    let pubkey_subscribed = keys_subscribed.public_key().to_string();
    ensure_consents(&pool, &pubkey_subscribed).await;
    insert_topic_subscription(&pool, &topic_id, &pubkey_subscribed).await;
    let auth_subscribed_id =
        send_auth(&mut ws_subscribed, &keys_subscribed, &challenge_subscribed).await;
    let auth_subscribed = wait_for_ws_json_any(
        &mut ws_subscribed,
        WAIT_TIMEOUT,
        "subscribed auth response",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(auth_subscribed_id.as_str())
        },
    )
    .await;
    assert_eq!(auth_subscribed.get(2).and_then(|v| v.as_bool()), Some(true));

    let event_subscribed = nostr::build_signed_event(
        &keys_subscribed,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "subscribed".to_string(),
    )
    .expect("build event with subscription");
    ws_subscribed
        .send(Message::Text(
            json!(["EVENT", event_subscribed.clone()])
                .to_string()
                .into(),
        ))
        .await
        .expect("send event with subscription");
    let event_accepted = wait_for_ws_json(
        &mut ws_subscribed,
        WAIT_TIMEOUT,
        "subscribed event accepted",
        |value| {
            value.get(0).and_then(|v| v.as_str()) == Some("OK")
                && value.get(1).and_then(|v| v.as_str()) == Some(event_subscribed.id.as_str())
        },
    )
    .await;
    assert_eq!(event_accepted.get(2).and_then(|v| v.as_bool()), Some(true));

    server_handle.abort();
}

#[tokio::test]
async fn rate_limit_rejects_second_connection_at_boundary() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let state = build_state_with_config(
        pool,
        json!({
            "auth": {
                "mode": "off",
                "enforce_at": null,
                "grace_seconds": 900,
                "ws_auth_timeout_seconds": 10
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            },
            "rate_limit": {
                "enabled": true,
                "ws": {
                    "events_per_minute": 100,
                    "reqs_per_minute": 100,
                    "conns_per_minute": 1
                }
            }
        }),
    );

    let (addr, server_handle) = spawn_relay_server(state).await;
    let _first = connect_ws(addr).await;

    let second = timeout(
        WAIT_TIMEOUT,
        tokio_tungstenite::connect_async(format!("ws://{}/relay", addr)),
    )
    .await
    .expect("second connection timeout");
    let err = second.expect_err("second connection should be rejected");
    match err {
        tokio_tungstenite::tungstenite::Error::Http(response) => {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }
        other => panic!("expected HTTP 429 reject, got {other:?}"),
    }

    server_handle.abort();
}

#[tokio::test]
async fn rate_limit_closes_second_req_at_boundary() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-req-limit-it-{}", Uuid::new_v4());
    let state = build_state_with_config(
        pool.clone(),
        json!({
            "auth": {
                "mode": "off",
                "enforce_at": null,
                "grace_seconds": 900,
                "ws_auth_timeout_seconds": 10
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            },
            "rate_limit": {
                "enabled": true,
                "ws": {
                    "events_per_minute": 100,
                    "reqs_per_minute": 1,
                    "conns_per_minute": 100
                }
            }
        }),
    );
    enable_topic(&pool, &state, &topic_id).await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut ws = connect_ws(addr).await;

    let first_sub = "sub-1";
    ws.send(Message::Text(
        json!(["REQ", first_sub, { "kinds": [1], "#t": [topic_id.clone()] }])
            .to_string()
            .into(),
    ))
    .await
    .expect("send first req");
    let _ = wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "first req eose", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("EOSE")
            && value.get(1).and_then(|v| v.as_str()) == Some(first_sub)
    })
    .await;

    let second_sub = "sub-2";
    ws.send(Message::Text(
        json!(["REQ", second_sub, { "kinds": [1], "#t": [topic_id.clone()] }])
            .to_string()
            .into(),
    ))
    .await
    .expect("send second req");
    let closed = wait_for_ws_json_any(&mut ws, WAIT_TIMEOUT, "second req closed", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("CLOSED")
            && value.get(1).and_then(|v| v.as_str()) == Some(second_sub)
    })
    .await;
    assert_eq!(closed.get(2).and_then(|v| v.as_str()), Some("rate-limited"));

    server_handle.abort();
}

#[tokio::test]
async fn rate_limit_rejects_second_event_at_boundary() {
    let _guard = acquire_integration_test_lock().await;

    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-event-limit-it-{}", Uuid::new_v4());
    let state = build_state_with_config(
        pool.clone(),
        json!({
            "auth": {
                "mode": "off",
                "enforce_at": null,
                "grace_seconds": 900,
                "ws_auth_timeout_seconds": 10
            },
            "limits": {
                "max_event_bytes": 32768,
                "max_tags": 200
            },
            "rate_limit": {
                "enabled": true,
                "ws": {
                    "events_per_minute": 1,
                    "reqs_per_minute": 100,
                    "conns_per_minute": 100
                }
            }
        }),
    );
    enable_topic(&pool, &state, &topic_id).await;

    let (addr, server_handle) = spawn_relay_server(state).await;
    let mut ws = connect_ws(addr).await;
    let keys = Keys::generate();

    let first = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "event-1".to_string(),
    )
    .expect("build first event");
    ws.send(Message::Text(
        json!(["EVENT", first.clone()]).to_string().into(),
    ))
    .await
    .expect("send first event");
    let first_ok = wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "first event ok", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(first.id.as_str())
    })
    .await;
    assert_eq!(first_ok.get(2).and_then(|v| v.as_bool()), Some(true));

    let second = nostr::build_signed_event(
        &keys,
        1,
        vec![vec!["t".to_string(), topic_id]],
        "event-2".to_string(),
    )
    .expect("build second event");
    ws.send(Message::Text(
        json!(["EVENT", second.clone()]).to_string().into(),
    ))
    .await
    .expect("send second event");
    let second_ok = wait_for_ws_json(&mut ws, WAIT_TIMEOUT, "second event reject", |value| {
        value.get(0).and_then(|v| v.as_str()) == Some("OK")
            && value.get(1).and_then(|v| v.as_str()) == Some(second.id.as_str())
    })
    .await;
    assert_eq!(second_ok.get(2).and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        second_ok.get(3).and_then(|v| v.as_str()),
        Some("rate-limited")
    );

    server_handle.abort();
}

#[tokio::test]
async fn bootstrap_hint_notify_bridges_bootstrap_events_to_gossip() {
    let _guard = acquire_integration_test_lock().await;
    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-bootstrap-hint-it-{}", Uuid::new_v4());
    let state = build_state(pool.clone());
    enable_topic(&pool, &state, &topic_id).await;

    let (gossip_sender, mut harness) = setup_gossip(&topic_id).await;
    {
        let mut senders = state.gossip_senders.write().await;
        senders.insert(topic_id.clone(), gossip_sender);
    }

    let keys = Keys::generate();
    let descriptor = nostr::build_signed_event(
        &keys,
        39000,
        vec![
            vec!["d".to_string(), "descriptor".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec![
                "exp".to_string(),
                (Timestamp::now().as_u64() + 3600).to_string(),
            ],
        ],
        json!({"schema": "kukuri-node-desc-v1"}).to_string(),
    )
    .expect("build descriptor event");
    let topic_service = nostr::build_signed_event(
        &keys,
        39001,
        vec![
            vec!["d".to_string(), format!("{}:relay:public", topic_id)],
            vec!["t".to_string(), topic_id.clone()],
            vec!["role".to_string(), "relay".to_string()],
            vec!["scope".to_string(), "public".to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec![
                "exp".to_string(),
                (Timestamp::now().as_u64() + 3600).to_string(),
            ],
        ],
        json!({"schema": "kukuri-topic-service-v1"}).to_string(),
    )
    .expect("build topic service event");

    sqlx::query(
        "INSERT INTO cn_bootstrap.events
            (event_id, kind, d_tag, topic_id, role, scope, event_json, created_at, expires_at, is_active)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE)",
    )
    .bind(&descriptor.id)
    .bind(39000_i32)
    .bind("descriptor")
    .bind(Option::<String>::None)
    .bind(Option::<String>::None)
    .bind(Option::<String>::None)
    .bind(serde_json::to_value(&descriptor).expect("descriptor json"))
    .bind(descriptor.created_at)
    .bind(descriptor.created_at + 3600)
    .execute(&pool)
    .await
    .expect("insert descriptor");

    sqlx::query(
        "INSERT INTO cn_bootstrap.events
            (event_id, kind, d_tag, topic_id, role, scope, event_json, created_at, expires_at, is_active)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE)",
    )
    .bind(&topic_service.id)
    .bind(39001_i32)
    .bind(format!("{}:relay:public", topic_id))
    .bind(Some(topic_id.clone()))
    .bind(Some("relay".to_string()))
    .bind(Some("public".to_string()))
    .bind(serde_json::to_value(&topic_service).expect("topic service json"))
    .bind(topic_service.created_at)
    .bind(topic_service.created_at + 3600)
    .execute(&pool)
    .await
    .expect("insert topic service");

    gossip::spawn_bootstrap_hint_bridge(state.clone());

    let payload = json!({
        "schema": "kukuri-bootstrap-update-hint-v1",
        "changed_topic_ids": [topic_id]
    })
    .to_string();
    sqlx::query("SELECT pg_notify('cn_bootstrap_hint', $1)")
        .bind(payload)
        .execute(&pool)
        .await
        .expect("notify bootstrap hint");

    wait_for_gossip_event(&mut harness.receiver, WAIT_TIMEOUT, &descriptor.id).await;
    wait_for_gossip_event(&mut harness.receiver, WAIT_TIMEOUT, &topic_service.id).await;

    let _ = timeout(WAIT_TIMEOUT, harness.router_a.shutdown()).await;
    let _ = timeout(WAIT_TIMEOUT, harness.router_b.shutdown()).await;
}
