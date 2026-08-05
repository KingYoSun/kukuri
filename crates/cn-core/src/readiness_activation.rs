//! 読み取り面の有効化記録（#616 T3）。
//!
//! `cn-cli readiness` が全項目合格を確認したときにのみ記録を追加し、cn-user-api は
//! 環境変数が真でも有効な記録が無ければ索引・信頼の読み取り面を公開しない。
//! 記録には評価時点の判定項目 id 集合を含め、判定基準が変わった後の古い記録は
//! 集合の不一致として無効に倒す（安全側）。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// 有効化記録 1 件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessActivation {
    pub activated_at: DateTime<Utc>,
    pub profile: String,
    /// 評価時点の判定項目 id の配列。
    pub check_ids: Vec<String>,
}

impl ReadinessActivation {
    /// 現行の判定項目集合と一致するか（順序は問わない集合一致）。
    ///
    /// 判定基準が変わった（項目が増減・改名した）後は、旧基準での合格記録を
    /// 有効化の根拠にしない。
    pub fn matches_check_ids(&self, expected: &[&str]) -> bool {
        let mut recorded: Vec<&str> = self.check_ids.iter().map(String::as_str).collect();
        let mut expected: Vec<&str> = expected.to_vec();
        recorded.sort_unstable();
        expected.sort_unstable();
        recorded == expected
    }
}

/// 全項目合格の有効化記録を追加する。
pub async fn record_readiness_activation(
    pool: &PgPool,
    activated_at: DateTime<Utc>,
    profile: &str,
    check_ids: &[&str],
    report: &serde_json::Value,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_admin.readiness_activations (activated_at, profile, check_ids, report) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(activated_at)
    .bind(profile)
    .bind(serde_json::json!(check_ids))
    .bind(report)
    .execute(pool)
    .await
    .context("failed to record the readiness activation")?;
    Ok(())
}

/// 最新の有効化記録を返す（無ければ None）。
pub async fn latest_readiness_activation(pool: &PgPool) -> Result<Option<ReadinessActivation>> {
    let row: Option<(DateTime<Utc>, String, serde_json::Value)> = sqlx::query_as(
        "SELECT activated_at, profile, check_ids FROM cn_admin.readiness_activations \
         ORDER BY activated_at DESC, id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .context("failed to fetch the latest readiness activation")?;
    Ok(
        row.map(|(activated_at, profile, check_ids)| ReadinessActivation {
            activated_at,
            profile,
            check_ids: check_ids
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn activation(check_ids: &[&str]) -> ReadinessActivation {
        ReadinessActivation {
            activated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            profile: "public-node".to_string(),
            check_ids: check_ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    #[test]
    fn matches_check_ids_is_order_insensitive_set_equality() {
        let recorded = activation(&["b", "a", "c"]);
        assert!(recorded.matches_check_ids(&["a", "b", "c"]));
        assert!(recorded.matches_check_ids(&["c", "b", "a"]));
        // 欠落・過剰・改名はいずれも不一致（旧基準の合格を有効化の根拠にしない）。
        assert!(!recorded.matches_check_ids(&["a", "b"]));
        assert!(!recorded.matches_check_ids(&["a", "b", "c", "d"]));
        assert!(!recorded.matches_check_ids(&["a", "b", "x"]));
    }
}
