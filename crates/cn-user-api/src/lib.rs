use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use kukuri_cn_core::PgIndexEntryStore;
use kukuri_cn_core::{
    AdmissionRejection, ApiError, ApiResult, AuthChallengeResponse, AuthVerifyResponse,
    BootstrapHeartbeatResponse, COMMUNITY_NODE_RENDEZVOUS_KEY_PREFIX_ENV,
    COMMUNITY_NODE_RENDEZVOUS_REDIS_URL_ENV, ChannelSecretCipher, ChannelSecretConflict,
    CommunityNodeBootstrapNode, CommunityNodeConsentStatus, CommunityNodeResolvedUrls,
    DatabaseInitMode, IndexScopeKind, JwtConfig, NewCommunityNodeReport, TopicRendezvousHeartbeat,
    TopicRendezvousHeartbeatResponse, TopicRendezvousStore, accept_consents, auth_required_error,
    clear_relation_optout, connect_postgres, create_auth_challenge, filter_relation_visible,
    get_consent_status, get_relation_optout, initialize_database, initialize_database_for_runtime,
    insert_community_node_report, insert_indexing_request, is_relation_opted_out,
    list_trust_risk_inputs, load_admission_config, load_bootstrap_nodes, load_bootstrap_seed_peers,
    normalize_http_url, normalize_http_url_list, normalize_pubkey, parse_bool_env, parse_csv_env,
    refresh_bootstrap_peer_registration, register_channel_secret, require_bearer_identity,
    require_bearer_pubkey, require_consents, set_relation_optout,
    verify_auth_envelope_and_issue_token,
};
use kukuri_cn_indexer::{
    ArcadeDbConfig, ArcadeDbProjection, ArcadeDbRelationGraph, FailClosedIndexQuery, IndexQuery,
};
use kukuri_cn_operator::{CommunityNodeManifest, build_manifest, load_and_validate};
use kukuri_cn_safety::RiskSignalTarget;
use kukuri_cn_trust::{
    PullAudience, RelationStore, TrustParams, TrustReadView, UniformRelationWeight,
    build_trust_read, cross_node_trust_disclosure,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::postgres::PgPool;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct UserApiState {
    pool: PgPool,
    rendezvous_store: TopicRendezvousStore,
    jwt_config: JwtConfig,
    self_node: CommunityNodeBootstrapNode,
    /// 公開する manifest（operator config が設定されている場合のみ）。
    manifest: Option<Arc<CommunityNodeManifest>>,
    /// private channel の indexing request で受け取る channel secret を at-rest 暗号化する cipher。
    /// 鍵 material（`COMMUNITY_NODE_CHANNEL_SECRET_KEY`）が未設定なら None で、private channel の
    /// indexing request は受け付けない（secret を平文保存しないため）。
    channel_secret_cipher: Option<Arc<ChannelSecretCipher>>,
    /// ユーザー向け search / discovery / recommendation の query 境界（#404）。
    /// fail-closed query gate（`FailClosedIndexQuery`）を通した読み口のみを持つ。
    /// None = 機能無効（`CommunityIndex` が `Availability::Planned` の既定状態）で、
    /// `/v1/index/*` は 404 を返す。
    index_query: Option<Arc<dyn IndexQuery>>,
    /// trust / relation read surface（#415 / ADR 0026）。
    /// None = 機能無効（`CommunityLocalTrust` が `Availability::Planned` の既定状態）で、
    /// `/v1/trust/*` / `/v1/relation/*` は 404 を返す。
    trust_read: Option<Arc<TrustReadState>>,
}

/// trust / relation read surface の依存一式（#415）。
///
/// trust の入力（risk signal）は Postgres（`UserApiState::pool`）から読み、relation は
/// graph backend（本番 = ArcadeDB、テスト = in-memory）から読む。
pub struct TrustReadState {
    /// trust 合成のパラメータ（operator 可変, ADR 0026 §6.2）。
    pub params: TrustParams,
    /// relation graph の読み口（graph-store 抽象境界, §6.1）。
    pub relation: Arc<dyn RelationStore>,
}

impl UserApiState {
    /// query 境界を差し替える（テスト用の in-memory 実装注入、または明示的な有効化）。
    ///
    /// 注入する実装は必ず fail-closed gate（`FailClosedIndexQuery`）を通したものにすること。
    pub fn with_index_query(mut self, index_query: Arc<dyn IndexQuery>) -> Self {
        self.index_query = Some(index_query);
        self
    }

    /// trust / relation read surface を差し替える（テスト用の in-memory relation 注入、
    /// または明示的な有効化）。
    pub fn with_trust_read(mut self, trust_read: Arc<TrustReadState>) -> Self {
        self.trust_read = Some(trust_read);
        self
    }
}

/// public manifest endpoint 用の最小 state。DB を必要としないため、
/// manifest 単独でテスト・配信できる。
#[derive(Clone)]
struct ManifestState {
    manifest: Option<Arc<CommunityNodeManifest>>,
}

#[derive(Clone)]
pub struct UserApiConfig {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub rendezvous_redis_url: String,
    pub rendezvous_key_prefix: String,
    pub base_url: String,
    pub public_base_url: String,
    pub connectivity_urls: Vec<String>,
    pub jwt_config: JwtConfig,
    /// 公開 manifest を生成する operator-config.yaml のパス。
    /// 未設定なら manifest endpoint は 404 を返す（client は別 node / 直接 P2P へ fallback）。
    pub operator_config_path: Option<PathBuf>,
    /// private channel の indexing request で渡される channel secret を at-rest 暗号化する鍵 material。
    /// 未設定なら private channel の indexing request は受け付けない（#413 / ADR 0025 §6.3）。
    pub channel_secret_key: Option<String>,
    /// ユーザー向け index query（search / discovery / recommendation）を公開するか（#404）。
    /// 既定 false（`CommunityIndex` が `Availability::Planned` の現状と整合。`/v1/index/*` は 404）。
    /// 有効化すると ArcadeDB（`COMMUNITY_NODE_ARCADEDB_*`）へ接続する。
    pub index_query_enabled: bool,
    /// trust / relation read surface を公開するか（#415）。
    /// 既定 false（`CommunityLocalTrust` が `Availability::Planned` の現状と整合。
    /// `/v1/trust/*` / `/v1/relation/*` は 404）。有効化すると relation graph の
    /// ArcadeDB（`COMMUNITY_NODE_ARCADEDB_*`）へ接続し、trust パラメータを
    /// `COMMUNITY_NODE_TRUST_*` env から読む。
    pub trust_read_enabled: bool,
}

impl std::fmt::Debug for UserApiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // channel_secret_key（at-rest 暗号鍵）を Debug 出力に含めない。
        f.debug_struct("UserApiConfig")
            .field("bind_addr", &self.bind_addr)
            .field("database_url", &self.database_url)
            .field("rendezvous_redis_url", &self.rendezvous_redis_url)
            .field("rendezvous_key_prefix", &self.rendezvous_key_prefix)
            .field("base_url", &self.base_url)
            .field("public_base_url", &self.public_base_url)
            .field("connectivity_urls", &self.connectivity_urls)
            .field("jwt_config", &self.jwt_config)
            .field("operator_config_path", &self.operator_config_path)
            .field(
                "channel_secret_key",
                &self.channel_secret_key.as_ref().map(|_| "<redacted>"),
            )
            .field("index_query_enabled", &self.index_query_enabled)
            .field("trust_read_enabled", &self.trust_read_enabled)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
struct AuthChallengeRequest {
    pubkey: String,
}

#[derive(Debug, Deserialize)]
struct AuthVerifyRequest {
    auth_envelope_json: Value,
    #[serde(default)]
    endpoint_id: Option<String>,
    #[serde(default)]
    addr_hint: Option<String>,
    /// invite mode の community node に参加するための招待コード（#383）。
    /// open / whitelist mode や既存 subscriber では不要。
    #[serde(default)]
    invite_code: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AcceptConsentsRequest {
    #[serde(default)]
    policy_slugs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BootstrapHeartbeatRequest {
    endpoint_id: String,
    #[serde(default)]
    addr_hint: Option<String>,
}

/// 通報受信リクエスト（#370）。client（#310）が provenance + manifest authority scope で
/// 通報先を解決し、この node の report endpoint へ POST する。
#[derive(Debug, Default, Deserialize)]
struct SubmitReportRequest {
    #[serde(default)]
    subject_kind: String,
    #[serde(default)]
    subject_id: String,
    #[serde(default)]
    capability: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    reporter_contact: Option<String>,
}

#[derive(Debug, Serialize)]
struct SubmitReportResponse {
    reference_id: String,
}

/// indexing request（#413 / ADR 0025 §2.2 / §6.3）。認証済み user が「この topic / channel を
/// index してほしい」と要求する。request は index を保証しない（operator 承認 + safety verdict の
/// 多段ゲート）。private channel は channel secret（capability）の提示が必須で、それ自体が権限の証明。
///
/// `Debug` は手動実装で `channel_secret_hex` の中身を秘匿する（誤ってログへ平文 secret を出さない）。
#[derive(Deserialize)]
struct SubmitIndexingRequestRequest {
    /// scope の種別（`public_topic` / `private_channel`）。
    #[serde(default)]
    kind: String,
    /// 対象識別子（topic_id / channel_id）。
    #[serde(default)]
    target_id: String,
    /// private channel の namespace secret hex（capability）。private_channel のときのみ必須。
    #[serde(default)]
    channel_secret_hex: Option<String>,
}

impl std::fmt::Debug for SubmitIndexingRequestRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmitIndexingRequestRequest")
            .field("kind", &self.kind)
            .field("target_id", &self.target_id)
            .field(
                "channel_secret_hex",
                &self.channel_secret_hex.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Debug, Serialize)]
struct SubmitIndexingRequestResponse {
    request_id: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct BootstrapNodesResponse {
    nodes: Vec<CommunityNodeBootstrapNode>,
}

impl UserApiConfig {
    pub fn from_env() -> Result<Self> {
        let bind_addr = std::env::var("COMMUNITY_NODE_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse::<SocketAddr>()
            .context("failed to parse COMMUNITY_NODE_BIND_ADDR")?;
        let database_url = std::env::var("COMMUNITY_NODE_DATABASE_URL")
            .context("COMMUNITY_NODE_DATABASE_URL is required")?;
        let rendezvous_redis_url = std::env::var(COMMUNITY_NODE_RENDEZVOUS_REDIS_URL_ENV)
            .with_context(|| format!("{COMMUNITY_NODE_RENDEZVOUS_REDIS_URL_ENV} is required"))?;
        let rendezvous_key_prefix = std::env::var(COMMUNITY_NODE_RENDEZVOUS_KEY_PREFIX_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "cn:rendezvous:v1".to_string());
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
        let connectivity_urls =
            normalize_http_url_list(parse_csv_env("COMMUNITY_NODE_CONNECTIVITY_URLS"))?;
        let operator_config_path = std::env::var("COMMUNITY_NODE_OPERATOR_CONFIG")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from);
        let channel_secret_key = std::env::var("COMMUNITY_NODE_CHANNEL_SECRET_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let index_query_enabled = parse_bool_env("COMMUNITY_NODE_INDEX_QUERY_ENABLED", false)?;
        let trust_read_enabled = parse_bool_env("COMMUNITY_NODE_TRUST_READ_ENABLED", false)?;
        Ok(Self {
            bind_addr,
            database_url,
            rendezvous_redis_url,
            rendezvous_key_prefix,
            base_url,
            public_base_url,
            connectivity_urls,
            jwt_config: JwtConfig::from_env()?,
            operator_config_path,
            channel_secret_key,
            index_query_enabled,
            trust_read_enabled,
        })
    }
}

/// Optional per-client rate limit for the public HTTP surface.
///
/// Disabled by default in code so unit/contract tests and library embeddings are
/// never throttled; the shipped `.env.community-node.example` turns it on. Behind a
/// trusted reverse proxy set `trust_forwarded_for` so each real client is limited
/// individually instead of sharing the proxy's connection IP. Leave it `false` when
/// the API is directly exposed, since `X-Forwarded-For` is attacker-controlled there.
#[derive(Clone, Copy, Debug)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub per_second: u64,
    pub burst: u32,
    pub trust_forwarded_for: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            per_second: 10,
            burst: 30,
            trust_forwarded_for: false,
        }
    }
}

impl RateLimitConfig {
    pub fn from_env() -> Result<Self> {
        let defaults = Self::default();
        Ok(Self {
            enabled: parse_bool_env("COMMUNITY_NODE_RATE_LIMIT_ENABLED", defaults.enabled)?,
            per_second: parse_u64_env("COMMUNITY_NODE_RATE_LIMIT_PER_SECOND", defaults.per_second)?
                .max(1),
            burst: parse_u32_env("COMMUNITY_NODE_RATE_LIMIT_BURST", defaults.burst)?.max(1),
            trust_forwarded_for: parse_bool_env(
                "COMMUNITY_NODE_RATE_LIMIT_TRUST_FORWARDED_FOR",
                defaults.trust_forwarded_for,
            )?,
        })
    }

    fn replenish_period_ms(&self) -> u64 {
        (1_000 / self.per_second.max(1)).max(1)
    }
}

/// Apply the rate limit layer to `router` when enabled. Layering returns a plain
/// `Router` regardless of the key-extractor type, so both branches unify cleanly.
/// A background task periodically drops idle per-IP buckets so a flood of distinct
/// source IPs cannot grow the limiter's state without bound; `retain_recent` resolves
/// through the `Arc`'s deref to the concrete governor limiter, so no governor-internal
/// type needs to be named here.
pub fn apply_rate_limit(router: Router, config: &RateLimitConfig) -> Result<Router> {
    if !config.enabled {
        return Ok(router);
    }
    let period_ms = config.replenish_period_ms();
    if config.trust_forwarded_for {
        let governor = GovernorConfigBuilder::default()
            .per_millisecond(period_ms)
            .burst_size(config.burst)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .context("failed to build rate limit configuration")?;
        let limiter = governor.limiter().clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                limiter.retain_recent();
            }
        });
        Ok(router.layer(GovernorLayer::new(governor)))
    } else {
        let governor = GovernorConfigBuilder::default()
            .per_millisecond(period_ms)
            .burst_size(config.burst)
            .finish()
            .context("failed to build rate limit configuration")?;
        let limiter = governor.limiter().clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                limiter.retain_recent();
            }
        });
        Ok(router.layer(GovernorLayer::new(governor)))
    }
}

pub async fn build_state(config: &UserApiConfig) -> Result<UserApiState> {
    let pool = connect_postgres(config.database_url.as_str()).await?;
    initialize_database(&pool).await?;
    build_state_from_pool(config, pool).await
}

async fn build_runtime_state(config: &UserApiConfig) -> Result<UserApiState> {
    let pool = connect_postgres(config.database_url.as_str()).await?;
    initialize_database_for_runtime(&pool, DatabaseInitMode::from_env()?).await?;
    build_state_from_pool(config, pool).await
}

async fn build_state_from_pool(config: &UserApiConfig, pool: PgPool) -> Result<UserApiState> {
    let rendezvous_store = TopicRendezvousStore::new(
        config.rendezvous_redis_url.as_str(),
        config.rendezvous_key_prefix.as_str(),
    )?;
    let manifest = load_manifest(config.operator_config_path.as_deref())?;
    let channel_secret_cipher = config
        .channel_secret_key
        .as_deref()
        .map(ChannelSecretCipher::from_key_material)
        .transpose()
        .context("invalid COMMUNITY_NODE_CHANNEL_SECRET_KEY")?
        .map(Arc::new);
    // ユーザー向け index query（#404）。有効時のみ ArcadeDB（投影）+ Postgres（真実源）を
    // fail-closed gate（`FailClosedIndexQuery`）で束ねる。読み口はこの gate 以外に作らない。
    let index_query: Option<Arc<dyn IndexQuery>> = if config.index_query_enabled {
        let projection = ArcadeDbProjection::new(ArcadeDbConfig::from_env())
            .context("failed to build ArcadeDB client for index query")?;
        let entries = PgIndexEntryStore::new(pool.clone());
        Some(Arc::new(FailClosedIndexQuery::new(
            Arc::new(projection),
            Arc::new(entries),
        )))
    } else {
        None
    };
    // trust / relation read surface（#415）。有効時のみ trust パラメータ（operator 可変）を
    // 検証つきで読み、relation graph（ArcadeDB。`cn-cli relation analyze` が構築する）へ接続する。
    let trust_read: Option<Arc<TrustReadState>> = if config.trust_read_enabled {
        let params = TrustParams::from_env().context("invalid COMMUNITY_NODE_TRUST_* params")?;
        let relation = ArcadeDbRelationGraph::new(ArcadeDbConfig::from_env())
            .context("failed to build ArcadeDB client for relation graph")?;
        Some(Arc::new(TrustReadState {
            params,
            relation: Arc::new(relation),
        }))
    } else {
        None
    };
    Ok(UserApiState {
        pool,
        rendezvous_store,
        jwt_config: config.jwt_config.clone(),
        self_node: CommunityNodeBootstrapNode {
            base_url: config.base_url.clone(),
            resolved_urls: CommunityNodeResolvedUrls::new(
                config.public_base_url.clone(),
                config.connectivity_urls.clone(),
                Vec::new(),
            )?,
        },
        manifest,
        channel_secret_cipher,
        index_query,
        trust_read,
    })
}

/// operator config から公開 manifest を構築する。
///
/// config が指定されているのに読込・検証に失敗した場合は起動を失敗させる
/// （運営者の設定ミスを黙って無視せず、明示的に止める）。
fn load_manifest(path: Option<&std::path::Path>) -> Result<Option<Arc<CommunityNodeManifest>>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let yaml = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read operator config at {}", path.display()))?;
    let resolved = load_and_validate(&yaml)
        .with_context(|| format!("invalid operator config at {}", path.display()))?;
    Ok(Some(Arc::new(build_manifest(&resolved))))
}

pub fn app_router(state: UserApiState) -> Router {
    let manifest = manifest_routes(state.manifest.clone());
    let api = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/auth/challenge", post(auth_challenge))
        .route("/v1/auth/verify", post(auth_verify))
        .route("/v1/consents/status", get(consent_status))
        .route("/v1/consents", post(accept_consents_handler))
        .route("/v1/bootstrap/nodes", get(bootstrap_nodes))
        .route("/v1/bootstrap/heartbeat", post(bootstrap_heartbeat))
        .route(
            "/v1/rendezvous/topics/heartbeat",
            post(topic_rendezvous_heartbeat),
        )
        .route("/v1/report", post(submit_report))
        .route("/v1/indexing/requests", post(submit_indexing_request))
        .route("/v1/index/search", get(index_search))
        .route("/v1/index/discovery", get(index_discovery))
        .route("/v1/index/recommendations", get(index_recommendations))
        .route("/v1/trust/users/{pubkey}", get(trust_user_read))
        .route("/v1/trust/pull/{pubkey}", get(trust_pull))
        .route("/v1/relation/users/{target}", get(relation_user_read))
        .route("/v1/relation/neighbors", get(relation_neighbors))
        .route(
            "/v1/relation/optout",
            put(relation_optout_set).delete(relation_optout_clear),
        )
        .with_state(state);
    api.merge(manifest).layer(TraceLayer::new_for_http())
}

/// 公開 manifest endpoint。unauthenticated で取得できる。
///
/// `GET /.well-known/kukuri/community-node.json` と `GET /v1/node/manifest` の
/// 両方を同じ handler で提供する。manifest 単独でテスト・配信できるよう、DB を
/// 必要としない最小 state を持つ独立 router にしている。
pub fn manifest_routes(manifest: Option<Arc<CommunityNodeManifest>>) -> Router {
    Router::new()
        .route(
            "/.well-known/kukuri/community-node.json",
            get(node_manifest),
        )
        .route("/v1/node/manifest", get(node_manifest))
        .with_state(ManifestState { manifest })
}

/// public manifest を返す。設定されていなければ 404（client は別経路へ fallback）。
async fn node_manifest(State(state): State<ManifestState>) -> Response {
    match state.manifest {
        Some(manifest) => {
            let mut response = Json(manifest.as_ref()).into_response();
            // client が安全に cache できるようにする（private secret は含まれない）。
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=300"),
            );
            response
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "manifest_not_configured",
                "message": "this community node does not publish a manifest"
            })),
        )
            .into_response(),
    }
}

pub async fn run_from_env() -> Result<()> {
    init_tracing();

    let config = UserApiConfig::from_env()?;
    let bind_addr = config.bind_addr;
    let rate_limit = RateLimitConfig::from_env()?;
    let state = build_runtime_state(&config).await?;
    let app = apply_rate_limit(app_router(state), &rate_limit)?;
    if rate_limit.enabled {
        tracing::info!(
            per_second = rate_limit.per_second,
            burst = rate_limit.burst,
            trust_forwarded_for = rate_limit.trust_forwarded_for,
            "community-node user-api rate limit enabled"
        );
    }
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

pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,kukuri_cn_user_api=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn auth_challenge(
    State(state): State<UserApiState>,
    Json(request): Json<AuthChallengeRequest>,
) -> ApiResult<Json<AuthChallengeResponse>> {
    let response = create_auth_challenge(&state.pool, request.pubkey.as_str())
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn auth_verify(
    State(state): State<UserApiState>,
    Json(request): Json<AuthVerifyRequest>,
) -> ApiResult<Json<AuthVerifyResponse>> {
    let admission_config = load_admission_config(&state.pool)
        .await
        .map_err(internal_error)?;
    let response = verify_auth_envelope_and_issue_token(
        &state.pool,
        &state.jwt_config,
        state.self_node.resolved_urls.public_base_url.as_str(),
        &request.auth_envelope_json,
        request.endpoint_id.as_deref(),
        request.addr_hint.as_deref(),
        admission_config.mode,
        request.invite_code.as_deref(),
    )
    .await
    .map_err(map_auth_verify_error)?;
    Ok(Json(response))
}

/// auth/verify の失敗を HTTP 応答へマップする。admission 拒否（#383）は 403 + 専用コードで
/// 区別し、それ以外の署名・challenge 検証失敗は従来通り 401 AUTH_FAILED にする。
fn map_auth_verify_error(error: anyhow::Error) -> ApiError {
    if let Some(rejection) = error.downcast_ref::<AdmissionRejection>() {
        return ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            rejection.code(),
            rejection.message(),
        );
    }
    ApiError::new(
        axum::http::StatusCode::UNAUTHORIZED,
        "AUTH_FAILED",
        error.to_string(),
    )
}

async fn consent_status(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<CommunityNodeConsentStatus>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let status = get_consent_status(&state.pool, pubkey.as_str())
        .await
        .map_err(internal_error)?;
    Ok(Json(status))
}

async fn accept_consents_handler(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Json(request): Json<AcceptConsentsRequest>,
) -> ApiResult<Json<CommunityNodeConsentStatus>> {
    let pubkey = require_bearer_pubkey(&state.pool, &state.jwt_config, &headers).await?;
    let status = accept_consents(&state.pool, pubkey.as_str(), &request.policy_slugs)
        .await
        .map_err(internal_error)?;
    Ok(Json(status))
}

/// 通報を受信して保存する（#370）。unauthenticated で受け付ける（匿名通報を許す）。
///
/// 受付可否は「この node が report_endpoint capability を有効化しているか」で判断する。これが
/// node の authority scope への opt-in であり、中央通報窓口を作らない。通報先の解決自体は client
/// （#310）が provenance + manifest authority scope で行っているため、ここへ届く時点で対象は
/// この node が関与した範囲に絞られている。reporter の identity / social graph は保持しない。
async fn submit_report(
    State(state): State<UserApiState>,
    Json(request): Json<SubmitReportRequest>,
) -> ApiResult<Json<SubmitReportResponse>> {
    // report endpoint capability が無効な node は通報を受け付けない。
    let report_enabled = state
        .manifest
        .as_ref()
        .map(|manifest| !manifest.report_endpoint.trim().is_empty())
        .unwrap_or(false);
    if !report_enabled {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "REPORT_NOT_CONFIGURED",
            "this community node does not accept reports",
        ));
    }

    let subject_kind = request.subject_kind.trim();
    let subject_id = request.subject_id.trim();
    let capability = request.capability.trim();
    let reason = request.reason.trim();
    if subject_kind.is_empty()
        || subject_id.is_empty()
        || capability.is_empty()
        || reason.is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REPORT",
            "subject_kind, subject_id, capability and reason are required",
        ));
    }

    let report = NewCommunityNodeReport {
        subject_kind: subject_kind.to_string(),
        subject_id: subject_id.to_string(),
        capability: capability.to_string(),
        reason: reason.to_string(),
        details: normalize_optional(request.details),
        reporter_contact: normalize_optional(request.reporter_contact),
    };
    let stored = insert_community_node_report(&state.pool, &report)
        .await
        .map_err(internal_error)?;
    Ok(Json(SubmitReportResponse {
        reference_id: stored.id,
    }))
}

/// 任意の文字列入力を正規化する。空白のみ / 空文字は None にする。
fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// user からの indexing request を受け付けて保存する（#413 / ADR 0025 §2.2 / §6.3）。
///
/// 認証済み（bearer）+ consent 済み user のみ要求できる。request は index を保証しない: operator が
/// supported set に入れ、さらに safety verdict が `allow` の content だけが index される多段ゲートの
/// 入口である。
///
/// - public topic: target_id（topic_id）を pending request として保存する。
/// - private channel: channel secret（capability）の提示が必須。secret を提示できること自体を channel
///   権限の証明とみなす（ADR 0025 §6.3。CN は新権限体系を作らない）。secret は at-rest 暗号化して保存し、
///   cn-indexer が Model C と同じ機構で `channel::` replica を sync する。channel secret 暗号鍵が未設定の
///   node は private channel request を受け付けない（平文保存しないため）。
async fn submit_indexing_request(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Json(request): Json<SubmitIndexingRequestRequest>,
) -> ApiResult<Json<SubmitIndexingRequestResponse>> {
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, &headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;

    let kind = match request.kind.trim() {
        "public_topic" => IndexScopeKind::PublicTopic,
        "private_channel" => IndexScopeKind::PrivateChannel,
        _ => {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_INDEXING_REQUEST",
                "kind must be `public_topic` or `private_channel`",
            ));
        }
    };
    let target_id = request.target_id.trim();
    if target_id.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INDEXING_REQUEST",
            "target_id is required",
        ));
    }

    // private channel は capability（secret）の提示が必須。これが権限の証明を兼ねる。
    if kind == IndexScopeKind::PrivateChannel {
        let secret_hex = request
            .channel_secret_hex
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let Some(secret_hex) = secret_hex else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "CHANNEL_SECRET_REQUIRED",
                "private channel indexing requires the channel secret",
            ));
        };
        // channel secret を平文保存しないため、暗号鍵未設定の node は受け付けない。
        let Some(cipher) = state.channel_secret_cipher.as_ref() else {
            return Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "CHANNEL_INDEXING_NOT_CONFIGURED",
                "this community node does not accept private channel indexing requests",
            ));
        };
        // first-writer-wins: 別 requester が別 secret で既存 capability を上書きできないようにする。
        // 同一 secret の再提示は冪等。別 secret による乗っ取りは 409 で拒否する。
        register_channel_secret(&state.pool, cipher, target_id, secret_hex)
            .await
            .map_err(map_channel_secret_error)?;
    }

    let stored = insert_indexing_request(&state.pool, identity.pubkey.as_str(), kind, target_id)
        .await
        .map_err(internal_error)?;
    Ok(Json(SubmitIndexingRequestResponse {
        request_id: stored.id,
        status: stored.status.as_str().to_string(),
    }))
}

/// index query の共通クエリパラメータ（#404）。
///
/// `scope_kind` + `scope_id` の組で topic 内（scope 内）読み、両方無指定で supported set 横断。
#[derive(Debug, Default, Deserialize)]
struct IndexQueryParams {
    /// 検索文字列（`/v1/index/search` のみ必須）。
    #[serde(default)]
    q: Option<String>,
    /// scope 種別（`public_topic` / `private_channel`）。
    #[serde(default)]
    scope_kind: Option<String>,
    /// scope 識別子（topic_id / channel_id）。
    #[serde(default)]
    scope_id: Option<String>,
    /// 返す entry 数の上限（`MAX_QUERY_LIMIT` に丸められる）。
    #[serde(default)]
    limit: Option<usize>,
}

/// index query の応答 entry。投影 entry のうちユーザー向けに公開するフィールドのみ
/// （`source_replica_id` は監査用の内部情報のため出さない）。
#[derive(Debug, Serialize)]
struct IndexEntryView {
    scope_kind: String,
    scope_id: String,
    object_id: String,
    author_pubkey: String,
    text: String,
    created_at: i64,
}

#[derive(Debug, Serialize)]
struct IndexQueryResponse {
    entries: Vec<IndexEntryView>,
}

impl From<Vec<kukuri_cn_indexer::IndexedEntry>> for IndexQueryResponse {
    fn from(entries: Vec<kukuri_cn_indexer::IndexedEntry>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|entry| IndexEntryView {
                    scope_kind: entry.scope_kind.as_str().to_string(),
                    scope_id: entry.scope_id,
                    object_id: entry.object_id,
                    author_pubkey: entry.author_pubkey,
                    text: entry.text,
                    created_at: entry.created_at,
                })
                .collect(),
        }
    }
}

/// index query 共通の前処理: 機能ゲート（未構成なら 404）+ 認証 + consent。
///
/// query 境界（`FailClosedIndexQuery`）を返す。`CommunityIndex` capability が
/// `Availability::Planned` の既定状態では index query は構成されず、この node は
/// search / discovery / recommendation を提供しない。
async fn require_index_query(
    state: &UserApiState,
    headers: &HeaderMap,
) -> ApiResult<Arc<dyn IndexQuery>> {
    let Some(index_query) = state.index_query.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "INDEX_QUERY_NOT_CONFIGURED",
            "this community node does not provide index queries",
        ));
    };
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    Ok(index_query)
}

/// `scope_kind` / `scope_id` パラメータの組を解釈する。
///
/// 両方指定 = scope 内読み、両方無指定 = 横断。片方のみは 400。
fn parse_index_scope_params(
    params: &IndexQueryParams,
) -> Result<Option<(IndexScopeKind, String)>, ApiError> {
    let scope_kind = params
        .scope_kind
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let scope_id = params
        .scope_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    match (scope_kind, scope_id) {
        (None, None) => Ok(None),
        (Some(kind), Some(id)) => {
            let kind = IndexScopeKind::parse(kind).map_err(|error| {
                ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_INDEX_QUERY",
                    error.to_string(),
                )
            })?;
            Ok(Some((kind, id.to_string())))
        }
        _ => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_INDEX_QUERY",
            "scope_kind and scope_id must be provided together",
        )),
    }
}

/// limit パラメータ（未指定は既定 20。gate 側で `MAX_QUERY_LIMIT` に丸められる）。
fn index_query_limit(params: &IndexQueryParams) -> usize {
    params.limit.unwrap_or(20)
}

/// ユーザー向け検索（#404 / ADR 0025 §2.7）。
///
/// `scope_kind` + `scope_id` 指定で topic 内検索（基本 UX）、無指定で supported set 横断検索
/// （別画面）。結果は fail-closed query gate を通った `allow` verdict の entry のみ。
async fn index_search(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> ApiResult<Json<IndexQueryResponse>> {
    let index_query = require_index_query(&state, &headers).await?;
    let query = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_INDEX_QUERY",
                "q is required",
            )
        })?;
    let limit = index_query_limit(&params);
    let entries = match parse_index_scope_params(&params)? {
        Some((scope_kind, scope_id)) => index_query
            .search_scope(scope_kind, scope_id.as_str(), query, limit)
            .await
            .map_err(internal_error)?,
        None => index_query
            .search_all(query, limit)
            .await
            .map_err(internal_error)?,
    };
    Ok(Json(entries.into()))
}

/// discovery（新着列挙。#404）。scope 指定で topic 内、無指定で supported set 横断。
///
/// ranking / 関連度スコアリングの具体は ADR 0025 §4 でスコープ外のため、最小 surface として
/// created_at 降順の新着を返す。critical / 非 allow verdict は fail-closed gate で入らない。
async fn index_discovery(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> ApiResult<Json<IndexQueryResponse>> {
    let index_query = require_index_query(&state, &headers).await?;
    let limit = index_query_limit(&params);
    let scope = parse_index_scope_params(&params)?;
    let entries = index_query
        .list_recent(scope.as_ref().map(|(kind, id)| (*kind, id.as_str())), limit)
        .await
        .map_err(internal_error)?;
    Ok(Json(entries.into()))
}

/// recommendation（#404）。supported set 横断の新着列挙を最小 surface として返す。
///
/// ranking アルゴリズムの具体は ADR 0025 §4 でスコープ外。critical verdict が recommendation に
/// 入らないことは fail-closed gate（真実源 + 最新 verdict 突合）が保証する。
async fn index_recommendations(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> ApiResult<Json<IndexQueryResponse>> {
    let index_query = require_index_query(&state, &headers).await?;
    let limit = index_query_limit(&params);
    let entries = index_query
        .list_recent(None, limit)
        .await
        .map_err(internal_error)?;
    Ok(Json(entries.into()))
}

// --- trust / relation read surface（#415 / ADR 0026） ---

/// trust / relation read 共通の前処理: 機能ゲート（未構成なら 404）+ 認証 + consent。
///
/// `CommunityLocalTrust` capability が `Availability::Planned` の既定状態では構成されず、
/// この node は trust / relation read を提供しない。認証は challenge への鍵署名を検証して
/// 発行された bearer（`BearerIdentity`）であり、**viewer = bearer の pubkey に固定**される
/// （`viewer_relative_read_requires_authenticated_viewer` / `relation_read_requires_authenticated_viewer`。
/// 他人を viewer に指定する手段を持たない = なりすまし防止）。
async fn require_trust_read(
    state: &UserApiState,
    headers: &HeaderMap,
) -> ApiResult<(Arc<TrustReadState>, String)> {
    let Some(trust_read) = state.trust_read.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "TRUST_READ_NOT_CONFIGURED",
            "this community node does not provide trust / relation reads",
        ));
    };
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    Ok((trust_read, identity.pubkey))
}

fn parse_target_pubkey(raw: &str) -> Result<String, ApiError> {
    normalize_pubkey(raw).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_TRUST_QUERY",
            error.to_string(),
        )
    })
}

/// trust read の応答（node-local advisory。断定ラベルなし・根拠つき）。
#[derive(Debug, Serialize)]
struct TrustUserReadResponse {
    /// この read の viewer（bearer identity の pubkey。相対成分の視点）。
    viewer_pubkey: String,
    #[serde(flatten)]
    view: TrustReadView,
}

/// per-user trust read（ADR 0026 §2.3 / §6.2）。
///
/// 絶対成分（viewer 非依存・relation 非依存・減衰なし）+ 相対成分（viewer / cluster 相対・
/// relation 重み付け・半減期減衰）+ 合成 trust を、寄与 signal の根拠つきで返す。
/// 相対成分の relation 重みは observer-attributed 観測の producer（非決定論的 moderation,
/// ADR 0028 系）実装まで一様 1.0（重み付けの seam は scoring 層で固定済み）。
async fn trust_user_read(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Path(pubkey): Path<String>,
) -> ApiResult<Json<TrustUserReadResponse>> {
    let (trust_read, viewer_pubkey) = require_trust_read(&state, &headers).await?;
    let target = parse_target_pubkey(pubkey.as_str())?;
    let now = chrono::Utc::now();
    let inputs = list_trust_risk_inputs(
        &state.pool,
        RiskSignalTarget::UserPubkey,
        target.as_str(),
        now.to_rfc3339().as_str(),
    )
    .await
    .map_err(internal_error)?;
    let view = build_trust_read(
        target.as_str(),
        &inputs,
        now,
        &trust_read.params,
        &UniformRelationWeight::default(),
    );
    Ok(Json(TrustUserReadResponse {
        viewer_pubkey,
        view,
    }))
}

/// cross-node pull（ADR 0026 §6.3）。
///
/// **confirmed（known-hash / provider-verdict）な絶対成分のみ**を根拠つきで返す。
/// 相対成分・relation・suspected は visibility に依らず返さない。`visibility` は
/// アクセス範囲: `Local` は返さず（既定）、`SubscribedNodes` は bearer で subscriber と
/// 認証できた要求者のみ、`Public` は匿名でも返す。絶対成分は viewer 非依存のため
/// viewer 証明は要さない（bearer は audience 判定のみに使う）。
async fn trust_pull(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Path(pubkey): Path<String>,
) -> ApiResult<Json<kukuri_cn_trust::CrossNodeTrustDisclosure>> {
    let Some(_) = state.trust_read.clone() else {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "TRUST_READ_NOT_CONFIGURED",
            "this community node does not provide trust / relation reads",
        ));
    };
    // bearer が有効な subscriber なら SubscribedNodes、無ければ匿名（Public visibility のみ）。
    let audience = match require_bearer_identity(&state.pool, &state.jwt_config, &headers).await {
        Ok(_) => PullAudience::SubscribedNodes,
        Err(_) => PullAudience::Public,
    };
    let target = parse_target_pubkey(pubkey.as_str())?;
    let now = chrono::Utc::now();
    let inputs = list_trust_risk_inputs(
        &state.pool,
        RiskSignalTarget::UserPubkey,
        target.as_str(),
        now.to_rfc3339().as_str(),
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(cross_node_trust_disclosure(
        target.as_str(),
        &inputs,
        audience,
        now,
    )))
}

/// relation read の応答（pairwise cluster proximity。根拠つき）。
#[derive(Debug, Serialize)]
struct RelationReadResponse {
    viewer_pubkey: String,
    target_pubkey: String,
    #[serde(flatten)]
    proximity: kukuri_cn_trust::Proximity,
}

/// pairwise relation read（ADR 0026 §2.4）。viewer = bearer identity。
///
/// target が opt-out（「見えない」）している場合は edge が無い場合と同じ 404 を返し、
/// opt-out 状態そのものを漏らさない。relation は情報として返すのみで、index / search /
/// discovery の結果集合をこの値で削らない（`relation_does_not_auto_suppress_cross_cluster_content`）。
async fn relation_user_read(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Path(target): Path<String>,
) -> ApiResult<Json<RelationReadResponse>> {
    let (trust_read, viewer_pubkey) = require_trust_read(&state, &headers).await?;
    let target = parse_target_pubkey(target.as_str())?;
    let not_found = || {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "RELATION_NOT_FOUND",
            "no relation observed for this pair",
        )
    };
    // 「見えない」opt-out: 他者から見た relation read に出さない（§6.3。可逆・trust 非影響）。
    if is_relation_opted_out(&state.pool, target.as_str())
        .await
        .map_err(internal_error)?
    {
        return Err(not_found());
    }
    let proximity = trust_read
        .relation
        .pairwise_proximity(viewer_pubkey.as_str(), target.as_str())
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found)?;
    Ok(Json(RelationReadResponse {
        viewer_pubkey,
        target_pubkey: target,
        proximity,
    }))
}

#[derive(Debug, Default, Deserialize)]
struct RelationNeighborsParams {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct RelationNeighborsResponse {
    viewer_pubkey: String,
    neighbors: Vec<String>,
}

/// discovery / surfacing 用の近接近傍（ADR 0026 §6.1 `neighbors`）。viewer = bearer identity。
///
/// opt-out 済み user は結果から除外する（「見えない」= discovery に出ない, §6.3）。
async fn relation_neighbors(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Query(params): Query<RelationNeighborsParams>,
) -> ApiResult<Json<RelationNeighborsResponse>> {
    let (trust_read, viewer_pubkey) = require_trust_read(&state, &headers).await?;
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let neighbors = trust_read
        .relation
        .neighbors(viewer_pubkey.as_str(), limit)
        .await
        .map_err(internal_error)?;
    let neighbors = filter_relation_visible(&state.pool, neighbors.as_slice())
        .await
        .map_err(internal_error)?;
    Ok(Json(RelationNeighborsResponse {
        viewer_pubkey,
        neighbors,
    }))
}

#[derive(Debug, Serialize)]
struct RelationOptoutResponse {
    pubkey: String,
    opted_out: bool,
    /// opt-out した時刻（説明可能性。解除済みなら null）。
    opted_out_at: Option<String>,
}

/// 「見えない」opt-out の設定（ADR 0026 §2.6 / §6.3）。
///
/// **自分自身のみ**設定できる（bearer identity に固定）。可逆（DELETE で解除）で、
/// trust には影響しない（troll 判定回避の手段にしない）。social graph canonical の削除でもない。
async fn relation_optout_set(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<RelationOptoutResponse>> {
    let (_, pubkey) = require_trust_read(&state, &headers).await?;
    set_relation_optout(&state.pool, pubkey.as_str())
        .await
        .map_err(internal_error)?;
    let opted_out_at = get_relation_optout(&state.pool, pubkey.as_str())
        .await
        .map_err(internal_error)?;
    Ok(Json(RelationOptoutResponse {
        pubkey,
        opted_out: true,
        opted_out_at: opted_out_at.map(|at| at.to_rfc3339()),
    }))
}

/// 「見えない」opt-out の解除（可逆性の実装）。
async fn relation_optout_clear(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<RelationOptoutResponse>> {
    let (_, pubkey) = require_trust_read(&state, &headers).await?;
    clear_relation_optout(&state.pool, pubkey.as_str())
        .await
        .map_err(internal_error)?;
    Ok(Json(RelationOptoutResponse {
        pubkey,
        opted_out: false,
        opted_out_at: None,
    }))
}

/// channel secret 登録失敗を HTTP 応答へマップする。
///
/// 既存 capability と異なる secret での上書き（乗っ取り試行）は 409、hex 形式不正等は 400。
fn map_channel_secret_error(error: anyhow::Error) -> ApiError {
    if error.downcast_ref::<ChannelSecretConflict>().is_some() {
        return ApiError::new(
            StatusCode::CONFLICT,
            "CHANNEL_SECRET_CONFLICT",
            error.to_string(),
        );
    }
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "INVALID_CHANNEL_SECRET",
        error.to_string(),
    )
}

async fn bootstrap_nodes(
    State(state): State<UserApiState>,
    headers: HeaderMap,
) -> ApiResult<Json<BootstrapNodesResponse>> {
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, &headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    let mut nodes = load_bootstrap_nodes(&state.pool, Some(state.self_node.clone()))
        .await
        .map_err(internal_error)?;
    let seed_peers = load_bootstrap_seed_peers(
        &state.pool,
        Some(identity.pubkey.as_str()),
        identity.endpoint_id.as_deref(),
    )
    .await
    .map_err(internal_error)?;
    for node in &mut nodes {
        if node.base_url == state.self_node.base_url {
            node.resolved_urls.seed_peers = seed_peers.clone();
        }
    }
    Ok(Json(BootstrapNodesResponse { nodes }))
}

async fn bootstrap_heartbeat(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Json(request): Json<BootstrapHeartbeatRequest>,
) -> ApiResult<Json<BootstrapHeartbeatResponse>> {
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, &headers).await?;
    if let Some(bound_endpoint_id) = identity.endpoint_id.as_deref()
        && bound_endpoint_id != request.endpoint_id
    {
        return Err(auth_required_error("bearer token endpoint mismatch"));
    }
    let response = refresh_bootstrap_peer_registration(
        &state.pool,
        identity.pubkey.as_str(),
        request.endpoint_id.as_str(),
        request.addr_hint.as_deref(),
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(response))
}

async fn topic_rendezvous_heartbeat(
    State(state): State<UserApiState>,
    headers: HeaderMap,
    Json(request): Json<TopicRendezvousHeartbeat>,
) -> ApiResult<Json<TopicRendezvousHeartbeatResponse>> {
    let identity = require_bearer_identity(&state.pool, &state.jwt_config, &headers).await?;
    let _ = require_consents(&state.pool, identity.pubkey.as_str()).await?;
    if let Some(bound_endpoint_id) = identity.endpoint_id.as_deref()
        && bound_endpoint_id != request.endpoint_id
    {
        return Err(auth_required_error("bearer token endpoint mismatch"));
    }
    let response = state
        .rendezvous_store
        .heartbeat(
            request,
            state.self_node.resolved_urls.connectivity_urls.as_slice(),
        )
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        error.to_string(),
    )
}

fn parse_u64_env(var_name: &str, default: u64) -> Result<u64> {
    match std::env::var(var_name) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<u64>()
            .with_context(|| format!("failed to parse {var_name}")),
        _ => Ok(default),
    }
}

fn parse_u32_env(var_name: &str, default: u32) -> Result<u32> {
    match std::env::var(var_name) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<u32>()
            .with_context(|| format!("failed to parse {var_name}")),
        _ => Ok(default),
    }
}
