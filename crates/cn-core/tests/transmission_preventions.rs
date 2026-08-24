use anyhow::Result;
use chrono::{Duration, Utc};
use kukuri_cn_core::{
    IndexScopeKind, NewIndexEntry, NewTransmissionPrevention, TestDatabase,
    TransmissionPreventionBasis, TransmissionPreventionCapability, apply_transmission_prevention,
    connect_postgres, filter_surfaceable_objects, get_active_transmission_prevention,
    get_index_entry, initialize_database, list_operator_actions, release_transmission_prevention,
    upsert_index_entry, upsert_scan_verdict,
};
use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{ReasonCode, SafetyAction, SafetyVerdict};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

fn allow_verdict() -> SafetyVerdict {
    SafetyVerdict {
        action: SafetyAction::Allow,
        labels: Vec::new(),
        critical: false,
        reason_code: ReasonCode::NoKnownMatch,
        confidence: None,
        provider: Some("mock-known-csam".to_string()),
        provider_capability: None,
        policy_version: "test".to_string(),
        scanned_at: "2026-08-25T00:00:00Z".to_string(),
    }
}

#[tokio::test]
async fn apply_survives_restart_gates_queries_and_requires_fresh_ingest_after_release() -> Result<()>
{
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping transmission-prevention integration test");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_transmission_prevention").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let verdict =
            upsert_scan_verdict(&pool, SubjectKind::Post, "post-761", &allow_verdict()).await?;
        let entry = NewIndexEntry {
            scope_kind: IndexScopeKind::PublicTopic,
            scope_id: "rust".to_string(),
            object_id: "post-761".to_string(),
            author_pubkey: "author".to_string(),
            created_at: 1,
            source_replica_id: "topic::rust".to_string(),
            verdict_id: verdict.id,
            verdict_action: "allow".to_string(),
            critical: false,
        };
        upsert_index_entry(&pool, &entry).await?;

        let applied = apply_transmission_prevention(
            &pool,
            "legal@node.example",
            &NewTransmissionPrevention {
                subject_kind: "post".to_string(),
                subject_id: "post-761".to_string(),
                basis: TransmissionPreventionBasis::Copyright,
                capabilities: vec![
                    TransmissionPreventionCapability::CommunityIndex,
                    TransmissionPreventionCapability::Search,
                    TransmissionPreventionCapability::Discovery,
                    TransmissionPreventionCapability::Recommendation,
                ],
                expires_at: None,
                related_report_id: None,
            },
        )
        .await?;
        assert_eq!(applied.removed_index_scopes.len(), 1);
        assert!(
            get_index_entry(&pool, IndexScopeKind::PublicTopic, "rust", "post-761")
                .await?
                .is_none()
        );
        assert!(
            filter_surfaceable_objects(
                &pool,
                IndexScopeKind::PublicTopic,
                &[("rust".to_string(), "post-761".to_string())],
            )
            .await?
            .is_empty()
        );
        assert!(upsert_index_entry(&pool, &entry).await.is_err());
        drop(pool.clone());
        let restarted = connect_postgres(database.database_url.as_str()).await?;
        assert!(
            get_active_transmission_prevention(&restarted, "post", "post-761")
                .await?
                .is_some()
        );

        release_transmission_prevention(
            &restarted,
            "legal@node.example",
            "post",
            "post-761",
            "claim resolved",
        )
        .await?;
        assert!(
            get_active_transmission_prevention(&restarted, "post", "post-761")
                .await?
                .is_none()
        );
        assert!(
            get_index_entry(&restarted, IndexScopeKind::PublicTopic, "rust", "post-761")
                .await?
                .is_none(),
            "release must not resurrect stale index state"
        );
        upsert_index_entry(&restarted, &entry).await?;

        let expired_subject = NewTransmissionPrevention {
            subject_kind: "post".to_string(),
            subject_id: "post-expired-761".to_string(),
            basis: TransmissionPreventionBasis::Privacy,
            capabilities: vec![TransmissionPreventionCapability::Moderation],
            expires_at: Some(Utc::now() - Duration::minutes(1)),
            related_report_id: None,
        };
        apply_transmission_prevention(&restarted, "legal@node.example", &expired_subject).await?;
        assert!(
            get_active_transmission_prevention(&restarted, "post", "post-expired-761")
                .await?
                .is_none()
        );
        let renewed = NewTransmissionPrevention {
            expires_at: None,
            ..expired_subject
        };
        apply_transmission_prevention(&restarted, "legal@node.example", &renewed).await?;
        assert!(
            get_active_transmission_prevention(&restarted, "post", "post-expired-761")
                .await?
                .is_some()
        );

        let audit = list_operator_actions(&restarted, 10, 0).await?;
        assert!(
            audit
                .iter()
                .any(|row| row.action == "transmission_prevention.apply")
        );
        assert!(
            audit
                .iter()
                .any(|row| row.action == "transmission_prevention.release")
        );
        assert!(
            audit
                .iter()
                .any(|row| row.action == "transmission_prevention.expire")
        );
        anyhow::Ok(())
    }
    .await;
    database.cleanup().await?;
    result
}
