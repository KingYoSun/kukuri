use anyhow::Result;
use kukuri_cn_core::{
    TestDatabase, TransmissionPreventionCapability, action_rights_request, connect_postgres,
    get_active_transmission_prevention, get_public_rights_request_status, get_rights_request,
    initialize_database, insert_rights_request, list_operator_actions, transition_rights_request,
};
use kukuri_cn_protocol::{
    RightsCategory, RightsRequestCreateRequest, RightsRequestScopeStatus, RightsRequestStatus,
    RightsRequesterKind,
};
use sqlx::Row;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

fn request() -> RightsRequestCreateRequest {
    RightsRequestCreateRequest {
        scope_revision: "scope-v1".to_string(),
        scope_acknowledged: true,
        requester_kind: RightsRequesterKind::RightsHolder,
        requester_name: "権利者".to_string(),
        organization: None,
        address: None,
        email: "rights@example.com".to_string(),
        phone: None,
        represented_rights_holder: None,
        authority_basis: None,
        rights_category: RightsCategory::Copyright,
        rights_basis: "著作権者である".to_string(),
        original_work_description: None,
        original_work_reference: None,
        subject_kind: "post".to_string(),
        subject_id: "post-760".to_string(),
        subject_url: None,
        infringement_description: "無断複製".to_string(),
        no_permission_statement: true,
        evidence_references: Vec::new(),
        requested_capabilities: vec!["moderation".to_string()],
    }
}

#[tokio::test]
async fn accountless_tracking_and_action_are_durable_and_redacted() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping rights-request integration test");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_rights_requests").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let created =
            insert_rights_request(&pool, &request(), RightsRequestScopeStatus::UnverifiedScope)
                .await?;
        assert_eq!(created.record.status, RightsRequestStatus::NeedsInformation);
        assert!(
            get_public_rights_request_status(&pool, &created.record.id, "wrong")
                .await?
                .is_none()
        );
        assert!(
            get_public_rights_request_status(&pool, &created.record.id, &created.tracking_secret)
                .await?
                .is_some()
        );
        let hash: String =
            sqlx::query("SELECT tracking_secret_hash FROM cn_legal.rights_requests WHERE id = $1")
                .bind(&created.record.id)
                .fetch_one(&pool)
                .await?
                .try_get("tracking_secret_hash")?;
        assert_ne!(hash, created.tracking_secret);

        let reviewing = transition_rights_request(
            &pool,
            &created.record.id,
            created.record.version,
            "legal@node.example",
            RightsRequestStatus::Reviewing,
            Some("審査を開始しました"),
            "status_surface",
        )
        .await?;
        let actioned = action_rights_request(
            &pool,
            &reviewing.id,
            reviewing.version,
            "legal@node.example",
            vec![TransmissionPreventionCapability::Moderation],
            "このノードの moderation 対象から除外しました",
        )
        .await?;
        assert_eq!(actioned.request.status, RightsRequestStatus::Actioned);
        assert_eq!(
            actioned.prevention.decision.related_report_id.as_deref(),
            Some(created.record.id.as_str())
        );
        assert!(
            get_active_transmission_prevention(&pool, "post", "post-760")
                .await?
                .is_some()
        );
        assert_eq!(
            get_rights_request(&pool, &created.record.id)
                .await?
                .unwrap()
                .status,
            RightsRequestStatus::Actioned
        );
        let audit = list_operator_actions(&pool, 20, 0).await?;
        assert!(
            audit
                .iter()
                .any(|row| row.action == "rights_request.transition")
        );
        assert!(
            audit
                .iter()
                .all(|row| !row.before.to_string().contains("rights@example.com")
                    && !row.after.to_string().contains("rights@example.com"))
        );
        assert!(
            sqlx::query("DELETE FROM cn_legal.rights_request_events WHERE request_id = $1")
                .bind(&created.record.id)
                .execute(&pool)
                .await
                .is_err(),
            "rights request events must be append-only"
        );
        anyhow::Ok(())
    }
    .await;
    database.cleanup().await?;
    result
}
