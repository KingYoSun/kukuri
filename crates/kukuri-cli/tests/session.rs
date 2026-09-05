use kukuri_cli::{
    dispatcher::{DispatchReply, Dispatcher},
    protocol::{PROTOCOL_VERSION, RequestEnvelope},
    session::ClientSession,
};
use serde_json::json;

#[tokio::test]
async fn fresh_session_exposes_consent_without_runtime_or_account_creation() {
    let dir = tempfile::tempdir().expect("一時profile");
    let session = ClientSession::start(dir.path().to_path_buf())
        .await
        .expect("session");
    let dispatcher = Dispatcher::for_session(session.clone());
    for (command, payload, success) in [
        ("get_desktop_startup_status", json!({}), true),
        ("get_app_consent_status", json!({}), true),
        (
            "accept_app_consents",
            json!({"documents": [], "language": "ja", "age_attested": false}),
            false,
        ),
        (
            "create_post",
            json!({"topic": "test", "content": "同意前の本文"}),
            false,
        ),
        ("cancel_device_backup", json!({}), true),
    ] {
        let request = RequestEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: "fresh".into(),
            command: command.into(),
            profile: "test".into(),
            payload,
            timeout_ms: None,
            secret_bytes: None,
            accepts_secret_output: false,
        };
        let DispatchReply::Unary(result, secret) =
            dispatcher.dispatch(request, None, "test", None).await
        else {
            panic!("JSON")
        };
        assert_eq!(result.ok, success, "{command}: {:?}", result.error);
        assert!(secret.is_none());
        if !success {
            assert_eq!(result.error_code(), Some("consent_required"));
        }
    }
    assert!(session.host().is_none());
    assert_eq!(
        std::fs::read_dir(dir.path()).expect("profile").count(),
        0,
        "未同意ではaccountやDBを作らない"
    );
    dispatcher.finish_operations().await;
    session.shutdown().await;
}
