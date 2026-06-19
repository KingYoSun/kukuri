//! community node の通報受信ストレージ（#370）。
//!
//! node は自分の authority scope 内の対象に対する通報のみ受理する（中央通報窓口ではない）。
//! 通報先の解決は client（#310）が provenance + manifest authority scope で行い、node 側は
//! 「report endpoint capability を有効化したかどうか」で受付可否を判断する。ここでは受理した
//! 通報の保存・参照のみを担う。
//!
//! reporter の identity / social graph は node-independent であり保持しない。明示的に入力された
//! 連絡先（任意）のみ保存する。

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};
use uuid::Uuid;

/// 受信直後の通報状態。
pub const COMMUNITY_NODE_REPORT_STATUS_RECEIVED: &str = "received";

/// 保存済みの通報レコード。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeReport {
    pub id: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub capability: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporter_contact: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// 新規通報の入力。reporter identity / social graph は受け取らない。
#[derive(Clone, Debug, Default)]
pub struct NewCommunityNodeReport {
    pub subject_kind: String,
    pub subject_id: String,
    pub capability: String,
    pub reason: String,
    pub details: Option<String>,
    pub reporter_contact: Option<String>,
}

/// 受信した通報を保存し、受付参照 ID を含むレコードを返す。
pub async fn insert_community_node_report(
    pool: &PgPool,
    input: &NewCommunityNodeReport,
) -> Result<CommunityNodeReport> {
    let id = Uuid::new_v4().to_string();
    let row = sqlx::query(
        "INSERT INTO cn_admin.reports
            (id, subject_kind, subject_id, capability, reason, details, reporter_contact, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, subject_kind, subject_id, capability, reason, details, reporter_contact, status, created_at",
    )
    .bind(&id)
    .bind(&input.subject_kind)
    .bind(&input.subject_id)
    .bind(&input.capability)
    .bind(&input.reason)
    .bind(&input.details)
    .bind(&input.reporter_contact)
    .bind(COMMUNITY_NODE_REPORT_STATUS_RECEIVED)
    .fetch_one(pool)
    .await?;
    report_from_row(&row)
}

/// 受信した通報を新着順で取得する（運営者の確認用）。
pub async fn list_community_node_reports(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<CommunityNodeReport>> {
    let rows = sqlx::query(
        "SELECT id, subject_kind, subject_id, capability, reason, details, reporter_contact, status, created_at
         FROM cn_admin.reports
         ORDER BY created_at DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    rows.iter().map(report_from_row).collect()
}

/// 単一の通報を ID で取得する。
pub async fn get_community_node_report(
    pool: &PgPool,
    id: &str,
) -> Result<Option<CommunityNodeReport>> {
    let row = sqlx::query(
        "SELECT id, subject_kind, subject_id, capability, reason, details, reporter_contact, status, created_at
         FROM cn_admin.reports
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(report_from_row).transpose()
}

fn report_from_row(row: &PgRow) -> Result<CommunityNodeReport> {
    Ok(CommunityNodeReport {
        id: row.try_get("id")?,
        subject_kind: row.try_get("subject_kind")?,
        subject_id: row.try_get("subject_id")?,
        capability: row.try_get("capability")?,
        reason: row.try_get("reason")?,
        details: row.try_get("details")?,
        reporter_contact: row.try_get("reporter_contact")?,
        status: row.try_get("status")?,
        created_at: row.try_get("created_at")?,
    })
}
