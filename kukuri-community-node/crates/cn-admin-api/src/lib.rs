use anyhow::Result;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use cn_core::{config, db, health, http, logging, metrics, node_key, server, service_config};
use nostr_sdk::prelude::Keys;
use serde::Serialize;
use serde_json::Value;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use utoipa::ToSchema;

mod access_control;
mod auth;
mod dashboard;
mod dsar;
mod moderation;
pub mod openapi;
mod policies;
mod reindex;
mod services;
mod subscriptions;
mod trust;

#[cfg(test)]
mod contract_tests;

const SERVICE_NAME: &str = "cn-admin-api";

#[derive(Clone)]
pub(crate) struct AppState {
    pool: Pool<Postgres>,
    admin_config: service_config::ServiceConfigHandle,
    health_targets: Arc<HashMap<String, String>>,
    health_client: reqwest::Client,
    dashboard_cache: Arc<tokio::sync::Mutex<dashboard::DashboardCache>>,
    node_keys: Keys,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ErrorResponse {
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
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
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

#[derive(Serialize, ToSchema)]
pub(crate) struct HealthStatus {
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
        dashboard_cache: Arc::new(tokio::sync::Mutex::new(dashboard::DashboardCache::default())),
        node_keys,
    };

    services::spawn_health_poll(
        state.clone(),
        Duration::from_secs(config.health_poll_seconds),
    );

    let router = build_router(state);
    let router = http::apply_standard_layers(router, SERVICE_NAME);
    server::serve(config.addr, router).await
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .route("/v1/openapi.json", get(openapi_json))
        .route("/v1/admin/auth/login", post(auth::login))
        .route("/v1/admin/auth/logout", post(auth::logout))
        .route("/v1/admin/auth/me", get(auth::me))
        .route(
            "/v1/admin/dashboard",
            get(dashboard::get_dashboard_snapshot),
        )
        .route("/v1/admin/services", get(services::list_services))
        .route(
            "/v1/admin/services/{service}/config",
            get(services::get_service_config).put(services::update_service_config),
        )
        .route(
            "/v1/admin/policies",
            get(policies::list_policies).post(policies::create_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}",
            put(policies::update_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}/publish",
            post(policies::publish_policy),
        )
        .route(
            "/v1/admin/policies/{policy_id}/make-current",
            post(policies::make_current_policy),
        )
        // Backward-compatible aliases for legacy Admin API documentation.
        .route(
            "/v1/policies",
            get(policies::list_policies).post(policies::create_policy),
        )
        .route("/v1/policies/{policy_id}", put(policies::update_policy))
        .route(
            "/v1/policies/{policy_id}/publish",
            post(policies::publish_policy),
        )
        .route(
            "/v1/policies/{policy_id}/make-current",
            post(policies::make_current_policy),
        )
        .route(
            "/v1/admin/moderation/rules",
            get(moderation::list_rules).post(moderation::create_rule),
        )
        .route(
            "/v1/admin/moderation/rules/{rule_id}",
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
        .route("/v1/labels", post(moderation::create_label))
        .route(
            "/v1/admin/subscription-requests",
            get(subscriptions::list_subscription_requests),
        )
        .route(
            "/v1/admin/subscription-requests/{request_id}/approve",
            post(subscriptions::approve_subscription_request),
        )
        .route(
            "/v1/admin/subscription-requests/{request_id}/reject",
            post(subscriptions::reject_subscription_request),
        )
        .route(
            "/v1/admin/node-subscriptions",
            get(subscriptions::list_node_subscriptions),
        )
        .route(
            "/v1/admin/node-subscriptions/{topic_id}",
            put(subscriptions::update_node_subscription),
        )
        .route(
            "/v1/admin/plans",
            get(subscriptions::list_plans).post(subscriptions::create_plan),
        )
        .route("/v1/admin/plans/{plan_id}", put(subscriptions::update_plan))
        .route(
            "/v1/admin/subscriptions",
            get(subscriptions::list_subscriptions),
        )
        .route(
            "/v1/admin/subscriptions/{subscriber_pubkey}",
            put(subscriptions::upsert_subscription),
        )
        .route("/v1/admin/usage", get(subscriptions::list_usage))
        .route("/v1/admin/audit-logs", get(services::list_audit_logs))
        .route("/v1/admin/personal-data-jobs", get(dsar::list_jobs))
        .route(
            "/v1/admin/personal-data-jobs/{job_type}/{job_id}/retry",
            post(dsar::retry_job),
        )
        .route(
            "/v1/admin/personal-data-jobs/{job_type}/{job_id}/cancel",
            post(dsar::cancel_job),
        )
        .route(
            "/v1/admin/access-control/memberships",
            get(access_control::list_memberships),
        )
        .route(
            "/v1/admin/access-control/rotate",
            post(access_control::rotate_epoch),
        )
        .route(
            "/v1/admin/access-control/revoke",
            post(access_control::revoke_member),
        )
        .route(
            "/v1/admin/trust/jobs",
            get(trust::list_jobs).post(trust::create_job),
        )
        .route("/v1/attestations", post(trust::create_job))
        .route("/v1/admin/trust/schedules", get(trust::list_schedules))
        .route(
            "/v1/admin/trust/schedules/{job_type}",
            put(trust::update_schedule),
        )
        .route("/v1/reindex", post(reindex::enqueue_reindex))
        .with_state(state)
}

async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let ready = async {
        db::check_ready(&state.pool).await?;
        health::ensure_health_targets_ready(&state.health_client, &state.health_targets).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match ready {
        Ok(_) => (
            StatusCode::OK,
            Json(HealthStatus {
                status: "ok".into(),
            }),
        ),
        Err(err) => {
            tracing::warn!(error = %err, "health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthStatus {
                    status: "unavailable".into(),
                }),
            )
        }
    }
}

async fn metrics_endpoint() -> impl IntoResponse {
    metrics::metrics_response(SERVICE_NAME)
}

async fn openapi_json(headers: HeaderMap) -> impl IntoResponse {
    let server_url = openapi::infer_server_url(&headers);
    Json(openapi::document(server_url.as_deref()))
}

fn parse_health_targets() -> HashMap<String, String> {
    health::parse_health_targets(
        "ADMIN_HEALTH_TARGETS",
        &[
            (
                "user-api",
                "USER_API_HEALTH_URL",
                "http://user-api:8080/healthz",
            ),
            ("relay", "RELAY_HEALTH_URL", "http://relay:8082/healthz"),
            (
                "bootstrap",
                "BOOTSTRAP_HEALTH_URL",
                "http://bootstrap:8083/healthz",
            ),
            ("index", "INDEX_HEALTH_URL", "http://index:8084/healthz"),
            (
                "moderation",
                "MODERATION_HEALTH_URL",
                "http://moderation:8085/healthz",
            ),
            ("trust", "TRUST_HEALTH_URL", "http://trust:8086/healthz"),
        ],
    )
}

#[cfg(test)]
mod router_smoke_tests {
    use super::*;
    use cn_core::service_config;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn router_initializes_with_axum_08_paths() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("create lazy pool");
        let admin_config = service_config::static_handle(serde_json::json!({
            "session_cookie": true,
            "session_ttl_seconds": 86400
        }));
        let state = AppState {
            pool,
            admin_config,
            health_targets: Arc::new(HashMap::new()),
            health_client: reqwest::Client::new(),
            dashboard_cache: Arc::new(
                tokio::sync::Mutex::new(dashboard::DashboardCache::default()),
            ),
            node_keys: Keys::generate(),
        };

        let _router = build_router(state);
    }
}
