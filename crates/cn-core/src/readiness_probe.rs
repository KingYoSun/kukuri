//! プロバイダ疎通確認の期限付き保存（#616）。
//!
//! `cn-cli readiness` が外部プロバイダへの疎通確認結果を保存し、有効期限内の再実行では
//! 外部プロバイダを叩かずに使い回すための最小の保存層。判定と要約のみを持ち、
//! 資格情報の値・応答本文は保存しない（呼び出し側もそれらを渡さない契約）。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// 保存された疎通確認の結果 1 件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessProbeRecord {
    /// 疎通確認の対象 slot（known_csam / general / unknown_csam）。
    pub provider_slot: String,
    /// slot に構成されたプロバイダ実装名。
    pub provider: String,
    /// 判定（true = pass）。
    pub pass: bool,
    /// 人間向けの要約（秘匿情報を含めない）。
    pub detail: String,
    /// 疎通確認を実行した時刻。
    pub checked_at: DateTime<Utc>,
}

/// 疎通確認の結果を slot 単位で upsert する。
pub async fn upsert_readiness_probe(pool: &PgPool, record: &ReadinessProbeRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_admin.readiness_probe_cache \
             (provider_slot, provider, status, detail, checked_at) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (provider_slot) DO UPDATE SET \
             provider = EXCLUDED.provider, \
             status = EXCLUDED.status, \
             detail = EXCLUDED.detail, \
             checked_at = EXCLUDED.checked_at",
    )
    .bind(record.provider_slot.as_str())
    .bind(record.provider.as_str())
    .bind(if record.pass { "pass" } else { "fail" })
    .bind(record.detail.as_str())
    .bind(record.checked_at)
    .execute(pool)
    .await
    .context("failed to upsert the readiness probe cache")?;
    Ok(())
}

/// 保存済みの疎通確認結果を slot 順で返す。
pub async fn list_readiness_probes(pool: &PgPool) -> Result<Vec<ReadinessProbeRecord>> {
    let rows: Vec<(String, String, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT provider_slot, provider, status, detail, checked_at \
         FROM cn_admin.readiness_probe_cache ORDER BY provider_slot",
    )
    .fetch_all(pool)
    .await
    .context("failed to list the readiness probe cache")?;
    Ok(rows
        .into_iter()
        .map(
            |(provider_slot, provider, status, detail, checked_at)| ReadinessProbeRecord {
                provider_slot,
                provider,
                pass: status == "pass",
                detail,
                checked_at,
            },
        )
        .collect())
}
