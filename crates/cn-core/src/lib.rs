use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;

use anyhow::{Context, Result, anyhow, bail};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header::AUTHORIZATION};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use nostr_sdk::prelude::Keys;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Executor, Row};
use url::Url;
use uuid::Uuid;

use kukuri_core::{Event as KukuriEnvelope, KukuriAuthEnvelopeContentV1, sign_envelope_json};

pub const AUTH_ENVELOPE_KIND: &str = "auth";
pub const AUTH_CHALLENGE_TTL_SECONDS: i64 = 300;
pub const AUTH_EVENT_MAX_SKEW_SECONDS: i64 = 600;
pub const DEFAULT_TOKEN_TTL_SECONDS: i64 = 86_400;
pub const RELAY_SERVICE_NAME: &str = "relay";
pub const USER_API_BEARER_CHALLENGE: &str = r#"Bearer realm="cn-user-api""#;
pub const COMMUNITY_NODE_DATABASE_INIT_MODE_ENV: &str = "COMMUNITY_NODE_DATABASE_INIT_MODE";
const DATABASE_PREPARE_HINT: &str =
    "run `cn-cli --database-url <url> prepare` before starting cn-user-api";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeResolvedUrls {
    pub public_base_url: String,
    pub connectivity_urls: Vec<String>,
}

impl CommunityNodeResolvedUrls {
    pub fn new(public_base_url: impl Into<String>, connectivity_urls: Vec<String>) -> Result<Self> {
        let public_base_url = normalize_http_url(public_base_url.into().as_str())?;
        let connectivity_urls = normalize_http_url_list(connectivity_urls)?;
        Ok(Self {
            public_base_url,
            connectivity_urls,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeBootstrapNode {
    pub base_url: String,
    pub resolved_urls: CommunityNodeResolvedUrls,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthChallengeResponse {
    pub challenge: String,
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthVerifyResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: i64,
    pub pubkey: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeConsentItem {
    pub policy_slug: String,
    pub policy_version: i32,
    pub title: String,
    pub required: bool,
    pub accepted_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeConsentStatus {
    pub all_required_accepted: bool,
    pub items: Vec<CommunityNodeConsentItem>,
}

#[derive(Clone, Debug)]
pub struct JwtConfig {
    issuer: String,
    secret: String,
    ttl_seconds: i64,
}

impl JwtConfig {
    pub fn new(issuer: impl Into<String>, secret: impl Into<String>, ttl_seconds: i64) -> Self {
        Self {
            issuer: issuer.into(),
            secret: secret.into(),
            ttl_seconds: ttl_seconds.max(60),
        }
    }

    pub fn from_env() -> Result<Self> {
        let issuer = std::env::var("COMMUNITY_NODE_JWT_ISSUER")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "kukuri-cn".to_string());
        let secret = std::env::var("COMMUNITY_NODE_JWT_SECRET")
            .context("COMMUNITY_NODE_JWT_SECRET is required")?;
        let ttl_seconds = std::env::var("COMMUNITY_NODE_JWT_TTL_SECONDS")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.parse::<i64>())
            .transpose()
            .context("failed to parse COMMUNITY_NODE_JWT_TTL_SECONDS")?
            .unwrap_or(DEFAULT_TOKEN_TTL_SECONDS);
        Ok(Self::new(issuer, secret, ttl_seconds))
    }

    fn encoding_key(&self) -> EncodingKey {
        EncodingKey::from_secret(self.secret.as_bytes())
    }

    fn decoding_key(&self) -> DecodingKey {
        DecodingKey::from_secret(self.secret.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseInitMode {
    RequireReady,
    Prepare,
}

impl DatabaseInitMode {
    pub fn from_env() -> Result<Self> {
        match std::env::var(COMMUNITY_NODE_DATABASE_INIT_MODE_ENV) {
            Ok(value) => Self::parse(value.as_str()),
            Err(std::env::VarError::NotPresent) => Ok(Self::RequireReady),
            Err(error) => Err(anyhow!("{COMMUNITY_NODE_DATABASE_INIT_MODE_ENV}: {error}")),
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value.trim() {
            "" | "require_ready" => Ok(Self::RequireReady),
            "prepare" => Ok(Self::Prepare),
            other => bail!(
                "unsupported {COMMUNITY_NODE_DATABASE_INIT_MODE_ENV} `{other}`: expected `require_ready` or `prepare`"
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Off,
    Required,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthRolloutConfig {
    pub mode: AuthMode,
    pub enforce_at: Option<i64>,
    pub grace_seconds: i64,
    pub ws_auth_timeout_seconds: i64,
}

impl Default for AuthRolloutConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::Off,
            enforce_at: None,
            grace_seconds: 900,
            ws_auth_timeout_seconds: 10,
        }
    }
}

impl AuthRolloutConfig {
    pub fn requires_auth(&self, now: i64) -> bool {
        match self.mode {
            AuthMode::Off => false,
            AuthMode::Required => self.enforce_at.map(|ts| now >= ts).unwrap_or(true),
        }
    }

    pub fn disconnect_deadline_for_connection(&self, connected_at: i64) -> Option<i64> {
        if self.mode != AuthMode::Required {
            return None;
        }
        let enforce_at = self.enforce_at?;
        if connected_at >= enforce_at {
            return None;
        }
        enforce_at.checked_add(self.grace_seconds.max(0))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    iss: String,
    iat: usize,
    exp: usize,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    headers: Vec<(HeaderName, HeaderValue)>,
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            headers: Vec::new(),
        }
    }

    pub fn with_header(
        mut self,
        name: impl Into<HeaderName>,
        value: impl TryInto<HeaderValue>,
    ) -> Self {
        if let Ok(value) = value.try_into() {
            self.headers.push((name.into(), value));
        }
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            axum::Json(json!({
                "code": self.code,
                "message": self.message,
            })),
        )
            .into_response();
        for (name, value) in self.headers {
            response.headers_mut().insert(name, value);
        }
        response
    }
}

pub async fn connect_postgres(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .with_context(|| "failed to connect to Postgres")
}

pub async fn migrate_postgres(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn initialize_database(pool: &PgPool) -> Result<()> {
    migrate_postgres(pool).await?;
    seed_default_policies(pool).await?;
    ensure_default_auth_rollout(pool).await?;
    Ok(())
}

pub async fn initialize_database_for_runtime(
    pool: &PgPool,
    init_mode: DatabaseInitMode,
) -> Result<()> {
    match init_mode {
        DatabaseInitMode::RequireReady => ensure_database_ready(pool).await,
        DatabaseInitMode::Prepare => initialize_database(pool).await,
    }
}

pub async fn ensure_database_ready(pool: &PgPool) -> Result<()> {
    for (schema, table) in [
        ("cn_auth", "auth_challenges"),
        ("cn_user", "subscriber_accounts"),
        ("cn_user", "policy_consents"),
        ("cn_admin", "policies"),
        ("cn_admin", "service_configs"),
        ("cn_bootstrap", "bootstrap_nodes"),
    ] {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1
                FROM information_schema.tables
                WHERE table_schema = $1
                  AND table_name = $2
            )",
        )
        .bind(schema)
        .bind(table)
        .fetch_one(pool)
        .await?;
        if !exists {
            bail!(
                "community-node database is not ready: missing `{schema}.{table}`; {DATABASE_PREPARE_HINT}"
            );
        }
    }

    let policy_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cn_admin.policies")
        .fetch_one(pool)
        .await?;
    if policy_count == 0 {
        bail!(
            "community-node database is not ready: required policy seed is missing; {DATABASE_PREPARE_HINT}"
        );
    }

    let rollout_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1
            FROM cn_admin.service_configs
            WHERE service_name = $1
        )",
    )
    .bind(RELAY_SERVICE_NAME)
    .fetch_one(pool)
    .await?;
    if !rollout_exists {
        bail!(
            "community-node database is not ready: relay auth rollout seed is missing; {DATABASE_PREPARE_HINT}"
        );
    }

    Ok(())
}

pub async fn seed_default_policies(pool: &PgPool) -> Result<()> {
    for (slug, title, body) in [
        (
            "terms_of_service",
            "Terms of Service",
            "You must follow the community node terms of service.",
        ),
        (
            "privacy_policy",
            "Privacy Policy",
            "You must acknowledge the community node privacy policy.",
        ),
    ] {
        sqlx::query(
            "INSERT INTO cn_admin.policies (policy_slug, policy_version, title, body_markdown, required)
             VALUES ($1, 1, $2, $3, TRUE)
             ON CONFLICT (policy_slug) DO UPDATE
             SET title = EXCLUDED.title,
                 body_markdown = EXCLUDED.body_markdown,
                 required = EXCLUDED.required,
                 updated_at = NOW()",
        )
        .bind(slug)
        .bind(title)
        .bind(body)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn ensure_default_auth_rollout(pool: &PgPool) -> Result<()> {
    let config_json = serde_json::to_value(AuthRolloutConfig::default())?;
    sqlx::query(
        "INSERT INTO cn_admin.service_configs (service_name, version, config_json)
         VALUES ($1, 1, $2)
         ON CONFLICT (service_name) DO NOTHING",
    )
    .bind(RELAY_SERVICE_NAME)
    .bind(config_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_auth_rollout(pool: &PgPool, service_name: &str) -> Result<AuthRolloutConfig> {
    let value = sqlx::query_scalar::<_, Value>(
        "SELECT config_json FROM cn_admin.service_configs WHERE service_name = $1",
    )
    .bind(service_name)
    .fetch_optional(pool)
    .await?;
    match value {
        Some(value) => Ok(serde_json::from_value(value).unwrap_or_default()),
        None => Ok(AuthRolloutConfig::default()),
    }
}

pub async fn store_auth_rollout(
    pool: &PgPool,
    service_name: &str,
    rollout: &AuthRolloutConfig,
) -> Result<()> {
    let config_json = serde_json::to_value(rollout)?;
    sqlx::query(
        "INSERT INTO cn_admin.service_configs (service_name, version, config_json)
         VALUES ($1, 1, $2)
         ON CONFLICT (service_name) DO UPDATE
         SET version = cn_admin.service_configs.version + 1,
             config_json = EXCLUDED.config_json,
             updated_at = NOW()",
    )
    .bind(service_name)
    .bind(config_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_auth_challenge(pool: &PgPool, pubkey: &str) -> Result<AuthChallengeResponse> {
    let pubkey = normalize_pubkey(pubkey)?;
    let challenge = Uuid::new_v4().to_string();
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
) -> Result<AuthVerifyResponse> {
    let public_base_url = normalize_http_url(public_base_url)?;
    let envelope = parse_auth_envelope(auth_envelope_json)?;
    verify_auth_envelope(&envelope)?;
    if envelope.kind != AUTH_ENVELOPE_KIND {
        bail!("auth envelope kind mismatch");
    }
    let now = Utc::now().timestamp();
    if (now - envelope.created_at).abs() > AUTH_EVENT_MAX_SKEW_SECONDS {
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

    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE cn_auth.auth_challenges
         SET used_at = NOW()
         WHERE challenge = $1",
    )
    .bind(challenge)
    .execute(&mut *tx)
    .await?;
    ensure_active_subscriber(&mut *tx, normalized_pubkey.as_str()).await?;
    tx.commit().await?;

    let (access_token, expires_at) = issue_access_token(jwt_config, normalized_pubkey.as_str())?;
    Ok(AuthVerifyResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_at,
        pubkey: normalized_pubkey,
    })
}

pub async fn require_bearer_pubkey(
    pool: &PgPool,
    jwt_config: &JwtConfig,
    headers: &HeaderMap,
) -> ApiResult<String> {
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
        Some("active") => Ok(pubkey),
        Some(_) => Err(auth_required_error("subscriber is not active")),
        None => Err(auth_required_error("subscriber is not registered")),
    }
}

pub async fn get_consent_status(pool: &PgPool, pubkey: &str) -> Result<CommunityNodeConsentStatus> {
    let pubkey = normalize_pubkey(pubkey)?;
    let rows = sqlx::query(
        "SELECT
            p.policy_slug,
            p.policy_version,
            p.title,
            p.required,
            c.accepted_at
         FROM cn_admin.policies p
         LEFT JOIN cn_user.policy_consents c
           ON c.policy_slug = p.policy_slug
          AND c.policy_version = p.policy_version
          AND c.subscriber_pubkey = $1
         ORDER BY p.policy_slug ASC",
    )
    .bind(&pubkey)
    .fetch_all(pool)
    .await?;
    let items = rows
        .into_iter()
        .map(|row| -> Result<CommunityNodeConsentItem> {
            let accepted_at = row
                .try_get::<Option<DateTime<Utc>>, _>("accepted_at")?
                .map(|value| value.timestamp());
            Ok(CommunityNodeConsentItem {
                policy_slug: row.try_get("policy_slug")?,
                policy_version: row.try_get("policy_version")?,
                title: row.try_get("title")?,
                required: row.try_get("required")?,
                accepted_at,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let all_required_accepted = items
        .iter()
        .filter(|item| item.required)
        .all(|item| item.accepted_at.is_some());
    Ok(CommunityNodeConsentStatus {
        all_required_accepted,
        items,
    })
}

pub async fn accept_consents(
    pool: &PgPool,
    pubkey: &str,
    policy_slugs: &[String],
) -> Result<CommunityNodeConsentStatus> {
    let pubkey = normalize_pubkey(pubkey)?;
    let desired = if policy_slugs.is_empty() {
        sqlx::query(
            "SELECT policy_slug, policy_version
             FROM cn_admin.policies
             WHERE required = TRUE",
        )
        .fetch_all(pool)
        .await?
    } else {
        let mut records = Vec::new();
        for slug in normalize_slug_list(policy_slugs) {
            let row = sqlx::query(
                "SELECT policy_slug, policy_version
                 FROM cn_admin.policies
                 WHERE policy_slug = $1",
            )
            .bind(&slug)
            .fetch_optional(pool)
            .await?;
            let Some(row) = row else {
                bail!("unknown policy slug `{slug}`");
            };
            records.push(row);
        }
        records
    };

    let mut tx = pool.begin().await?;
    ensure_active_subscriber(&mut *tx, pubkey.as_str()).await?;
    for row in desired {
        let slug: String = row.try_get("policy_slug")?;
        let version: i32 = row.try_get("policy_version")?;
        sqlx::query(
            "INSERT INTO cn_user.policy_consents
                (subscriber_pubkey, policy_slug, policy_version, accepted_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (subscriber_pubkey, policy_slug, policy_version) DO UPDATE
             SET accepted_at = EXCLUDED.accepted_at",
        )
        .bind(&pubkey)
        .bind(slug)
        .bind(version)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    get_consent_status(pool, pubkey.as_str()).await
}

pub fn consent_required_error(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, "CONSENT_REQUIRED", message)
}

pub async fn require_consents(
    pool: &PgPool,
    pubkey: &str,
) -> ApiResult<CommunityNodeConsentStatus> {
    let status = get_consent_status(pool, pubkey).await.map_err(|error| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            error.to_string(),
        )
    })?;
    if !status.all_required_accepted {
        return Err(consent_required_error(
            "required policies have not been accepted",
        ));
    }
    Ok(status)
}

pub async fn load_bootstrap_nodes(
    pool: &PgPool,
    self_node: Option<CommunityNodeBootstrapNode>,
) -> Result<Vec<CommunityNodeBootstrapNode>> {
    let rows = sqlx::query(
        "SELECT base_url, public_base_url, connectivity_urls
         FROM cn_bootstrap.bootstrap_nodes
         WHERE is_active = TRUE
         ORDER BY base_url ASC",
    )
    .fetch_all(pool)
    .await?;
    let mut nodes = BTreeMap::new();
    if let Some(node) = self_node {
        nodes.insert(node.base_url.clone(), node);
    }
    for row in rows {
        let base_url: String = row.try_get("base_url")?;
        let public_base_url: String = row.try_get("public_base_url")?;
        let connectivity_urls: Value = row.try_get("connectivity_urls")?;
        let connectivity_urls = serde_json::from_value::<Vec<String>>(connectivity_urls)?;
        let node = CommunityNodeBootstrapNode {
            base_url: normalize_http_url(base_url.as_str())?,
            resolved_urls: CommunityNodeResolvedUrls::new(public_base_url, connectivity_urls)?,
        };
        nodes.insert(node.base_url.clone(), node);
    }
    Ok(nodes.into_values().collect())
}

pub async fn upsert_bootstrap_node(pool: &PgPool, node: &CommunityNodeBootstrapNode) -> Result<()> {
    let relay_urls = serde_json::to_value(&node.resolved_urls.connectivity_urls)?;
    sqlx::query(
        "INSERT INTO cn_bootstrap.bootstrap_nodes
            (base_url, public_base_url, connectivity_urls, is_active)
         VALUES ($1, $2, $3, TRUE)
         ON CONFLICT (base_url) DO UPDATE
         SET public_base_url = EXCLUDED.public_base_url,
             connectivity_urls = EXCLUDED.connectivity_urls,
             is_active = TRUE,
             updated_at = NOW()",
    )
    .bind(&node.base_url)
    .bind(&node.resolved_urls.public_base_url)
    .bind(relay_urls)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn build_auth_envelope_json(
    keys: &Keys,
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

#[derive(Clone, Debug)]
pub struct TestDatabase {
    admin_database_url: String,
    pub database_name: String,
    pub database_url: String,
}

impl TestDatabase {
    pub async fn create(admin_database_url: &str, prefix: &str) -> Result<Self> {
        let sanitized_prefix = prefix
            .chars()
            .map(|ch| match ch {
                'a'..='z' | '0'..='9' => ch,
                'A'..='Z' => ch.to_ascii_lowercase(),
                _ => '_',
            })
            .collect::<String>();
        let sanitized_prefix = sanitized_prefix.trim_matches('_');
        let prefix = if sanitized_prefix.is_empty() {
            "cn_test"
        } else {
            sanitized_prefix
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let mut database_name = format!("{prefix}_{suffix}");
        database_name.truncate(63);

        let admin_pool = connect_postgres(admin_database_url).await?;
        let create_sql = format!("CREATE DATABASE \"{}\"", database_name.replace('"', "\"\""));
        admin_pool.execute(create_sql.as_str()).await?;

        let mut parsed =
            Url::parse(admin_database_url).context("failed to parse admin database url")?;
        parsed.set_path(format!("/{database_name}").as_str());
        Ok(Self {
            admin_database_url: admin_database_url.to_string(),
            database_name,
            database_url: parsed.to_string(),
        })
    }

    pub async fn cleanup(&self) -> Result<()> {
        let admin_pool = connect_postgres(self.admin_database_url.as_str()).await?;
        sqlx::query(
            "SELECT pg_terminate_backend(pid)
             FROM pg_stat_activity
             WHERE datname = $1
               AND pid <> pg_backend_pid()",
        )
        .bind(&self.database_name)
        .execute(&admin_pool)
        .await?;
        let drop_sql = format!(
            "DROP DATABASE IF EXISTS \"{}\"",
            self.database_name.replace('"', "\"\"")
        );
        admin_pool.execute(drop_sql.as_str()).await?;
        Ok(())
    }
}

pub fn normalize_http_url(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let parsed = Url::parse(trimmed).with_context(|| format!("invalid url `{trimmed}`"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => bail!("unsupported url scheme `{other}`"),
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        bail!("url must not contain query or fragment");
    }
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

pub fn normalize_ws_url(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let parsed = Url::parse(trimmed).with_context(|| format!("invalid ws url `{trimmed}`"))?;
    match parsed.scheme() {
        "ws" | "wss" => {}
        other => bail!("unsupported websocket url scheme `{other}`"),
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        bail!("websocket url must not contain query or fragment");
    }
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

pub fn normalize_http_url_list(values: Vec<String>) -> Result<Vec<String>> {
    let mut deduped = BTreeSet::new();
    for value in values {
        let normalized = normalize_http_url(value.as_str())?;
        deduped.insert(normalized);
    }
    Ok(deduped.into_iter().collect())
}

pub fn parse_auth_envelope(value: &Value) -> Result<KukuriEnvelope> {
    serde_json::from_value(value.clone()).context("invalid auth envelope json")
}

pub fn verify_auth_envelope(raw: &KukuriEnvelope) -> Result<()> {
    raw.verify()
        .context("auth envelope signature verification failed")?;
    Ok(())
}

pub fn first_tag_value<'a>(envelope: &'a KukuriEnvelope, name: &str) -> Option<&'a str> {
    envelope.tags.iter().find_map(|tag| {
        if tag.first().map(String::as_str) == Some(name) {
            tag.get(1).map(String::as_str)
        } else {
            None
        }
    })
}

pub fn normalize_pubkey(value: &str) -> Result<String> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.len() != 64 || !trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("invalid pubkey");
    }
    Ok(trimmed)
}

pub fn parse_socket_addr_env(var_name: &str, default: &str) -> Result<SocketAddr> {
    let value = std::env::var(var_name).unwrap_or_else(|_| default.to_string());
    value
        .parse::<SocketAddr>()
        .with_context(|| format!("failed to parse {var_name}"))
}

pub fn auth_required_error(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED", message).with_header(
        HeaderName::from_static("www-authenticate"),
        USER_API_BEARER_CHALLENGE,
    )
}

fn issue_access_token(jwt_config: &JwtConfig, pubkey: &str) -> Result<(String, i64)> {
    let issued_at = Utc::now().timestamp();
    let expires_at = issued_at + jwt_config.ttl_seconds;
    let claims = AccessTokenClaims {
        sub: pubkey.to_string(),
        iss: jwt_config.issuer.clone(),
        iat: issued_at as usize,
        exp: expires_at as usize,
    };
    let token = encode(&Header::default(), &claims, &jwt_config.encoding_key())?;
    Ok((token, expires_at))
}

fn verify_access_token(jwt_config: &JwtConfig, token: &str) -> Result<AccessTokenClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[jwt_config.issuer.as_str()]);
    let decoded = decode::<AccessTokenClaims>(token, &jwt_config.decoding_key(), &validation)?;
    Ok(decoded.claims)
}

async fn ensure_active_subscriber<'e, E>(executor: E, pubkey: &str) -> Result<()>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "INSERT INTO cn_user.subscriber_accounts
            (subscriber_pubkey, status, last_authenticated_at)
         VALUES ($1, 'active', NOW())
         ON CONFLICT (subscriber_pubkey) DO UPDATE
         SET status = 'active',
             last_authenticated_at = NOW()",
    )
    .bind(pubkey)
    .execute(executor)
    .await?;
    Ok(())
}

fn normalize_slug_list(values: &[String]) -> Vec<String> {
    let mut deduped = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            deduped.insert(trimmed.to_string());
        }
    }
    deduped.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_init_mode_defaults_to_require_ready() {
        let parsed = DatabaseInitMode::parse("").expect("parse");
        assert_eq!(parsed, DatabaseInitMode::RequireReady);
    }

    #[test]
    fn database_init_mode_accepts_prepare() {
        let parsed = DatabaseInitMode::parse("prepare").expect("parse");
        assert_eq!(parsed, DatabaseInitMode::Prepare);
    }

    #[test]
    fn auth_rollout_defaults_to_off() {
        let rollout = AuthRolloutConfig::default();
        assert!(!rollout.requires_auth(Utc::now().timestamp()));
    }

    #[test]
    fn http_url_normalization_trims_trailing_slash() {
        let normalized = normalize_http_url("https://example.com/").expect("normalize");
        assert_eq!(normalized, "https://example.com");
    }

    #[test]
    fn ws_url_normalization_trims_trailing_slash() {
        let normalized = normalize_ws_url("wss://example.com/relay/").expect("normalize");
        assert_eq!(normalized, "wss://example.com/relay");
    }
}
