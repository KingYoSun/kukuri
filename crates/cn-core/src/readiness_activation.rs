//! 読み取り面の有効化記録（#616 T3）。
//!
//! `cn-cli readiness` が全項目合格を確認したときにのみ記録を追加し、cn-user-api は
//! 環境変数が真でも有効な記録が無ければ索引・信頼の読み取り面を公開しない。
//! 記録には評価時点の判定項目 id 集合を含め、判定基準が変わった後の古い記録は
//! 集合の不一致として無効に倒す（安全側）。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

/// 有効化記録 1 件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessActivation {
    pub activated_at: DateTime<Utc>,
    pub profile: String,
    /// 評価時点の判定項目 id の配列。
    pub check_ids: Vec<String>,
    /// operator config と deployment revision を束ねた安全な指紋。
    pub context_fingerprint: String,
    /// readiness 不合格による明示的な失効記録か。
    pub revoked: bool,
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

    /// 現在の profile / deploy context / 判定基準 / 有効期限に対して有効か。
    pub fn is_valid(
        &self,
        expected_profile: &str,
        expected_check_ids: &[&str],
        expected_context_fingerprint: &str,
        now: DateTime<Utc>,
        max_age: chrono::Duration,
    ) -> bool {
        let age = now.signed_duration_since(self.activated_at);
        !self.revoked
            && self.profile == expected_profile
            && self.context_fingerprint == expected_context_fingerprint
            && self.matches_check_ids(expected_check_ids)
            && age >= chrono::Duration::zero()
            && age <= max_age
    }
}

/// profile、operator-config、image/deployment revision を同じ activation context に束ねる。
pub fn readiness_context_fingerprint(
    profile: &str,
    deployment_revision: &str,
    operator_config_yaml: &[u8],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"kukuri-readiness-activation-v1\0");
    hasher.update(profile.trim().as_bytes());
    hasher.update(b"\0");
    hasher.update(deployment_revision.trim().as_bytes());
    hasher.update(b"\0");
    hasher.update(operator_config_yaml);
    hex::encode(hasher.finalize())
}

/// 全項目合格の有効化記録を追加する。
pub async fn record_readiness_activation(
    pool: &PgPool,
    activated_at: DateTime<Utc>,
    profile: &str,
    check_ids: &[&str],
    context_fingerprint: &str,
    report: &serde_json::Value,
) -> Result<()> {
    let report = serde_json::json!({
        "context_fingerprint": context_fingerprint,
        "revoked": false,
        "checks": report,
    });
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

/// readiness 不合格をappend-onlyな失効記録として残す。
pub async fn record_readiness_revocation(
    pool: &PgPool,
    revoked_at: DateTime<Utc>,
    profile: &str,
    context_fingerprint: &str,
    reason: &str,
) -> Result<()> {
    let report = serde_json::json!({
        "context_fingerprint": context_fingerprint,
        "revoked": true,
        "reason": reason,
    });
    sqlx::query(
        "INSERT INTO cn_admin.readiness_activations (activated_at, profile, check_ids, report) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(revoked_at)
    .bind(profile)
    .bind(serde_json::json!([]))
    .bind(report)
    .execute(pool)
    .await
    .context("failed to record the readiness revocation")?;
    Ok(())
}

/// 最新の有効化記録を返す（無ければ None）。
pub async fn latest_readiness_activation(pool: &PgPool) -> Result<Option<ReadinessActivation>> {
    let row: Option<(DateTime<Utc>, String, serde_json::Value, serde_json::Value)> =
        sqlx::query_as(
            "SELECT activated_at, profile, check_ids, report FROM cn_admin.readiness_activations \
         ORDER BY activated_at DESC, id DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .context("failed to fetch the latest readiness activation")?;
    Ok(row.map(
        |(activated_at, profile, check_ids, report)| ReadinessActivation {
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
            context_fingerprint: report
                .get("context_fingerprint")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            revoked: report
                .get("revoked")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        },
    ))
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
            context_fingerprint: "deployment-v1".to_string(),
            revoked: false,
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

    #[test]
    fn validity_binds_profile_context_and_age() {
        let recorded = activation(&["a", "b"]);
        let now = recorded.activated_at + chrono::Duration::seconds(60);
        assert!(recorded.is_valid(
            "public-node",
            &["a", "b"],
            "deployment-v1",
            now,
            chrono::Duration::seconds(300),
        ));
        assert!(!recorded.is_valid(
            "private-node",
            &["a", "b"],
            "deployment-v1",
            now,
            chrono::Duration::seconds(300),
        ));
        assert!(!recorded.is_valid(
            "public-node",
            &["a", "b"],
            "deployment-v2",
            now,
            chrono::Duration::seconds(300),
        ));
        assert!(!recorded.is_valid(
            "public-node",
            &["a", "b"],
            "deployment-v1",
            recorded.activated_at + chrono::Duration::seconds(301),
            chrono::Duration::seconds(300),
        ));
    }

    #[test]
    fn revoked_activation_is_never_valid() {
        let mut recorded = activation(&["a"]);
        recorded.revoked = true;
        assert!(!recorded.is_valid(
            "public-node",
            &["a"],
            "deployment-v1",
            recorded.activated_at,
            chrono::Duration::seconds(300),
        ));
    }
}
