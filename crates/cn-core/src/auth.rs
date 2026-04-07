use anyhow::{Context, Result, anyhow, bail};
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use sqlx::postgres::PgPool;

use kukuri_core::{KukuriAuthEnvelopeContentV1, KukuriKeys, sign_envelope_json};

use crate::bootstrap::{
    prune_expired_bootstrap_peer_registrations, upsert_bootstrap_peer_registration,
};
use crate::config::{
    AUTH_CHALLENGE_TTL_SECONDS, AUTH_ENVELOPE_KIND, AUTH_EVENT_MAX_SKEW_SECONDS,
    JWT_CRYPTO_PROVIDER_INIT, JwtConfig,
};
use crate::database::ensure_active_subscriber;
use crate::errors::{ApiError, ApiResult, auth_required_error};
use crate::models::{
    AuthChallengeResponse, AuthVerifyResponse, BearerIdentity, CommunityNodeSeedPeer,
};
use crate::normalize::{
    first_tag_value, normalize_http_url, normalize_pubkey, parse_auth_envelope,
    verify_auth_envelope,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    iss: String,
    iat: usize,
    exp: usize,
    #[serde(default)]
    endpoint_id: Option<String>,
}

pub async fn create_auth_challenge(pool: &PgPool, pubkey: &str) -> Result<AuthChallengeResponse> {
    let pubkey = normalize_pubkey(pubkey)?;
    let challenge = uuid::Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::seconds(AUTH_CHALLENGE_TTL_SECONDS);
    sqlx::query(
        "INSERT INTO cn_auth.auth_challenges (challenge, pubkey, expires_at)
         VALUES ($1, $2, $3)",
    )
    .bind(&challenge)
    .bind(&pubkey)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(AuthChallengeResponse {
        challenge,
        expires_at: expires_at.timestamp(),
    })
}

pub async fn verify_auth_envelope_and_issue_token(
    pool: &PgPool,
    jwt_config: &JwtConfig,
    public_base_url: &str,
    auth_envelope_json: &Value,
    endpoint_id: Option<&str>,
    addr_hint: Option<&str>,
) -> Result<AuthVerifyResponse> {
    let public_base_url = normalize_http_url(public_base_url)?;
    let envelope = parse_auth_envelope(auth_envelope_json)?;
    verify_auth_envelope(&envelope)?;
    if envelope.kind != AUTH_ENVELOPE_KIND {
        bail!("auth envelope kind mismatch");
    }
    let now = Utc::now();
    if (now.timestamp() - envelope.created_at).abs() > AUTH_EVENT_MAX_SKEW_SECONDS {
        bail!("auth envelope is stale");
    }
    let capability_url = first_tag_value(&envelope, "capability_url")
        .ok_or_else(|| anyhow!("missing capability_url tag"))?;
    if capability_url != public_base_url {
        bail!("capability_url tag mismatch");
    }
    let challenge =
        first_tag_value(&envelope, "challenge").ok_or_else(|| anyhow!("missing challenge tag"))?;
    let row = sqlx::query(
        "SELECT pubkey, expires_at, used_at
         FROM cn_auth.auth_challenges
         WHERE challenge = $1",
    )
    .bind(challenge)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        bail!("challenge not found");
    };
    let stored_pubkey: String = row.try_get("pubkey")?;
    let expires_at: DateTime<Utc> = row.try_get("expires_at")?;
    let used_at: Option<DateTime<Utc>> = row.try_get("used_at")?;
    if used_at.is_some() || Utc::now() > expires_at {
        bail!("challenge expired or already used");
    }
    let normalized_pubkey = normalize_pubkey(envelope.pubkey.as_str())?;
    if normalized_pubkey != stored_pubkey {
        bail!("pubkey mismatch");
    }
    let registered_endpoint = endpoint_id
        .map(|value| CommunityNodeSeedPeer::new(value, addr_hint.map(str::to_string)))
        .transpose()?;

    let mut tx = pool.begin().await?;
    prune_expired_bootstrap_peer_registrations(&mut *tx).await?;
    sqlx::query(
        "UPDATE cn_auth.auth_challenges
         SET used_at = NOW()
         WHERE challenge = $1",
    )
    .bind(challenge)
    .execute(&mut *tx)
    .await?;
    ensure_active_subscriber(&mut *tx, normalized_pubkey.as_str()).await?;
    if let Some(seed_peer) = registered_endpoint.as_ref() {
        upsert_bootstrap_peer_registration(&mut *tx, normalized_pubkey.as_str(), seed_peer, now)
            .await?;
    }
    tx.commit().await?;

    let (access_token, expires_at) = issue_access_token(
        jwt_config,
        normalized_pubkey.as_str(),
        registered_endpoint
            .as_ref()
            .map(|seed_peer| seed_peer.endpoint_id.as_str()),
    )?;
    Ok(AuthVerifyResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_at,
        pubkey: normalized_pubkey,
    })
}

pub async fn require_bearer_identity(
    pool: &PgPool,
    jwt_config: &JwtConfig,
    headers: &HeaderMap,
) -> ApiResult<BearerIdentity> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| auth_required_error("missing bearer token"))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| auth_required_error("invalid bearer token"))?;
    let claims = verify_access_token(jwt_config, token)
        .map_err(|error| auth_required_error(format!("invalid bearer token: {error}")))?;
    let pubkey = normalize_pubkey(claims.sub.as_str())
        .map_err(|error| auth_required_error(error.to_string()))?;
    let endpoint_id = claims
        .endpoint_id
        .as_deref()
        .map(|value| CommunityNodeSeedPeer::new(value, None))
        .transpose()
        .map_err(|error| auth_required_error(format!("invalid bearer token endpoint: {error}")))?
        .map(|seed_peer| seed_peer.endpoint_id);
    let active = sqlx::query_scalar::<_, String>(
        "SELECT status FROM cn_user.subscriber_accounts WHERE subscriber_pubkey = $1",
    )
    .bind(&pubkey)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            error.to_string(),
        )
    })?;
    match active.as_deref() {
        Some("active") => Ok(BearerIdentity {
            pubkey,
            endpoint_id,
        }),
        Some(_) => Err(auth_required_error("subscriber is not active")),
        None => Err(auth_required_error("subscriber is not registered")),
    }
}

pub async fn require_bearer_pubkey(
    pool: &PgPool,
    jwt_config: &JwtConfig,
    headers: &HeaderMap,
) -> ApiResult<String> {
    Ok(require_bearer_identity(pool, jwt_config, headers)
        .await?
        .pubkey)
}

pub fn build_auth_envelope_json(
    keys: &KukuriKeys,
    challenge: &str,
    public_base_url: &str,
) -> Result<Value> {
    let signed = sign_envelope_json(
        keys,
        AUTH_ENVELOPE_KIND,
        vec![
            vec!["challenge".into(), challenge.to_string()],
            vec![
                "capability_url".into(),
                normalize_http_url(public_base_url)?,
            ],
        ],
        &KukuriAuthEnvelopeContentV1 {
            scope: "community-node-auth".into(),
        },
    )?;
    serde_json::to_value(signed).context("failed to encode auth envelope json")
}

fn ensure_jwt_crypto_provider() {
    JWT_CRYPTO_PROVIDER_INIT.call_once(|| {
        let _ = jsonwebtoken::crypto::aws_lc::DEFAULT_PROVIDER.install_default();
    });
}

fn issue_access_token(
    jwt_config: &JwtConfig,
    pubkey: &str,
    endpoint_id: Option<&str>,
) -> Result<(String, i64)> {
    ensure_jwt_crypto_provider();
    let issued_at = Utc::now().timestamp();
    let expires_at = issued_at + jwt_config.ttl_seconds();
    let claims = AccessTokenClaims {
        sub: pubkey.to_string(),
        iss: jwt_config.issuer().to_string(),
        iat: issued_at as usize,
        exp: expires_at as usize,
        endpoint_id: endpoint_id.map(str::to_string),
    };
    let token = encode(&Header::default(), &claims, &jwt_config.encoding_key())?;
    Ok((token, expires_at))
}

fn verify_access_token(jwt_config: &JwtConfig, token: &str) -> Result<AccessTokenClaims> {
    ensure_jwt_crypto_provider();
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[jwt_config.issuer()]);
    let decoded = decode::<AccessTokenClaims>(token, &jwt_config.decoding_key(), &validation)?;
    Ok(decoded.claims)
}
