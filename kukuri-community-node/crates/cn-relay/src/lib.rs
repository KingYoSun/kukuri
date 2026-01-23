use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use cn_core::{config, db, http, logging, metrics, server};
use serde::Serialize;
use sqlx::{Pool, Postgres};

const SERVICE_NAME: &str = "cn-relay";

#[derive(Clone)]
struct AppState {
    pool: Pool<Postgres>,
}

#[derive(Serialize)]
struct HealthStatus {
    status: String,
}

pub struct RelayConfig {
    pub addr: std::net::SocketAddr,
    pub database_url: String,
}

pub fn load_config() -> Result<RelayConfig> {
    let addr = config::socket_addr_from_env("RELAY_ADDR", "0.0.0.0:8082")?;
    let database_url = config::required_env("DATABASE_URL")?;
    Ok(RelayConfig { addr, database_url })
}

pub async fn run(config: RelayConfig) -> Result<()> {
    logging::init(SERVICE_NAME);
    let pool = db::connect(&config.database_url).await?;
    let state = AppState { pool };

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics_endpoint))
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
