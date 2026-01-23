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
            return Ok((
                StatusCode::NOT_MODIFIED,
                [(axum::http::header::ETAG, etag)],
            )
                .into_response());
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
                    return Ok((
                        StatusCode::NOT_MODIFIED,
                        [(axum::http::header::ETAG, etag)],
                    )
                        .into_response());
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
    response
        .headers_mut()
        .insert(axum::http::header::ETAG, HeaderValue::from_str(&etag).unwrap());
    if let Some(last_modified) = latest {
        let value = httpdate::fmt_http_date(last_modified.into());
        if let Ok(header) = HeaderValue::from_str(&value) {
            response.headers_mut().insert(
                axum::http::header::LAST_MODIFIED,
                header,
            );
        }
    }

    Ok(response)
}
