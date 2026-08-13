use kukuri_cn_protocol::{
    ApiErrorBody, INDEX_DISCOVERY_PATH, INDEX_RECOMMENDATIONS_PATH, INDEX_SEARCH_PATH,
    INDEXING_REQUESTS_PATH, IndexEntryView, IndexQueryParams, IndexQueryResponse, IndexScopeKind,
    IndexingRequestStatus, SubmitIndexingRequestRequest, SubmitIndexingRequestResponse,
};

#[test]
fn index_paths_are_stable() {
    assert_eq!(INDEX_SEARCH_PATH, "/v1/index/search");
    assert_eq!(INDEX_DISCOVERY_PATH, "/v1/index/discovery");
    assert_eq!(INDEX_RECOMMENDATIONS_PATH, "/v1/index/recommendations");
    assert_eq!(INDEXING_REQUESTS_PATH, "/v1/indexing/requests");
}

#[test]
fn indexing_request_wire_shape_and_redaction_are_stable() {
    let request = SubmitIndexingRequestRequest {
        kind: IndexScopeKind::PrivateChannel.as_str().to_string(),
        target_id: "channel-1".to_string(),
        channel_secret_hex: Some("ab".repeat(32)),
    };
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({
            "kind": "private_channel",
            "target_id": "channel-1",
            "channel_secret_hex": "ab".repeat(32)
        })
    );
    let debug = format!("{request:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains(&"ab".repeat(32)));

    let response = SubmitIndexingRequestResponse {
        request_id: "request-1".to_string(),
        status: IndexingRequestStatus::Approved,
    };
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::json!({"request_id": "request-1", "status": "approved"})
    );
}

#[test]
fn index_query_params_preserve_scope_pair_and_optional_fields() {
    let params = IndexQueryParams {
        q: Some("hello world".to_string()),
        scope_kind: Some(IndexScopeKind::PublicTopic.as_str().to_string()),
        scope_id: Some("rust".to_string()),
        limit: Some(20),
    };

    assert_eq!(
        serde_json::to_value(&params).unwrap(),
        serde_json::json!({
            "q": "hello world",
            "scope_kind": "public_topic",
            "scope_id": "rust",
            "limit": 20
        })
    );
    let decoded: IndexQueryParams = serde_json::from_value(serde_json::json!({
        "scope_kind": "private_channel",
        "scope_id": "channel-1"
    }))
    .unwrap();
    assert_eq!(decoded.q, None);
    assert_eq!(decoded.scope_kind.as_deref(), Some("private_channel"));
    assert_eq!(decoded.scope_id.as_deref(), Some("channel-1"));
    assert_eq!(decoded.limit, None);
}

#[test]
fn index_query_response_wire_shape_is_stable() {
    let response = IndexQueryResponse {
        entries: vec![IndexEntryView {
            scope_kind: IndexScopeKind::PublicTopic,
            scope_id: "rust".to_string(),
            object_id: "post-1".to_string(),
            author_pubkey: "author".to_string(),
            text: "body\nderived-tag".to_string(),
            created_at: 42,
        }],
    };

    assert_eq!(
        serde_json::to_value(&response).unwrap(),
        serde_json::json!({
            "entries": [{
                "scope_kind": "public_topic",
                "scope_id": "rust",
                "object_id": "post-1",
                "author_pubkey": "author",
                "text": "body\nderived-tag",
                "created_at": 42
            }]
        })
    );
}

#[test]
fn api_error_body_wire_shape_is_stable() {
    let body = ApiErrorBody {
        code: "INDEX_QUERY_NOT_CONFIGURED".to_string(),
        message: "this community node does not provide index queries".to_string(),
    };
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        serde_json::json!({
            "code": "INDEX_QUERY_NOT_CONFIGURED",
            "message": "this community node does not provide index queries"
        })
    );
}
