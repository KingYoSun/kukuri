//! #1109: 再 scan 済み subject に残る advisory signal を失効させる migration
//! （`202609170003_expire_superseded_advisory_signals.sql`）の Postgres integration テスト。
//!
//! `KUKURI_CN_RUN_INTEGRATION_TESTS=1` のときだけ実 DB に接続して実行する。
//! - 直前の schema（`202609170002`）まで適用した DB に本番相当の行を seed し、残りの migration を
//!   適用して、現在の verdict に無い scanner 由来の advisory signal だけが失効することを確認する
//!   （`expire_superseded_advisory_signals_migration`、AC-5 / TR-7）。
//! - migration SQL の再実行で差分が出ないこと（冪等）。

use anyhow::Result;
use kukuri_cn_core::{TestDatabase, connect_postgres, migrate_postgres, migrate_postgres_up_to};
use sqlx::PgPool;

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";
const PREVIOUS_MIGRATION_VERSION: i64 = 202609170002;
const EXPIRY_MIGRATION_SQL: &str =
    include_str!("../migrations/202609170003_expire_superseded_advisory_signals.sql");
const ISSUER: &str = "issuer-node";
const SIGNAL_AT: &str = "2026-09-15T00:00:00Z";
const VERDICT_AT: &str = "2026-09-17T00:00:00Z";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
    )
}

struct Signal<'a> {
    id: &'a str,
    target: &'a str,
    target_id: &'a str,
    category: &'a str,
    basis: &'a str,
    appeal_status: &'a str,
    persisted_at: &'a str,
    operator_adjusted: bool,
}

impl<'a> Signal<'a> {
    fn post(id: &'a str, target_id: &'a str, category: &'a str) -> Self {
        Self {
            id,
            target: "post_id",
            target_id,
            category,
            basis: "classifier_score",
            appeal_status: "none",
            persisted_at: SIGNAL_AT,
            operator_adjusted: false,
        }
    }
}

async fn seed_signal(pool: &PgPool, signal: Signal<'_>) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_safety.risk_signals
            (id, issuer_node_id, target, target_id, category, severity, basis, visibility,
             confidence, expires_at, appeal_status, persisted_at, operator_adjusted_at,
             operator_origin_category)
         VALUES ($1, $2, $3, $4, $5, 'high', $6, 'local', 84, NULL, $7, $8::timestamptz,
                 CASE WHEN $9 THEN $8::timestamptz END, CASE WHEN $9 THEN $5 END)",
    )
    .bind(signal.id)
    .bind(ISSUER)
    .bind(signal.target)
    .bind(signal.target_id)
    .bind(signal.category)
    .bind(signal.basis)
    .bind(signal.appeal_status)
    .bind(signal.persisted_at)
    .bind(signal.operator_adjusted)
    .execute(pool)
    .await?;
    Ok(())
}

/// verdict 行。`advisories` は (subject_kind, subject_id, category) の自 subject 分・同梱分。
async fn seed_verdict(
    pool: &PgPool,
    subject_kind: &str,
    subject_id: &str,
    action: &str,
    advisories: &[(&str, &str, &str)],
) -> Result<()> {
    let advisory_labels: Vec<serde_json::Value> = advisories
        .iter()
        .map(|(kind, id, category)| {
            serde_json::json!({
                "issuer_node_id": ISSUER,
                "subject_kind": kind,
                "subject_id": id,
                "category": category,
                "label": if *category == "nsfw" { "adult" } else { "sensitive" },
                "confidence": 80,
                "signal_id": "any",
                "basis": "classifier_score",
            })
        })
        .collect();
    sqlx::query(
        "INSERT INTO cn_safety.scan_verdicts
            (id, subject_kind, subject_id, action, critical, reason_code, confidence, provider,
             policy_version, scanned_at, advisory_labels, updated_at)
         VALUES ($1, $2, $3, $4, false, 'general_moderation', NULL, 'openai',
                 '2026-09-public-node-v3', $5, $6, $5::timestamptz)",
    )
    .bind(format!("verdict-{subject_id}"))
    .bind(subject_kind)
    .bind(subject_id)
    .bind(action)
    .bind(VERDICT_AT)
    .bind(serde_json::Value::Array(advisory_labels))
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed(pool: &PgPool) -> Result<()> {
    // 本番の 2 件相当: 旧 VLM の exclude signal が残り、新構成の verdict は allow・advisory なし。
    seed_signal(pool, Signal::post("stale-post", "post-6f0b053b", "nsfw")).await?;
    seed_verdict(pool, "post", "post-6f0b053b", "allow", &[]).await?;
    seed_signal(
        pool,
        Signal {
            target: "blob_cid",
            ..Signal::post("stale-blob", "blob-4956168b", "objectionable")
        },
    )
    .await?;
    seed_verdict(pool, "blob", "blob-4956168b", "allow", &[]).await?;

    // 現在の verdict に残る category は失効させず、消えた category だけ失効させる。
    seed_signal(
        pool,
        Signal::post("kept-labeled", "post-labeled", "objectionable"),
    )
    .await?;
    seed_signal(pool, Signal::post("stale-labeled", "post-labeled", "nsfw")).await?;
    seed_verdict(
        pool,
        "post",
        "post-labeled",
        "allow",
        &[
            ("post_id", "post-labeled", "objectionable"),
            // post 行に同梱した参照 blob の nsfw は、この post の nsfw を支えない。
            ("blob_cid", "blob-other", "nsfw"),
        ],
    )
    .await?;

    // 非 allow・verdict 無し・verdict より新しい signal は触らない。
    seed_signal(pool, Signal::post("kept-excluded", "post-excluded", "nsfw")).await?;
    seed_verdict(pool, "post", "post-excluded", "exclude", &[]).await?;
    seed_signal(
        pool,
        Signal::post("kept-no-verdict", "post-no-verdict", "nsfw"),
    )
    .await?;
    seed_signal(
        pool,
        Signal {
            persisted_at: "2026-09-18T00:00:00Z",
            ..Signal::post("kept-newer", "post-newer", "nsfw")
        },
    )
    .await?;
    seed_verdict(pool, "post", "post-newer", "allow", &[]).await?;

    // 保護対象: appeal 中・認容済み・operator 確定・棄却済み、advisory-only 以外。
    for (id, target_id, status) in [
        ("kept-disputed", "post-disputed", "disputed"),
        ("kept-cleared", "post-cleared", "cleared"),
    ] {
        seed_signal(
            pool,
            Signal {
                appeal_status: status,
                ..Signal::post(id, target_id, "nsfw")
            },
        )
        .await?;
        seed_verdict(pool, "post", target_id, "allow", &[]).await?;
    }
    seed_signal(
        pool,
        Signal {
            operator_adjusted: true,
            ..Signal::post("kept-adjusted", "post-adjusted", "nsfw")
        },
    )
    .await?;
    seed_verdict(pool, "post", "post-adjusted", "allow", &[]).await?;
    seed_signal(pool, Signal::post("kept-rejected", "post-rejected", "nsfw")).await?;
    seed_verdict(pool, "post", "post-rejected", "allow", &[]).await?;
    sqlx::query(
        "INSERT INTO cn_admin.reports
            (id, subject_kind, subject_id, capability, reason, appeal_risk_signal_id, status)
         VALUES ('report-rejected', 'post', 'post-rejected', 'moderation', 'false_positive',
                 'kept-rejected', 'actioned')",
    )
    .execute(pool)
    .await?;
    seed_signal(pool, Signal::post("kept-spam", "post-spam", "spam")).await?;
    seed_verdict(pool, "post", "post-spam", "allow", &[]).await?;
    seed_signal(
        pool,
        Signal {
            basis: "confirmed",
            ..Signal::post("kept-confirmed", "post-confirmed", "nsfw")
        },
    )
    .await?;
    seed_verdict(pool, "post", "post-confirmed", "allow", &[]).await?;
    Ok(())
}

/// (id, expires_at) を id 順で返す。signal 以外の列が変わらないことも併せて見る。
async fn snapshot(pool: &PgPool) -> Result<Vec<(String, Option<String>, String, String)>> {
    Ok(sqlx::query_as(
        "SELECT id, expires_at, COALESCE(appeal_status, ''), severity
         FROM cn_safety.risk_signals ORDER BY id",
    )
    .fetch_all(pool)
    .await?)
}

#[tokio::test]
async fn expire_superseded_advisory_signals_migration() -> Result<()> {
    let Some(admin_url) = integration_test_admin_database_url() else {
        eprintln!("skipping #1109 migration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database = TestDatabase::create(admin_url.as_str(), "cn_1109_migration").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;
    let result = async {
        migrate_postgres_up_to(&pool, PREVIOUS_MIGRATION_VERSION).await?;
        seed(&pool).await?;
        let events_before: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM cn_safety.signed_moderation_events")
                .fetch_one(&pool)
                .await?;

        migrate_postgres(&pool).await?;

        let after = snapshot(&pool).await?;
        assert_eq!(after.len(), 13, "no row is deleted or added");
        for (id, expires_at, _, severity) in &after {
            assert_eq!(
                expires_at.is_some(),
                id.starts_with("stale-"),
                "unexpected expiry state for {id}"
            );
            assert_eq!(severity, "high");
            if let Some(expires_at) = expires_at {
                chrono::DateTime::parse_from_rfc3339(expires_at)?;
            }
        }
        let statuses: Vec<_> = after
            .iter()
            .map(|(id, _, status, _)| (id.as_str(), status.as_str()))
            .filter(|(id, _)| ["kept-disputed", "kept-cleared"].contains(id))
            .collect();
        assert_eq!(
            statuses,
            vec![("kept-cleared", "cleared"), ("kept-disputed", "disputed")]
        );
        let events_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM cn_safety.signed_moderation_events")
                .fetch_one(&pool)
                .await?;
        assert_eq!(events_after, events_before);

        // 冪等: 再実行しても失効時刻を含めて差分が出ない。
        sqlx::raw_sql(EXPIRY_MIGRATION_SQL).execute(&pool).await?;
        assert_eq!(snapshot(&pool).await?, after);
        Ok::<(), anyhow::Error>(())
    }
    .await;
    pool.close().await;
    database.cleanup().await?;
    result
}
