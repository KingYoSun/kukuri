//! ArcadeDB index 投影 adapter（#413 / T6 / ADR 0025 投影ストア + ADR 0026 §6.1）。
//!
//! index 投影を ArcadeDB（Lucene 全文検索付き multi-model DB）に置く。ADR 0026 §6.1 で relation
//! graph 用に採用する ArcadeDB に index 投影も相乗りさせ、FTS + graph を単一エンジンに統合する。
//! ベクトル検索は今回のスコープ外（ADR 0025 §4。画像類似は除外、全文のみ）。
//!
//! ArcadeDB の成熟した native crate が無いため HTTP API（`/api/v1/command`）経由で操作する。
//! ここは境界 `IndexProjection` の 1 実装であり、pipeline は trait 越しに扱う。ユーザー向け search
//! クエリ本体は #404 が載せる（本 adapter は upsert / 存在 read / de-index まで）。

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};

use kukuri_cn_core::IndexScopeKind;

use crate::config::ArcadeDbConfig;
use crate::projection::{IndexProjection, IndexedEntry};

/// index 投影 document の ArcadeDB type 名。
const ENTRY_TYPE: &str = "IndexedEntry";

/// ArcadeDB HTTP client 越しの index 投影。
pub struct ArcadeDbProjection {
    client: Client,
    config: ArcadeDbConfig,
}

impl ArcadeDbProjection {
    pub fn new(config: ArcadeDbConfig) -> Result<Self> {
        let client = Client::builder()
            .build()
            .context("failed to build ArcadeDB HTTP client")?;
        Ok(Self { client, config })
    }

    /// index 投影 schema（document type + unique index）を用意する。
    ///
    /// ArcadeDB の `CREATE ... IF NOT EXISTS` で冪等に作る。object_id は scope 内で一意なので
    /// (scope_kind, scope_id, object_id) に unique index を張り、upsert を成立させる。
    pub async fn ensure_schema(&self) -> Result<()> {
        self.command(
            "sql",
            &format!("CREATE DOCUMENT TYPE {ENTRY_TYPE} IF NOT EXISTS"),
        )
        .await?;
        for property in [
            "scope_kind STRING",
            "scope_id STRING",
            "object_id STRING",
            "author_pubkey STRING",
            "text STRING",
            "created_at LONG",
            "source_replica_id STRING",
        ] {
            self.command(
                "sql",
                &format!("CREATE PROPERTY {ENTRY_TYPE}.{property} IF NOT EXISTS"),
            )
            .await?;
        }
        // scope 内 object 一意の複合 index（upsert / 存在 read の access path）。
        self.command(
            "sql",
            &format!(
                "CREATE INDEX IF NOT EXISTS ON {ENTRY_TYPE} (scope_kind, scope_id, object_id) UNIQUE"
            ),
        )
        .await?;
        // 全文検索 index（Lucene）。ユーザー向け search は #404 が使う。
        self.command(
            "sql",
            &format!("CREATE INDEX IF NOT EXISTS ON {ENTRY_TYPE} (text) FULL_TEXT ENGINE LUCENE"),
        )
        .await?;
        Ok(())
    }

    /// ArcadeDB `/api/v1/command/<database>` を叩く。
    async fn command(&self, language: &str, command: &str) -> Result<Value> {
        self.command_with_params(language, command, json!({})).await
    }

    async fn command_with_params(
        &self,
        language: &str,
        command: &str,
        params: Value,
    ) -> Result<Value> {
        let url = format!(
            "{}/api/v1/command/{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.database
        );
        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .json(&json!({
                "language": language,
                "command": command,
                "params": params,
            }))
            .send()
            .await
            .context("failed to send ArcadeDB command")?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("ArcadeDB command failed ({status}): {body}");
        }
        response
            .json::<Value>()
            .await
            .context("failed to decode ArcadeDB response")
    }

    fn scope_kind_str(scope_kind: IndexScopeKind) -> &'static str {
        scope_kind.as_str()
    }
}

#[async_trait]
impl IndexProjection for ArcadeDbProjection {
    async fn upsert_entry(&self, entry: &IndexedEntry) -> Result<()> {
        // UPDATE ... UPSERT で (scope_kind, scope_id, object_id) 一意に冪等 upsert する。
        let command = format!(
            "UPDATE {ENTRY_TYPE} SET scope_kind = :scope_kind, scope_id = :scope_id, \
             object_id = :object_id, author_pubkey = :author_pubkey, text = :text, \
             created_at = :created_at, source_replica_id = :source_replica_id \
             UPSERT WHERE scope_kind = :scope_kind AND scope_id = :scope_id \
             AND object_id = :object_id"
        );
        self.command_with_params(
            "sql",
            &command,
            json!({
                "scope_kind": Self::scope_kind_str(entry.scope_kind),
                "scope_id": entry.scope_id,
                "object_id": entry.object_id,
                "author_pubkey": entry.author_pubkey,
                "text": entry.text,
                "created_at": entry.created_at,
                "source_replica_id": entry.source_replica_id,
            }),
        )
        .await?;
        Ok(())
    }

    async fn contains_object(
        &self,
        scope_kind: IndexScopeKind,
        scope_id: &str,
        object_id: &str,
    ) -> Result<bool> {
        let command = format!(
            "SELECT count(*) AS total FROM {ENTRY_TYPE} WHERE scope_kind = :scope_kind \
             AND scope_id = :scope_id AND object_id = :object_id"
        );
        let value = self
            .command_with_params(
                "sql",
                &command,
                json!({
                    "scope_kind": Self::scope_kind_str(scope_kind),
                    "scope_id": scope_id,
                    "object_id": object_id,
                }),
            )
            .await?;
        Ok(count_from_result(&value) > 0)
    }

    async fn count_scope(&self, scope_kind: IndexScopeKind, scope_id: &str) -> Result<usize> {
        let command = format!(
            "SELECT count(*) AS total FROM {ENTRY_TYPE} WHERE scope_kind = :scope_kind \
             AND scope_id = :scope_id"
        );
        let value = self
            .command_with_params(
                "sql",
                &command,
                json!({
                    "scope_kind": Self::scope_kind_str(scope_kind),
                    "scope_id": scope_id,
                }),
            )
            .await?;
        Ok(count_from_result(&value) as usize)
    }

    async fn remove_scope(&self, scope_kind: IndexScopeKind, scope_id: &str) -> Result<()> {
        let command = format!(
            "DELETE FROM {ENTRY_TYPE} WHERE scope_kind = :scope_kind AND scope_id = :scope_id"
        );
        self.command_with_params(
            "sql",
            &command,
            json!({
                "scope_kind": Self::scope_kind_str(scope_kind),
                "scope_id": scope_id,
            }),
        )
        .await?;
        Ok(())
    }

    async fn remove_object(
        &self,
        scope_kind: IndexScopeKind,
        scope_id: &str,
        object_id: &str,
    ) -> Result<()> {
        let command = format!(
            "DELETE FROM {ENTRY_TYPE} WHERE scope_kind = :scope_kind AND scope_id = :scope_id \
             AND object_id = :object_id"
        );
        self.command_with_params(
            "sql",
            &command,
            json!({
                "scope_kind": Self::scope_kind_str(scope_kind),
                "scope_id": scope_id,
                "object_id": object_id,
            }),
        )
        .await?;
        Ok(())
    }
}

/// ArcadeDB の `SELECT count(*) AS total` 応答（`{ "result": [{ "total": N }] }`）から件数を読む。
fn count_from_result(value: &Value) -> i64 {
    value
        .get("result")
        .and_then(|result| result.as_array())
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("total"))
        .and_then(|total| total.as_i64())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_from_result_reads_total() {
        let value = json!({ "result": [{ "total": 3 }] });
        assert_eq!(count_from_result(&value), 3);
    }

    #[test]
    fn count_from_result_defaults_to_zero() {
        assert_eq!(count_from_result(&json!({ "result": [] })), 0);
        assert_eq!(count_from_result(&json!({})), 0);
    }
}
