use super::super::*;
use axum::extract::Query;
use axum::http::{Uri, header::RETRY_AFTER};
use kukuri_cn_protocol::{
    ApiErrorBody, IndexEntryView, IndexQueryParams, IndexQueryResponse, IndexScopeKind,
    IndexingRequestStatus, SubmitIndexingRequestRequest, SubmitIndexingRequestResponse,
};

type ForcedIndexError = (StatusCode, ApiErrorBody, Option<&'static str>);

#[derive(Clone)]
struct MockIndexQueryState {
    expected_token: Arc<Mutex<String>>,
    requests: Arc<Mutex<Vec<(String, IndexQueryParams)>>>,
    indexing_requests: Arc<Mutex<Vec<SubmitIndexingRequestRequest>>>,
    forced_error: Arc<Mutex<Option<ForcedIndexError>>>,
    unauthorized_remaining: Arc<AtomicUsize>,
}

async fn mock_indexing_request(
    State(state): State<MockIndexQueryState>,
    headers: HeaderMap,
    Json(request): Json<SubmitIndexingRequestRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let expected = state.expected_token.lock().await.clone();
    let expected_header = format!("Bearer {expected}");
    if headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        != Some(expected_header.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorBody {
                code: "AUTH_REQUIRED".to_string(),
                message: "community node authentication is required".to_string(),
            }),
        )
            .into_response();
    }
    if let Some((status, body, retry_after)) = state.forced_error.lock().await.clone() {
        let mut response = (status, Json(body)).into_response();
        if let Some(value) = retry_after {
            response
                .headers_mut()
                .insert(RETRY_AFTER, value.parse().expect("retry-after"));
        }
        return response;
    }
    state.indexing_requests.lock().await.push(request);
    Json(SubmitIndexingRequestResponse {
        request_id: "request-1".to_string(),
        status: IndexingRequestStatus::Pending,
    })
    .into_response()
}

async fn mock_index_rendezvous(
    Json(_request): Json<serde_json::Value>,
) -> Json<kukuri_cn_protocol::TopicRendezvousHeartbeatResponse> {
    Json(kukuri_cn_protocol::TopicRendezvousHeartbeatResponse {
        expires_in_seconds: 45,
        topics: Vec::new(),
    })
}

async fn mock_index_query(
    State(state): State<MockIndexQueryState>,
    uri: Uri,
    headers: HeaderMap,
    Query(params): Query<IndexQueryParams>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let path = uri.path().to_string();
    state.requests.lock().await.push((path, params.clone()));
    let expected = state.expected_token.lock().await.clone();
    let expected_header = format!("Bearer {expected}");
    if headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        != Some(expected_header.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorBody {
                code: "AUTH_REQUIRED".to_string(),
                message: "community node authentication is required".to_string(),
            }),
        )
            .into_response();
    }
    if state
        .unauthorized_remaining
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
            remaining.checked_sub(1)
        })
        .is_ok()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorBody {
                code: "AUTH_REQUIRED".to_string(),
                message: "community node authentication is required".to_string(),
            }),
        )
            .into_response();
    }
    if let Some((status, body, retry_after)) = state.forced_error.lock().await.clone() {
        let mut response = (status, Json(body)).into_response();
        if let Some(value) = retry_after {
            response
                .headers_mut()
                .insert(RETRY_AFTER, value.parse().expect("retry-after"));
        }
        return response;
    }
    Json(IndexQueryResponse {
        entries: vec![IndexEntryView {
            scope_kind: IndexScopeKind::PublicTopic,
            scope_id: "rust".to_string(),
            object_id: "post-1".to_string(),
            author_pubkey: "author".to_string(),
            text: "hello\nderived-tag".to_string(),
            created_at: 42,
        }],
    })
    .into_response()
}

async fn index_runtime(
    forced_error: Option<ForcedIndexError>,
) -> (
    DesktopRuntime,
    String,
    Arc<MockManagedCommunityNodeState>,
    MockIndexQueryState,
    tokio::task::JoinHandle<()>,
    tempfile::TempDir,
) {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-index-query.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let expected_token = Arc::new(Mutex::new("index-token".to_string()));
    let managed = Arc::new(MockManagedCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Vec::new(),
        consent_accepted: Arc::new(AtomicBool::new(true)),
        current_token: Arc::clone(&expected_token),
        challenge_hits: Arc::new(AtomicUsize::new(0)),
        verify_hits: Arc::new(AtomicUsize::new(0)),
        consent_status_hits: Arc::new(AtomicUsize::new(0)),
        consent_accept_hits: Arc::new(AtomicUsize::new(0)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
        simulate_pending_update: Arc::new(AtomicBool::new(false)),
    });
    let index = MockIndexQueryState {
        expected_token,
        requests: Arc::new(Mutex::new(Vec::new())),
        indexing_requests: Arc::new(Mutex::new(Vec::new())),
        forced_error: Arc::new(Mutex::new(forced_error)),
        unauthorized_remaining: Arc::new(AtomicUsize::new(0)),
    };
    let managed_router = Router::new()
        .route("/v1/auth/challenge", post(mock_managed_auth_challenge))
        .route("/v1/auth/verify", post(mock_managed_auth_verify))
        .route("/v1/consents/status", get(mock_managed_consent_status))
        .route("/v1/consents", post(mock_managed_accept_consents))
        .route(
            "/v1/bootstrap/heartbeat",
            post(mock_managed_bootstrap_heartbeat),
        )
        .route("/v1/bootstrap/nodes", get(mock_managed_bootstrap_nodes))
        .with_state(Arc::clone(&managed));
    let index_router = Router::new()
        .route("/v1/index/search", get(mock_index_query))
        .route("/v1/index/discovery", get(mock_index_query))
        .route("/v1/index/recommendations", get(mock_index_query))
        .route("/v1/indexing/requests", post(mock_indexing_request))
        .route(
            "/v1/rendezvous/topics/heartbeat",
            post(mock_index_rendezvous),
        )
        .with_state(index.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, managed_router.merge(index_router))
            .await
            .expect("server");
    });
    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "index-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };
    (runtime, base_url, managed, index, server, dir)
}

fn scoped_request(base_url: &str) -> CommunityNodeIndexQueryRequest {
    CommunityNodeIndexQueryRequest {
        base_url: base_url.to_string(),
        query: Some("hello".to_string()),
        scope_kind: Some(IndexScopeKind::PublicTopic),
        scope_id: Some("rust".to_string()),
        limit: Some(10),
    }
}

#[tokio::test]
async fn community_node_index_client_uses_session_and_preserves_query_contract() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;

    let search = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search");
    assert_eq!(search.entries[0].object_id, "post-1");
    runtime
        .discover_community_node_index(CommunityNodeIndexQueryRequest {
            query: None,
            ..scoped_request(base_url.as_str())
        })
        .await
        .expect("discovery");
    runtime
        .recommend_community_node_index(CommunityNodeIndexQueryRequest {
            query: None,
            scope_kind: None,
            scope_id: None,
            ..scoped_request(base_url.as_str())
        })
        .await
        .expect("recommendations");

    let requests = state.requests.lock().await.clone();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].1.q.as_deref(), Some("hello"));
    assert_eq!(requests[0].1.scope_kind.as_deref(), Some("public_topic"));
    assert_eq!(requests[0].1.scope_id.as_deref(), Some("rust"));
    assert_eq!(requests[2].1.scope_kind, None);
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_index_client_preserves_stable_error_and_retry_after() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let body = ApiErrorBody {
        code: "INDEX_QUERY_NOT_ACTIVATED".to_string(),
        message: "this community node index activation is not current".to_string(),
    };
    let (runtime, base_url, _managed, _state, server, _dir) =
        index_runtime(Some((StatusCode::TOO_MANY_REQUESTS, body, Some("17")))).await;

    let error = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect_err("query should fail");
    assert_eq!(error.status, Some(429));
    assert_eq!(error.code, "INDEX_QUERY_NOT_ACTIVATED");
    assert_eq!(error.retry_after_seconds, Some(17));

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_index_client_reauthenticates_once_after_unauthorized() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, managed, state, server, _dir) = index_runtime(None).await;
    state.unauthorized_remaining.store(1, Ordering::SeqCst);

    let response = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search after reauthentication");

    assert_eq!(response.entries[0].object_id, "post-1");
    assert_eq!(managed.challenge_hits.load(Ordering::SeqCst), 1);
    assert_eq!(managed.verify_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.requests.lock().await.len(), 2);
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_index_client_rejects_half_scope_before_http() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    let error = runtime
        .search_community_node_index(CommunityNodeIndexQueryRequest {
            scope_id: None,
            ..scoped_request(base_url.as_str())
        })
        .await
        .expect_err("half scope should fail");
    assert_eq!(error.code, "INVALID_INDEX_QUERY");
    assert!(state.requests.lock().await.is_empty());
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_indexing_request_preserves_public_and_private_contracts() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    let topic = "kukuri:topic:indexing-request";

    let public_response = runtime
        .submit_community_node_indexing_request(CommunityNodeIndexingRequest {
            base_url: base_url.clone(),
            scope_kind: IndexScopeKind::PublicTopic,
            topic_id: topic.to_string(),
            channel_id: None,
            confirm_private_channel_secret_disclosure: false,
        })
        .await
        .expect("public indexing request");
    assert_eq!(public_response.status, IndexingRequestStatus::Pending);

    let channel = runtime
        .create_private_channel(CreatePrivateChannelRequest {
            topic: topic.to_string(),
            label: "indexable private channel".to_string(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");
    let private_response = runtime
        .submit_community_node_indexing_request(CommunityNodeIndexingRequest {
            base_url,
            scope_kind: IndexScopeKind::PrivateChannel,
            topic_id: topic.to_string(),
            channel_id: Some(channel.channel_id.clone()),
            confirm_private_channel_secret_disclosure: true,
        })
        .await
        .expect("private indexing request");
    assert_eq!(private_response.status, IndexingRequestStatus::Pending);

    let requests = state.indexing_requests.lock().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].kind, "public_topic");
    assert_eq!(requests[0].target_id, topic);
    assert_eq!(requests[0].channel_secret_hex, None);
    assert_eq!(requests[1].kind, "private_channel");
    assert_eq!(requests[1].target_id, channel.channel_id);
    assert!(
        requests[1]
            .channel_secret_hex
            .as_deref()
            .is_some_and(|secret| !secret.is_empty())
    );
    drop(requests);
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_private_indexing_requires_confirmation_before_http() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;

    let error = runtime
        .submit_community_node_indexing_request(CommunityNodeIndexingRequest {
            base_url,
            scope_kind: IndexScopeKind::PrivateChannel,
            topic_id: "kukuri:topic:indexing-request".to_string(),
            channel_id: Some("private-channel".to_string()),
            confirm_private_channel_secret_disclosure: false,
        })
        .await
        .expect_err("confirmation must be required");

    assert_eq!(
        error.code,
        "PRIVATE_CHANNEL_SECRET_DISCLOSURE_NOT_CONFIRMED"
    );
    assert!(state.indexing_requests.lock().await.is_empty());
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_indexing_request_preserves_stable_error() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let body = ApiErrorBody {
        code: "CHANNEL_SECRET_CONFLICT".to_string(),
        message: "the submitted channel secret conflicts with the current request".to_string(),
    };
    let (runtime, base_url, _managed, _state, server, _dir) =
        index_runtime(Some((StatusCode::CONFLICT, body, None))).await;

    let error = runtime
        .submit_community_node_indexing_request(CommunityNodeIndexingRequest {
            base_url,
            scope_kind: IndexScopeKind::PublicTopic,
            topic_id: "kukuri:topic:indexing-request".to_string(),
            channel_id: None,
            confirm_private_channel_secret_disclosure: false,
        })
        .await
        .expect_err("request should fail");
    assert_eq!(error.status, Some(409));
    assert_eq!(error.code, "CHANNEL_SECRET_CONFLICT");

    runtime.shutdown().await;
    server.abort();
}
