//! route 定義と起動(serve / tracing)。desktop 対向のパスは kukuri-cn-protocol の
//! 定数を参照する(パス変更が client / server 両側にコンパイルエラーとして届く)。

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use kukuri_cn_operator::CommunityNodeManifest;
use kukuri_cn_protocol::{
    AUTH_CHALLENGE_PATH, AUTH_VERIFY_PATH, BOOTSTRAP_HEARTBEAT_PATH, BOOTSTRAP_NODES_PATH,
    CONSENTS_PATH, CONSENTS_STATUS_PATH, NODE_MANIFEST_PATH, REPORT_PATH,
    TOPIC_RENDEZVOUS_HEARTBEAT_PATH,
};
use serde_json::{Value, json};
use tower_http::trace::TraceLayer;

use crate::config::{RateLimitConfig, UserApiConfig};
use crate::handlers::auth::{auth_challenge, auth_verify};
use crate::handlers::bootstrap::{
    bootstrap_heartbeat, bootstrap_nodes, topic_rendezvous_heartbeat,
};
use crate::handlers::consents::{accept_consents_handler, consent_status};
use crate::handlers::indexing::{
    index_discovery, index_recommendations, index_search, submit_indexing_request,
};
use crate::handlers::reports::submit_report;
use crate::handlers::trust_relation::{
    relation_neighbors, relation_optout_clear, relation_optout_set, relation_user_read, trust_pull,
    trust_user_read,
};
use crate::rate_limit::apply_rate_limit;
use crate::state::{ManifestState, UserApiState, build_runtime_state};

pub fn app_router(state: UserApiState) -> Router {
    let manifest = manifest_routes(
        state.manifest.clone(),
        Arc::clone(&state.public_disclosures),
    );
    let api = Router::new()
        .route("/healthz", get(healthz))
        .route(AUTH_CHALLENGE_PATH, post(auth_challenge))
        .route(AUTH_VERIFY_PATH, post(auth_verify))
        .route(CONSENTS_STATUS_PATH, get(consent_status))
        .route(CONSENTS_PATH, post(accept_consents_handler))
        .route(BOOTSTRAP_NODES_PATH, get(bootstrap_nodes))
        .route(BOOTSTRAP_HEARTBEAT_PATH, post(bootstrap_heartbeat))
        .route(
            TOPIC_RENDEZVOUS_HEARTBEAT_PATH,
            post(topic_rendezvous_heartbeat),
        )
        .route(REPORT_PATH, post(submit_report))
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
pub fn manifest_routes(
    manifest: Option<Arc<CommunityNodeManifest>>,
    public_disclosures: Arc<BTreeMap<String, String>>,
) -> Router {
    Router::new()
        .route(
            "/.well-known/kukuri/community-node.json",
            get(node_manifest),
        )
        .route(NODE_MANIFEST_PATH, get(node_manifest))
        .route("/terms", get(terms))
        .route("/privacy", get(privacy))
        .route("/external-transmission", get(external_transmission))
        .route("/moderation-policy", get(moderation_policy))
        .route("/abuse-policy", get(abuse_policy))
        .with_state(ManifestState {
            manifest,
            public_disclosures,
        })
}

async fn terms(State(state): State<ManifestState>) -> Response {
    disclosure_response(&state, "terms.md")
}

async fn privacy(State(state): State<ManifestState>) -> Response {
    disclosure_response(&state, "privacy-policy.md")
}

async fn external_transmission(State(state): State<ManifestState>) -> Response {
    disclosure_response(&state, "external-transmission-notice.md")
}

async fn moderation_policy(State(state): State<ManifestState>) -> Response {
    disclosure_response(&state, "moderation-policy.md")
}

async fn abuse_policy(State(state): State<ManifestState>) -> Response {
    disclosure_response(&state, "abuse-policy.md")
}

fn disclosure_response(state: &ManifestState, filename: &str) -> Response {
    let Some(content) = state.public_disclosures.get(filename) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "disclosure_not_configured",
                "message": "this community node does not publish this disclosure"
            })),
        )
            .into_response();
    };
    let mut response = content.clone().into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/markdown; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    response
}

/// public manifest を返す。設定されていなければ 404(client は別経路へ fallback)。
async fn node_manifest(State(state): State<ManifestState>) -> Response {
    match state.manifest {
        Some(manifest) => {
            let mut response = Json(manifest.as_ref()).into_response();
            // client が安全に cache できるようにする(private secret は含まれない)。
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

pub(crate) fn init_tracing() {
    kukuri_cn_runtime_support::init_tracing("info,kukuri_cn_user_api=debug");
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
