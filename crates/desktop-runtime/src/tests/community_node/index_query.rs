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
    channel_secret_headers: Arc<Mutex<Vec<Option<String>>>>,
    indexing_requests: Arc<Mutex<Vec<SubmitIndexingRequestRequest>>>,
    forced_error: Arc<Mutex<Option<ForcedIndexError>>>,
    unauthorized_remaining: Arc<AtomicUsize>,
    response_object_id: Arc<Mutex<String>>,
    response_author_pubkey: Arc<Mutex<String>>,
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
    state.channel_secret_headers.lock().await.push(
        headers
            .get("x-kukuri-channel-secret")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
    );
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
            object_id: state.response_object_id.lock().await.clone(),
            author_pubkey: state.response_author_pubkey.lock().await.clone(),
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
    let managed = Arc::new(MockManagedCommunityNodeState::new(
        base_url.clone(),
        Vec::new(),
        true,
        Arc::clone(&expected_token),
    ));
    let index = MockIndexQueryState {
        expected_token,
        requests: Arc::new(Mutex::new(Vec::new())),
        channel_secret_headers: Arc::new(Mutex::new(Vec::new())),
        indexing_requests: Arc::new(Mutex::new(Vec::new())),
        forced_error: Arc::new(Mutex::new(forced_error)),
        unauthorized_remaining: Arc::new(AtomicUsize::new(0)),
        response_object_id: Arc::new(Mutex::new("post-1".to_string())),
        response_author_pubkey: Arc::new(Mutex::new("author".to_string())),
    };
    let managed_router = Router::new()
        .route("/v1/auth/challenge", post(mock_managed_auth_challenge))
        .route("/v1/auth/verify", post(mock_managed_auth_verify))
        .route("/v1/policies", get(mock_managed_policies))
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
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };
    seed_local_community_node_consents(&runtime, base_url.as_str(), 1);
    (runtime, base_url, managed, index, server, dir)
}

fn scoped_request(base_url: &str) -> CommunityNodeIndexQueryRequest {
    CommunityNodeIndexQueryRequest {
        base_url: base_url.to_string(),
        query: Some("hello".to_string()),
        scope_kind: Some(IndexScopeKind::PublicTopic),
        scope_id: Some("rust".to_string()),
        topic_id: None,
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
async fn community_node_index_records_and_restores_existing_local_subjects() {
    use kukuri_store::ContentObservationStore;

    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, dir) = index_runtime(None).await;
    let db_path = dir.path().join("community-index-query.db");

    runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search without local subject");
    assert!(
        runtime
            .store
            .list_content_observations("post", "post-1")
            .await
            .expect("list absent observations")
            .is_empty()
    );

    runtime
        .set_my_profile(SetMyProfileRequest {
            name: Some("observed-author".to_string()),
            display_name: Some("Observed Author".to_string()),
            about: Some("profile retained across restart".to_string()),
            picture_upload: None,
            clear_picture: false,
        })
        .await
        .expect("set profile");

    let object_id = runtime
        .create_post(CreatePostRequest {
            topic: "kukuri:topic:rust".to_string(),
            content: "locally stored post".to_string(),
            reply_to: None,
            channel_ref: ChannelRef::Public,
            attachments: vec![image_attachment_request(
                "observed.png",
                "image/png",
                b"observed attachment",
            )],
            content_labels: Vec::new(),
        })
        .await
        .expect("create post");
    *state.response_object_id.lock().await = object_id.clone();
    *state.response_author_pubkey.lock().await = runtime.author_keys.public_key_hex();

    runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("search with local subject");
    let post_observations = runtime
        .store
        .list_content_observations("post", object_id.as_str())
        .await
        .expect("list post observations");
    assert_eq!(post_observations.len(), 1);
    assert_eq!(post_observations[0].node_base_url, base_url);
    assert_eq!(post_observations[0].capability, "community_index");

    let timeline = runtime
        .list_timeline(ListTimelineRequest {
            topic: "kukuri:topic:rust".to_string(),
            scope: Default::default(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("timeline");
    let observed_post = timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("observed post");
    assert_eq!(
        observed_post
            .provenance
            .as_ref()
            .expect("provenance")
            .observed_via[0]
            .node_base_url,
        post_observations[0].node_base_url
    );
    let attachment_provenance = observed_post.attachments[0]
        .provenance
        .as_ref()
        .expect("attachment provenance");
    assert_eq!(attachment_provenance.canonical_source, "blob");
    assert_eq!(
        attachment_provenance.observed_via,
        observed_post
            .provenance
            .as_ref()
            .expect("post provenance")
            .observed_via
    );
    let author_pubkey = runtime.author_keys.public_key_hex();
    let observed_author = runtime
        .get_author_social_view(AuthorRequest {
            pubkey: author_pubkey.clone(),
        })
        .await
        .expect("author view");
    assert_eq!(
        observed_author
            .provenance
            .as_ref()
            .expect("author provenance")
            .observed_via[0]
            .node_base_url,
        base_url
    );

    runtime.shutdown().await;
    let restored = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("restored runtime");
    let restored_timeline = restored
        .list_timeline(ListTimelineRequest {
            topic: "kukuri:topic:rust".to_string(),
            scope: Default::default(),
            cursor: None,
            limit: Some(20),
        })
        .await
        .expect("restored timeline");
    let restored_post = restored_timeline
        .items
        .iter()
        .find(|post| post.object_id == object_id)
        .expect("restored observed post");
    assert_eq!(
        restored_post.attachments[0]
            .provenance
            .as_ref()
            .expect("restored attachment provenance")
            .canonical_source,
        "blob"
    );
    let restored_author = restored
        .get_author_social_view(AuthorRequest {
            pubkey: author_pubkey.clone(),
        })
        .await
        .expect("restored author view");
    assert_eq!(
        restored_author
            .provenance
            .as_ref()
            .expect("restored author provenance")
            .observed_via[0]
            .node_base_url,
        base_url
    );
    restored.shutdown().await;
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

// #698: 必須同意が未承認のノードへは、検索語も非公開チャンネルの秘密値も送らない。
#[tokio::test]
async fn community_node_index_query_stops_before_http_when_consent_is_pending() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, managed, state, server, _dir) = index_runtime(None).await;
    managed.consent_accepted.store(false, Ordering::SeqCst);
    // #857: ローカル同意(v1)がカバーしない新版(v2)への更新 = 再同意待ち。
    managed
        .simulate_pending_update
        .store(true, Ordering::SeqCst);

    let error = runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect_err("pending consent must stop the query");

    assert_eq!(error.code, "CONSENT_REQUIRED");
    assert!(
        state.requests.lock().await.is_empty(),
        "no index request may reach the node while required consent is pending"
    );
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_private_indexing_stops_before_secret_when_consent_is_pending() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, managed, state, server, _dir) = index_runtime(None).await;
    managed.consent_accepted.store(false, Ordering::SeqCst);
    // #857: ローカル同意(v1)がカバーしない新版(v2)への更新 = 再同意待ち。
    managed
        .simulate_pending_update
        .store(true, Ordering::SeqCst);

    let error = runtime
        .submit_community_node_indexing_request(CommunityNodeIndexingRequest {
            base_url,
            scope_kind: IndexScopeKind::PrivateChannel,
            topic_id: "kukuri:topic:indexing-request".to_string(),
            channel_id: Some("private-channel".to_string()),
            confirm_private_channel_secret_disclosure: true,
        })
        .await
        .expect_err("pending consent must stop the request");

    // 秘密値の取得(PRIVATE_CHANNEL_CAPABILITY_UNAVAILABLE)より前に同意で止まる。
    assert_eq!(error.code, "CONSENT_REQUIRED");
    assert!(state.indexing_requests.lock().await.is_empty());
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn retrying_session_stops_indexing_request_before_http() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, managed, state, server, _dir) = index_runtime(None).await;
    runtime
        .set_community_node_retry_state(
            base_url.as_str(),
            anyhow::anyhow!("temporary session failure"),
        )
        .await;

    let error = runtime
        .submit_community_node_indexing_request(CommunityNodeIndexingRequest {
            base_url,
            scope_kind: IndexScopeKind::PublicTopic,
            topic_id: "kukuri:topic:indexing-request".to_string(),
            channel_id: None,
            confirm_private_channel_secret_disclosure: false,
        })
        .await
        .expect_err("retrying session must defer the indexing request");

    assert_eq!(error.code, "COMMUNITY_NODE_SESSION_DEFERRED");
    assert_eq!(managed.policies_hits.load(Ordering::SeqCst), 1);
    assert!(state.indexing_requests.lock().await.is_empty());
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn retrying_session_preflights_and_stops_index_query_before_http() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, managed, state, server, _dir) = index_runtime(None).await;
    runtime
        .set_community_node_retry_state(
            base_url.as_str(),
            anyhow::anyhow!("temporary session failure"),
        )
        .await;

    runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect_err("retrying session must defer the protected query");

    assert_eq!(managed.policies_hits.load(Ordering::SeqCst), 1);
    assert_eq!(managed.challenge_hits.load(Ordering::SeqCst), 0);
    assert_eq!(managed.verify_hits.load(Ordering::SeqCst), 0);
    assert_eq!(managed.consent_status_hits.load(Ordering::SeqCst), 0);
    assert!(state.requests.lock().await.is_empty());
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn awaiting_admission_manual_refresh_rechecks_policy_and_blocks_protected_http() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, managed, state, server, _dir) = index_runtime(None).await;
    managed
        .simulate_snapshot_update
        .store(true, Ordering::SeqCst);
    runtime
        .set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::AwaitingAdmission,
        )
        .await;
    crate::identity::persist_optional_secret(
        &runtime.db_path,
        runtime.identity_mode,
        crate::community_node::COMMUNITY_NODE_TOKEN_PURPOSE,
        base_url.as_str(),
        "invalid persisted token",
    )
    .expect("persist invalid token sentinel");

    let status = runtime
        .refresh_community_node_metadata(CommunityNodeTargetRequest {
            base_url: base_url.clone(),
        })
        .await
        .expect("manual refresh should return the consent-required status");
    assert!(status.consent_update_pending);
    assert_eq!(status.session_phase, CommunityNodeSessionPhase::Idle);

    runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect_err("policy update must stop an awaiting-admission query");

    assert_eq!(managed.policies_hits.load(Ordering::SeqCst), 2);
    assert_eq!(managed.challenge_hits.load(Ordering::SeqCst), 0);
    assert_eq!(managed.verify_hits.load(Ordering::SeqCst), 0);
    assert_eq!(managed.consent_status_hits.load(Ordering::SeqCst), 0);
    assert!(state.requests.lock().await.is_empty());
    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_private_scoped_query_attaches_membership_proof() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    let topic = "kukuri:topic:private-scope";
    let channel = runtime
        .create_private_channel(CreatePrivateChannelRequest {
            topic: topic.to_string(),
            label: "searchable private channel".to_string(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");

    // #711: topic_id なしの非公開チャンネル範囲指定は HTTP 前に拒否する。
    let error = runtime
        .search_community_node_index(CommunityNodeIndexQueryRequest {
            scope_kind: Some(IndexScopeKind::PrivateChannel),
            scope_id: Some(channel.channel_id.clone()),
            ..scoped_request(base_url.as_str())
        })
        .await
        .expect_err("missing topic_id should fail");
    assert_eq!(error.code, "INVALID_INDEX_QUERY");
    // 参加していないチャンネルは所属証明を引けないため HTTP 前に拒否する。
    let error = runtime
        .search_community_node_index(CommunityNodeIndexQueryRequest {
            scope_kind: Some(IndexScopeKind::PrivateChannel),
            scope_id: Some("not-joined-channel".to_string()),
            topic_id: Some(topic.to_string()),
            ..scoped_request(base_url.as_str())
        })
        .await
        .expect_err("not joined channel should fail");
    assert_eq!(error.code, "PRIVATE_CHANNEL_CAPABILITY_UNAVAILABLE");
    assert!(state.requests.lock().await.is_empty());

    // 参加中チャンネルの範囲指定は所属証明(channel secret)をヘッダで同伴する。
    runtime
        .search_community_node_index(CommunityNodeIndexQueryRequest {
            scope_kind: Some(IndexScopeKind::PrivateChannel),
            scope_id: Some(channel.channel_id.clone()),
            topic_id: Some(topic.to_string()),
            ..scoped_request(base_url.as_str())
        })
        .await
        .expect("private scoped search");
    let headers = state.channel_secret_headers.lock().await.clone();
    assert_eq!(headers.len(), 1);
    assert!(
        headers[0]
            .as_deref()
            .is_some_and(|secret| !secret.is_empty()),
        "所属証明ヘッダを同伴する"
    );
    let requests = state.requests.lock().await.clone();
    assert_eq!(requests[0].1.scope_kind.as_deref(), Some("private_channel"));

    // 公開範囲の検索には秘密値ヘッダを付けない。
    runtime
        .search_community_node_index(scoped_request(base_url.as_str()))
        .await
        .expect("public scoped search");
    let headers = state.channel_secret_headers.lock().await.clone();
    assert_eq!(headers.len(), 2);
    assert!(headers[1].is_none());
    runtime.shutdown().await;
    server.abort();
}
