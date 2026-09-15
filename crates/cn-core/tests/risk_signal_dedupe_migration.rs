//! #1050: 既存の重複 risk signal を圧縮する migration（`202609150002_risk_signal_dedupe.sql`）の
//! Postgres integration テスト。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//! - 直前の schema（`202609150001`）まで適用した DB に重複行を seed し、残りの migration を
//!   適用して圧縮結果（生存行の選び方・値の集約・参照行の保護）を確認する。
//! - 圧縮 SQL の再実行で差分が出ないこと（冪等）。
//! - 部分 UNIQUE index が同一鍵の 2 行目を拒否し、appeal / cn-cli の再発行経路は通ること。

use anyhow::Result;
use kukuri_cn_core::{
    TestDatabase, connect_postgres, initialize_database, migrate_postgres, migrate_postgres_up_to,
};
use sqlx::PgPool;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const FINGERPRINT_MIGRATION_VERSION: i64 = 202609150001;
const DEDUPE_MIGRATION_SQL: &str =
    include_str!("../migrations/202609150002_risk_signal_dedupe.sql");
const ISSUER: &str = "issuer-node";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

struct SeedSignal<'a> {
    id: &'a str,
    target_id: &'a str,
    category: &'a str,
    confidence: i16,
    appeal_status: &'a str,
    expires_at: Option<&'a str>,
    persisted_at: &'a str,
}

async fn seed_signal(pool: &PgPool, seed: &SeedSignal<'_>) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_safety.risk_signals
            (id, issuer_node_id, target, target_id, category, severity, basis, visibility,
             confidence, expires_at, appeal_status, persisted_at)
         VALUES ($1, $2, 'post_id', $3, $4, 'high', 'classifier_score', 'local', $5, $6, $7,
                 $8::timestamptz)",
    )
    .bind(seed.id)
    .bind(ISSUER)
    .bind(seed.target_id)
    .bind(seed.category)
    .bind(seed.confidence)
    .bind(seed.expires_at)
    .bind(seed.appeal_status)
    .bind(seed.persisted_at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_report_referencing(pool: &PgPool, report_id: &str, signal_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_admin.reports
            (id, subject_kind, subject_id, capability, reason, appeal_risk_signal_id)
         VALUES ($1, 'post', 'post-a', 'moderation', 'false_positive', $2)",
    )
    .bind(report_id)
    .bind(signal_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow, PartialEq)]
struct SignalRow {
    id: String,
    confidence: Option<i16>,
    appeal_status: Option<String>,
    expires_at: Option<String>,
    persisted_at: chrono::DateTime<chrono::Utc>,
}

async fn rows_for(pool: &PgPool, target_id: &str) -> Result<Vec<SignalRow>> {
    Ok(sqlx::query_as::<_, SignalRow>(
        "SELECT id, confidence, appeal_status, expires_at, persisted_at
         FROM cn_safety.risk_signals WHERE target_id = $1 ORDER BY persisted_at, id",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await?)
}

async fn seed_duplicates(pool: &PgPool) -> Result<()> {
    // 鍵 A（post-a / nsfw）: 3 件の活性重複。2 件目が通報から参照され、3 件目が disputed。
    seed_signal(
        pool,
        &SeedSignal {
            id: "a-oldest",
            target_id: "post-a",
            category: "nsfw",
            confidence: 80,
            appeal_status: "none",
            expires_at: None,
            persisted_at: "2026-09-15T10:00:00Z",
        },
    )
    .await?;
    seed_signal(
        pool,
        &SeedSignal {
            id: "a-referenced",
            target_id: "post-a",
            category: "nsfw",
            confidence: 84,
            appeal_status: "none",
            expires_at: None,
            persisted_at: "2026-09-15T10:05:00Z",
        },
    )
    .await?;
    seed_signal(
        pool,
        &SeedSignal {
            id: "a-disputed",
            target_id: "post-a",
            category: "nsfw",
            confidence: 91,
            appeal_status: "disputed",
            expires_at: None,
            persisted_at: "2026-09-15T10:10:00Z",
        },
    )
    .await?;
    seed_report_referencing(pool, "report-a", "a-referenced").await?;
    // 鍵 A の cleared 済み行は活性ではないので圧縮対象にしない。
    seed_signal(
        pool,
        &SeedSignal {
            id: "a-cleared",
            target_id: "post-a",
            category: "nsfw",
            confidence: 70,
            appeal_status: "cleared",
            expires_at: None,
            persisted_at: "2026-09-15T09:00:00Z",
        },
    )
    .await?;
    // 鍵 B（post-b / spam）: 2 件の素の重複 → 最古が残る。
    seed_signal(
        pool,
        &SeedSignal {
            id: "b-oldest",
            target_id: "post-b",
            category: "spam",
            confidence: 60,
            appeal_status: "none",
            expires_at: None,
            persisted_at: "2026-09-15T10:00:00Z",
        },
    )
    .await?;
    seed_signal(
        pool,
        &SeedSignal {
            id: "b-newer",
            target_id: "post-b",
            category: "spam",
            confidence: 65,
            appeal_status: "none",
            expires_at: None,
            persisted_at: "2026-09-15T10:20:00Z",
        },
    )
    .await?;
    // 鍵 C（post-c / nsfw）: 単独行 → 変化なし。
    seed_signal(
        pool,
        &SeedSignal {
            id: "c-single",
            target_id: "post-c",
            category: "nsfw",
            confidence: 75,
            appeal_status: "none",
            expires_at: None,
            persisted_at: "2026-09-15T10:00:00Z",
        },
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn dedupe_migration_compresses_duplicates_keeping_referenced_disputed_and_oldest()
-> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core dedupe migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database =
        TestDatabase::create(admin_url.as_str(), "cn_core_signal_dedupe_migration").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        migrate_postgres_up_to(&pool, FINGERPRINT_MIGRATION_VERSION).await?;
        seed_duplicates(&pool).await?;

        // 残りの migration（圧縮 + 部分 UNIQUE）を適用する。
        migrate_postgres(&pool).await?;

        let a = rows_for(&pool, "post-a").await?;
        let ids: Vec<&str> = a.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["a-cleared", "a-referenced"],
            "referenced row survives, cleared row untouched"
        );
        let survivor = &a[1];
        assert_eq!(
            survivor.persisted_at,
            "2026-09-15T10:00:00Z".parse::<chrono::DateTime<chrono::Utc>>()?,
            "oldest persisted_at is kept"
        );
        assert_eq!(
            survivor.appeal_status.as_deref(),
            Some("disputed"),
            "disputed state is kept"
        );
        assert_eq!(survivor.confidence, Some(91), "newest confidence is kept");
        assert!(survivor.expires_at.is_none());
        let cleared = &a[0];
        assert_eq!(cleared.confidence, Some(70));
        assert_eq!(cleared.appeal_status.as_deref(), Some("cleared"));

        let referencing: Option<String> = sqlx::query_scalar(
            "SELECT appeal_risk_signal_id FROM cn_admin.reports WHERE id = 'report-a'",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(referencing.as_deref(), Some("a-referenced"));

        let b = rows_for(&pool, "post-b").await?;
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].id, "b-oldest");
        assert_eq!(
            b[0].confidence,
            Some(65),
            "newest confidence merged into oldest row"
        );

        let c = rows_for(&pool, "post-c").await?;
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].id, "c-single");
        assert_eq!(c[0].confidence, Some(75));

        let index_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname = 'cn_safety'
               AND indexname = 'uq_cn_safety_risk_signals_active_key')",
        )
        .fetch_one(&pool)
        .await?;
        assert!(index_exists);
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn dedupe_migration_expires_extra_referenced_rows_instead_of_deleting() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core dedupe migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_signal_dedupe_refs").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        migrate_postgres_up_to(&pool, FINGERPRINT_MIGRATION_VERSION).await?;
        for (id, at) in [
            ("r-1", "2026-09-15T10:00:00Z"),
            ("r-2", "2026-09-15T10:05:00Z"),
        ] {
            seed_signal(
                &pool,
                &SeedSignal {
                    id,
                    target_id: "post-r",
                    category: "nsfw",
                    confidence: 80,
                    appeal_status: "none",
                    expires_at: None,
                    persisted_at: at,
                },
            )
            .await?;
        }
        seed_report_referencing(&pool, "report-r1", "r-1").await?;
        seed_report_referencing(&pool, "report-r2", "r-2").await?;

        migrate_postgres(&pool).await?;

        let rows = rows_for(&pool, "post-r").await?;
        assert_eq!(rows.len(), 2, "referenced rows are never deleted");
        assert!(
            rows[0].expires_at.is_none(),
            "oldest referenced row stays active"
        );
        assert!(
            rows[1].expires_at.is_some(),
            "second referenced row is expired"
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn dedupe_migration_is_idempotent() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core dedupe migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_signal_dedupe_idem").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        migrate_postgres_up_to(&pool, FINGERPRINT_MIGRATION_VERSION).await?;
        seed_duplicates(&pool).await?;
        migrate_postgres(&pool).await?;
        let before = sqlx::query_as::<_, SignalRow>(
            "SELECT id, confidence, appeal_status, expires_at, persisted_at
             FROM cn_safety.risk_signals ORDER BY id",
        )
        .fetch_all(&pool)
        .await?;

        sqlx::raw_sql(DEDUPE_MIGRATION_SQL).execute(&pool).await?;

        let after = sqlx::query_as::<_, SignalRow>(
            "SELECT id, confidence, appeal_status, expires_at, persisted_at
             FROM cn_safety.risk_signals ORDER BY id",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(after, before);
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn unique_index_blocks_second_active_row_but_allows_reissue_paths() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core dedupe migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_signal_unique").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        initialize_database(&pool).await?;
        let base = SeedSignal {
            id: "u-1",
            target_id: "post-u",
            category: "nsfw",
            confidence: 80,
            appeal_status: "none",
            expires_at: None,
            persisted_at: "2026-09-15T10:00:00Z",
        };
        seed_signal(&pool, &base).await?;

        // 同一鍵の 2 行目（活性）は拒否される。
        let duplicate = seed_signal(&pool, &SeedSignal { id: "u-2", ..base }).await;
        assert!(duplicate.is_err(), "partial unique index must reject a second active row");

        // appeal 審査の再発行: 旧行を cleared にしてから新行を挿入できる。
        sqlx::query("UPDATE cn_safety.risk_signals SET appeal_status = 'cleared' WHERE id = 'u-1'")
            .execute(&pool)
            .await?;
        seed_signal(&pool, &SeedSignal { id: "u-3", ..base }).await?;

        // cn-cli の再発行: 旧行を失効させてから新行を挿入できる。
        sqlx::query(
            "UPDATE cn_safety.risk_signals SET expires_at = '2026-09-15T11:00:00Z' WHERE id = 'u-3'",
        )
        .execute(&pool)
        .await?;
        seed_signal(&pool, &SeedSignal { id: "u-4", ..base }).await?;

        let rows = rows_for(&pool, "post-u").await?;
        assert_eq!(rows.len(), 3);
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}
