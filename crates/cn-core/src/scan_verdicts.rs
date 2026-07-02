//! scan 対象ごとの最新 safety verdict state の永続化（#404）。
//!
//! signed moderation event / risk signal（#405）は非 allow 時のみ生成される artifact であり、
//! 「対象の最新判定そのもの」は残らなかった。ADR 0025 §2.5 は「index entry は対応する safety
//! verdict state を必ず伴う」ことを要求するため、`allow` を含む全 scan の最新 verdict を
//! `cn_safety.scan_verdicts` に対象ごと 1 行で保持する。
//!
//! 行は対象（subject_kind, subject_id）ごとに upsert し、id は初回採番のまま据え置く。これにより
//! index 真実源（`cn_index.index_entries`）からの FK 参照が常に最新 verdict を指し、query 境界の
//! fail-closed 再確認（join して現在の action / critical を見る）が成立する。

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};
use uuid::Uuid;

use kukuri_cn_safety::provider::SubjectKind;
use kukuri_cn_safety::{ReasonCode, SafetyAction, SafetyVerdict};

use crate::safety_events::{from_db_enum, to_db_enum};

/// 永続化された scan 対象の最新 verdict。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredScanVerdict {
    /// verdict record id（初回採番のまま据え置き。index entry の FK 参照先）。
    pub id: String,
    /// scan 対象種別。
    pub subject_kind: SubjectKind,
    /// scan 対象識別子。
    pub subject_id: String,
    /// 最終 action。
    pub action: SafetyAction,
    /// critical safety（CSAM / CSE / grooming）検知か。
    pub critical: bool,
    /// reason code。
    pub reason_code: ReasonCode,
    /// classifier confidence（0-100。任意）。
    pub confidence: Option<u8>,
    /// 判定した provider 名（fail-closed 経路では None のことがある）。
    pub provider: Option<String>,
    /// policy バージョン。
    pub policy_version: String,
    /// scan 時刻（RFC3339 文字列。`SafetyVerdict::scanned_at` の原文）。
    pub scanned_at: String,
    /// この行を最後に更新した時刻。
    pub updated_at: DateTime<Utc>,
}

impl StoredScanVerdict {
    /// この verdict が index / discovery / recommendation への surfacing を許すか。
    ///
    /// `SafetyAction::allows_indexing()`（単一判定点）に委譲する。
    pub fn is_indexable(&self) -> bool {
        self.action.allows_indexing()
    }
}

/// scan 対象の最新 verdict を upsert する（対象ごとに 1 行。id は初回採番のまま据え置き）。
pub async fn upsert_scan_verdict(
    pool: &PgPool,
    subject_kind: SubjectKind,
    subject_id: &str,
    verdict: &SafetyVerdict,
) -> Result<StoredScanVerdict> {
    if subject_id.trim().is_empty() {
        bail!("scan verdict subject_id must not be empty");
    }
    let id = Uuid::new_v4().to_string();
    let row = sqlx::query(
        "INSERT INTO cn_safety.scan_verdicts
            (id, subject_kind, subject_id, action, critical, reason_code, confidence, provider,
             policy_version, scanned_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (subject_kind, subject_id) DO UPDATE
         SET action = EXCLUDED.action,
             critical = EXCLUDED.critical,
             reason_code = EXCLUDED.reason_code,
             confidence = EXCLUDED.confidence,
             provider = EXCLUDED.provider,
             policy_version = EXCLUDED.policy_version,
             scanned_at = EXCLUDED.scanned_at,
             updated_at = NOW()
         RETURNING id, subject_kind, subject_id, action, critical, reason_code, confidence,
                   provider, policy_version, scanned_at, updated_at",
    )
    .bind(&id)
    .bind(to_db_enum(&subject_kind)?)
    .bind(subject_id)
    .bind(to_db_enum(&verdict.action)?)
    .bind(verdict.critical)
    .bind(to_db_enum(&verdict.reason_code)?)
    .bind(verdict.confidence.map(i16::from))
    .bind(verdict.provider.as_deref())
    .bind(&verdict.policy_version)
    .bind(&verdict.scanned_at)
    .fetch_one(pool)
    .await?;
    scan_verdict_from_row(&row)
}

/// scan 対象の最新 verdict を取得する。
pub async fn get_scan_verdict(
    pool: &PgPool,
    subject_kind: SubjectKind,
    subject_id: &str,
) -> Result<Option<StoredScanVerdict>> {
    let row = sqlx::query(
        "SELECT id, subject_kind, subject_id, action, critical, reason_code, confidence,
                provider, policy_version, scanned_at, updated_at
         FROM cn_safety.scan_verdicts
         WHERE subject_kind = $1 AND subject_id = $2",
    )
    .bind(to_db_enum(&subject_kind)?)
    .bind(subject_id)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(scan_verdict_from_row).transpose()
}

fn scan_verdict_from_row(row: &PgRow) -> Result<StoredScanVerdict> {
    let confidence: Option<i16> = row.try_get("confidence")?;
    Ok(StoredScanVerdict {
        id: row.try_get("id")?,
        subject_kind: from_db_enum("subject_kind", &row.try_get::<String, _>("subject_kind")?)?,
        subject_id: row.try_get("subject_id")?,
        action: from_db_enum("action", &row.try_get::<String, _>("action")?)?,
        critical: row.try_get("critical")?,
        reason_code: from_db_enum("reason_code", &row.try_get::<String, _>("reason_code")?)?,
        confidence: confidence.map(|v| v as u8),
        provider: row.try_get("provider")?,
        policy_version: row.try_get("policy_version")?,
        scanned_at: row.try_get("scanned_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
