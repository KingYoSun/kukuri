use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{
    config as core_config, db, http, logging, metrics, rate_limit, server, service_config,
};
use serde::Serialize;
use sqlx::{Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

mod config;
mod filters;
mod gossip;
mod ingest;
#[cfg(test)]
mod integration_tests;
mod policy;
mod retention;
mod ws;

pub(crate) const SERVICE_NAME: &str = "cn-relay";

#[derive(Clone)]
pub(crate) struct AppState {
    pub pool: Pool<Postgres>,
    pub config: service_config::ServiceConfigHandle,
    pub rate_limiter: Arc<rate_limit::RateLimiter>,
    pub realtime_tx: broadcast::Sender<ingest::RelayEvent>,
    pub gossip_senders: Arc<RwLock<HashMap<String, iroh_gossip::api::GossipSender>>>,
    pub node_topics: Arc<RwLock<HashSet<String>>>,
    pub relay_public_url: Option<String>,
}

#[derive(Serialize)]
struct HealthStatus {
    status: String,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum ReadyStatus {
    Ok,
    Degraded,
    Unavailable,
}

impl ReadyStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }

    fn http_status(self) -> StatusCode {
        match self {
            Self::Ok => StatusCode::OK,
            Self::Degraded | Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn merge(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }
}

#[derive(Clone)]
pub struct RelayConfig {
    pub addr: SocketAddr,
    pub database_url: String,
    pub p2p_bind_addr: SocketAddr,
    pub p2p_secret_key: Option<String>,
    pub topic_poll_seconds: u64,
    pub config_poll_seconds: u64,
    pub relay_public_url: Option<String>,
}

pub fn load_config() -> Result<RelayConfig> {
    let addr = core_config::socket_addr_from_env("RELAY_ADDR", "0.0.0.0:8082")?;
    let database_url = core_config::required_env("DATABASE_URL")?;
    let p2p_bind_addr = core_config::socket_addr_from_env("RELAY_P2P_BIND", "0.0.0.0:11223")?;
    let p2p_secret_key = std::env::var("RELAY_P2P_SECRET_KEY").ok();
    let topic_poll_seconds = std::env::var("RELAY_TOPIC_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    let config_poll_seconds = std::env::var("RELAY_CONFIG_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    let relay_public_url = std::env::var("RELAY_PUBLIC_URL").ok();

    Ok(RelayConfig {
        addr,
        database_url,
        p2p_bind_addr,
        p2p_secret_key,
        topic_poll_seconds,
        config_poll_seconds,
        relay_public_url,
    })
}

pub async fn run(config: RelayConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    metrics::init(SERVICE_NAME);

    let pool = db::connect(&config.database_url).await?;
    let default_config = serde_json::json!({
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
                "events_per_minute": 120,
                "reqs_per_minute": 60,
                "conns_per_minute": 30
            },
            "gossip": {
                "msgs_per_minute": 600
            }
        },
        "node_subscription": {
            "max_concurrent_topics": cn_core::service_config::DEFAULT_MAX_CONCURRENT_NODE_TOPICS
        },
        "retention": {
            "events_days": 30,
            "tombstone_days": 180,
            "dedupe_days": 180,
            "outbox_days": 30,
            "cleanup_interval_seconds": 3600
        }
    });
    let config_handle = service_config::watch_service_config(
        pool.clone(),
        "relay",
        default_config,
        Duration::from_secs(config.config_poll_seconds),
    )
    .await?;

    let (realtime_tx, _) = broadcast::channel(1024);
    let state = AppState {
        pool: pool.clone(),
        config: config_handle,
        rate_limiter: Arc::new(rate_limit::RateLimiter::new()),
        realtime_tx,
        gossip_senders: Arc::new(RwLock::new(HashMap::new())),
        node_topics: Arc::new(RwLock::new(HashSet::new())),
        relay_public_url: config.relay_public_url.clone(),
    };

    gossip::start_gossip(state.clone(), config.clone()).await?;
    retention::spawn_cleanup_loop(state.clone());

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .route("/relay", get(ws::ws_handler))
        .with_state(state);

    let router = http::apply_standard_layers(router, SERVICE_NAME);
    server::serve(config.addr, router).await
}

async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let status = evaluate_relay_ready_status(&state).await;
    (
        status.http_status(),
        Json(HealthStatus {
            status: status.as_str().into(),
        }),
    )
}

async fn evaluate_relay_ready_status(state: &AppState) -> ReadyStatus {
    if db::check_ready(&state.pool).await.is_err() {
        return ReadyStatus::Unavailable;
    }

    let config_snapshot = state.config.get().await;
    let runtime = config::RelayRuntimeConfig::from_json(&config_snapshot.config_json);
    let desired_topics =
        match load_enabled_topics(&state.pool, runtime.node_subscription.max_concurrent_topics)
            .await
        {
            Ok(topics) => topics,
            Err(err) => {
                tracing::warn!(error = %err, "healthz failed to load enabled topics");
                return ReadyStatus::Unavailable;
            }
        };

    let node_topics = state.node_topics.read().await.clone();
    let gossip_topics = {
        let senders = state.gossip_senders.read().await;
        senders.keys().cloned().collect::<HashSet<_>>()
    };

    ReadyStatus::Ok
        .merge(evaluate_gossip_participation(
            &desired_topics,
            &gossip_topics,
        ))
        .merge(evaluate_topic_sync(
            &desired_topics,
            &node_topics,
            &gossip_topics,
        ))
}

fn evaluate_gossip_participation(
    desired_topics: &HashSet<String>,
    gossip_topics: &HashSet<String>,
) -> ReadyStatus {
    if desired_topics.is_empty() {
        return if gossip_topics.is_empty() {
            ReadyStatus::Ok
        } else {
            ReadyStatus::Degraded
        };
    }

    let matched_topics = desired_topics.intersection(gossip_topics).count();
    if matched_topics == 0 {
        ReadyStatus::Unavailable
    } else if matched_topics < desired_topics.len() || gossip_topics.len() != desired_topics.len() {
        ReadyStatus::Degraded
    } else {
        ReadyStatus::Ok
    }
}

fn evaluate_topic_sync(
    desired_topics: &HashSet<String>,
    node_topics: &HashSet<String>,
    gossip_topics: &HashSet<String>,
) -> ReadyStatus {
    if desired_topics == node_topics && desired_topics == gossip_topics {
        return ReadyStatus::Ok;
    }

    if desired_topics.is_empty() {
        return ReadyStatus::Degraded;
    }

    let synced_node_topics = desired_topics.intersection(node_topics).count();
    let synced_gossip_topics = desired_topics.intersection(gossip_topics).count();
    if synced_node_topics == 0 || synced_gossip_topics == 0 {
        ReadyStatus::Unavailable
    } else {
        ReadyStatus::Degraded
    }
}

async fn load_enabled_topics(
    pool: &Pool<Postgres>,
    max_concurrent_topics: i64,
) -> Result<HashSet<String>, sqlx::Error> {
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

async fn metrics_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    if let Ok(max_seq) =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(seq), 0) FROM cn_relay.events_outbox")
            .fetch_one(&state.pool)
            .await
    {
        if let Ok(rows) = sqlx::query("SELECT consumer, last_seq FROM cn_relay.consumer_offsets")
            .fetch_all(&state.pool)
            .await
        {
            for row in rows {
                let consumer: String = row.try_get("consumer").unwrap_or_default();
                let last_seq: i64 = row.try_get("last_seq").unwrap_or(0);
                let backlog = max_seq.saturating_sub(last_seq);
                metrics::set_outbox_backlog(SERVICE_NAME, &consumer, backlog);
            }
        }
    }

    metrics::metrics_response(SERVICE_NAME)
}
