//! #680 異議申し立て受理・運営者審査の Postgres 結合試験。

use anyhow::Result;
use kukuri_cn_core::{
    AppealReviewOperation, NewCommunityNodeReport, RiskSignalCorrection, RiskSignalMetadataEdit,
    TestDatabase, apply_appeal_review_action, connect_postgres, get_appeal_review,
    get_community_node_report, get_risk_signal, initialize_database, insert_community_node_appeal,
    list_appeal_reviews, list_operator_actions, list_trust_risk_inputs, persist_risk_signal,
};
use kukuri_cn_safety::{
    AppealStatus, Basis, RiskSignalTarget, SafetyCategory, SafetyRiskSignal, Severity, Visibility,
};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const ISSUER: &str = "issuer-node-1";
const NOW: &str = "2026-08-15T00:00:00Z";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

fn signal(target_id: &str, issuer_status: AppealStatus) -> SafetyRiskSignal {
    SafetyRiskSignal {
        target: RiskSignalTarget::UserPubkey,
        target_id: target_id.to_string(),
        category: SafetyCategory::Nsfw,
        severity: Severity::High,
        basis: Basis::ClassifierScore,
        confidence: Some(90),
        visibility: Visibility::Local,
        expires_at: None,
        appeal_status: Some(issuer_status),
    }
}

fn report(target_id: &str, details: &str) -> NewCommunityNodeReport {
    NewCommunityNodeReport {
        subject_kind: "profile".to_string(),
        subject_id: target_id.to_string(),
        capability: "moderation".to_string(),
        reason: "false_positive".to_string(),
        details: Some(details.to_string()),
        reporter_contact: Some("must-not-be-stored@example.com".to_string()),
        appeal_risk_signal_id: None,
    }
}

#[tokio::test]
async fn appeal_intake_is_linked_atomic_anonymous_and_grouped() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping appeal review integration test");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_appeal_intake_680").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let stored =
            persist_risk_signal(&pool, ISSUER, &signal("alice", AppealStatus::None)).await?;

        let first = insert_community_node_appeal(
            &pool,
            ISSUER,
            &stored.id,
            &report("alice", "一件目の説明"),
        )
        .await?;
        let second = insert_community_node_appeal(
            &pool,
            ISSUER,
            &stored.id,
            &report("alice", "二件目の説明"),
        )
        .await?;
        assert_ne!(first.id, second.id);
        assert_eq!(
            first.appeal_risk_signal_id.as_deref(),
            Some(stored.id.as_str())
        );
        assert!(first.reporter_contact.is_none());

        let reviews = list_appeal_reviews(&pool, 50, 0).await?;
        assert_eq!(reviews.len(), 1, "同じ判定は一つの審査対象にまとめる");
        assert_eq!(reviews[0].reports.len(), 2);

        let foreign = persist_risk_signal(
            &pool,
            "foreign-node",
            &signal("mallory", AppealStatus::None),
        )
        .await?;
        assert!(
            insert_community_node_appeal(
                &pool,
                ISSUER,
                &foreign.id,
                &report("mallory", "拒否される説明"),
            )
            .await
            .is_err()
        );
        let foreign_after = get_risk_signal(&pool, &foreign.id)
            .await?
            .expect("foreign signal");
        assert_eq!(foreign_after.signal.appeal_status, Some(AppealStatus::None));
        assert!(get_appeal_review(&pool, &foreign.id).await?.is_some());
        assert!(
            get_appeal_review(&pool, &foreign.id)
                .await?
                .expect("foreign review shell")
                .reports
                .is_empty()
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn operator_review_revalidates_and_commits_state_reports_and_audit_together() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping appeal review integration test");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_appeal_actions_680").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let stored =
            persist_risk_signal(&pool, ISSUER, &signal("alice", AppealStatus::None)).await?;
        let first = insert_community_node_appeal(
            &pool,
            ISSUER,
            &stored.id,
            &report("alice", "監査へ入れてはならない本文"),
        )
        .await?;
        let stale = get_appeal_review(&pool, &stored.id)
            .await?
            .expect("review")
            .version();
        insert_community_node_appeal(
            &pool,
            ISSUER,
            &stored.id,
            &report("alice", "確認後に届いた本文"),
        )
        .await?;
        assert!(
            apply_appeal_review_action(
                &pool,
                "ops@kukuri.app",
                &stored.id,
                &AppealReviewOperation::Accept { expected: stale },
                true,
            )
            .await
            .is_err(),
            "古い確認内容は拒否する"
        );
        assert!(list_operator_actions(&pool, 50, 0).await?.is_empty());

        let current = get_appeal_review(&pool, &stored.id)
            .await?
            .expect("review")
            .version();
        assert!(
            apply_appeal_review_action(
                &pool,
                "ops@kukuri.app",
                &stored.id,
                &AppealReviewOperation::Accept {
                    expected: current.clone(),
                },
                false,
            )
            .await
            .is_err(),
            "機能無効時は拒否する"
        );
        apply_appeal_review_action(
            &pool,
            "ops@kukuri.app",
            &stored.id,
            &AppealReviewOperation::Accept { expected: current },
            true,
        )
        .await?;
        let cleared = get_risk_signal(&pool, &stored.id).await?.expect("signal");
        assert_eq!(cleared.signal.appeal_status, Some(AppealStatus::Cleared));
        assert_eq!(
            get_community_node_report(&pool, &first.id)
                .await?
                .expect("report")
                .status,
            "actioned"
        );
        let trust =
            list_trust_risk_inputs(&pool, RiskSignalTarget::UserPubkey, "alice", NOW).await?;
        assert!(trust.absolute.is_empty());
        assert_eq!(trust.relative.len(), 1);
        assert_eq!(trust.relative[0].appeal_status, AppealStatus::Cleared);

        let rejected =
            persist_risk_signal(&pool, ISSUER, &signal("bob", AppealStatus::None)).await?;
        let rejected_report =
            insert_community_node_appeal(&pool, ISSUER, &rejected.id, &report("bob", "棄却対象"))
                .await?;
        let expected = get_appeal_review(&pool, &rejected.id)
            .await?
            .expect("review")
            .version();
        apply_appeal_review_action(
            &pool,
            "ops@kukuri.app",
            &rejected.id,
            &AppealReviewOperation::Reject { expected },
            true,
        )
        .await?;
        assert_eq!(
            get_risk_signal(&pool, &rejected.id)
                .await?
                .expect("signal")
                .signal
                .appeal_status,
            Some(AppealStatus::None)
        );
        assert_eq!(
            get_community_node_report(&pool, &rejected_report.id)
                .await?
                .expect("report")
                .status,
            "dismissed"
        );
        let trust = list_trust_risk_inputs(&pool, RiskSignalTarget::UserPubkey, "bob", NOW).await?;
        assert_eq!(trust.relative.len(), 1);

        let edited =
            persist_risk_signal(&pool, ISSUER, &signal("carol", AppealStatus::None)).await?;
        insert_community_node_appeal(&pool, ISSUER, &edited.id, &report("carol", "調整対象"))
            .await?;
        let expected = get_appeal_review(&pool, &edited.id)
            .await?
            .expect("review")
            .version();
        apply_appeal_review_action(
            &pool,
            "ops@kukuri.app",
            &edited.id,
            &AppealReviewOperation::Edit {
                expected,
                edit: RiskSignalMetadataEdit {
                    category: Some(SafetyCategory::Spam),
                    severity: Some(Severity::Low),
                    confidence: Some(20),
                    expires_at: None,
                },
            },
            true,
        )
        .await?;
        let expected = get_appeal_review(&pool, &edited.id)
            .await?
            .expect("review")
            .version();
        let before_ids =
            sqlx::query_scalar::<_, String>("SELECT id FROM cn_safety.risk_signals ORDER BY id")
                .fetch_all(&pool)
                .await?;
        apply_appeal_review_action(
            &pool,
            "ops@kukuri.app",
            &edited.id,
            &AppealReviewOperation::Reissue {
                expected,
                correction: RiskSignalCorrection {
                    category: None,
                    severity: None,
                    confidence: Some(10),
                    visibility: Some(Visibility::Public),
                },
            },
            true,
        )
        .await?;
        let after_ids =
            sqlx::query_scalar::<_, String>("SELECT id FROM cn_safety.risk_signals ORDER BY id")
                .fetch_all(&pool)
                .await?;
        assert_eq!(after_ids.len(), before_ids.len() + 1);
        assert!(
            get_risk_signal(&pool, &edited.id)
                .await?
                .expect("old signal")
                .signal
                .expires_at
                .is_some()
        );

        let audit = list_operator_actions(&pool, 50, 0).await?;
        assert_eq!(audit.len(), 4);
        let serialized = serde_json::to_string(&audit)?;
        assert!(!serialized.contains("監査へ入れてはならない本文"));
        assert!(!serialized.contains("must-not-be-stored@example.com"));
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}
