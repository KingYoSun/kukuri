use super::super::*;
use axum::response::{IntoResponse, Response};
use kukuri_cn_protocol::ApiErrorBody;
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct MockReportState {
    received: Arc<Mutex<Vec<Value>>>,
    invalid_appeal: bool,
}

async fn mock_report(State(state): State<MockReportState>, Json(payload): Json<Value>) -> Response {
    state.received.lock().await.push(payload);
    if state.invalid_appeal {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiErrorBody {
                code: "INVALID_APPEAL".to_string(),
                message: "異議申し立ての対象を確認できません".to_string(),
            }),
        )
            .into_response();
    }
    Json(json!({
        "reference_id": "report-1",
        "disputed_risk_signal_id": "signal-1"
    }))
    .into_response()
}

async fn report_runtime(
    invalid_appeal: bool,
) -> (
    DesktopRuntime,
    String,
    MockReportState,
    tokio::task::JoinHandle<()>,
    tempfile::TempDir,
) {
    let dir = tempdir().expect("tempdir");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        dir.path().join("community-report.db"),
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = MockReportState {
        received: Arc::new(Mutex::new(Vec::new())),
        invalid_appeal,
    };
    let app = Router::new()
        .route("/v1/report", post(mock_report))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    (runtime, base_url, state, server, dir)
}

fn appeal_request(base_url: &str) -> SubmitCommunityNodeReportRequest {
    SubmitCommunityNodeReportRequest {
        node_base_url: base_url.to_string(),
        report_endpoint: format!("{base_url}/v1/report"),
        subject_kind: "profile".to_string(),
        subject_id: "author-pubkey".to_string(),
        capability: "trust_signal".to_string(),
        reason: "other".to_string(),
        details: Some("誤検知として再確認を求めます".to_string()),
        reporter_contact: None,
        appeal: Some(CommunityNodeReportAppeal {
            risk_signal_id: "signal-1".to_string(),
        }),
    }
}

#[tokio::test]
async fn community_node_report_client_sends_anonymous_appeal_and_reads_disputed_signal() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, state, server, _dir) = report_runtime(false).await;

    let result = runtime
        .submit_community_node_report(appeal_request(base_url.as_str()))
        .await
        .expect("appeal submitted");
    assert_eq!(result.reference_id.as_deref(), Some("report-1"));
    assert_eq!(result.disputed_risk_signal_id.as_deref(), Some("signal-1"));
    let received = state.received.lock().await;
    assert_eq!(received.len(), 1);
    assert_eq!(received[0]["appeal"]["risk_signal_id"], "signal-1");
    assert!(received[0].get("reporter_contact").is_none());

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_report_client_preserves_invalid_appeal_code() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let (runtime, base_url, _state, server, _dir) = report_runtime(true).await;

    let error = runtime
        .submit_community_node_report(appeal_request(base_url.as_str()))
        .await
        .expect_err("invalid appeal");
    assert_eq!(error.code, "INVALID_APPEAL");
    assert_eq!(error.status, Some(400));

    runtime.shutdown().await;
    server.abort();
}
