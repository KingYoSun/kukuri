use super::AppState;
use crate::ws;
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
const WAIT_TIMEOUT: Duration = Duration::from_secs(15);

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

fn build_state(pool: Pool<Postgres>) -> AppState {
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
        },
        "rate_limit": {
            "enabled": false
        }
    }));
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
    let (sender_a, receiver_a) = topic_a.split();

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

    let _ = timeout(WAIT_TIMEOUT, receiver_b.joined()).await;

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

async fn wait_for_gossip_event(receiver: &mut GossipReceiver, wait: Duration, expected_id: &str) {
    let result = timeout(wait, async {
        while let Some(event) = receiver.next().await {
            let event = event.expect("gossip event");
            match event {
                GossipEvent::Received(message) => {
                    let value: serde_json::Value =
                        serde_json::from_slice(&message.content).expect("gossip json");
                    let raw = nostr::parse_event(&value).expect("gossip event");
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

    result.expect("gossip timeout");
}

#[tokio::test]
async fn ingest_outbox_ws_gossip_integration() {
    let pool = PgPoolOptions::new()
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;

    let topic_id = format!("kukuri:relay-it-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count) \
         VALUES ($1, TRUE, 1) \
         ON CONFLICT (topic_id) DO UPDATE SET enabled = TRUE, updated_at = NOW()",
    )
    .bind(&topic_id)
    .execute(&pool)
    .await
    .expect("insert node subscription");

    let state = build_state(pool.clone());
    {
        let mut topics = state.node_topics.write().await;
        topics.insert(topic_id.clone());
    }

    let (gossip_sender, mut gossip) = setup_gossip(&topic_id).await;
    {
        let mut senders = state.gossip_senders.write().await;
        senders.insert(topic_id.clone(), gossip_sender);
    }

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

    let (mut subscriber, _) = timeout(
        WAIT_TIMEOUT,
        tokio_tungstenite::connect_async(format!("ws://{}/relay", addr)),
    )
    .await
    .expect("subscriber connect timeout")
    .expect("subscriber connect");
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

    let (mut publisher, _) = timeout(
        WAIT_TIMEOUT,
        tokio_tungstenite::connect_async(format!("ws://{}/relay", addr)),
    )
    .await
    .expect("publisher connect timeout")
    .expect("publisher connect");
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
