//! #1054: `cn_safety.scan_verdicts.advisory_labels` を追加する migration
//! （`202609160001_scan_verdict_advisory_labels.sql`）の Postgres integration テスト。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//! - 直前の schema（`202609150002`）まで適用した DB に verdict 行を seed し、残りの migration を
//!   適用しても行が保たれ `advisory_labels` が既定 `[]` になること（up）。
//! - migration SQL の再実行で失敗も差分も出ないこと（冪等）。
//! - `cn_index.index_entries` の CHECK / FK は変えないこと（INVAR-1）。

use anyhow::Result;
use kukuri_cn_core::{
    TestDatabase, connect_postgres, get_scan_verdict, migrate_postgres, migrate_postgres_up_to,
};
use kukuri_cn_safety::provider::SubjectKind;
use sqlx::PgPool;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const PREVIOUS_MIGRATION_VERSION: i64 = 202609150002;
const ADVISORY_MIGRATION_SQL: &str =
    include_str!("../migrations/202609160001_scan_verdict_advisory_labels.sql");

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

async fn seed_legacy_verdict(pool: &PgPool, subject_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_safety.scan_verdicts
            (id, subject_kind, subject_id, action, critical, reason_code, confidence, provider,
             policy_version, scanned_at)
         VALUES ($1, 'post', $2, 'allow', false, 'no_known_match', NULL, 'mock',
                 '2026-07-public-node-v2', '2026-09-15T00:00:00Z')",
    )
    .bind(format!("verdict-{subject_id}"))
    .bind(subject_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn column_default(pool: &PgPool) -> Result<Option<String>> {
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT column_default FROM information_schema.columns
         WHERE table_schema = 'cn_safety' AND table_name = 'scan_verdicts'
           AND column_name = 'advisory_labels'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|(default,)| default))
}

async fn index_entry_checks(pool: &PgPool) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT pg_get_constraintdef(c.oid)
         FROM pg_constraint c
         JOIN pg_class t ON t.oid = c.conrelid
         JOIN pg_namespace n ON n.oid = t.relnamespace
         WHERE n.nspname = 'cn_index' AND t.relname = 'index_entries'
           AND c.contype IN ('c', 'f')
         ORDER BY 1",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(def,)| def).collect())
}

#[tokio::test]
async fn advisory_labels_migration_defaults_to_empty_and_is_idempotent() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core advisory migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_verdict_advisory").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        migrate_postgres_up_to(&pool, PREVIOUS_MIGRATION_VERSION).await?;
        assert!(
            column_default(&pool).await?.is_none(),
            "column must not exist before the migration"
        );
        seed_legacy_verdict(&pool, "post-legacy").await?;
        let checks_before = index_entry_checks(&pool).await?;

        // up: 既存行は保たれ、advisory_labels は既定 `[]`。
        migrate_postgres(&pool).await?;
        assert!(column_default(&pool).await?.is_some());
        let stored = get_scan_verdict(&pool, SubjectKind::Post, "post-legacy")
            .await?
            .expect("legacy verdict survives the migration");
        assert!(stored.advisory_labels.is_empty());
        assert!(stored.to_record().advisories.is_empty());
        assert!(stored.to_record().verdict.advisory_labels.is_empty());
        assert!(stored.is_indexable());

        // 冪等: 同じ SQL を再実行しても失敗せず、列定義も行も変わらない。
        sqlx::raw_sql(ADVISORY_MIGRATION_SQL).execute(&pool).await?;
        let again = get_scan_verdict(&pool, SubjectKind::Post, "post-legacy")
            .await?
            .expect("verdict");
        assert_eq!(again, stored);

        // INVAR-1: index_entries の CHECK / FK は不変。
        let checks_after = index_entry_checks(&pool).await?;
        assert_eq!(checks_after, checks_before);
        assert!(
            checks_after
                .iter()
                .any(|def| def.contains("verdict_action") && def.contains("allow"))
        );
        assert!(checks_after.iter().any(|def| def.contains("NOT critical")));
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}
