use crate::*;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kukuri_cn_protocol::ApiErrorBody;
use kukuri_desktop_runtime::{
    CommunityNodeRelationNeighborsRequest, CommunityNodeTargetRequest,
    CommunityNodeUserAdvisoryRequest, SetCommunityNodeConfigNode,
};

const ACCESS_TOKEN: &str = "harness-trust-relation-token";

#[derive(Clone)]
struct ServerState {
    base_url: String,
    opted_out: Arc<AtomicBool>,
}

fn authenticated(headers: &HeaderMap) -> bool {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("Bearer {ACCESS_TOKEN}"))
}

fn require_auth(headers: &HeaderMap) -> Option<Response> {
    (!authenticated(headers)).then(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorBody {
                code: "AUTH_REQUIRED".to_string(),
                message: "authentication is required".to_string(),
            }),
        )
            .into_response()
    })
}

async fn auth_challenge() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "challenge": "harness-trust-relation-challenge",
        "expires_at": 4_102_444_800_i64
    }))
}

async fn auth_verify() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "access_token": ACCESS_TOKEN,
        "token_type": "Bearer",
        "expires_at": 4_102_444_800_i64,
        "pubkey": "harness-user"
    }))
}

async fn consent_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "all_required_accepted": true, "items": [] }))
}

async fn heartbeat() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "expires_at": 4_102_444_800_i64 }))
}

async fn bootstrap_nodes(State(state): State<ServerState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "nodes": [{
            "base_url": state.base_url.clone(),
            "resolved_urls": {
                "public_base_url": state.base_url,
                "connectivity_urls": [],
                "seed_peers": []
            }
        }]
    }))
}

fn api_error(code: &str, message: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiErrorBody {
            code: code.to_string(),
            message: message.to_string(),
        }),
    )
        .into_response()
}

async fn trust_user(AxumPath(target): AxumPath<String>, headers: HeaderMap) -> Response {
    if let Some(response) = require_auth(&headers) {
        return response;
    }
    if target.starts_with('c') {
        return api_error("TRUST_READ_NOT_CONFIGURED", "trust read is not configured");
    }
    if target.starts_with('d') {
        return api_error("TRUST_READ_NOT_ACTIVATED", "trust read is not activated");
    }
    Json(serde_json::json!({
        "viewer_pubkey": "harness-user",
        "target_id": target,
        "absolute": 0.5,
        "relative": 0.6,
        "trust": 0.55,
        "w_abs_applied": 0.5,
        "computed_at": "2026-08-14T00:00:00Z",
        "basis": []
    }))
    .into_response()
}

async fn relation_user(AxumPath(target): AxumPath<String>, headers: HeaderMap) -> Response {
    if let Some(response) = require_auth(&headers) {
        return response;
    }
    if target.starts_with('e') {
        return api_error("RELATION_NOT_FOUND", "relation is unavailable");
    }
    Json(serde_json::json!({
        "viewer_pubkey": "harness-user",
        "target_pubkey": target,
        "score": 0.42,
        "basis": [{"feature": "shared_topics", "value": 1.0, "weight": 0.42, "contribution": 0.42}]
    }))
    .into_response()
}

async fn relation_neighbors(
    headers: HeaderMap,
    Query(_query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    if let Some(response) = require_auth(&headers) {
        return response;
    }
    Json(serde_json::json!({
        "viewer_pubkey": "harness-user",
        "neighbors": ["bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"]
    }))
    .into_response()
}

fn optout_value(state: &ServerState) -> serde_json::Value {
    let enabled = state.opted_out.load(Ordering::SeqCst);
    serde_json::json!({
        "pubkey": "harness-user",
        "opted_out": enabled,
        "opted_out_at": enabled.then_some("2026-08-14T00:00:00Z"),
        "min_proximity": 0.25
    })
}

async fn get_optout(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Some(response) = require_auth(&headers) {
        return response;
    }
    Json(optout_value(&state)).into_response()
}

async fn set_optout(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Some(response) = require_auth(&headers) {
        return response;
    }
    state.opted_out.store(true, Ordering::SeqCst);
    Json(optout_value(&state)).into_response()
}

async fn clear_optout(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Some(response) = require_auth(&headers) {
        return response;
    }
    state.opted_out.store(false, Ordering::SeqCst);
    Json(optout_value(&state)).into_response()
}

pub(crate) async fn run_community_node_trust_relation_client(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let state = ServerState {
        base_url: base_url.clone(),
        opted_out: Arc::new(AtomicBool::new(false)),
    };
    let router = Router::new()
        .route("/v1/auth/challenge", post(auth_challenge))
        .route("/v1/auth/verify", post(auth_verify))
        .route("/v1/consents/status", get(consent_status))
        .route("/v1/bootstrap/heartbeat", post(heartbeat))
        .route("/v1/bootstrap/nodes", get(bootstrap_nodes))
        .route("/v1/trust/users/{target}", get(trust_user))
        .route("/v1/relation/users/{target}", get(relation_user))
        .route("/v1/relation/neighbors", get(relation_neighbors))
        .route(
            "/v1/relation/optout",
            get(get_optout).put(set_optout).delete(clear_optout),
        )
        .with_state(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await });

    let runtime_dir = tempfile::Builder::new()
        .prefix("trust-relation-client-")
        .tempdir_in(artifacts_dir)?;
    let db_path = runtime_dir.path().join("runtime.db");
    let runtime = DesktopRuntime::new(&db_path).await?;
    runtime
        .set_community_node_config(SetCommunityNodeConfigRequest {
            nodes: vec![SetCommunityNodeConfigNode {
                base_url: base_url.clone(),
                auto_approve: false,
            }],
        })
        .await?;
    runtime
        .authenticate_community_node(CommunityNodeTargetRequest {
            base_url: base_url.clone(),
        })
        .await?;

    let run_result = timeout(Duration::from_millis(scenario.timeouts.overall_ms), async {
        let mut steps = Vec::new();
        for step in &scenario.steps {
            let started_at = Instant::now();
            match step {
                ScenarioStep::ReadCommunityTrust {
                    target_pubkey,
                    expect_trust_millis,
                } => {
                    let value = runtime
                        .read_community_node_trust_user(CommunityNodeUserAdvisoryRequest {
                            base_url: base_url.clone(),
                            target_pubkey: target_pubkey.clone(),
                        })
                        .await?;
                    anyhow::ensure!(
                        (value.view.trust * 1000.0).round() as i64 == *expect_trust_millis,
                        "trust mismatch"
                    );
                }
                ScenarioStep::ReadCommunityRelation {
                    target_pubkey,
                    expect_score_millis,
                } => {
                    let value = runtime
                        .read_community_node_relation_user(CommunityNodeUserAdvisoryRequest {
                            base_url: base_url.clone(),
                            target_pubkey: target_pubkey.clone(),
                        })
                        .await?;
                    anyhow::ensure!(
                        (value.proximity.score * 1000.0).round() as i64 == *expect_score_millis,
                        "relation mismatch"
                    );
                }
                ScenarioStep::AssertCommunityRelationNeighbor { pubkey } => {
                    let value = runtime
                        .list_community_node_relation_neighbors(
                            CommunityNodeRelationNeighborsRequest {
                                base_url: base_url.clone(),
                                limit: Some(20),
                            },
                        )
                        .await?;
                    anyhow::ensure!(value.neighbors.contains(pubkey), "neighbor missing");
                }
                ScenarioStep::AssertRelationOptout {
                    operation,
                    expect_enabled,
                } => {
                    let request = CommunityNodeTargetRequest {
                        base_url: base_url.clone(),
                    };
                    let value = match operation.as_str() {
                        "get" => runtime.get_community_node_relation_optout(request).await?,
                        "set" => runtime.set_community_node_relation_optout(request).await?,
                        "clear" => {
                            runtime
                                .clear_community_node_relation_optout(request)
                                .await?
                        }
                        _ => anyhow::bail!("unsupported optout operation: {operation}"),
                    };
                    anyhow::ensure!(value.opted_out == *expect_enabled, "optout mismatch");
                }
                ScenarioStep::AssertTrustRelationError {
                    endpoint,
                    target_pubkey,
                    code,
                } => {
                    let request = CommunityNodeUserAdvisoryRequest {
                        base_url: base_url.clone(),
                        target_pubkey: target_pubkey.clone(),
                    };
                    let error = if endpoint == "trust" {
                        runtime
                            .read_community_node_trust_user(request)
                            .await
                            .expect_err("trust request should fail")
                    } else {
                        runtime
                            .read_community_node_relation_user(request)
                            .await
                            .expect_err("relation request should fail")
                    };
                    anyhow::ensure!(
                        error.code == *code,
                        "error code mismatch: expected {code}, got {}",
                        error.code
                    );
                }
                other => anyhow::bail!(
                    "unsupported trust relation client step: {}",
                    step_name(other)
                ),
            }
            push_named_step(&mut steps, step_name(step), started_at);
        }
        Ok::<_, anyhow::Error>(steps)
    })
    .await
    .context("community trust relation client scenario timed out")?;

    runtime.shutdown().await;
    server.abort();
    let result = HarnessResult {
        status: HarnessStatus::Pass,
        scenario: scenario.name.clone(),
        steps: run_result?,
        artifacts: vec![db_path.display().to_string()],
        metrics_snapshot: None,
    };
    write_result_artifact(root, artifacts_dir, &result)?;
    Ok(result)
}
