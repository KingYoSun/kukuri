use anyhow::Result;
use kukuri_cn_core::{
    TestDatabase, accept_consents, connect_postgres, get_policy_revision, initialize_database,
    list_policies, list_policy_revisions, sync_policies,
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
        policy_snapshot_revision: Some(format!("snapshot-{version}")),
        authoritative_language: Some("ja".to_string()),
        reference_translation: false,
        translation_revision: None,
        translation_of_version: None,
        fallback: false,
        requested_language: None,
        material_change: false,
        requires_reconsent: false,
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
    let pubkey = "a".repeat(64);
    sqlx::query("INSERT INTO cn_user.subscriber_accounts (subscriber_pubkey) VALUES ($1)")
        .bind(&pubkey)
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO cn_user.policy_consents
            (subscriber_pubkey, policy_slug, policy_version)
         VALUES ($1, 'terms_of_service', 1)",
    )
    .bind(&pubkey)
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
         WHERE subscriber_pubkey = $1",
    )
    .bind(&pubkey)
    .fetch_one(&pool)
    .await?;
    assert_eq!(legacy_consent_count, 0);

    let mut snapshot_only_change = v1.clone();
    snapshot_only_change.policy_snapshot_revision = Some("snapshot-catalog-2".to_string());
    sync_policies(&pool, &[snapshot_only_change]).await?;
    let snapshot_revisions = list_policy_revisions(&pool, "terms_of_service").await?;
    assert_eq!(
        snapshot_revisions.len(),
        1,
        "正文が同じなら版を水増ししない"
    );
    assert!(snapshot_revisions[0].material_change);
    assert!(snapshot_revisions[0].requires_reconsent);

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
    sync_policies(&pool, std::slice::from_ref(&v2)).await?;
    let revisions = list_policy_revisions(&pool, "terms_of_service").await?;
    assert_eq!(revisions.len(), 2, "公開済み正文は削除しない");
    assert_eq!(revisions[0].policy_version, 2);
    assert_eq!(revisions[1].policy_version, 1);
    assert!(revisions[0].material_change);
    assert!(revisions[0].requires_reconsent);

    let mut translation = v2.clone();
    translation.title = "Terms of Service".to_string();
    translation.body_markdown = "English reference translation.".to_string();
    translation.language = Some("en".to_string());
    translation.reference_translation = true;
    translation.translation_revision = Some(1);
    translation.translation_of_version = Some(2);
    sync_policies(&pool, &[v2.clone(), translation]).await?;
    let localized = get_policy_revision(&pool, "terms_of_service", 2, Some("en"))
        .await?
        .expect("localized revision");
    assert!(localized.reference_translation);
    assert!(!localized.fallback);
    assert_eq!(localized.translation_of_version, Some(2));
    let fallback = get_policy_revision(&pool, "terms_of_service", 2, Some("fr"))
        .await?
        .expect("authoritative fallback");
    assert!(!fallback.reference_translation);
    assert!(fallback.fallback);
    assert_eq!(fallback.language.as_deref(), Some("ja"));

    let stale_accept = accept_consents(&pool, &pubkey, &[], Some("snapshot-1"))
        .await
        .expect_err("stale snapshot must not be accepted");
    assert!(stale_accept.to_string().contains("policy snapshot changed"));
    let rollback_error = sync_policies(&pool, &[v1])
        .await
        .expect_err("version rollback must fail");
    assert!(rollback_error.to_string().contains("version rollback"));

    pool.close().await;
    database.cleanup().await
}
