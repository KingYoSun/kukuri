use axum::extract::{ConnectInfo, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use cn_core::{auth, metrics};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Row};
use std::net::SocketAddr;
use std::time::Duration;
use uuid::Uuid;

use crate::{ApiError, ApiResult, AppState};

const AUTH_KIND: u32 = 22242;
const AUTHENTICATE_BEARER_CHALLENGE: &str = r#"Bearer realm="cn-user-api""#;

#[derive(Deserialize)]
pub struct AuthChallengeRequest {
    pub pubkey: String,
}

#[derive(Serialize)]
pub struct AuthChallengeResponse {
    pub challenge: String,
    pub expires_at: i64,
}

#[derive(Deserialize)]
pub struct AuthVerifyRequest {
    pub auth_event_json: Value,
}

#[derive(Serialize)]
pub struct AuthVerifyResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_at: i64,
    pub pubkey: String,
}

#[derive(Clone, Copy)]
pub(crate) struct UserRateLimitConfig {
    pub enabled: bool,
    pub auth_per_minute: u64,
    pub public_per_minute: u64,
    pub protected_per_minute: u64,
}

#[derive(Clone, Serialize)]
pub(crate) struct AuthContext {
    pub pubkey: String,
}

pub async fn auth_challenge(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthChallengeRequest>,
) -> ApiResult<Json<AuthChallengeResponse>> {
    let rate = current_rate_limit(&state).await;
    if rate.enabled {
        let key = format!("auth:{}", addr.ip());
        enforce_rate_limit(&state, &key, rate.auth_per_minute).await?;
    }

    let pubkey = normalize_pubkey(&payload.pubkey)?;
    let challenge = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(300);

    sqlx::query(
        "INSERT INTO cn_user.auth_challenges          (challenge, pubkey, expires_at)          VALUES ($1, $2, $3)",
    )
    .bind(&challenge)
    .bind(&pubkey)
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;

    Ok(Json(AuthChallengeResponse {
        challenge,
        expires_at: expires_at.timestamp(),
    }))
}

pub async fn auth_verify(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthVerifyRequest>,
) -> ApiResult<Json<AuthVerifyResponse>> {
    let rate = current_rate_limit(&state).await;
    if rate.enabled {
        let key = format!("auth:{}", addr.ip());
        enforce_rate_limit(&state, &key, rate.auth_per_minute).await?;
    }

    let raw = cn_core::nostr::parse_event(&payload.auth_event_json)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string()))?;
    if raw.kind != AUTH_KIND {
        metrics::inc_auth_failure(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVENT",
            "auth event kind mismatch",
        ));
    }
    cn_core::nostr::verify_event(&raw)
        .map_err(|err| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_EVENT", err.to_string()))?;

    let now = auth::unix_seconds().unwrap_or(0) as i64;
    if (now - raw.created_at).abs() > 600 {
        metrics::inc_auth_failure(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVENT",
            "auth event is stale",
        ));
    }

    let relay_tag = raw.first_tag_value("relay");
    if relay_tag.as_deref() != Some(state.public_base_url.as_str()) {
        metrics::inc_auth_failure(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVENT",
            "relay tag mismatch",
        ));
    }

    let challenge = raw.first_tag_value("challenge").ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_EVENT",
            "missing challenge",
        )
    })?;

    let row = sqlx::query(
        "SELECT pubkey, expires_at, used_at FROM cn_user.auth_challenges WHERE challenge = $1",
    )
    .bind(&challenge)
    .fetch_optional(&state.pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    let Some(row) = row else {
        metrics::inc_auth_failure(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_FAILED",
            "challenge not found",
        ));
    };

    let stored_pubkey: String = row.try_get("pubkey").unwrap_or_default();
    let expires_at: chrono::DateTime<chrono::Utc> = row.try_get("expires_at").map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;
    let used_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("used_at").map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    if used_at.is_some() || chrono::Utc::now() > expires_at {
        metrics::inc_auth_failure(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_FAILED",
            "challenge expired or used",
        ));
    }

    if stored_pubkey != raw.pubkey {
        metrics::inc_auth_failure(crate::SERVICE_NAME);
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "AUTH_FAILED",
            "pubkey mismatch",
        ));
    }

    mark_challenge_used(&state.pool, &challenge).await?;
    ensure_active_subscriber(&state.pool, &raw.pubkey).await?;

    let (token, claims) = auth::issue_token(&raw.pubkey, &state.jwt_config).map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "AUTH_ERROR",
            err.to_string(),
        )
    })?;
    metrics::inc_auth_success(crate::SERVICE_NAME);

    Ok(Json(AuthVerifyResponse {
        access_token: token,
        token_type: "Bearer",
        expires_at: claims.exp as i64,
        pubkey: raw.pubkey,
    }))
}

pub(crate) async fn require_auth(state: &AppState, headers: &HeaderMap) -> ApiResult<AuthContext> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| auth_required_error("missing token"))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| auth_required_error("invalid token"))?;
    let claims = auth::verify_token(token, &state.jwt_config)
        .map_err(|err| auth_required_error(err.to_string()))?;
    let pubkey = claims.sub;
    ensure_active_subscriber(&state.pool, &pubkey).await?;
    Ok(AuthContext { pubkey })
}

fn auth_required_error(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED", message).with_header(
        "WWW-Authenticate",
        AUTHENTICATE_BEARER_CHALLENGE.to_string(),
    )
}

async fn mark_challenge_used(pool: &sqlx::Pool<Postgres>, challenge: &str) -> ApiResult<()> {
    sqlx::query("UPDATE cn_user.auth_challenges SET used_at = NOW() WHERE challenge = $1")
        .bind(challenge)
        .execute(pool)
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                err.to_string(),
            )
        })?;
    Ok(())
}

async fn ensure_active_subscriber(pool: &sqlx::Pool<Postgres>, pubkey: &str) -> ApiResult<()> {
    let existing = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.subscriber_accounts WHERE subscriber_pubkey = $1",
    )
    .bind(pubkey)
    .fetch_optional(pool)
    .await
    .map_err(|err| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            err.to_string(),
        )
    })?;

    match existing.as_deref() {
        Some("active") => {}
        Some("disabled") => {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "ACCOUNT_DISABLED",
                "account disabled",
            ));
        }
        Some("deleting") | Some("deleted") => {
            return Err(ApiError::new(
                StatusCode::GONE,
                "ACCOUNT_DELETED",
                "account deleted",
            ));
        }
        _ => {
            sqlx::query(
                "INSERT INTO cn_user.subscriber_accounts                  (subscriber_pubkey, status)                  VALUES ($1, 'active')                  ON CONFLICT (subscriber_pubkey) DO UPDATE SET status = 'active', updated_at = NOW()",
            )
            .bind(pubkey)
            .execute(pool)
            .await
            .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", err.to_string()))?;
        }
    }

    Ok(())
}

pub(crate) async fn current_rate_limit(state: &AppState) -> UserRateLimitConfig {
    let snapshot = state.user_config.get().await;
    let rate = snapshot
        .config_json
        .get("rate_limit")
        .and_then(|value| value.as_object());
    let per_minute = rate
        .and_then(|value| value.get("per_minute"))
        .and_then(|value| value.as_u64());
    UserRateLimitConfig {
        enabled: rate
            .and_then(|value| value.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true),
        auth_per_minute: rate
            .and_then(|value| value.get("auth_per_minute"))
            .and_then(|value| value.as_u64())
            .unwrap_or(20),
        public_per_minute: rate
            .and_then(|value| value.get("public_per_minute"))
            .and_then(|value| value.as_u64())
            .or(per_minute)
            .unwrap_or(120),
        protected_per_minute: rate
            .and_then(|value| value.get("protected_per_minute"))
            .and_then(|value| value.as_u64())
            .or(per_minute)
            .unwrap_or(120),
    }
}

pub(crate) async fn enforce_rate_limit(state: &AppState, key: &str, limit: u64) -> ApiResult<()> {
    let outcome = state
        .rate_limiter
        .check(key, limit, Duration::from_secs(60))
        .await;
    if !outcome.allowed {
        let retry_after = outcome
            .retry_after
            .map(|dur| dur.as_secs().max(1))
            .unwrap_or(60);
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            "rate limited",
        )
        .with_header("Retry-After", retry_after.to_string()));
    }
    Ok(())
}

#[allow(clippy::result_large_err)]
fn normalize_pubkey(pubkey: &str) -> ApiResult<String> {
    let parsed = nostr_sdk::prelude::PublicKey::parse(pubkey)
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "INVALID_PUBKEY", "invalid pubkey"))?;
    Ok(parsed.to_hex())
}
