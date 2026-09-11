//! #975: 索引状況読取り(`read_community_node_indexing_status`)の client 契約。
//!
//! mock server と runtime の組立ては `index_query` と共有する。一覧取得と公開 target は秘密値を
//! 送らず、非公開 target は明示確認と参加中 capability がそろうまで HTTP に到達しない。
use super::super::*;
use super::index_query::index_runtime;
use kukuri_cn_protocol::{
    ApiErrorBody, IndexScopeKind, IndexingRequestStatus, IndexingStatusParams, IndexingTargetStatus,
};
fn status_request(base_url: &str) -> CommunityNodeIndexingStatusRequest {
    CommunityNodeIndexingStatusRequest {
        base_url: base_url.to_string(),
        scope_kind: None,
        topic_id: None,
        channel_id: None,
        confirm_private_channel_secret_disclosure: false,
    }
}

// #975 AC-1 / INVAR-3: 一覧取得と公開 target の判定は所属証明を送らず、wire 契約を保つ。
#[tokio::test]
async fn community_node_indexing_status_list_and_public_target_send_no_secret() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;

    let listed = runtime
        .read_community_node_indexing_status(status_request(base_url.as_str()))
        .await
        .expect("list-only status");
    assert_eq!(listed.requests.len(), 1);
    assert_eq!(listed.requests[0].status, IndexingRequestStatus::Rejected);
    assert_eq!(listed.requests[0].decided_at, Some(2_000));
    assert_eq!(listed.target, None);

    let public = runtime
        .read_community_node_indexing_status(CommunityNodeIndexingStatusRequest {
            scope_kind: Some(IndexScopeKind::PublicTopic),
            topic_id: Some("rust".to_string()),
            ..status_request(base_url.as_str())
        })
        .await
        .expect("public target status");
    assert_eq!(
        public.target,
        Some(IndexingTargetStatus {
            scope_kind: IndexScopeKind::PublicTopic,
            scope_id: "rust".to_string(),
            supported: true,
        })
    );

    let calls = state.indexing_status_calls.lock().await.clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, IndexingStatusParams::default());
    assert!(calls[0].1.is_none(), "一覧取得は秘密値ヘッダを付けない");
    assert_eq!(calls[1].0.scope_kind.as_deref(), Some("public_topic"));
    assert_eq!(calls[1].0.scope_id.as_deref(), Some("rust"));
    assert!(calls[1].1.is_none(), "公開 target は秘密値ヘッダを付けない");

    // public に channel_id、target なしに channel_id は HTTP 前に拒否する。
    let error = runtime
        .read_community_node_indexing_status(CommunityNodeIndexingStatusRequest {
            scope_kind: Some(IndexScopeKind::PublicTopic),
            topic_id: Some("rust".to_string()),
            channel_id: Some("channel".to_string()),
            ..status_request(base_url.as_str())
        })
        .await
        .expect_err("channel_id with public target");
    assert_eq!(error.code, "INVALID_INDEXING_STATUS_REQUEST");
    let error = runtime
        .read_community_node_indexing_status(CommunityNodeIndexingStatusRequest {
            scope_kind: Some(IndexScopeKind::PublicTopic),
            ..status_request(base_url.as_str())
        })
        .await
        .expect_err("public target without topic");
    assert_eq!(error.code, "INVALID_INDEXING_STATUS_REQUEST");
    assert_eq!(state.indexing_status_calls.lock().await.len(), 2);

    runtime.shutdown().await;
    server.abort();
}

// #975 INVAR-3 / #711: 非公開 target は明示確認と参加中 capability がそろって初めて所属証明を送る。
#[tokio::test]
async fn community_node_indexing_status_private_target_requires_confirmation_and_proof() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _managed, state, server, _dir) = index_runtime(None).await;
    let topic = "kukuri:topic:private-status";
    let channel = runtime
        .create_private_channel(CreatePrivateChannelRequest {
            topic: topic.to_string(),
            label: "status private channel".to_string(),
            audience_kind: ChannelAudienceKind::InviteOnly,
        })
        .await
        .expect("create private channel");

    let error = runtime
        .read_community_node_indexing_status(CommunityNodeIndexingStatusRequest {
            scope_kind: Some(IndexScopeKind::PrivateChannel),
            topic_id: Some(topic.to_string()),
            channel_id: Some(channel.channel_id.clone()),
            confirm_private_channel_secret_disclosure: false,
            ..status_request(base_url.as_str())
        })
        .await
        .expect_err("unconfirmed private target");
    assert_eq!(
        error.code,
        "PRIVATE_CHANNEL_SECRET_DISCLOSURE_NOT_CONFIRMED"
    );

    let error = runtime
        .read_community_node_indexing_status(CommunityNodeIndexingStatusRequest {
            scope_kind: Some(IndexScopeKind::PrivateChannel),
            topic_id: Some(topic.to_string()),
            channel_id: Some("not-joined-channel".to_string()),
            confirm_private_channel_secret_disclosure: true,
            ..status_request(base_url.as_str())
        })
        .await
        .expect_err("not joined channel");
    assert_eq!(error.code, "PRIVATE_CHANNEL_CAPABILITY_UNAVAILABLE");
    assert!(
        state.indexing_status_calls.lock().await.is_empty(),
        "確認なし・capability なしでは HTTP に到達しない"
    );

    let proven = runtime
        .read_community_node_indexing_status(CommunityNodeIndexingStatusRequest {
            scope_kind: Some(IndexScopeKind::PrivateChannel),
            topic_id: Some(topic.to_string()),
            channel_id: Some(channel.channel_id.clone()),
            confirm_private_channel_secret_disclosure: true,
            ..status_request(base_url.as_str())
        })
        .await
        .expect("private target status");
    assert_eq!(
        proven.target.as_ref().map(|target| target.scope_kind),
        Some(IndexScopeKind::PrivateChannel)
    );
    let calls = state.indexing_status_calls.lock().await.clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0.scope_kind.as_deref(), Some("private_channel"));
    assert_eq!(
        calls[0].0.scope_id.as_deref(),
        Some(channel.channel_id.as_str())
    );
    assert!(
        calls[0]
            .1
            .as_deref()
            .is_some_and(|secret| !secret.is_empty()),
        "所属証明ヘッダを同伴する"
    );

    runtime.shutdown().await;
    server.abort();
}

// #975 INVAR-3 / #698: 必須同意が未承認・session 再試行中は HTTP 前に停止する。
#[tokio::test]
async fn community_node_indexing_status_stops_before_http_when_session_is_not_ready() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, managed, state, server, _dir) = index_runtime(None).await;
    managed.consent_accepted.store(false, Ordering::SeqCst);
    managed
        .simulate_pending_update
        .store(true, Ordering::SeqCst);

    let error = runtime
        .read_community_node_indexing_status(CommunityNodeIndexingStatusRequest {
            scope_kind: Some(IndexScopeKind::PrivateChannel),
            topic_id: Some("kukuri:topic:private-status".to_string()),
            channel_id: Some("private-channel".to_string()),
            confirm_private_channel_secret_disclosure: true,
            ..status_request(base_url.as_str())
        })
        .await
        .expect_err("pending consent must stop the read");
    // 秘密値の取得(PRIVATE_CHANNEL_CAPABILITY_UNAVAILABLE)より前に同意で止まる。
    assert_eq!(error.code, "CONSENT_REQUIRED");

    managed.consent_accepted.store(true, Ordering::SeqCst);
    managed
        .simulate_pending_update
        .store(false, Ordering::SeqCst);
    runtime
        .set_community_node_retry_state(
            base_url.as_str(),
            anyhow::anyhow!("temporary session failure"),
        )
        .await;
    let error = runtime
        .read_community_node_indexing_status(status_request(base_url.as_str()))
        .await
        .expect_err("retrying session must defer the read");
    assert_eq!(error.code, "COMMUNITY_NODE_SESSION_DEFERRED");
    assert!(state.indexing_status_calls.lock().await.is_empty());

    runtime.shutdown().await;
    server.abort();
}

// #975: 安定 error code / retry-after を保持し、401 では一度だけ再認証する。
#[tokio::test]
async fn community_node_indexing_status_preserves_stable_error_and_reauthenticates_once() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let body = ApiErrorBody {
        code: "CHANNEL_MEMBERSHIP_REQUIRED".to_string(),
        message: "private channel index queries require the channel secret of a participant"
            .to_string(),
    };
    let (runtime, base_url, managed, state, server, _dir) =
        index_runtime(Some((StatusCode::FORBIDDEN, body, Some("5")))).await;

    let error = runtime
        .read_community_node_indexing_status(status_request(base_url.as_str()))
        .await
        .expect_err("forced error");
    assert_eq!(error.status, Some(403));
    assert_eq!(error.code, "CHANNEL_MEMBERSHIP_REQUIRED");
    assert_eq!(error.retry_after_seconds, Some(5));

    *state.forced_error.lock().await = None;
    state.unauthorized_remaining.store(1, Ordering::SeqCst);
    let verifies_before = managed.verify_hits.load(Ordering::SeqCst);
    runtime
        .read_community_node_indexing_status(status_request(base_url.as_str()))
        .await
        .expect("reauthenticated read");
    assert_eq!(
        managed.verify_hits.load(Ordering::SeqCst),
        verifies_before + 1,
        "401 では一度だけ再認証する"
    );
    assert_eq!(state.indexing_status_calls.lock().await.len(), 3);

    runtime.shutdown().await;
    server.abort();
}
