use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use cn_core::{config, db, http, logging, meili, metrics, node_key, server, service_config};
use nostr_sdk::prelude::Keys;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use sqlx::{Pool, Postgres};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use utoipa::ToSchema;

mod auth;
mod billing;
mod bootstrap;
pub mod openapi;
mod personal_data;
mod policies;
mod subscriptions;

#[cfg(test)]
mod openapi_contract_tests;

const SERVICE_NAME: &str = "cn-user-api";
const TOKEN_AUDIENCE: &str = "kukuri-community-node:user-api";

#[derive(Clone)]
pub(crate) struct AppState {
    pool: Pool<Postgres>,
    jwt_config: cn_core::auth::JwtConfig,
    public_base_url: String,
    user_config: service_config::ServiceConfigHandle,
    bootstrap_config: service_config::ServiceConfigHandle,
    rate_limiter: Arc<cn_core::rate_limit::RateLimiter>,
    node_keys: Keys,
    export_dir: PathBuf,
    hmac_secret: Vec<u8>,
    meili: meili::MeiliClient,
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
    headers: HeaderMap,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
            headers: HeaderMap::new(),
        }
    }

    fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    fn with_header(mut self, name: &'static str, value: String) -> Self {
        if let Ok(header_name) = name.parse::<axum::http::header::HeaderName>() {
            if let Ok(header_value) = value.parse::<HeaderValue>() {
                self.headers.insert(header_name, header_value);
            }
        }
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
        let mut response = (self.status, Json(payload)).into_response();
        for (name, value) in self.headers.iter() {
            response.headers_mut().insert(name, value.clone());
        }
        response
    }
}

type ApiResult<T> = Result<T, ApiError>;

#[derive(Serialize, ToSchema)]
pub(crate) struct HealthStatus {
    status: String,
}

pub struct UserApiConfig {
    pub addr: SocketAddr,
    pub database_url: String,
    pub public_base_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_seconds: u64,
    pub node_key_path: PathBuf,
    pub config_poll_seconds: u64,
    pub export_dir: PathBuf,
    pub hmac_secret: Vec<u8>,
    pub meili_url: String,
    pub meili_master_key: Option<String>,
}

pub fn load_config() -> Result<UserApiConfig> {
    let addr = config::socket_addr_from_env("USER_API_ADDR", "0.0.0.0:8080")?;
    let database_url = config::required_env("DATABASE_URL")?;
    let public_base_url = config::required_env("PUBLIC_BASE_URL")?;
    let jwt_secret = config::required_env("USER_JWT_SECRET")?;
    let jwt_ttl_seconds = std::env::var("USER_JWT_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(900);
    let node_key_path = node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")?;
    let config_poll_seconds = std::env::var("USER_CONFIG_POLL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    let export_dir = std::env::var("PERSONAL_DATA_EXPORT_DIR")
        .unwrap_or_else(|_| "data/personal_exports".to_string());
    let export_dir = PathBuf::from(export_dir);
    let hmac_secret = std::env::var("PERSONAL_DATA_HMAC_SECRET")
        .ok()
        .unwrap_or_else(|| jwt_secret.clone());
    let meili_url = config::required_env("MEILI_URL")?;
    let meili_master_key = std::env::var("MEILI_MASTER_KEY").ok();
    Ok(UserApiConfig {
        addr,
        database_url,
        public_base_url,
        jwt_secret,
        jwt_ttl_seconds,
        node_key_path,
        config_poll_seconds,
        export_dir,
        hmac_secret: hmac_secret.into_bytes(),
        meili_url,
        meili_master_key,
    })
}

pub async fn run(config: UserApiConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    metrics::init(SERVICE_NAME);

    let pool = db::connect(&config.database_url).await?;
    let node_keys = node_key::load_or_generate(&config.node_key_path)?;
    std::fs::create_dir_all(&config.export_dir)?;

    let user_default = json!({
        "rate_limit": {
            "enabled": true,
            "auth_per_minute": 20,
            "public_per_minute": 120,
            "protected_per_minute": 120
        }
    });
    let bootstrap_default = json!({
        "auth": {
            "mode": "off",
            "enforce_at": null,
            "grace_seconds": 900
        }
    });

    let user_config = service_config::watch_service_config(
        pool.clone(),
        "user-api",
        user_default,
        Duration::from_secs(config.config_poll_seconds),
    )
    .await?;
    let bootstrap_config = service_config::watch_service_config(
        pool.clone(),
        "bootstrap",
        bootstrap_default,
        Duration::from_secs(config.config_poll_seconds),
    )
    .await?;

    billing::ensure_default_plan(&pool).await?;

    let jwt_config = cn_core::auth::JwtConfig {
        issuer: config.public_base_url.clone(),
        audience: TOKEN_AUDIENCE.to_string(),
        secret: config.jwt_secret,
        ttl_seconds: config.jwt_ttl_seconds,
    };

    let meili_client = meili::MeiliClient::new(config.meili_url, config.meili_master_key)?;
    let state = AppState {
        pool,
        jwt_config,
        public_base_url: config.public_base_url,
        user_config,
        bootstrap_config,
        rate_limiter: Arc::new(cn_core::rate_limit::RateLimiter::new()),
        node_keys,
        export_dir: config.export_dir,
        hmac_secret: config.hmac_secret,
        meili: meili_client,
    };

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .route("/v1/openapi.json", get(openapi_json))
        .route("/v1/auth/challenge", post(auth::auth_challenge))
        .route("/v1/auth/verify", post(auth::auth_verify))
        .route("/v1/policies/current", get(policies::get_current_policies))
        .route(
            "/v1/policies/{policy_type}/{version}",
            get(policies::get_policy_by_version),
        )
        .route("/v1/consents/status", get(policies::get_consent_status))
        .route("/v1/consents", post(policies::accept_consents))
        .route("/v1/bootstrap/nodes", get(bootstrap::get_bootstrap_nodes))
        .route(
            "/v1/bootstrap/topics/{topic_id}/services",
            get(bootstrap::get_bootstrap_services),
        )
        .route(
            "/v1/topic-subscription-requests",
            post(subscriptions::create_subscription_request),
        )
        .route(
            "/v1/topic-subscriptions",
            get(subscriptions::list_topic_subscriptions),
        )
        .route(
            "/v1/topic-subscriptions/{topic_id}",
            delete(subscriptions::delete_topic_subscription),
        )
        .route("/v1/search", get(subscriptions::search))
        .route("/v1/trending", get(subscriptions::trending))
        .route("/v1/reports", post(subscriptions::submit_report))
        .route("/v1/labels", get(subscriptions::list_labels))
        .route(
            "/v1/trust/report-based",
            get(subscriptions::trust_report_based),
        )
        .route(
            "/v1/trust/communication-density",
            get(subscriptions::trust_communication_density),
        )
        .route(
            "/v1/personal-data-export-requests",
            post(personal_data::create_export_request),
        )
        .route(
            "/v1/personal-data-export-requests/{export_request_id}",
            get(personal_data::get_export_request),
        )
        .route(
            "/v1/personal-data-export-requests/{export_request_id}/download",
            get(personal_data::download_export),
        )
        .route(
            "/v1/personal-data-deletion-requests",
            post(personal_data::create_deletion_request),
        )
        .route(
            "/v1/personal-data-deletion-requests/{deletion_request_id}",
            get(personal_data::get_deletion_request),
        )
        .with_state(state);

    let router = http::apply_standard_layers(router, SERVICE_NAME);
    server::serve(config.addr, router).await
}

async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let ready = async {
        db::check_ready(&state.pool).await?;
        state.meili.check_ready().await?;
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
