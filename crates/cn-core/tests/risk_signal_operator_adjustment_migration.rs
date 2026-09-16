//! #1058: operator 確定の印を追加する migration
//! （`202609160002_risk_signal_operator_adjustment.sql`）の Postgres integration テスト。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//! - 直前の schema（`202609160001`）まで適用した DB に risk signal と審査の監査記録を seed し、
//!   残りの migration を適用して、過去の審査による訂正行へ印と訂正前の category が復元されることを
//!   確認する（編集 → 再発行の連鎖を含む）。
//! - migration SQL の再実行で差分が出ないこと（冪等）。

use anyhow::Result;
use kukuri_cn_core::{TestDatabase, connect_postgres, migrate_postgres, migrate_postgres_up_to};
use sqlx::PgPool;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const ADVISORY_MIGRATION_VERSION: i64 = 202609160001;
const OPERATOR_ADJUSTMENT_MIGRATION_SQL: &str =
    include_str!("../migrations/202609160002_risk_signal_operator_adjustment.sql");
const ISSUER: &str = "issuer-node";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

async fn seed_signal(
    pool: &PgPool,
    id: &str,
    category: &str,
    appeal_status: &str,
    persisted_at: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_safety.risk_signals
            (id, issuer_node_id, target, target_id, category, severity, basis, visibility,
             confidence, expires_at, appeal_status, persisted_at)
         VALUES ($1, $2, 'post_id', $1, $3, 'high', 'classifier_score', 'local', 50, NULL, $4,
                 $5::timestamptz)",
    )
    .bind(id)
    .bind(ISSUER)
    .bind(category)
    .bind(appeal_status)
    .bind(persisted_at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_action(
    pool: &PgPool,
    id: &str,
    action: &str,
    target_id: &str,
    occurred_at: &str,
    before_category: &str,
    reissued_id: Option<&str>,
) -> Result<()> {
    let after = match reissued_id {
        Some(reissued_id) => serde_json::json!({
            "appeal_status": "cleared",
            "reissued_risk_signal": { "id": reissued_id },
        }),
        None => serde_json::json!({ "appeal_status": "disputed" }),
    };
    sqlx::query(
        "INSERT INTO cn_admin.operator_actions
            (id, occurred_at, actor, action, target_kind, target_id, before_json, after_json)
         VALUES ($1, $2::timestamptz, 'ops@kukuri.app', $3, 'risk_signal_appeal', $4, $5, $6)",
    )
    .bind(id)
    .bind(occurred_at)
    .bind(action)
    .bind(target_id)
    .bind(serde_json::json!({ "category": before_category }))
    .bind(after)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_history(pool: &PgPool) -> Result<()> {
    // 審査の編集で nsfw → spam に変えた行。
    seed_signal(pool, "edited", "spam", "disputed", "2026-09-10T00:00:00Z").await?;
    seed_action(
        pool,
        "act-edit",
        "appeal.edit_detection",
        "edited",
        "2026-09-11T00:00:00Z",
        "nsfw",
        None,
    )
    .await?;

    // 編集（nsfw → spam）の後に訂正版を再発行した連鎖。訂正版は元の nsfw を引き継ぐ。
    seed_signal(pool, "chain-old", "spam", "cleared", "2026-09-10T00:00:00Z").await?;
    seed_signal(pool, "chain-new", "spam", "none", "2026-09-12T00:00:00Z").await?;
    seed_action(
        pool,
        "act-chain-edit",
        "appeal.edit_detection",
        "chain-old",
        "2026-09-11T00:00:00Z",
        "nsfw",
        None,
    )
    .await?;
    seed_action(
        pool,
        "act-chain-reissue",
        "appeal.reissue_correction",
        "chain-old",
        "2026-09-12T00:00:00Z",
        "spam",
        Some("chain-new"),
    )
    .await?;

    // 認容（値を変えない審査）と、審査を経ていない行には印を付けない。
    seed_signal(pool, "accepted", "nsfw", "cleared", "2026-09-10T00:00:00Z").await?;
    seed_action(
        pool,
        "act-accept",
        "appeal.accept",
        "accepted",
        "2026-09-11T00:00:00Z",
        "nsfw",
        None,
    )
    .await?;
    seed_signal(pool, "plain", "nsfw", "none", "2026-09-10T00:00:00Z").await?;

    // 訂正版の行が既に無い再発行記録は無視する。
    seed_signal(
        pool,
        "orphan-old",
        "nsfw",
        "cleared",
        "2026-09-10T00:00:00Z",
    )
    .await?;
    seed_action(
        pool,
        "act-orphan",
        "appeal.reissue_correction",
        "orphan-old",
        "2026-09-11T00:00:00Z",
        "nsfw",
        Some("missing-row"),
    )
    .await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow, PartialEq)]
struct AdjustmentRow {
    id: String,
    operator_adjusted_at: Option<chrono::DateTime<chrono::Utc>>,
    operator_origin_category: Option<String>,
}

async fn adjustments(pool: &PgPool) -> Result<Vec<AdjustmentRow>> {
    Ok(sqlx::query_as::<_, AdjustmentRow>(
        "SELECT id, operator_adjusted_at, operator_origin_category
         FROM cn_safety.risk_signals ORDER BY id",
    )
    .fetch_all(pool)
    .await?)
}

fn at(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    Some(value.parse().expect("valid timestamp"))
}

#[tokio::test]
async fn operator_adjustment_migration_backfills_appeal_review_corrections() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core operator adjustment migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database =
        TestDatabase::create(admin_url.as_str(), "cn_core_operator_adjust_migration").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        migrate_postgres_up_to(&pool, ADVISORY_MIGRATION_VERSION).await?;
        seed_history(&pool).await?;
        migrate_postgres(&pool).await?;

        let rows = adjustments(&pool).await?;
        let expected = vec![
            AdjustmentRow {
                id: "accepted".to_string(),
                operator_adjusted_at: None,
                operator_origin_category: None,
            },
            AdjustmentRow {
                id: "chain-new".to_string(),
                operator_adjusted_at: at("2026-09-12T00:00:00Z"),
                operator_origin_category: Some("nsfw".to_string()),
            },
            AdjustmentRow {
                id: "chain-old".to_string(),
                operator_adjusted_at: at("2026-09-11T00:00:00Z"),
                operator_origin_category: Some("nsfw".to_string()),
            },
            AdjustmentRow {
                id: "edited".to_string(),
                operator_adjusted_at: at("2026-09-11T00:00:00Z"),
                operator_origin_category: Some("nsfw".to_string()),
            },
            AdjustmentRow {
                id: "orphan-old".to_string(),
                operator_adjusted_at: None,
                operator_origin_category: None,
            },
            AdjustmentRow {
                id: "plain".to_string(),
                operator_adjusted_at: None,
                operator_origin_category: None,
            },
        ];
        assert_eq!(rows, expected);

        // 印には訂正前の category が必ず伴う。
        let violation = sqlx::query(
            "UPDATE cn_safety.risk_signals SET operator_adjusted_at = NOW() WHERE id = 'plain'",
        )
        .execute(&pool)
        .await;
        assert!(violation.is_err(), "印だけの行は CHECK で拒否する");
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}

#[tokio::test]
async fn operator_adjustment_migration_is_idempotent() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!(
            "skipping cn-core operator adjustment migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1"
        );
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_core_operator_adjust_idem").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        migrate_postgres_up_to(&pool, ADVISORY_MIGRATION_VERSION).await?;
        seed_history(&pool).await?;
        migrate_postgres(&pool).await?;
        let before = adjustments(&pool).await?;

        sqlx::raw_sql(OPERATOR_ADJUSTMENT_MIGRATION_SQL)
            .execute(&pool)
            .await?;

        assert_eq!(adjustments(&pool).await?, before);
        Ok::<(), anyhow::Error>(())
    }
    .await;
    database.cleanup().await?;
    result
}
