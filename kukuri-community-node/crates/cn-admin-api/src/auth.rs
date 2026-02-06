use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;

use crate::{ApiError, ApiResult, AppState};

const SESSION_COOKIE: &str = "cn_admin_session";

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub admin_user_id: String,
    pub username: String,
    pub expires_at: i64,
}

#[derive(Serialize, ToSchema)]
pub struct AdminUser {
    pub admin_user_id: String,
    pub username: String,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<(CookieJar, Json<LoginResponse>)> {
    let row = sqlx::query(
        "SELECT admin_user_id, password_hash, is_active FROM cn_admin.admin_users WHERE username = $1",
    )
    .bind(&payload.username)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_FAILED",
            "invalid credentials",
        ));
    };

    let admin_user_id: String = row.try_get("admin_user_id")?;
    let password_hash: String = row.try_get("password_hash")?;
    let is_active: bool = row.try_get("is_active")?;
    if !is_active {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "ACCOUNT_DISABLED",
            "admin disabled",
        ));
    }

    let verified =
        cn_core::admin::verify_password(&payload.password, &password_hash).map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUTH_ERROR",
                err.to_string(),
            )
        })?;
    if !verified {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_FAILED",
            "invalid credentials",
        ));
    }

    let ttl = session_ttl_seconds(&state).await;
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl);
    let session_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO cn_admin.admin_sessions          (session_id, admin_user_id, expires_at)          VALUES ($1, $2, $3)",
    )
    .bind(&session_id)
    .bind(&admin_user_id)
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    cn_core::admin::log_audit(
        &state.pool,
        &admin_user_id,
        "admin.login",
        &format!("admin_user:{admin_user_id}"),
        None,
        None,
    )
    .await
    .ok();

    let cookie = Cookie::build((SESSION_COOKIE, session_id))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build();
    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(LoginResponse {
            admin_user_id,
            username: payload.username,
            expires_at: expires_at.timestamp(),
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<serde_json::Value>)> {
    let mut jar = jar;
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let session_id = cookie.value().to_string();
        sqlx::query("DELETE FROM cn_admin.admin_sessions WHERE session_id = $1")
            .bind(&session_id)
            .execute(&state.pool)
            .await
            .ok();
        jar = jar.remove(Cookie::from(SESSION_COOKIE));
    }

    Ok((jar, Json(serde_json::json!({ "status": "ok" }))))
}

pub async fn me(State(state): State<AppState>, jar: CookieJar) -> ApiResult<Json<AdminUser>> {
    let admin = require_admin(&state, &jar).await?;
    Ok(Json(admin))
}

pub(crate) async fn require_admin(state: &AppState, jar: &CookieJar) -> ApiResult<AdminUser> {
    let Some(cookie) = jar.get(SESSION_COOKIE) else {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_REQUIRED",
            "missing session",
        ));
    };
    let session_id = cookie.value();
    let row = sqlx::query(
        "SELECT u.admin_user_id, u.username, s.expires_at          FROM cn_admin.admin_sessions s          JOIN cn_admin.admin_users u ON s.admin_user_id = u.admin_user_id          WHERE s.session_id = $1",
    )
    .bind(session_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    let Some(row) = row else {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_REQUIRED",
            "invalid session",
        ));
    };

    let expires_at: chrono::DateTime<chrono::Utc> = row.try_get("expires_at")?;
    if chrono::Utc::now() > expires_at {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_REQUIRED",
            "session expired",
        ));
    }

    Ok(AdminUser {
        admin_user_id: row.try_get("admin_user_id")?,
        username: row.try_get("username")?,
    })
}

async fn session_ttl_seconds(state: &AppState) -> i64 {
    let snapshot = state.admin_config.get().await;
    snapshot
        .config_json
        .get("session_ttl_seconds")
        .and_then(|value| value.as_i64())
        .unwrap_or(86400)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use cn_core::service_config;
    use nostr_sdk::prelude::Keys;
    use sqlx::postgres::PgPoolOptions;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state() -> crate::AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/postgres")
            .expect("lazy pool");
        let admin_config = service_config::static_handle(serde_json::json!({
            "session_cookie": true,
            "session_ttl_seconds": 86400
        }));
        crate::AppState {
            pool,
            admin_config,
            health_targets: Arc::new(HashMap::new()),
            health_client: reqwest::Client::new(),
            node_keys: Keys::generate(),
        }
    }

    #[tokio::test]
    async fn me_requires_session_cookie() {
        let app = Router::new()
            .route("/v1/admin/auth/me", get(me))
            .with_state(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/admin/auth/me")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn login_rejects_invalid_json() {
        let app = Router::new()
            .route("/v1/admin/auth/login", post(login))
            .with_state(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/admin/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from("{invalid"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn logout_without_cookie_returns_ok() {
        let app = Router::new()
            .route("/v1/admin/auth/logout", post(logout))
            .with_state(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/admin/auth/logout")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
