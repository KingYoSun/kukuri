use super::AppState;
use crate::ws;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use cn_core::nostr;
use cn_core::rate_limit::RateLimiter;
use cn_core::service_config;
use futures_util::{SinkExt, StreamExt};
use iroh::discovery::static_provider::StaticProvider;
use iroh::endpoint::Connection;
use iroh::protocol::Router as IrohRouter;
use iroh::Endpoint;
use iroh_gossip::api::{Event as GossipEvent, GossipReceiver, GossipSender};
use iroh_gossip::{Gossip, TopicId};
use nostr_sdk::prelude::Keys;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, OnceCell, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
const WAIT_TIMEOUT: Duration = Duration::from_secs(30);

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

#[tokio::test]
async fn ingest_outbox_ws_gossip_integration() {
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
async fn auth_enforce_switches_from_off_to_on_and_times_out() {
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
                "enforce_at": now + 1,
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

    let auth = wait_for_ws_json_any(
        &mut ws,
        Duration::from_secs(20),
        "auth challenge",
        |value| value.get(0).and_then(|v| v.as_str()) == Some("AUTH"),
    )
    .await;
    let challenge = auth
        .get(1)
        .and_then(|v| v.as_str())
        .expect("auth challenge");
    assert!(!challenge.is_empty(), "AUTH challenge should not be empty");

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

    let notice = wait_for_ws_json_any(
        &mut ws,
        Duration::from_secs(20),
        "auth timeout notice",
        |value| value.get(0).and_then(|v| v.as_str()) == Some("NOTICE"),
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
async fn rate_limit_rejects_second_connection_at_boundary() {
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
