use crate::*;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, Uri, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use kukuri_cn_protocol::{
    ApiErrorBody, IndexEntryView, IndexQueryParams, IndexQueryResponse, IndexScopeKind,
};
use kukuri_desktop_runtime::{CommunityNodeIndexQueryRequest, SetCommunityNodeConfigNode};

const ACCESS_TOKEN: &str = "harness-index-token";

async fn auth_challenge() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "challenge": "harness-index-challenge",
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

async fn bootstrap_nodes(State(base_url): State<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "nodes": [{
            "base_url": base_url.clone(),
            "resolved_urls": {
                "public_base_url": base_url,
                "connectivity_urls": [],
                "seed_peers": []
            }
        }]
    }))
}

async fn index_query(
    uri: Uri,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> Response {
    let authenticated = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("Bearer {ACCESS_TOKEN}"));
    if !authenticated {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorBody {
                code: "AUTH_REQUIRED".to_string(),
                message: "authentication is required".to_string(),
            }),
        )
            .into_response();
    }
    if params.q.as_deref() == Some("unconfigured") {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiErrorBody {
                code: "INDEX_QUERY_NOT_CONFIGURED".to_string(),
                message: "index query is not configured".to_string(),
            }),
        )
            .into_response();
    }
    if params.q.as_deref() == Some("inactive") {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiErrorBody {
                code: "INDEX_QUERY_NOT_ACTIVATED".to_string(),
                message: "index query activation is not current".to_string(),
            }),
        )
            .into_response();
    }
    let operation = uri.path().rsplit('/').next().unwrap_or("search");
    let (scope_kind, scope_id) = match (params.scope_kind.as_deref(), params.scope_id) {
        (Some(kind), Some(scope_id)) => (
            IndexScopeKind::parse(kind).unwrap_or(IndexScopeKind::PublicTopic),
            scope_id,
        ),
        _ => (IndexScopeKind::PublicTopic, "cross-topic".to_string()),
    };
    Json(IndexQueryResponse {
        entries: vec![IndexEntryView {
            scope_kind,
            scope_id,
            object_id: format!("{operation}-object"),
            author_pubkey: "harness-author".to_string(),
            text: format!("{operation} preview\nderived-tag"),
            created_at: 42,
        }],
    })
    .into_response()
}

fn request(base_url: &str) -> CommunityNodeIndexQueryRequest {
    CommunityNodeIndexQueryRequest {
        base_url: base_url.to_string(),
        query: None,
        scope_kind: None,
        scope_id: None,
        limit: Some(20),
    }
}

pub(crate) async fn run_community_node_index_query_client(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base_url = format!("http://{}", listener.local_addr()?);
    let router = Router::new()
        .route("/v1/auth/challenge", post(auth_challenge))
        .route("/v1/auth/verify", post(auth_verify))
        .route("/v1/consents/status", get(consent_status))
        .route("/v1/bootstrap/heartbeat", post(heartbeat))
        .route("/v1/bootstrap/nodes", get(bootstrap_nodes))
        .route("/v1/index/search", get(index_query))
        .route("/v1/index/discovery", get(index_query))
        .route("/v1/index/recommendations", get(index_query))
        .with_state(base_url.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await });

    let runtime_dir = tempfile::Builder::new()
        .prefix("community-index-client-")
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

    let overall_timeout = Duration::from_millis(scenario.timeouts.overall_ms);
    let run_result = timeout(overall_timeout, async {
        let mut steps = Vec::new();
        for step in &scenario.steps {
            let started_at = Instant::now();
            match step {
                ScenarioStep::SearchCommunityIndex {
                    query,
                    scope_kind,
                    scope_id,
                    expect_object_id,
                } => {
                    let response = runtime
                        .search_community_node_index(CommunityNodeIndexQueryRequest {
                            query: Some(query.clone()),
                            scope_kind: Some(IndexScopeKind::parse(scope_kind)?),
                            scope_id: Some(scope_id.clone()),
                            ..request(base_url.as_str())
                        })
                        .await?;
                    anyhow::ensure!(
                        response
                            .entries
                            .first()
                            .map(|entry| entry.object_id.as_str())
                            == Some(expect_object_id.as_str()),
                        "search result mismatch: {:?}",
                        response.entries
                    );
                }
                ScenarioStep::DiscoverCommunityIndex { expect_object_id } => {
                    let response = runtime
                        .discover_community_node_index(request(base_url.as_str()))
                        .await?;
                    anyhow::ensure!(
                        response
                            .entries
                            .first()
                            .map(|entry| entry.object_id.as_str())
                            == Some(expect_object_id.as_str()),
                        "discovery result mismatch: {:?}",
                        response.entries
                    );
                }
                ScenarioStep::RecommendCommunityIndex { expect_object_id } => {
                    let response = runtime
                        .recommend_community_node_index(request(base_url.as_str()))
                        .await?;
                    anyhow::ensure!(
                        response
                            .entries
                            .first()
                            .map(|entry| entry.object_id.as_str())
                            == Some(expect_object_id.as_str()),
                        "recommendation result mismatch: {:?}",
                        response.entries
                    );
                }
                ScenarioStep::AssertCommunityIndexError { query, code } => {
                    let error = runtime
                        .search_community_node_index(CommunityNodeIndexQueryRequest {
                            query: Some(query.clone()),
                            scope_kind: Some(IndexScopeKind::PublicTopic),
                            scope_id: Some(scenario.fixtures.topic.clone()),
                            ..request(base_url.as_str())
                        })
                        .await
                        .expect_err("index query should fail");
                    anyhow::ensure!(
                        error.code == *code,
                        "index error mismatch: expected {code}, got {}",
                        error.code
                    );
                }
                other => anyhow::bail!(
                    "unsupported step for community index client scenario: {}",
                    step_name(other)
                ),
            }
            push_named_step(&mut steps, step_name(step), started_at);
        }
        Ok::<_, anyhow::Error>(steps)
    })
    .await
    .context("community index client scenario timed out")?;

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
