use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{
    config as core_config, db, http, logging, metrics, rate_limit, server, service_config,
};
use iroh::address_lookup::MemoryLookup;
use iroh::protocol::Router as IrohRouter;
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
    pub p2p_public_host: Option<String>,
    pub p2p_public_port: Option<u16>,
    pub p2p_node_id: Arc<RwLock<Option<String>>>,
    pub p2p_address_lookup: Arc<MemoryLookup>,
    pub p2p_bind_addr: SocketAddr,
    pub p2p_relay_urls: Arc<Vec<String>>,
    pub p2p_advertised_relay_urls: Arc<Vec<String>>,
    pub p2p_router: Arc<RwLock<Option<Arc<IrohRouter>>>>,
    pub bootstrap_hint_rejoin_requests: Arc<RwLock<HashSet<String>>>,
}

#[derive(Serialize)]
struct HealthStatus {
    status: String,
}

#[derive(Serialize)]
struct RelayP2pInfoResponse {
    node_id: Option<String>,
    bind_addr: String,
    bootstrap_nodes: Vec<String>,
    bootstrap_hints: Vec<String>,
    relay_urls: Vec<String>,
}

#[derive(Serialize)]
struct RelayP2pStatusResponse {
    status: String,
    node_id: Option<String>,
    bind_addr: String,
    relay_urls: Vec<String>,
    desired_topics: Vec<String>,
    node_topics: Vec<String>,
    gossip_topics: Vec<String>,
    router_ready: bool,
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
    pub p2p_public_host: Option<String>,
    pub p2p_public_port: Option<u16>,
    pub p2p_secret_key: Option<String>,
    pub p2p_relay_urls: Vec<String>,
    pub p2p_advertised_relay_urls: Vec<String>,
    pub p2p_relay_mode_default: bool,
    pub topic_poll_seconds: u64,
    pub config_poll_seconds: u64,
    pub relay_public_url: Option<String>,
}

pub fn load_config() -> Result<RelayConfig> {
    let addr = core_config::socket_addr_from_env("RELAY_ADDR", "0.0.0.0:8082")?;
    let database_url = core_config::required_env("DATABASE_URL")?;
    let p2p_bind_addr = core_config::socket_addr_from_env("RELAY_P2P_BIND", "0.0.0.0:11223")?;
    let p2p_public_host = std::env::var("RELAY_P2P_PUBLIC_HOST")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let p2p_public_port = std::env::var("RELAY_P2P_PUBLIC_PORT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            value.parse::<u16>().map_err(|err| {
                anyhow::anyhow!("invalid RELAY_P2P_PUBLIC_PORT entry `{value}`: {err}")
            })
        })
        .transpose()?;
    let p2p_secret_key = std::env::var("RELAY_P2P_SECRET_KEY").ok();
    let p2p_relay_urls = parse_csv_env("RELAY_IROH_RELAY_URLS");
    let p2p_advertised_relay_urls = parse_csv_env("RELAY_IROH_ADVERTISED_URLS");
    let p2p_relay_mode_default = std::env::var("RELAY_IROH_RELAY_MODE")
        .ok()
        .map(|value| relay_mode_uses_default(&value))
        .unwrap_or(false);
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
        p2p_public_host,
        p2p_public_port,
        p2p_secret_key,
        p2p_relay_urls,
        p2p_advertised_relay_urls,
        p2p_relay_mode_default,
        topic_poll_seconds,
        config_poll_seconds,
        relay_public_url,
    })
}

pub async fn run(config: RelayConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    metrics::init(SERVICE_NAME);

    let pool = db::connect(&config.database_url).await?;
    ensure_default_public_node_subscription(&pool).await?;
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
    let p2p_address_lookup = Arc::new(MemoryLookup::new());
    let state = AppState {
        pool: pool.clone(),
        config: config_handle,
        rate_limiter: Arc::new(rate_limit::RateLimiter::new()),
        realtime_tx,
        gossip_senders: Arc::new(RwLock::new(HashMap::new())),
        node_topics: Arc::new(RwLock::new(HashSet::new())),
        relay_public_url: config.relay_public_url.clone(),
        p2p_public_host: config.p2p_public_host.clone(),
        p2p_public_port: config.p2p_public_port,
        p2p_node_id: Arc::new(RwLock::new(None)),
        p2p_address_lookup,
        p2p_bind_addr: config.p2p_bind_addr,
        p2p_relay_urls: Arc::new(config.p2p_relay_urls.clone()),
        p2p_advertised_relay_urls: Arc::new(config.p2p_advertised_relay_urls.clone()),
        p2p_router: Arc::new(RwLock::new(None)),
        bootstrap_hint_rejoin_requests: Arc::new(RwLock::new(HashSet::new())),
    };

    gossip::start_gossip(state.clone(), config.clone()).await?;
    retention::spawn_cleanup_loop(state.clone());

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .route("/v1/p2p/info", get(p2p_info))
        .route("/v1/p2p/status", get(p2p_status))
        .route("/relay", get(ws::ws_handler))
        .with_state(state);

    let router = http::apply_standard_layers(router, SERVICE_NAME);
    server::serve(config.addr, router).await
}

async fn ensure_default_public_node_subscription(pool: &Pool<Postgres>) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO cn_admin.node_subscriptions (topic_id, enabled, ref_count)
         VALUES ($1, TRUE, 1)
         ON CONFLICT (topic_id) DO UPDATE
             SET enabled = TRUE,
                 ref_count = GREATEST(cn_admin.node_subscriptions.ref_count, 1),
                 updated_at = NOW()",
    )
    .bind(cn_core::topic::DEFAULT_PUBLIC_TOPIC_ID)
    .execute(&mut *tx)
    .await?;
    cn_core::topic_services::sync_default_topic_services(
        &mut *tx,
        cn_core::topic::DEFAULT_PUBLIC_TOPIC_ID,
        true,
        "cn-relay.startup",
    )
    .await?;
    tx.commit().await?;
    Ok(())
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

async fn p2p_info(State(state): State<AppState>) -> impl IntoResponse {
    let node_id = state.p2p_node_id.read().await.clone();
    let bind_addr = state.p2p_bind_addr.to_string();
    let advertised_host = resolve_advertised_host(&state);
    let advertised_port = resolve_advertised_port(&state);
    let relay_urls = resolve_p2p_relay_urls_for_info(&state);
    let (bootstrap_nodes, bootstrap_hints) = build_bootstrap_endpoint_hints(
        node_id.as_deref(),
        advertised_host.as_deref(),
        advertised_port,
        &relay_urls,
        include_direct_addr_hints_for_info(),
    );

    Json(RelayP2pInfoResponse {
        node_id,
        bind_addr,
        bootstrap_nodes,
        bootstrap_hints,
        relay_urls,
    })
}

async fn p2p_status(State(state): State<AppState>) -> impl IntoResponse {
    let config_snapshot = state.config.get().await;
    let runtime = config::RelayRuntimeConfig::from_json(&config_snapshot.config_json);
    let status = evaluate_relay_ready_status(&state).await;
    let desired_topics =
        match load_enabled_topics(&state.pool, runtime.node_subscription.max_concurrent_topics).await
        {
            Ok(topics) => topics,
            Err(err) => {
                tracing::warn!(error = %err, "p2p status failed to load enabled topics");
                return Json(RelayP2pStatusResponse {
                    status: ReadyStatus::Unavailable.as_str().into(),
                    node_id: state.p2p_node_id.read().await.clone(),
                    bind_addr: state.p2p_bind_addr.to_string(),
                    relay_urls: resolve_p2p_relay_urls_for_info(&state),
                    desired_topics: Vec::new(),
                    node_topics: sorted_strings(state.node_topics.read().await.clone()),
                    gossip_topics: {
                        let senders = state.gossip_senders.read().await;
                        sorted_strings(senders.keys().cloned().collect::<HashSet<_>>())
                    },
                    router_ready: state.p2p_router.read().await.is_some(),
                })
                    .into_response();
            }
        };
    let node_topics = state.node_topics.read().await.clone();
    let gossip_topics = {
        let senders = state.gossip_senders.read().await;
        senders.keys().cloned().collect::<HashSet<_>>()
    };
    let router_ready = state.p2p_router.read().await.is_some();

    Json(RelayP2pStatusResponse {
        status: status.as_str().into(),
        node_id: state.p2p_node_id.read().await.clone(),
        bind_addr: state.p2p_bind_addr.to_string(),
        relay_urls: resolve_p2p_relay_urls_for_info(&state),
        desired_topics: sorted_strings(desired_topics),
        node_topics: sorted_strings(node_topics),
        gossip_topics: sorted_strings(gossip_topics),
        router_ready,
    })
        .into_response()
}

fn resolve_p2p_relay_urls_for_info(state: &AppState) -> Vec<String> {
    let mut relay_urls = if state.p2p_advertised_relay_urls.is_empty() {
        state.p2p_relay_urls.as_ref().clone()
    } else {
        state.p2p_advertised_relay_urls.as_ref().clone()
    };
    if relay_urls.is_empty() {
        if let Some(relay_hint_url) = state
            .relay_public_url
            .as_deref()
            .and_then(normalize_relay_url_for_hint)
        {
            relay_urls.push(relay_hint_url);
        }
    }
    dedupe_in_order(relay_urls)
}

fn include_direct_addr_hints_for_info() -> bool {
    match std::env::var("RELAY_P2P_INCLUDE_DIRECT_ADDR_HINTS") {
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        ),
        Err(_) => true,
    }
}

fn build_bootstrap_endpoint_hints(
    node_id: Option<&str>,
    advertised_host: Option<&str>,
    advertised_port: Option<u16>,
    relay_urls: &[String],
    include_direct_addr_hints: bool,
) -> (Vec<String>, Vec<String>) {
    let (Some(node_id), Some(host), Some(port)) = (node_id, advertised_host, advertised_port)
    else {
        return (Vec::new(), Vec::new());
    };

    let endpoint = format_host_port(host, port);
    if relay_urls.is_empty() {
        let direct = format!("{node_id}@{endpoint}");
        return (vec![direct.clone()], vec![direct]);
    }

    let bootstrap_nodes = if include_direct_addr_hints {
        vec![format!("{node_id}@{endpoint}")]
    } else {
        Vec::new()
    };

    let mut bootstrap_hints = Vec::new();
    for relay_url in relay_urls {
        if include_direct_addr_hints {
            bootstrap_hints.push(format!("{node_id}|relay={relay_url}|addr={endpoint}"));
        }
        bootstrap_hints.push(format!("{node_id}|relay={relay_url}"));
    }

    (bootstrap_nodes, bootstrap_hints)
}

fn format_host_port(host: &str, port: u16) -> String {
    let trimmed = host.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.contains(':') {
        format!("[{trimmed}]:{port}")
    } else {
        format!("{trimmed}:{port}")
    }
}

fn normalize_relay_url_for_hint(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parsed = reqwest::Url::parse(trimmed).ok()?;
    match parsed.scheme() {
        "http" | "https" => {}
        "ws" => {
            parsed.set_scheme("http").ok()?;
        }
        "wss" => {
            parsed.set_scheme("https").ok()?;
        }
        _ => return None,
    }
    parsed.set_query(None);
    parsed.set_fragment(None);
    Some(parsed.to_string())
}

fn relay_mode_uses_default(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case("default")
}

fn parse_csv_env(name: &str) -> Vec<String> {
    std::env::var(name)
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(|entry| entry.trim().to_string())
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn dedupe_in_order(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn sorted_strings(values: HashSet<String>) -> Vec<String> {
    let mut items = values.into_iter().collect::<Vec<_>>();
    items.sort();
    items
}

fn resolve_advertised_host(state: &AppState) -> Option<String> {
    if let Some(host) = state.p2p_public_host.as_ref() {
        let host = host.trim();
        if !host.is_empty() {
            return Some(host.to_string());
        }
    }

    if let Some(url) = state.relay_public_url.as_ref() {
        if let Some(host) = extract_host_from_url_like(url) {
            return Some(host);
        }
    }

    match state.p2p_bind_addr.ip() {
        std::net::IpAddr::V4(ip) if ip.is_unspecified() => None,
        std::net::IpAddr::V6(ip) if ip.is_unspecified() => None,
        ip => Some(ip.to_string()),
    }
}

fn resolve_advertised_port(state: &AppState) -> Option<u16> {
    if let Some(port) = state.p2p_public_port {
        return Some(port);
    }

    let has_explicit_host = state
        .p2p_public_host
        .as_deref()
        .map(str::trim)
        .is_some_and(|host| !host.is_empty());
    if has_explicit_host || state.relay_public_url.is_some() {
        return Some(state.p2p_bind_addr.port());
    }

    match state.p2p_bind_addr.ip() {
        std::net::IpAddr::V4(ip) if ip.is_unspecified() => None,
        std::net::IpAddr::V6(ip) if ip.is_unspecified() => None,
        _ => Some(state.p2p_bind_addr.port()),
    }
}

fn extract_host_from_url_like(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let authority = authority.trim();
    if authority.is_empty() {
        return None;
    }

    if let Some(host) = authority
        .strip_prefix('[')
        .and_then(|value| value.split_once(']'))
        .map(|(host, _)| host.trim().to_string())
    {
        if !host.is_empty() {
            return Some(host);
        }
    }

    let host = authority
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(authority)
        .trim();
    if host.is_empty() {
        return None;
    }

    Some(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        build_bootstrap_endpoint_hints, dedupe_in_order, extract_host_from_url_like,
        format_host_port, normalize_relay_url_for_hint, relay_mode_uses_default,
        resolve_advertised_host, resolve_advertised_port, resolve_p2p_relay_urls_for_info,
        AppState,
    };
    use cn_core::rate_limit;
    use cn_core::service_config;
    use iroh::address_lookup::MemoryLookup;
    use std::collections::{HashMap, HashSet};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};

    fn app_state_for_advertised_endpoint() -> AppState {
        let (realtime_tx, _) = broadcast::channel(1);
        AppState {
            pool: sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://unused:unused@localhost/unused")
                .expect("lazy pool"),
            config: service_config::static_handle(serde_json::json!({})),
            rate_limiter: Arc::new(rate_limit::RateLimiter::new()),
            realtime_tx,
            gossip_senders: Arc::new(RwLock::new(HashMap::new())),
            node_topics: Arc::new(RwLock::new(HashSet::new())),
            relay_public_url: None,
            p2p_public_host: None,
            p2p_public_port: None,
            p2p_node_id: Arc::new(RwLock::new(None)),
            p2p_address_lookup: Arc::new(MemoryLookup::new()),
            p2p_bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 11223),
            p2p_relay_urls: Arc::new(Vec::new()),
            p2p_advertised_relay_urls: Arc::new(Vec::new()),
            p2p_router: Arc::new(RwLock::new(None)),
            bootstrap_hint_rejoin_requests: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    #[test]
    fn extract_host_from_url_like_parses_ws_url_with_port_and_path() {
        let host = extract_host_from_url_like("ws://localhost:8082/relay");
        assert_eq!(host.as_deref(), Some("localhost"));
    }

    #[test]
    fn extract_host_from_url_like_parses_ipv6_authority() {
        let host = extract_host_from_url_like("wss://[2001:db8::1]:443/relay");
        assert_eq!(host.as_deref(), Some("2001:db8::1"));
    }

    #[test]
    fn extract_host_from_url_like_returns_none_for_empty_input() {
        assert_eq!(extract_host_from_url_like("   "), None);
    }

    #[tokio::test]
    async fn advertised_endpoint_prefers_explicit_p2p_public_values() {
        let mut state = app_state_for_advertised_endpoint();
        state.relay_public_url = Some("wss://relay.kukuri.app/relay".to_string());
        state.p2p_public_host = Some("relay-p2p.kukuri.app".to_string());
        state.p2p_public_port = Some(40123);

        assert_eq!(
            resolve_advertised_host(&state).as_deref(),
            Some("relay-p2p.kukuri.app")
        );
        assert_eq!(resolve_advertised_port(&state), Some(40123));
    }

    #[tokio::test]
    async fn advertised_endpoint_uses_bind_port_when_public_host_is_explicit() {
        let mut state = app_state_for_advertised_endpoint();
        state.p2p_public_host = Some("relay.kukuri.app".to_string());

        assert_eq!(
            resolve_advertised_host(&state).as_deref(),
            Some("relay.kukuri.app")
        );
        assert_eq!(resolve_advertised_port(&state), Some(11223));
    }

    #[tokio::test]
    async fn advertised_endpoint_falls_back_to_relay_public_url_host() {
        let mut state = app_state_for_advertised_endpoint();
        state.relay_public_url = Some("wss://relay.kukuri.app/relay".to_string());

        assert_eq!(
            resolve_advertised_host(&state).as_deref(),
            Some("relay.kukuri.app")
        );
        assert_eq!(resolve_advertised_port(&state), Some(11223));
    }

    #[test]
    fn normalize_relay_url_for_hint_maps_wss_scheme_to_https() {
        let normalized = normalize_relay_url_for_hint("wss://relay.example/ws?x=1#frag");
        assert_eq!(normalized.as_deref(), Some("https://relay.example/ws"));
    }

    #[test]
    fn format_host_port_brackets_ipv6_host() {
        assert_eq!(
            format_host_port("2001:db8::10", 11223),
            "[2001:db8::10]:11223"
        );
    }

    #[test]
    fn relay_mode_uses_default_handles_case_and_whitespace() {
        assert!(relay_mode_uses_default(" default "));
        assert!(relay_mode_uses_default("DEFAULT"));
        assert!(!relay_mode_uses_default("custom"));
    }

    #[test]
    fn dedupe_in_order_keeps_first_entries() {
        let deduped = dedupe_in_order(vec![
            "https://relay-a.example/".to_string(),
            "https://relay-b.example/".to_string(),
            "https://relay-a.example/".to_string(),
        ]);
        assert_eq!(
            deduped,
            vec![
                "https://relay-a.example/".to_string(),
                "https://relay-b.example/".to_string()
            ]
        );
    }

    #[test]
    fn relay_info_prefers_explicit_advertised_relay_urls() {
        let mut state = app_state_for_advertised_endpoint();
        state.p2p_relay_urls = Arc::new(vec!["http://internal-relay:3340".to_string()]);
        state.p2p_advertised_relay_urls = Arc::new(vec!["http://127.0.0.1:3340".to_string()]);

        assert_eq!(
            resolve_p2p_relay_urls_for_info(&state),
            vec!["http://127.0.0.1:3340".to_string()]
        );
    }

    #[test]
    fn bootstrap_hints_include_direct_addr_when_enabled() {
        let (bootstrap_nodes, bootstrap_hints) = build_bootstrap_endpoint_hints(
            Some("node"),
            Some("127.0.0.1"),
            Some(11223),
            &["http://127.0.0.1:3340".to_string()],
            true,
        );

        assert_eq!(bootstrap_nodes, vec!["node@127.0.0.1:11223".to_string()]);
        assert_eq!(
            bootstrap_hints,
            vec![
                "node|relay=http://127.0.0.1:3340|addr=127.0.0.1:11223".to_string(),
                "node|relay=http://127.0.0.1:3340".to_string()
            ]
        );
    }

    #[test]
    fn bootstrap_hints_can_be_forced_to_relay_only() {
        let (bootstrap_nodes, bootstrap_hints) = build_bootstrap_endpoint_hints(
            Some("node"),
            Some("127.0.0.1"),
            Some(11223),
            &["http://127.0.0.1:3340".to_string()],
            false,
        );

        assert!(bootstrap_nodes.is_empty());
        assert_eq!(
            bootstrap_hints,
            vec!["node|relay=http://127.0.0.1:3340".to_string()]
        );
    }
}
