//! ingest pipeline（#413 / T5 / ADR 0025 §2.5 / §6.2）。
//!
//! 共有 replica に **実在する** post entry のみを対象に、post 本文 text を `cn-safety-runtime` の
//! `SafetyOrchestrator` で scan し、verdict が `allow`（`SafetyVerdict::is_indexable()`）の entry のみを
//! index 投影へ書く。以下を不変条件として守る:
//!   - ghost 注入を作らない: 対象は共有 replica の entry のみ（CN 直渡しは経路が無い。§6.2）。
//!   - fail-closed: unscanned / scan_failed / provider_unavailable / 非 allow は投影しない（§2.5）。
//!   - no permanent blob storage: blob は scan 用の一時 fetch のみで、投影に raw blob を入れない（§2.3）。
//!   - media は scan/tag pipeline へ渡す接続点まで（VLM タグ生成本体は #411）。
//!
//! docs replica からの entry 取得は `DocsSync`（`query_replica_with_policy`）越しに行うため、本番
//! （iroh-docs）でも in-memory（テスト）でも同じ pipeline を駆動できる。

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{debug, warn};

use kukuri_cn_core::IndexScopeKind;
use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety_runtime::SafetyOrchestrator;
use kukuri_core::{KukuriEnvelope, ObjectStatus, PayloadRef, ReplicaId};
use kukuri_docs_sync::{DocFetchPolicy, DocQuery, DocRecord, DocsSync, stable_key};

use crate::projection::{IndexProjection, IndexedEntry};

/// 単一 scope（topic / channel）を ingest した結果のサマリ（監査 / テスト用）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IngestSummary {
    /// 走査した object state entry 数。
    pub scanned: usize,
    /// `allow` verdict で投影へ書いた entry 数。
    pub indexed: usize,
    /// fail-closed（unscanned / scan_failed / 非 allow）で投影しなかった entry 数。
    pub skipped_non_allow: usize,
    /// tombstone / deleted で de-index した entry 数。
    pub deindexed: usize,
}

/// ingest pipeline。docs replica + safety orchestrator + index 投影を束ねる。
pub struct IngestPipeline {
    docs_sync: Arc<dyn DocsSync>,
    orchestrator: Arc<SafetyOrchestrator>,
    projection: Arc<dyn IndexProjection>,
}

impl IngestPipeline {
    pub fn new(
        docs_sync: Arc<dyn DocsSync>,
        orchestrator: Arc<SafetyOrchestrator>,
        projection: Arc<dyn IndexProjection>,
    ) -> Self {
        Self {
            docs_sync,
            orchestrator,
            projection,
        }
    }

    /// scope の共有 replica を走査し、実在する post entry のみを scan→allow 判定して投影へ反映する。
    ///
    /// replica id は scope から導出される共有 replica（public は `topic::<id>`、private は
    /// `channel::<id>`）。この関数は「共有 replica に実在する entry のみ」を対象にするため、CN へ
    /// 直接渡された（replica に存在しない）content を index する経路を持たない。
    pub async fn ingest_scope(
        &self,
        scope_kind: IndexScopeKind,
        scope_id: &str,
        replica_id: &ReplicaId,
    ) -> Result<IngestSummary> {
        // scope の replica を open してから走査する（未 open だと sync 対象にならない）。
        self.docs_sync.open_replica(replica_id).await?;

        // post object の state entry を prefix 走査する（`objects/<id>/state`）。
        let records = self
            .docs_sync
            .query_replica_with_policy(
                replica_id,
                DocQuery::Prefix(stable_key("objects", "")),
                DocFetchPolicy::LocalThenRemote,
            )
            .await
            .with_context(|| format!("failed to query replica {}", replica_id.as_str()))?;

        // 同一 prefix scan の envelope entry を object_id -> envelope record で index 化し、
        // blob text の本文取得で追加クエリ（N+1）を発生させないようにする。
        let mut envelopes: HashMap<String, DocRecord> = HashMap::new();
        let mut state_records: Vec<DocRecord> = Vec::new();
        for record in records {
            if let Some(object_id) = record.key.strip_suffix("/envelope") {
                if let Some(object_id) = object_id.strip_prefix("objects/") {
                    envelopes.insert(object_id.to_string(), record);
                }
            } else if record.key.ends_with("/state") {
                state_records.push(record);
            }
        }

        let mut summary = IngestSummary::default();
        for record in &state_records {
            summary.scanned += 1;
            match self
                .ingest_object_record(scope_kind, scope_id, replica_id, record, &envelopes)
                .await
            {
                Ok(IngestOutcome::Indexed) => summary.indexed += 1,
                Ok(IngestOutcome::SkippedNonAllow) => summary.skipped_non_allow += 1,
                Ok(IngestOutcome::Deindexed) => summary.deindexed += 1,
                Ok(IngestOutcome::Ignored) => {}
                Err(error) => {
                    // 単一 entry の失敗で scope 全体を止めない。fail-closed（投影しない）側に倒す。
                    warn!(
                        replica_id = %replica_id.as_str(),
                        key = %record.key,
                        error = %error,
                        "failed to ingest object record; skipping (fail-closed)"
                    );
                    summary.skipped_non_allow += 1;
                }
            }
        }
        Ok(summary)
    }

    async fn ingest_object_record(
        &self,
        scope_kind: IndexScopeKind,
        scope_id: &str,
        replica_id: &ReplicaId,
        record: &DocRecord,
        envelopes: &HashMap<String, DocRecord>,
    ) -> Result<IngestOutcome> {
        let object: PostObjectView = match serde_json::from_slice(&record.value) {
            Ok(object) => object,
            Err(error) => {
                debug!(key = %record.key, error = %error, "record is not a post object; ignoring");
                return Ok(IngestOutcome::Ignored);
            }
        };

        // tombstone / deleted は de-index する（replica 上で消えた content を投影に残さない）。
        if matches!(
            object.status,
            ObjectStatus::Deleted | ObjectStatus::Tombstoned
        ) {
            self.projection
                .remove_object(scope_kind, scope_id, object.object_id.as_str())
                .await?;
            return Ok(IngestOutcome::Deindexed);
        }

        // 本文 text を取り出す。blob 参照は scan 用の一時 fetch のみ（恒久保存しない）。
        let text = self
            .resolve_body_text(replica_id, &object, envelopes)
            .await?;

        // safety scan（fail-closed）。post 本文 text を orchestrator に渡す。
        let request = ProviderScanRequest::for_subject(SubjectKind::Post, object.object_id.clone())
            .with_text(text.clone());
        let report = self.orchestrator.scan_subject(&request).await;

        if !report.verdict.is_indexable() {
            // unscanned / scan_failed / provider_unavailable / 非 allow は投影しない。
            // 既に投影済みなら de-index する（後から verdict が変わった場合の整合）。
            self.projection
                .remove_object(scope_kind, scope_id, object.object_id.as_str())
                .await?;
            debug!(
                object_id = %object.object_id,
                reason = ?report.verdict.reason_code,
                "verdict is not allow; not indexing (fail-closed)"
            );
            return Ok(IngestOutcome::SkippedNonAllow);
        }

        let entry = IndexedEntry {
            scope_kind,
            scope_id: scope_id.to_string(),
            object_id: object.object_id.clone(),
            author_pubkey: object.author,
            text,
            created_at: object.created_at,
            source_replica_id: replica_id.as_str().to_string(),
        };
        self.projection.upsert_entry(&entry).await?;
        Ok(IngestOutcome::Indexed)
    }

    /// post 本文 text を取り出す。
    ///
    /// inline text はそのまま返す。blob text は同一 scope scan で取得済みの envelope（`envelopes`）から
    /// 本文を得る。envelope は scan 用途のみで、raw blob を恒久保存しない。envelope が無ければ空文字
    /// （scan は fail-closed 側に倒る）。同一 prefix scan の結果を再利用するため追加クエリを発生させない。
    async fn resolve_body_text(
        &self,
        _replica_id: &ReplicaId,
        object: &PostObjectView,
        envelopes: &HashMap<String, DocRecord>,
    ) -> Result<String> {
        match &object.payload_ref {
            PayloadRef::InlineText { text } => Ok(text.clone()),
            PayloadRef::BlobText { hash, .. } => {
                let Some(record) = envelopes.get(object.object_id.as_str()) else {
                    debug!(hash = %hash.as_str(), "blob text envelope missing; scanning empty body");
                    return Ok(String::new());
                };
                let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)
                    .context("failed to decode post envelope for blob text")?;
                // 共有 replica の entry が本物であることを署名検証する。
                envelope
                    .verify()
                    .context("post envelope failed verification")?;
                Ok(inline_text_from_envelope(&envelope).unwrap_or_default())
            }
        }
    }
}

enum IngestOutcome {
    Indexed,
    SkippedNonAllow,
    Deindexed,
    Ignored,
}

/// docs replica に保存された post object state の最小 view。
///
/// `object_persistence_support` の `CanonicalPostHeader`（= `KukuriPostObjectV1`）と同じ JSON を
/// 部分的に読む。cn-indexer は index に必要な最小フィールドのみを取り出す。
#[derive(Debug, serde::Deserialize)]
struct PostObjectView {
    object_id: String,
    author: String,
    created_at: i64,
    payload_ref: PayloadRef,
    #[serde(default)]
    status: ObjectStatus,
}

/// envelope から inline 本文 text を取り出す（blob text の scan 用フォールバック）。
fn inline_text_from_envelope(envelope: &KukuriEnvelope) -> Option<String> {
    let content = envelope.post_content().ok().flatten()?;
    match content.payload_ref {
        PayloadRef::InlineText { text } => Some(text),
        PayloadRef::BlobText { .. } => None,
    }
}
