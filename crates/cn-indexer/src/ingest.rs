//! ingest pipeline（#413 / T5 / ADR 0025 §2.5 / §6.2）。
//!
//! 共有 replica に **実在する** post entry のみを対象に、post 本文 text を `cn-core` の
//! `SafetyScanService`（#406。内部で `cn-safety-runtime` の `SafetyOrchestrator` を駆動し、
//! moderation artifact を署名・永続化する）で scan し、verdict が `allow`
//! （`SafetyVerdict::is_indexable()`）の entry のみを index 投影へ書く。以下を不変条件として守る:
//!   - ghost 注入を作らない: 対象は共有 replica の entry のみ（CN 直渡しは経路が無い。§6.2）。
//!   - fail-closed: unscanned / scan_failed / provider_unavailable / 非 allow は投影しない（§2.5）。
//!   - no permanent blob storage: blob は scan 用の一時 fetch のみで、投影に raw blob を入れない（§2.3）。
//!   - media 参照 post は media scan/tag pipeline 実装（#411）まで unscanned として de-index する。
//!
//! docs replica からの entry 取得は `DocsSync`（`query_replica_with_policy`）越しに行うため、本番
//! （iroh-docs）でも in-memory（テスト）でも同じ pipeline を駆動できる。
//!
//! 書き込みは二段（#404）: `allow` verdict の entry は ① index 真実源
//! （`IndexEntryStore`。Postgres の DB 制約が fail-closed を保証する）→ ② 全文検索投影
//! （`IndexProjection`。ArcadeDB）の順で書く。① が失敗したら ② は書かない（真実源に無い
//! entry は query 境界の突合で surfacing されないため、投影残留も安全側に倒れる）。
//! de-index は真実源 → 投影の順で両方から消す。

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tracing::{debug, warn};

use kukuri_cn_core::{IndexEntryStore, IndexScopeKind, NewIndexEntry};
use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety_runtime::SafetyScanService;
use kukuri_core::{AssetRef, KukuriEnvelope, ObjectStatus, PayloadRef, ReplicaId};
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

/// ingest pipeline。docs replica + safety scan service + index 投影を束ねる。
///
/// safety scan は `SafetyScanService`（#406）経由で行い、scan と同時に moderation artifact
/// （signed moderation event / risk signal）が署名・永続化される。verdict gate（allow のみ投影）
/// は従来どおり本 pipeline が担う。
pub struct IngestPipeline {
    docs_sync: Arc<dyn DocsSync>,
    safety: Arc<SafetyScanService>,
    entries: Arc<dyn IndexEntryStore>,
    projection: Arc<dyn IndexProjection>,
}

impl IngestPipeline {
    pub fn new(
        docs_sync: Arc<dyn DocsSync>,
        safety: Arc<SafetyScanService>,
        entries: Arc<dyn IndexEntryStore>,
        projection: Arc<dyn IndexProjection>,
    ) -> Self {
        Self {
            docs_sync,
            safety,
            entries,
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

        // tombstone / deleted は de-index する（replica 上で消えた content を真実源にも投影にも
        // 残さない）。
        if matches!(
            object.status,
            ObjectStatus::Deleted | ObjectStatus::Tombstoned
        ) {
            self.deindex_object(scope_kind, scope_id, object.object_id.as_str())
                .await?;
            return Ok(IngestOutcome::Deindexed);
        }

        if object.has_unscanned_media() {
            self.deindex_object(scope_kind, scope_id, object.object_id.as_str())
                .await?;
            debug!(
                object_id = %object.object_id,
                "post references media without a media safety scan; not indexing (fail-closed)"
            );
            return Ok(IngestOutcome::SkippedNonAllow);
        }

        // 本文 text を取り出す。blob 参照は scan 用の一時 fetch のみ（恒久保存しない）。
        let text = self
            .resolve_body_text(replica_id, &object, envelopes)
            .await?;

        // safety scan（fail-closed）。post 本文 text を scan service に渡す。生成された
        // moderation artifact（risk signal / signed event）は service が署名・永続化する（#406）。
        // 永続化失敗は `?` で呼び出し側の per-entry fail-closed（投影しない）に乗る。
        let request = ProviderScanRequest::for_subject(SubjectKind::Post, object.object_id.clone())
            .with_text(text.clone());
        let outcome = self.safety.scan_and_record(&request).await?;
        let report = &outcome.report;

        if !report.verdict.is_indexable() {
            // unscanned / scan_failed / provider_unavailable / 非 allow は index しない。
            // 既に index 済みなら真実源・投影の両方から de-index する（後から verdict が
            // 変わった場合の整合）。
            self.deindex_object(scope_kind, scope_id, object.object_id.as_str())
                .await?;
            debug!(
                object_id = %object.object_id,
                reason = ?report.verdict.reason_code,
                "verdict is not allow; not indexing (fail-closed)"
            );
            return Ok(IngestOutcome::SkippedNonAllow);
        }

        // ① index 真実源（Postgres）へ upsert する。verdict record への FK と CHECK 制約
        // （allow のみ / 非 critical のみ）が fail-closed を DB 層でも保証する（#404）。
        // subject を渡した scan は必ず verdict state を記録するため verdict_id は存在するはずだが、
        // 無ければ index しない（fail-closed）。
        let Some(verdict_id) = outcome.verdict_id.as_deref() else {
            bail!(
                "scan outcome for object `{}` has no verdict record; refusing to index (fail-closed)",
                object.object_id
            );
        };
        self.entries
            .upsert_entry(&NewIndexEntry {
                scope_kind,
                scope_id: scope_id.to_string(),
                object_id: object.object_id.clone(),
                author_pubkey: object.author.clone(),
                created_at: object.created_at,
                source_replica_id: replica_id.as_str().to_string(),
                verdict_id: verdict_id.to_string(),
                verdict_action: report.verdict.action.as_str().to_string(),
                critical: report.verdict.critical,
            })
            .await
            .context("failed to record index entry in the authoritative store")?;

        // ② 全文検索投影（ArcadeDB）へ upsert する。ここが失敗しても真実源には entry が残るが、
        // 投影に無い entry は検索に出ないだけで安全側（fail-closed）に倒れる。
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

    /// object を index 真実源 → 投影の順で両方から消す。
    ///
    /// 真実源を先に消すことで、投影側の削除が失敗して hit が残留しても query 境界の突合
    /// （真実源に無い hit は返さない）が即座に効く。
    async fn deindex_object(
        &self,
        scope_kind: IndexScopeKind,
        scope_id: &str,
        object_id: &str,
    ) -> Result<()> {
        self.entries
            .remove_entry(scope_kind, scope_id, object_id)
            .await?;
        self.projection
            .remove_object(scope_kind, scope_id, object_id)
            .await?;
        Ok(())
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
    attachments: Vec<AssetRef>,
    #[serde(default)]
    media_manifest_refs: Vec<String>,
    #[serde(default)]
    status: ObjectStatus,
}

impl PostObjectView {
    fn has_unscanned_media(&self) -> bool {
        !self.attachments.is_empty() || !self.media_manifest_refs.is_empty()
    }
}

/// envelope から inline 本文 text を取り出す（blob text の scan 用フォールバック）。
fn inline_text_from_envelope(envelope: &KukuriEnvelope) -> Option<String> {
    let content = envelope.post_content().ok().flatten()?;
    match content.payload_ref {
        PayloadRef::InlineText { text } => Some(text),
        PayloadRef::BlobText { .. } => None,
    }
}
