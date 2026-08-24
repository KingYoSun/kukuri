//! Public node-local transmission-prevention status contract (#761).

use anyhow::Result;
use kukuri_cn_core::{
    NewTransmissionPrevention, TransmissionPreventionBasis, TransmissionPreventionCapability,
    apply_transmission_prevention, connect_postgres, release_transmission_prevention,
};
use reqwest::{Client, StatusCode};

mod support;
use support::{TestServer, integration_test_admin_database_url};

#[tokio::test]
async fn public_status_exposes_scope_and_appeal_path_without_operator_identity() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping transmission-prevention user-api integration test");
        return Ok(());
    };
    let server = TestServer::spawn(admin_url.as_str(), "transmission-prevention-status").await?;
    let pool = connect_postgres(server.database.database_url.as_str()).await?;
    apply_transmission_prevention(
        &pool,
        "operator@example.net",
        &NewTransmissionPrevention {
            subject_kind: "post".to_string(),
            subject_id: "post-761-status".to_string(),
            basis: TransmissionPreventionBasis::Privacy,
            capabilities: vec![
                TransmissionPreventionCapability::CommunityIndex,
                TransmissionPreventionCapability::Search,
            ],
            expires_at: None,
            related_report_id: Some("private-report-reference".to_string()),
        },
    )
    .await?;

    let client = Client::new();
    let response = client
        .get(format!(
            "{}/v1/transmission-preventions/post/post-761-status",
            server.base_url
        ))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.json::<serde_json::Value>().await?;
    assert_eq!(body["active"], true);
    assert_eq!(body["basis"], "privacy");
    assert_eq!(body["appeal_path"], "/v1/report");
    assert_eq!(
        body["capabilities"],
        serde_json::json!(["community_index", "search"])
    );
    assert!(body.get("decided_by").is_none());
    assert!(body.get("related_report_id").is_none());

    release_transmission_prevention(
        &pool,
        "operator@example.net",
        "post",
        "post-761-status",
        "claim resolved",
    )
    .await?;
    let body = client
        .get(format!(
            "{}/v1/transmission-preventions/post/post-761-status",
            server.base_url
        ))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(body["active"], false);
    assert_eq!(body["appeal_path"], "/v1/report");

    server.shutdown().await
}
