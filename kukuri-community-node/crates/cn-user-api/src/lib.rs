use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{config, db, http, logging, metrics, server};
use serde::Serialize;
use sqlx::{Pool, Postgres};
use utoipa::OpenApi;

const SERVICE_NAME: &str = "cn-user-api";

#[derive(Clone)]
struct AppState {
    pool: Pool<Postgres>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct HealthStatus {
    status: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(healthz, metrics_endpoint),
    components(schemas(HealthStatus)),
    tags((name = "health", description = "Service health"))
)]
struct ApiDoc;

pub struct UserApiConfig {
    pub addr: std::net::SocketAddr,
    pub database_url: String,
}

pub fn load_config() -> Result<UserApiConfig> {
    let addr = config::socket_addr_from_env("USER_API_ADDR", "0.0.0.0:8080")?;
    let database_url = config::required_env("DATABASE_URL")?;
    Ok(UserApiConfig { addr, database_url })
}

pub async fn run(config: UserApiConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    let pool = db::connect(&config.database_url).await?;
    let state = AppState { pool };

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
        .route("/v1/openapi.json", get(openapi_json))
        .with_state(state);

    let router = http::apply_standard_layers(router, SERVICE_NAME);
    server::serve(config.addr, router).await
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, description = "Ready", body = HealthStatus))
)]
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

#[utoipa::path(
    get,
    path = "/metrics",
    responses((status = 200, description = "Prometheus metrics"))
)]
async fn metrics_endpoint() -> impl IntoResponse {
    metrics::metrics_response(SERVICE_NAME)
}

async fn openapi_json() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}
