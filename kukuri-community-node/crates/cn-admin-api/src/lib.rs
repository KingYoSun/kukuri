use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use cn_core::{config, db, http, logging, metrics, node_key, server, service_config};
use nostr_sdk::prelude::Keys;
use serde::Serialize;
use serde_json::Value;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

mod auth;
mod moderation;
mod policies;
mod reindex;
mod services;
mod subscriptions;

const SERVICE_NAME: &str = "cn-admin-api";

#[derive(Clone)]
pub(crate) struct AppState {
    pool: Pool<Postgres>,
    admin_config: service_config::ServiceConfigHandle,
    health_targets: Arc<HashMap<String, String>>,
    health_client: reqwest::Client,
    node_keys: Keys,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
    details: Option<Value>,
}

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<Value>,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }

    fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let payload = ErrorResponse {
            code: self.code,
            message: self.message,
            details: self.details,
        };
        (self.status, Json(payload)).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

#[derive(Serialize)]
struct HealthStatus {
    status: String,
}

pub struct AdminApiConfig {
    pub addr: SocketAddr,
    pub database_url: String,
    pub config_poll_seconds: u64,
    pub health_poll_seconds: u64,
    pub node_key_path: std::path::PathBuf,
}

pub fn load_config() -> Result<AdminApiConfig> {
    let addr = config::socket_addr_from_env("ADMIN_API_ADDR", "0.0.0.0:8081")?;
    let database_url = config::required_env("DATABASE_URL")?;
    let config_poll_seconds = std::env::var("ADMIN_CONFIG_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    let health_poll_seconds = std::env::var("ADMIN_HEALTH_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(10);
    let node_key_path = node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")?;
    Ok(AdminApiConfig {
        addr,
        database_url,
        config_poll_seconds,
        health_poll_seconds,
        node_key_path,
    })
}

pub async fn run(config: AdminApiConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    metrics::init(SERVICE_NAME);

    let pool = db::connect(&config.database_url).await?;
    let admin_default = serde_json::json!({
        "session_cookie": true,
        "session_ttl_seconds": 86400
    });
    let admin_config = service_config::watch_service_config(
        pool.clone(),
        "admin-api",
        admin_default,
        Duration::from_secs(config.config_poll_seconds),
    )
    .await?;
    let node_keys = node_key::load_or_generate(&config.node_key_path)?;

    let health_targets = Arc::new(parse_health_targets());
    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let state = AppState {
        pool,
        admin_config,
        health_targets: Arc::clone(&health_targets),
        health_client,
        node_keys,
    };

    services::spawn_health_poll(state.clone(), Duration::from_secs(config.health_poll_seconds));

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .route("/v1/openapi.json", get(openapi_json))
        .route("/v1/admin/auth/login", post(auth::login))
        .route("/v1/admin/auth/logout", post(auth::logout))
        .route("/v1/admin/auth/me", get(auth::me))
        .route("/v1/admin/services", get(services::list_services))
        .route(
            "/v1/admin/services/:service/config",
            get(services::get_service_config).put(services::update_service_config),
        )
        .route("/v1/admin/policies", get(policies::list_policies).post(policies::create_policy))
        .route(
            "/v1/admin/policies/:policy_id",
            put(policies::update_policy),
        )
        .route(
            "/v1/admin/policies/:policy_id/publish",
            post(policies::publish_policy),
        )
        .route(
            "/v1/admin/policies/:policy_id/make-current",
            post(policies::make_current_policy),
        )
        .route(
            "/v1/admin/moderation/rules",
            get(moderation::list_rules).post(moderation::create_rule),
        )
        .route(
            "/v1/admin/moderation/rules/:rule_id",
            put(moderation::update_rule).delete(moderation::delete_rule),
        )
        .route(
            "/v1/admin/moderation/reports",
            get(moderation::list_reports),
        )
        .route(
            "/v1/admin/moderation/labels",
            get(moderation::list_labels).post(moderation::create_label),
        )
        .route(
            "/v1/admin/subscription-requests",
            get(subscriptions::list_subscription_requests),
        )
        .route(
            "/v1/admin/subscription-requests/:request_id/approve",
            post(subscriptions::approve_subscription_request),
        )
        .route(
            "/v1/admin/subscription-requests/:request_id/reject",
            post(subscriptions::reject_subscription_request),
        )
        .route(
            "/v1/admin/node-subscriptions",
            get(subscriptions::list_node_subscriptions),
        )
        .route(
            "/v1/admin/node-subscriptions/:topic_id",
            put(subscriptions::update_node_subscription),
        )
        .route("/v1/admin/plans", get(subscriptions::list_plans).post(subscriptions::create_plan))
        .route(
            "/v1/admin/plans/:plan_id",
            put(subscriptions::update_plan),
        )
        .route(
            "/v1/admin/subscriptions",
            get(subscriptions::list_subscriptions),
        )
        .route(
            "/v1/admin/subscriptions/:subscriber_pubkey",
            put(subscriptions::upsert_subscription),
        )
        .route("/v1/admin/usage", get(subscriptions::list_usage))
        .route("/v1/admin/audit-logs", get(services::list_audit_logs))
        .route("/v1/reindex", post(reindex::enqueue_reindex))
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

async fn metrics_endpoint() -> impl IntoResponse {
    metrics::metrics_response(SERVICE_NAME)
}

async fn openapi_json() -> impl IntoResponse {
    Json(serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "cn-admin-api", "version": "0.1.0" },
        "paths": {}
    }))
}

fn parse_health_targets() -> HashMap<String, String> {
    let mut targets = HashMap::new();
    if let Ok(raw) = std::env::var("ADMIN_HEALTH_TARGETS") {
        for entry in raw.split(',') {
            if let Some((name, url)) = entry.split_once('=') {
                if !name.trim().is_empty() && !url.trim().is_empty() {
                    targets.insert(name.trim().to_string(), url.trim().to_string());
                }
            }
        }
    }

    let fallback = [
        ("user-api", "USER_API_HEALTH_URL", "http://user-api:8080/healthz"),
        ("relay", "RELAY_HEALTH_URL", "http://relay:8082/healthz"),
        ("bootstrap", "BOOTSTRAP_HEALTH_URL", "http://bootstrap:8083/healthz"),
        ("index", "INDEX_HEALTH_URL", "http://index:8084/healthz"),
        (
            "moderation",
            "MODERATION_HEALTH_URL",
            "http://moderation:8085/healthz",
        ),
    ];
    for (name, env, default_url) in fallback {
        if targets.contains_key(name) {
            continue;
        }
        let value = std::env::var(env).unwrap_or_else(|_| default_url.to_string());
        targets.insert(name.to_string(), value);
    }

    targets
}
