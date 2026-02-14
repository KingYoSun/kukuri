use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use cn_core::service_config;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::auth::{current_rate_limit, enforce_rate_limit, require_auth};
use crate::policies::require_consents;
use crate::{ApiError, ApiResult, AppState};

#[derive(Debug, Default, Deserialize)]
pub struct BootstrapHintQuery {
    #[serde(default)]
    since: Option<u64>,
}

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

pub async fn get_bootstrap_hint(
    State(state): State<AppState>,
    Query(query): Query<BootstrapHintQuery>,
    headers: HeaderMap,
    connect: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> ApiResult<impl IntoResponse> {
    apply_bootstrap_auth(&state, &headers).await?;
    apply_public_rate_limit(&state, connect).await?;

    let since = query.since.unwrap_or(0);
    let latest = state.bootstrap_hints.latest_after(since).await;
    if let Some(snapshot) = latest {
        let mut response = Json(json!({
            "seq": snapshot.seq,
            "received_at": snapshot.received_at,
            "hint": snapshot.hint,
        }))
        .into_response();
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
        return Ok(response);
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    Ok(response)
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

    let payload = json!({
        "items": events,
        "next_refresh_at": next_refresh
    });
    let payload_bytes = serde_json::to_vec(&payload).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "SERIALIZATION_ERROR",
            err.to_string(),
        )
    })?;
    let etag = format!("W/\"{}\"", blake3::hash(&payload_bytes).to_hex());

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
                if last_modified.timestamp() <= parsed.timestamp() {
                    return Ok(
                        (StatusCode::NOT_MODIFIED, [(axum::http::header::ETAG, etag)])
                            .into_response(),
                    );
                }
            }
        }
    }

    let mut response = Json(payload).into_response();

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
    use axum::http::{header, HeaderMap, Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use cn_core::service_config;
    use nostr_sdk::prelude::Keys;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::{Pool, Postgres};
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::OnceCell;
    use tower::ServiceExt;
    use uuid::Uuid;

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:5432/cn".to_string())
    }

    async fn ensure_migrated(pool: &Pool<Postgres>) {
        MIGRATIONS
            .get_or_init(|| async {
                cn_core::migrations::run(pool)
                    .await
                    .expect("run migrations");
            })
            .await;
    }

    fn test_state_with_auth_mode(auth_mode: &str) -> crate::AppState {
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
            "auth": { "mode": auth_mode }
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
            bootstrap_hints: Arc::new(crate::BootstrapHintStore::default()),
        }
    }

    fn test_state() -> crate::AppState {
        test_state_with_auth_mode("required")
    }

    async fn test_state_with_auth_mode_and_user_config(
        auth_mode: &str,
        user_config_json: serde_json::Value,
    ) -> crate::AppState {
        let pool = PgPoolOptions::new()
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        crate::billing::ensure_default_plan(&pool)
            .await
            .expect("seed plans");

        let jwt_config = cn_core::auth::JwtConfig {
            issuer: "http://localhost".to_string(),
            audience: crate::TOKEN_AUDIENCE.to_string(),
            secret: "test-secret".to_string(),
            ttl_seconds: 3600,
        };
        let user_config = service_config::static_handle(user_config_json);
        let bootstrap_config = service_config::static_handle(serde_json::json!({
            "auth": { "mode": auth_mode }
        }));
        let meili = cn_core::meili::MeiliClient::new("http://localhost:7700".to_string(), None)
            .expect("meili");
        let export_dir = PathBuf::from(format!("tmp/test_exports/{}", Uuid::new_v4()));
        std::fs::create_dir_all(&export_dir).expect("create test export dir");

        crate::AppState {
            pool,
            jwt_config,
            public_base_url: "http://localhost".to_string(),
            user_config,
            bootstrap_config,
            rate_limiter: Arc::new(cn_core::rate_limit::RateLimiter::new()),
            node_keys: Keys::generate(),
            export_dir,
            hmac_secret: b"test-secret".to_vec(),
            meili,
            bootstrap_hints: Arc::new(crate::BootstrapHintStore::default()),
        }
    }

    async fn request_status_and_headers(app: Router, uri: &str) -> (StatusCode, HeaderMap) {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        (response.status(), response.headers().clone())
    }

    async fn request_status_and_body(app: Router, uri: &str) -> (StatusCode, String) {
        let mut request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let body = response.into_body();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .expect("read body");
        let text = String::from_utf8(bytes.to_vec()).expect("utf8 body");
        (status, text)
    }

    #[tokio::test]
    async fn bootstrap_nodes_requires_auth_when_enabled() {
        let app = Router::new()
            .route("/v1/bootstrap/nodes", get(get_bootstrap_nodes))
            .with_state(test_state());
        let (status, headers) = request_status_and_headers(app, "/v1/bootstrap/nodes").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            headers
                .get(header::WWW_AUTHENTICATE)
                .and_then(|value| value.to_str().ok()),
            Some(r#"Bearer realm="cn-user-api""#)
        );
    }

    #[tokio::test]
    async fn bootstrap_services_requires_auth_when_enabled() {
        let app = Router::new()
            .route(
                "/v1/bootstrap/topics/{topic_id}/services",
                get(get_bootstrap_services),
            )
            .with_state(test_state());
        let (status, headers) =
            request_status_and_headers(app, "/v1/bootstrap/topics/kukuri:topic1/services").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            headers
                .get(header::WWW_AUTHENTICATE)
                .and_then(|value| value.to_str().ok()),
            Some(r#"Bearer realm="cn-user-api""#)
        );
    }

    #[tokio::test]
    async fn bootstrap_hint_latest_contract_no_content_without_updates() {
        let app = Router::new()
            .route("/v1/bootstrap/hints/latest", get(get_bootstrap_hint))
            .with_state(test_state_with_auth_mode("off"));
        let (status, headers) = request_status_and_headers(app, "/v1/bootstrap/hints/latest").await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            headers
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
    }

    #[tokio::test]
    async fn bootstrap_hint_latest_contract_returns_latest_payload_after_since() {
        let state = test_state_with_auth_mode("off");
        state
            .bootstrap_hints
            .push_hint(serde_json::json!({
                "schema": "kukuri-bootstrap-update-hint-v1",
                "refresh_paths": ["/v1/bootstrap/nodes"]
            }))
            .await;
        let app = Router::new()
            .route("/v1/bootstrap/hints/latest", get(get_bootstrap_hint))
            .with_state(state);

        let (status, body) =
            request_status_and_body(app.clone(), "/v1/bootstrap/hints/latest?since=0").await;
        assert_eq!(status, StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_str(&body).expect("hint payload");
        assert_eq!(payload.get("seq").and_then(|value| value.as_u64()), Some(1));
        assert_eq!(
            payload
                .get("hint")
                .and_then(|value| value.get("schema"))
                .and_then(|value| value.as_str()),
            Some("kukuri-bootstrap-update-hint-v1")
        );

        let (status, _) = request_status_and_body(app, "/v1/bootstrap/hints/latest?since=1").await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    #[tokio::test]
    async fn bootstrap_hint_latest_requires_auth_when_enabled() {
        let app = Router::new()
            .route("/v1/bootstrap/hints/latest", get(get_bootstrap_hint))
            .with_state(test_state());

        let (status, headers) = request_status_and_headers(app, "/v1/bootstrap/hints/latest").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            headers
                .get(header::WWW_AUTHENTICATE)
                .and_then(|value| value.to_str().ok()),
            Some(r#"Bearer realm="cn-user-api""#)
        );
    }

    #[tokio::test]
    async fn bootstrap_hint_latest_requires_consents_when_missing() {
        let state = test_state_with_auth_mode_and_user_config(
            "required",
            serde_json::json!({ "rate_limit": { "enabled": false } }),
        )
        .await;
        let pubkey = Keys::generate().public_key().to_hex();
        let (token, _) = cn_core::auth::issue_token(&pubkey, &state.jwt_config).expect("token");
        let policy_id = format!("terms-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO cn_admin.policies \
                (policy_id, type, version, locale, title, content_md, content_hash, published_at, effective_at, is_current) \
             VALUES ($1, 'terms', 'v1', 'ja-JP', 'Terms', '# Terms', $2, NOW(), NOW(), TRUE)",
        )
        .bind(&policy_id)
        .bind(format!("sha256:{policy_id}"))
        .execute(&state.pool)
        .await
        .expect("insert current policy");

        let app = Router::new()
            .route("/v1/bootstrap/hints/latest", get(get_bootstrap_hint))
            .with_state(state);
        let mut request = Request::builder()
            .method("GET")
            .uri("/v1/bootstrap/hints/latest")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))));
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::PRECONDITION_REQUIRED);
    }

    #[tokio::test]
    async fn bootstrap_hint_latest_rate_limited_returns_retry_after() {
        let state = test_state_with_auth_mode_and_user_config(
            "off",
            serde_json::json!({
                "rate_limit": {
                    "enabled": true,
                    "public_per_minute": 1,
                    "protected_per_minute": 120,
                    "auth_per_minute": 20
                }
            }),
        )
        .await;
        let app = Router::new()
            .route("/v1/bootstrap/hints/latest", get(get_bootstrap_hint))
            .with_state(state);

        let (first_status, _) = request_status_and_headers(app.clone(), "/v1/bootstrap/hints/latest").await;
        assert_eq!(first_status, StatusCode::NO_CONTENT);

        let (status, headers) = request_status_and_headers(app, "/v1/bootstrap/hints/latest").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        let retry_after = headers
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        assert!(retry_after >= 1, "Retry-After must be >= 1: {retry_after}");
    }

}
