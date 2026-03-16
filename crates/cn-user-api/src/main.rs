use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use kukuri_cn_core::{
    ApiError, ApiResult, AuthChallengeResponse, AuthVerifyResponse, CommunityNodeBootstrapNode,
    CommunityNodeResolvedUrls, JwtConfig, accept_consents, connect_postgres, create_auth_challenge,
    get_consent_status, initialize_database, load_bootstrap_nodes, normalize_http_url,
    normalize_http_url_list, normalize_ws_url, require_bearer_pubkey, require_consents,
    verify_auth_event_and_issue_token,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    jwt_config: JwtConfig,
    self_node: CommunityNodeBootstrapNode,
}

#[derive(Debug, Deserialize)]
struct AuthChallengeRequest {
    pubkey: String,
}

#[derive(Debug, Deserialize)]
struct AuthVerifyRequest {
    auth_event_json: Value,
}

#[derive(Debug, Default, Deserialize)]
struct AcceptConsentsRequest {
    #[serde(default)]
    policy_slugs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BootstrapNodesResponse {
    nodes: Vec<CommunityNodeBootstrapNode>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let bind_addr = std::env::var("COMMUNITY_NODE_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse::<SocketAddr>()
        .context("failed to parse COMMUNITY_NODE_BIND_ADDR")?;
    let database_url = std::env::var("COMMUNITY_NODE_DATABASE_URL")
        .context("COMMUNITY_NODE_DATABASE_URL is required")?;
    let base_url = normalize_http_url(
        std::env::var("COMMUNITY_NODE_BASE_URL")
            .context("COMMUNITY_NODE_BASE_URL is required")?
            .as_str(),
    )?;
    let public_base_url = normalize_http_url(
        std::env::var("COMMUNITY_NODE_PUBLIC_BASE_URL")
            .ok()
            .as_deref()
            .unwrap_or(base_url.as_str()),
    )?;
    let relay_ws_url = normalize_ws_url(
        std::env::var("COMMUNITY_NODE_RELAY_WS_URL")
            .context("COMMUNITY_NODE_RELAY_WS_URL is required")?
            .as_str(),
    )?;
    let iroh_relay_urls = normalize_http_url_list(parse_csv_env("COMMUNITY_NODE_IROH_RELAY_URLS"))?;
    let pool = connect_postgres(database_url.as_str()).await?;
    initialize_database(&pool).await?;

    let state = AppState {
        pool,
        jwt_config: JwtConfig::from_env()?,
        self_node: CommunityNodeBootstrapNode {
            base_url,
            resolved_urls: CommunityNodeResolvedUrls::new(
                public_base_url,
                relay_ws_url,
                iroh_relay_urls,
            )?,
        },
    };
    let app = app_router(state);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind user api at {bind_addr}"))?;
    tracing::info!(bind_addr = %bind_addr, "community-node user-api listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/auth/challenge", post(auth_challenge))
        .route("/v1/auth/verify", post(auth_verify))
        .route("/v1/consents/status", get(consent_status))
        .route("/v1/consents", post(accept_consents_handler))
        .route("/v1/bootstrap/nodes", get(bootstrap_nodes))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn auth_challenge(
    State(state): State<AppState>,
    Json(request): Json<AuthChallengeRequest>,
) -> ApiResult<Json<AuthChallengeResponse>> {
    let response = create_auth_challenge(&state.pool, request.pubkey.as_str())
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn auth_verify(
    State(state): State<AppState>,
    Json(request): Json<AuthVerifyRequest>,
) -> ApiResult<Json<AuthVerifyResponse>> {
    let response = verify_auth_event_and_issue_token(
        &state.pool,
        &state.jwt_config,
        state.self_node.resolved_urls.public_base_url.as_str(),
        &request.auth_event_json,
    )
    .await
    .map_err(|error| {
        ApiError::new(
            axum::http::StatusCode::UNAUTHORIZED,
            "AUTH_FAILED",
            error.to_string(),
        )
    })?;
    Ok(Json(response))
}

async fn consent_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<kukuri_cn_core::CommunityNodeConsentStatus>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let status = get_consent_status(&state.pool, pubkey.as_str())
        .await
        .map_err(internal_error)?;
    Ok(Json(status))
}

async fn accept_consents_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AcceptConsentsRequest>,
) -> ApiResult<Json<kukuri_cn_core::CommunityNodeConsentStatus>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let status = accept_consents(&state.pool, pubkey.as_str(), &request.policy_slugs)
        .await
        .map_err(internal_error)?;
    Ok(Json(status))
}

async fn bootstrap_nodes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<BootstrapNodesResponse>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let _ = require_consents(&state.pool, pubkey.as_str()).await?;
    let nodes = load_bootstrap_nodes(&state.pool, Some(state.self_node.clone()))
        .await
        .map_err(internal_error)?;
    Ok(Json(BootstrapNodesResponse { nodes }))
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kukuri_cn_user_api=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        error.to_string(),
    )
}

fn parse_csv_env(var_name: &str) -> Vec<String> {
    std::env::var(var_name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|item| {
                    let trimmed = item.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
