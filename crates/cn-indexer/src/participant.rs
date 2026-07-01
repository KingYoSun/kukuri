//! docs replica sync participant（#413 / T4 / ADR 0025 §6.2 / §6.3）。
//!
//! CN は現状 docs 非参加のため、cn-indexer が iroh-docs を駆動する常駐 participant を新設する。
//! 本モジュールは scope 管理 state（cn-core / Postgres）を真実源に、supported topic / 許可 channel の
//! 共有 replica を open して sync し、ingest pipeline を回し、supported 除外 / channel secret 失効時に
//! sync 停止 + de-index する制御を担う。
//!
//! ここでは `DocsSync` trait 越しに replica を扱うため、本番（iroh-docs）でも in-memory（テスト）でも
//! 同じ制御ロジックを駆動できる。実際の docs node 生成（`IrohDocsNode` / relay 設定）は起動側
//! （`runtime` / `main`）が行い、本モジュールへ `DocsSync` として注入する。

use std::sync::Arc;

use anyhow::Result;
use sqlx::postgres::PgPool;
use tracing::{info, warn};

use kukuri_cn_core::{
    ChannelSecretCipher, IndexScopeKind, list_channel_secrets, list_supported_topics,
};
use kukuri_core::ReplicaId;
use kukuri_docs_sync::{DocsSync, private_channel_replica_id, topic_replica_id};

use crate::ingest::{IngestPipeline, IngestSummary};
use crate::projection::IndexProjection;

/// scope（種別 + id）と、それが指す共有 replica id。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeReplica {
    pub kind: IndexScopeKind,
    pub id: String,
    pub replica_id: ReplicaId,
}

impl ScopeReplica {
    /// scope 種別 + id から共有 replica id を導出する（ADR 0025 §6.2 / §6.3）。
    ///
    /// - public topic: `topic::<id>`（`public_replica_secret` で open 可能）。
    /// - private channel: `channel::<id>`（登録 capability が必要）。
    pub fn from_scope(kind: IndexScopeKind, id: &str) -> Self {
        let replica_id = match kind {
            IndexScopeKind::PublicTopic => topic_replica_id(id),
            IndexScopeKind::PrivateChannel => private_channel_replica_id(id),
        };
        Self {
            kind,
            id: id.to_string(),
            replica_id,
        }
    }
}

/// docs replica sync participant の制御面。
pub struct IndexerParticipant {
    pool: PgPool,
    docs_sync: Arc<dyn DocsSync>,
    projection: Arc<dyn IndexProjection>,
    pipeline: IngestPipeline,
    channel_secret_cipher: ChannelSecretCipher,
}

impl IndexerParticipant {
    pub fn new(
        pool: PgPool,
        docs_sync: Arc<dyn DocsSync>,
        projection: Arc<dyn IndexProjection>,
        pipeline: IngestPipeline,
        channel_secret_cipher: ChannelSecretCipher,
    ) -> Self {
        Self {
            pool,
            docs_sync,
            projection,
            pipeline,
            channel_secret_cipher,
        }
    }

    /// 起動時 / 再起動時に scope 管理 state から replica を open して sync 復元する（E13）。
    ///
    /// supported topic（public / private channel）の replica を open し、private channel は登録済み
    /// capability（channel secret）を docs へ登録してから open する。secret 未登録の private channel は
    /// open せず warn する（secret 無しでは index しない）。
    pub async fn restore_scopes(&self) -> Result<Vec<ScopeReplica>> {
        // private channel の capability を先に docs へ登録する。
        let secrets = list_channel_secrets(&self.pool, &self.channel_secret_cipher).await?;
        for secret in &secrets {
            let replica_id = private_channel_replica_id(secret.channel_id.as_str());
            self.docs_sync
                .register_private_replica_secret(&replica_id, secret.namespace_secret_hex.as_str())
                .await?;
        }

        let mut opened = Vec::new();
        for supported in list_supported_topics(&self.pool).await? {
            let scope = ScopeReplica::from_scope(supported.kind, supported.id.as_str());
            // private channel は capability が登録されていなければ open しない。
            if scope.kind == IndexScopeKind::PrivateChannel
                && !secrets.iter().any(|secret| secret.channel_id == scope.id)
            {
                warn!(
                    channel_id = %scope.id,
                    "private channel is supported but has no registered capability; skipping (no secret, no index)"
                );
                continue;
            }
            match self.docs_sync.open_replica(&scope.replica_id).await {
                Ok(()) => {
                    info!(
                        kind = scope.kind.as_str(),
                        scope_id = %scope.id,
                        replica_id = %scope.replica_id.as_str(),
                        "opened supported replica for sync"
                    );
                    opened.push(scope);
                }
                Err(error) => {
                    warn!(
                        kind = scope.kind.as_str(),
                        scope_id = %scope.id,
                        error = %error,
                        "failed to open supported replica; skipping"
                    );
                }
            }
        }
        Ok(opened)
    }

    /// 単一 scope を ingest する（scan→allow→投影）。
    pub async fn ingest_scope(&self, scope: &ScopeReplica) -> Result<IngestSummary> {
        self.pipeline
            .ingest_scope(scope.kind, scope.id.as_str(), &scope.replica_id)
            .await
    }

    /// supported set 全体を 1 巡 ingest する。
    pub async fn ingest_all_supported(&self) -> Result<IngestSummary> {
        let scopes = self.restore_scopes().await?;
        let mut total = IngestSummary::default();
        for scope in scopes {
            match self.ingest_scope(&scope).await {
                Ok(summary) => {
                    total.scanned += summary.scanned;
                    total.indexed += summary.indexed;
                    total.skipped_non_allow += summary.skipped_non_allow;
                    total.deindexed += summary.deindexed;
                }
                Err(error) => warn!(
                    kind = scope.kind.as_str(),
                    scope_id = %scope.id,
                    error = %error,
                    "failed to ingest scope; continuing"
                ),
            }
        }
        Ok(total)
    }

    /// supported topic 除外時の sync 停止 + de-index（E2 / E5）。
    ///
    /// public topic の replica はここでは docs から secret を外せない（導出 secret のため）が、
    /// 投影を de-index することで検索面から消える。private channel は capability も外す。
    pub async fn stop_and_deindex_scope(&self, kind: IndexScopeKind, id: &str) -> Result<()> {
        let scope = ScopeReplica::from_scope(kind, id);
        if kind == IndexScopeKind::PrivateChannel {
            // capability を外して sync 停止する（remove_private_replica_secret が replica も閉じる）。
            self.docs_sync
                .remove_private_replica_secret(&scope.replica_id)
                .await?;
        }
        self.projection.remove_scope(kind, id).await?;
        info!(
            kind = kind.as_str(),
            scope_id = %id,
            "stopped sync and de-indexed scope"
        );
        Ok(())
    }

    /// channel secret 失効時の capability 除去 + sync 停止 + de-index（E4）。
    pub async fn revoke_channel_and_deindex(&self, channel_id: &str) -> Result<()> {
        self.stop_and_deindex_scope(IndexScopeKind::PrivateChannel, channel_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_topic_scope_maps_to_topic_replica() {
        let scope = ScopeReplica::from_scope(IndexScopeKind::PublicTopic, "rust");
        assert_eq!(scope.replica_id.as_str(), "topic::rust");
    }

    #[test]
    fn private_channel_scope_maps_to_channel_replica() {
        let scope = ScopeReplica::from_scope(IndexScopeKind::PrivateChannel, "secret-room");
        assert_eq!(scope.replica_id.as_str(), "channel::secret-room");
    }
}
