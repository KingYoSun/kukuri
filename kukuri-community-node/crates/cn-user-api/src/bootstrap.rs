use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use cn_core::service_config;
use serde_json::json;
use sqlx::Row;

use crate::auth::{current_rate_limit, enforce_rate_limit, require_auth};
use crate::policies::require_consents;
use crate::{ApiError, ApiResult, AppState};

pub async fn get_bootstrap_nodes(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> ApiResult<impl IntoResponse> {
    apply_bootstrap_auth(&state, &headers).await?;
    apply_public_rate_limit(&state, connect).await?;

    let rows = sqlx::query(
        "SELECT event_json, updated_at, expires_at FROM cn_bootstrap.events          WHERE kind = 39000 AND is_active = TRUE",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    respond_with_events(&headers, rows).await
}

pub async fn get_bootstrap_services(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(topic_id): Path<String>,
    connect: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> ApiResult<impl IntoResponse> {
    apply_bootstrap_auth(&state, &headers).await?;
    apply_public_rate_limit(&state, connect).await?;

    let rows = sqlx::query(
        "SELECT event_json, updated_at, expires_at FROM cn_bootstrap.events          WHERE kind = 39001 AND topic_id = $1 AND is_active = TRUE",
    )
    .bind(topic_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    respond_with_events(&headers, rows).await
}

async fn apply_bootstrap_auth(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    let snapshot = state.bootstrap_config.get().await;
    let auth_config = service_config::auth_config_from_json(&snapshot.config_json);
    let now = cn_core::auth::unix_seconds().unwrap_or(0) as i64;
    if auth_config.requires_auth(now) {
        let auth = require_auth(state, headers).await?;
        require_consents(state, &auth).await?;
    }
    Ok(())
}

async fn apply_public_rate_limit(
    state: &AppState,
    connect: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> ApiResult<()> {
    let rate = current_rate_limit(state).await;
    if rate.enabled {
        let key = format!("public:{}", connect.0.ip());
        enforce_rate_limit(state, &key, rate.public_per_minute).await?;
    }
    Ok(())
}

async fn respond_with_events(
    headers: &HeaderMap,
    rows: Vec<sqlx::postgres::PgRow>,
) -> ApiResult<impl IntoResponse> {
    let mut events = Vec::new();
    let mut latest: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut next_refresh: Option<i64> = None;

    for row in rows {
        let event_json: serde_json::Value = row.try_get("event_json")?;
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
        let expires_at: i64 = row.try_get("expires_at")?;
        if latest.map(|value| updated_at > value).unwrap_or(true) {
            latest = Some(updated_at);
        }
        if next_refresh.map(|value| expires_at < value).unwrap_or(true) {
            next_refresh = Some(expires_at);
        }
        events.push(event_json);
    }

    let etag = format!(
        "W/\"{}-{}\"",
        latest.map(|value| value.timestamp()).unwrap_or(0),
        events.len()
    );

    if let Some(value) = headers.get("if-none-match").and_then(|v| v.to_str().ok()) {
        if value == etag {
            return Ok(
                (StatusCode::NOT_MODIFIED, [(axum::http::header::ETAG, etag)]).into_response(),
            );
        }
    }

    if let Some(last_modified) = latest {
        if let Some(value) = headers
            .get("if-modified-since")
            .and_then(|v| v.to_str().ok())
        {
            if let Ok(parsed) = httpdate::parse_http_date(value) {
                let parsed = chrono::DateTime::<chrono::Utc>::from(parsed);
                if last_modified <= parsed {
                    return Ok(
                        (StatusCode::NOT_MODIFIED, [(axum::http::header::ETAG, etag)])
                            .into_response(),
                    );
                }
            }
        }
    }

    let mut response = Json(json!({
        "items": events,
        "next_refresh_at": next_refresh
    }))
    .into_response();

    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    response.headers_mut().insert(
        axum::http::header::ETAG,
        HeaderValue::from_str(&etag).unwrap(),
    );
    if let Some(last_modified) = latest {
        let value = httpdate::fmt_http_date(last_modified.into());
        if let Ok(header) = HeaderValue::from_str(&value) {
            response
                .headers_mut()
                .insert(axum::http::header::LAST_MODIFIED, header);
        }
    }

    Ok(response)
}

#[cfg(test)]
mod api_contract_tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use cn_core::service_config;
    use nostr_sdk::prelude::Keys;
    use sqlx::postgres::PgPoolOptions;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state() -> crate::AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool");
        let jwt_config = cn_core::auth::JwtConfig {
            issuer: "http://localhost".to_string(),
            audience: crate::TOKEN_AUDIENCE.to_string(),
            secret: "test-secret".to_string(),
            ttl_seconds: 3600,
        };
        let user_config = service_config::static_handle(serde_json::json!({
            "rate_limit": { "enabled": false }
        }));
        let bootstrap_config = service_config::static_handle(serde_json::json!({
            "auth": { "mode": "required" }
        }));
        let meili = cn_core::meili::MeiliClient::new("http://localhost:7700".to_string(), None)
            .expect("meili");

        crate::AppState {
            pool,
            jwt_config,
            public_base_url: "http://localhost".to_string(),
            user_config,
            bootstrap_config,
            rate_limiter: Arc::new(cn_core::rate_limit::RateLimiter::new()),
            node_keys: Keys::generate(),
            export_dir: PathBuf::from("tmp/test_exports"),
            hmac_secret: b"test-secret".to_vec(),
            meili,
        }
    }

    async fn request_status(app: Router, uri: &str) -> StatusCode {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        response.status()
    }

    #[tokio::test]
    async fn bootstrap_nodes_requires_auth_when_enabled() {
        let app = Router::new()
            .route("/v1/bootstrap/nodes", get(get_bootstrap_nodes))
            .with_state(test_state());
        let status = request_status(app, "/v1/bootstrap/nodes").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bootstrap_services_requires_auth_when_enabled() {
        let app = Router::new()
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(get_bootstrap_services),
            )
            .with_state(test_state());
        let status = request_status(app, "/v1/bootstrap/topics/kukuri:topic1/services").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
