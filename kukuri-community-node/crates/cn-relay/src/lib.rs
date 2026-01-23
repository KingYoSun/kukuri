use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{config, db, http, logging, metrics, rate_limit, server, service_config};
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
    let addr = config::socket_addr_from_env("RELAY_ADDR", "0.0.0.0:8082")?;
    let database_url = config::required_env("DATABASE_URL")?;
    let p2p_bind_addr = config::socket_addr_from_env("RELAY_P2P_BIND", "0.0.0.0:11223")?;
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
    match db::check_ready(&state.pool).await {
        Ok(_) => (StatusCode::OK, Json(HealthStatus { status: "ok".into() })),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthStatus {
                status: "unavailable".into(),
            }),
        ),
    }
}

async fn metrics_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    if let Ok(max_seq) = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(seq), 0) FROM cn_relay.events_outbox",
    )
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
