use anyhow::Result;
use kukuri_cn_core::{
    TestDatabase, connect_postgres, initialize_database, list_policies, sync_policies,
};
use kukuri_cn_protocol::CommunityNodePolicyDocument;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

fn policy(version: i32, body: &str) -> CommunityNodePolicyDocument {
    CommunityNodePolicyDocument {
        policy_slug: "terms_of_service".to_string(),
        policy_version: version,
        title: "KingYoSun Node 利用規約".to_string(),
        body_markdown: body.to_string(),
        required: true,
        effective_date: Some("2026-09-02".to_string()),
        language: Some("ja".to_string()),
    }
}

#[tokio::test]
async fn operator_policy_sync_replaces_placeholder_and_requires_version_bump() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_database_url.as_str(), "cn_policy_sync").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    initialize_database(&pool).await?;
    sqlx::query(
        "INSERT INTO cn_user.subscriber_accounts (subscriber_pubkey) VALUES ('legacy-consent-user')",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO cn_user.policy_consents
            (subscriber_pubkey, policy_slug, policy_version)
         VALUES ('legacy-consent-user', 'terms_of_service', 1)",
    )
    .execute(&pool)
    .await?;

    let v1 = policy(
        1,
        "# KingYoSun Node 利用規約\n\n現行の運用実態に基づく本文です。",
    );
    sync_policies(&pool, std::slice::from_ref(&v1)).await?;
    let stored = list_policies(&pool).await?;
    let terms = stored
        .iter()
        .find(|document| document.policy_slug == "terms_of_service")
        .expect("terms policy exists");
    assert_eq!(terms.body_markdown, v1.body_markdown);
    assert_eq!(terms.effective_date.as_deref(), Some("2026-09-02"));
    assert_eq!(terms.language.as_deref(), Some("ja"));
    let legacy_consent_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cn_user.policy_consents
         WHERE subscriber_pubkey = 'legacy-consent-user'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(legacy_consent_count, 0);

    let changed_without_bump = policy(1, "本文だけを変更");
    let same_version_error = sync_policies(&pool, &[changed_without_bump])
        .await
        .expect_err("same-version content replacement must fail");
    assert!(
        same_version_error
            .to_string()
            .contains("content changed without a version increase")
    );

    let v2 = policy(2, "# KingYoSun Node 利用規約 v2");
    sync_policies(&pool, &[v2]).await?;
    let rollback_error = sync_policies(&pool, &[v1])
        .await
        .expect_err("version rollback must fail");
    assert!(rollback_error.to_string().contains("version rollback"));

    pool.close().await;
    database.cleanup().await
}
