//! DB非依存の safety scan service と構築境界。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;

use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety::{
    AdvisorySubjectKind, AppealStatus, Basis, ContentAdvisory, GeneralAction,
    ModerationEventSigner, RiskSignalTarget, SafetyCategory, SafetyPolicy, SafetyProvider,
    SafetyRiskSignal, SafetyVerdict, SignedModerationEvent, Visibility, issue_signed_event,
};

use crate::artifacts::risk_target_for;
use crate::content_cache::{
    ContentScanCoordinator, ContentScanStore, MemoryContentScanStore, content_scan_key,
};
use crate::reuse::{
    PersistedSignal, ReuseDecision, ReuseInputs, ScanDisposition, StoredVerdictRecord,
    VerdictPersistMeta, decide, verdict_changed,
};
use crate::{
    SAFETY_SIGNING_KEY_ENV, SafetyOrchestrator, SafetyScanReport, Secp256k1ModerationEventSigner,
    SystemScanClock, UuidEventIdGenerator,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SafetyRuntimeProvidersConfig {
    pub known_csam: Option<SafetyRuntimeProviderEntry>,
    pub general: Option<SafetyRuntimeProviderEntry>,
    pub unknown_csam: Option<SafetyRuntimeProviderEntry>,
}

impl SafetyRuntimeProvidersConfig {
    pub fn is_empty(&self) -> bool {
        self.known_csam.is_none() && self.general.is_none() && self.unknown_csam.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyRuntimeProviderEntry {
    pub provider: String,
    pub required: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SafetyRuntimeConfig {
    pub providers: SafetyRuntimeProvidersConfig,
    pub signing_key: Option<String>,
    pub emit_signed_events: bool,
    pub issuer_node_id: Option<String>,
    /// suspected 判定の classifier スコア閾値の operator override（1-100。ADR 0028 §2.2）。
    ///
    /// `None` なら `SafetyPolicy::public_node_default()` の既定（70）を使う。
    pub suspected_threshold: Option<u8>,
    /// suspected（`ClassifierScore`）advisory の配布 visibility の operator override
    /// （ADR 0028 §2.4 / §2.7）。`None` なら既定 `Local`。
    pub suspected_signal_visibility: Option<Visibility>,
    /// nsfw / objectionable の suspected に対する action の operator override
    /// （ADR 0028 §8.7）。`None` なら既定 `label`（advisory 付きで index）。
    pub general_action: Option<GeneralAction>,
}

impl Default for SafetyRuntimeConfig {
    fn default() -> Self {
        Self {
            providers: SafetyRuntimeProvidersConfig::default(),
            signing_key: None,
            emit_signed_events: true,
            issuer_node_id: None,
            suspected_threshold: None,
            suspected_signal_visibility: None,
            general_action: None,
        }
    }
}

impl std::fmt::Debug for SafetyRuntimeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SafetyRuntimeConfig")
            .field("providers", &self.providers)
            .field(
                "signing_key",
                &self.signing_key.as_ref().map(|_| "<redacted>"),
            )
            .field("emit_signed_events", &self.emit_signed_events)
            .field("issuer_node_id", &self.issuer_node_id)
            .field("suspected_threshold", &self.suspected_threshold)
            .field(
                "suspected_signal_visibility",
                &self.suspected_signal_visibility,
            )
            .field("general_action", &self.general_action)
            .finish()
    }
}

/// operator override を適用した router policy を組み立てる。
///
/// 既定は `SafetyPolicy::public_node_default()`（fail-closed 寄り）。閾値は 1-100 のみ受理する
/// （0 は「すべて suspected」で意図の取り違えが濃厚、100 超は u8 の範囲外の意図）。
pub fn resolve_safety_policy(config: &SafetyRuntimeConfig) -> Result<SafetyPolicy> {
    let mut policy = SafetyPolicy::public_node_default();
    if let Some(threshold) = config.suspected_threshold {
        if threshold == 0 || threshold > 100 {
            bail!(
                "safety suspected_threshold must be between 1 and 100 (got {threshold}); \
                 refusing to build a scan service (fail-closed)"
            );
        }
        policy.suspected_threshold = threshold;
    }
    if let Some(visibility) = config.suspected_signal_visibility {
        policy.suspected_signal_visibility = visibility;
    }
    if let Some(general_action) = config.general_action {
        policy.general_action = general_action;
    }
    Ok(policy)
}

/// ラベル付き allow の verdict から、index entry に同梱する content advisory を組み立てる
/// （ADR 0028 §8.6）。
///
/// advisory は risk signal（appeal の入口）と 1 対 1 で結ぶため `signal_id` を必須とする。
/// subject が post / blob 以外（user / peer）なら advisory は作らない。
pub fn content_advisories_for(
    verdict: &SafetyVerdict,
    subject_kind: SubjectKind,
    subject_id: &str,
    issuer_node_id: &str,
    signal_id: &str,
) -> Vec<ContentAdvisory> {
    if !verdict.is_labeled_allow() {
        return Vec::new();
    }
    let subject_kind = match subject_kind {
        SubjectKind::Post => AdvisorySubjectKind::PostId,
        SubjectKind::Blob => AdvisorySubjectKind::BlobCid,
        SubjectKind::User | SubjectKind::Peer => return Vec::new(),
    };
    verdict
        .advisory_labels
        .iter()
        .filter_map(|label| {
            let display = label.category.advisory_display_label()?;
            Some(ContentAdvisory {
                issuer_node_id: issuer_node_id.to_string(),
                subject_kind,
                subject_id: subject_id.to_string(),
                category: label.category,
                label: display.to_string(),
                confidence: label.confidence.or(verdict.confidence),
                signal_id: signal_id.to_string(),
                // 分類器由来の suspected。confirmed へ昇格しない（INVAR-3）。
                basis: Basis::ClassifierScore,
            })
        })
        .collect()
}

/// scan 結果（verdict / risk signal / signed event）の永続化境界。
///
/// #1050 以降は読み戻し（`load_verdict`）と集約（`persist_signal` の dedupe、
/// `attribute_subject_author`）も含む。実装は cn-core の Postgres store と、テスト用の
/// [`MemorySafetyArtifactStore`]。
#[async_trait]
pub trait SafetyArtifactStore: Send + Sync {
    fn content_store(&self) -> Option<Arc<dyn ContentScanStore>> {
        None
    }
    async fn persist_event(&self, event: &SignedModerationEvent) -> Result<()>;

    /// risk signal を保存する。
    ///
    /// 同一鍵 `(issuer_node_id, target, target_id, category, basis)` の活性 signal
    /// （`appeal_status != cleared` かつ `expires_at` 無し）が既にあれば新規行を作らず、
    /// severity / confidence / visibility を更新して既存 id を返す（`newly_created = false`）。
    /// 活性行が無く、失効していない `cleared` 行があれば、その判定を尊重して新規行を作らない。
    async fn persist_signal(
        &self,
        issuer_node_id: &str,
        signal: &SafetyRiskSignal,
        subject_author: Option<&str>,
    ) -> Result<PersistedSignal>;

    /// subject の最新 verdict を upsert する（対象ごと 1 行、id は据え置き）。
    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
        meta: &VerdictPersistMeta,
    ) -> Result<String>;

    /// subject の保存済み verdict を読み戻す（再利用判定の入力）。
    async fn load_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
    ) -> Result<Option<StoredVerdictRecord>>;

    /// subject の保存済み verdict 行の content advisory を差し替える（ADR 0028 §8.3）。
    ///
    /// indexer が post 本文と参照 blob の advisory の和集合を post 行へ確定させるために使う。
    /// 値が同じなら書かない。verdict 行が無ければ何もしない。
    async fn persist_advisories(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        advisories: &[ContentAdvisory],
    ) -> Result<()>;

    /// content target の risk signal を著者へ関連付ける（既にあれば何もしない）。
    ///
    /// 保存済み verdict を再利用したときも、共有 blob の 2 人目の著者が trust 入力から漏れない
    /// ようにするために使う。
    async fn attribute_subject_author(
        &self,
        target: RiskSignalTarget,
        target_id: &str,
        author: &str,
    ) -> Result<()>;

    /// 再 scan の現在の判定に無い advisory-only category の scanner 由来 signal を失効させる
    /// （#1109 / ADR 0028 §8.14）。失効させた件数を返す。
    ///
    /// 対象は issuer / target / target_id が一致し、category が nsfw / objectionable のうち
    /// `current_categories` に無く、basis が `ClassifierScore` で、未失効・`appeal_status` が
    /// `None`・operator 確定の印が無く、appeal 通報から参照されない行。`expires_at` を刻むだけで、
    /// 行の削除・他の列の変更はしない。
    async fn expire_superseded_advisory_signals(
        &self,
        issuer_node_id: &str,
        target: RiskSignalTarget,
        target_id: &str,
        current_categories: &[SafetyCategory],
        expires_at: &str,
    ) -> Result<u64>;
}

/// 失効対象になり得る advisory-only category のうち、現在の判定に無いもの（#1109）。
pub fn superseded_advisory_categories(
    current_categories: &[SafetyCategory],
) -> Vec<SafetyCategory> {
    [SafetyCategory::Nsfw, SafetyCategory::Objectionable]
        .into_iter()
        .filter(|category| category.is_advisory_only() && !current_categories.contains(category))
        .collect()
}

#[derive(Clone, Debug)]
struct MemorySignal {
    id: String,
    issuer_node_id: String,
    signal: SafetyRiskSignal,
}

#[derive(Debug, Default)]
pub struct MemorySafetyArtifactStore {
    content: Arc<MemoryContentScanStore>,
    events: Mutex<Vec<SignedModerationEvent>>,
    signals: Mutex<Vec<MemorySignal>>,
    signal_subject_authors: Mutex<Vec<(RiskSignalTarget, String, String)>>,
    verdicts: Mutex<HashMap<(String, String), StoredVerdictRecord>>,
}

impl MemorySafetyArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<SignedModerationEvent> {
        self.events.lock().expect("events mutex poisoned").clone()
    }

    /// 保存済み signal を `(issuer_node_id, signal)` で返す（保存順）。
    pub fn signals(&self) -> Vec<(String, SafetyRiskSignal)> {
        self.signals
            .lock()
            .expect("signals mutex poisoned")
            .iter()
            .map(|entry| (entry.issuer_node_id.clone(), entry.signal.clone()))
            .collect()
    }

    /// 保存済み signal を `(id, issuer_node_id, signal)` で返す（保存順）。
    pub fn signals_with_ids(&self) -> Vec<(String, String, SafetyRiskSignal)> {
        self.signals
            .lock()
            .expect("signals mutex poisoned")
            .iter()
            .map(|entry| {
                (
                    entry.id.clone(),
                    entry.issuer_node_id.clone(),
                    entry.signal.clone(),
                )
            })
            .collect()
    }

    /// テスト用: 保存済み signal の appeal 状態を差し替える（審査結果の再現）。
    pub fn set_signal_appeal_status(&self, signal_id: &str, status: AppealStatus) -> bool {
        let mut signals = self.signals.lock().expect("signals mutex poisoned");
        match signals.iter_mut().find(|entry| entry.id == signal_id) {
            Some(entry) => {
                entry.signal.appeal_status = Some(status);
                true
            }
            None => false,
        }
    }

    pub fn signal_subject_authors(&self) -> Vec<(RiskSignalTarget, String, String)> {
        self.signal_subject_authors
            .lock()
            .expect("signal subject authors mutex poisoned")
            .clone()
    }

    pub fn verdict_for(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
    ) -> Option<(String, SafetyVerdict)> {
        self.verdicts
            .lock()
            .expect("verdicts mutex poisoned")
            .get(&(subject_kind_key(subject_kind), subject_id.to_string()))
            .map(|record| (record.id.clone(), record.verdict.clone()))
    }

    /// 保存済み verdict を fingerprint / タグ込みで返す。
    pub fn stored_verdict_for(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
    ) -> Option<StoredVerdictRecord> {
        self.verdicts
            .lock()
            .expect("verdicts mutex poisoned")
            .get(&(subject_kind_key(subject_kind), subject_id.to_string()))
            .cloned()
    }

    pub fn verdict_by_id(&self, verdict_id: &str) -> Option<SafetyVerdict> {
        self.stored_verdict_by_id(verdict_id)
            .map(|record| record.verdict)
    }

    /// verdict record id から保存済み record（advisory 込み）を返す。
    pub fn stored_verdict_by_id(&self, verdict_id: &str) -> Option<StoredVerdictRecord> {
        self.verdicts
            .lock()
            .expect("verdicts mutex poisoned")
            .values()
            .find(|record| record.id == verdict_id)
            .cloned()
    }
}

fn same_signal_key(a: &SafetyRiskSignal, b: &SafetyRiskSignal) -> bool {
    a.target == b.target
        && a.target_id == b.target_id
        && a.category == b.category
        && a.basis == b.basis
}

fn is_active_signal(signal: &SafetyRiskSignal) -> bool {
    signal.appeal_status.unwrap_or_default() != AppealStatus::Cleared && signal.expires_at.is_none()
}

fn subject_kind_key(subject_kind: SubjectKind) -> String {
    match subject_kind {
        SubjectKind::Post => "post",
        SubjectKind::Blob => "blob",
        SubjectKind::User => "user",
        SubjectKind::Peer => "peer",
    }
    .to_string()
}

#[async_trait]
impl SafetyArtifactStore for MemorySafetyArtifactStore {
    fn content_store(&self) -> Option<Arc<dyn ContentScanStore>> {
        Some(self.content.clone())
    }
    async fn persist_event(&self, event: &SignedModerationEvent) -> Result<()> {
        self.events
            .lock()
            .expect("events mutex poisoned")
            .push(event.clone());
        Ok(())
    }

    async fn persist_signal(
        &self,
        issuer_node_id: &str,
        signal: &SafetyRiskSignal,
        subject_author: Option<&str>,
    ) -> Result<PersistedSignal> {
        if signal.target_id.trim().is_empty() {
            bail!("risk signal target_id must not be empty");
        }
        let persisted = {
            let mut signals = self.signals.lock().expect("signals mutex poisoned");
            let existing_active = signals.iter_mut().find(|entry| {
                entry.issuer_node_id == issuer_node_id
                    && same_signal_key(&entry.signal, signal)
                    && is_active_signal(&entry.signal)
            });
            if let Some(entry) = existing_active {
                entry.signal.severity = signal.severity;
                entry.signal.confidence = signal.confidence;
                entry.signal.visibility = signal.visibility;
                PersistedSignal {
                    id: entry.id.clone(),
                    newly_created: false,
                }
            } else if let Some(cleared) = signals.iter().rev().find(|entry| {
                entry.issuer_node_id == issuer_node_id
                    && same_signal_key(&entry.signal, signal)
                    && entry.signal.appeal_status == Some(AppealStatus::Cleared)
                    && entry.signal.expires_at.is_none()
            }) {
                PersistedSignal {
                    id: cleared.id.clone(),
                    newly_created: false,
                }
            } else {
                let id = format!("memory-signal-{}", signals.len() + 1);
                signals.push(MemorySignal {
                    id: id.clone(),
                    issuer_node_id: issuer_node_id.to_string(),
                    signal: signal.clone(),
                });
                PersistedSignal {
                    id,
                    newly_created: true,
                }
            }
        };
        if let Some(author) = subject_author {
            self.attribute_subject_author(signal.target, &signal.target_id, author)
                .await?;
        }
        Ok(persisted)
    }

    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
        meta: &VerdictPersistMeta,
    ) -> Result<String> {
        if subject_id.trim().is_empty() {
            bail!("scan verdict subject_id must not be empty");
        }
        let mut verdicts = self.verdicts.lock().expect("verdicts mutex poisoned");
        let key = (subject_kind_key(subject_kind), subject_id.to_string());
        let next_id = format!("memory-verdict-{}", verdicts.len() + 1);
        let record = verdicts.entry(key).or_insert_with(|| StoredVerdictRecord {
            id: next_id,
            verdict: verdict.clone(),
            derived_tags: Vec::new(),
            advisories: Vec::new(),
            source_fingerprint: None,
            scan_config_fingerprint: None,
        });
        record.verdict = verdict.clone();
        record.derived_tags = meta.derived_tags.clone();
        record.advisories = meta.advisories.clone();
        record.source_fingerprint = meta.source_fingerprint.clone();
        record.scan_config_fingerprint = meta.scan_config_fingerprint.clone();
        Ok(record.id.clone())
    }

    async fn persist_advisories(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        advisories: &[ContentAdvisory],
    ) -> Result<()> {
        let mut verdicts = self.verdicts.lock().expect("verdicts mutex poisoned");
        if let Some(record) =
            verdicts.get_mut(&(subject_kind_key(subject_kind), subject_id.to_string()))
            && record.advisories != advisories
        {
            record.advisories = advisories.to_vec();
        }
        Ok(())
    }

    async fn load_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
    ) -> Result<Option<StoredVerdictRecord>> {
        Ok(self.stored_verdict_for(subject_kind, subject_id))
    }

    async fn attribute_subject_author(
        &self,
        target: RiskSignalTarget,
        target_id: &str,
        author: &str,
    ) -> Result<()> {
        if author.trim().is_empty() {
            bail!("risk signal subject author must not be empty");
        }
        if !matches!(target, RiskSignalTarget::PostId | RiskSignalTarget::BlobCid) {
            bail!("only post_id/blob_cid risk signals can be attributed to an author");
        }
        let mut authors = self
            .signal_subject_authors
            .lock()
            .expect("signal subject authors mutex poisoned");
        let entry = (target, target_id.to_string(), author.to_string());
        if !authors.contains(&entry) {
            authors.push(entry);
        }
        Ok(())
    }

    /// memory store は operator 確定の印と appeal 通報を持たないため、`appeal_status` だけで保護する。
    async fn expire_superseded_advisory_signals(
        &self,
        issuer_node_id: &str,
        target: RiskSignalTarget,
        target_id: &str,
        current_categories: &[SafetyCategory],
        expires_at: &str,
    ) -> Result<u64> {
        let superseded = superseded_advisory_categories(current_categories);
        let mut signals = self.signals.lock().expect("signals mutex poisoned");
        let mut expired = 0;
        for entry in signals.iter_mut().filter(|entry| {
            entry.issuer_node_id == issuer_node_id
                && entry.signal.target == target
                && entry.signal.target_id == target_id
                && superseded.contains(&entry.signal.category)
                && entry.signal.basis == Basis::ClassifierScore
                && entry.signal.expires_at.is_none()
                && entry.signal.appeal_status.unwrap_or_default() == AppealStatus::None
        }) {
            entry.signal.expires_at = Some(expires_at.to_string());
            expired += 1;
        }
        Ok(expired)
    }
}

#[derive(Clone, Debug)]
pub struct SafetyScanOutcome {
    pub report: SafetyScanReport,
    pub signed_event: Option<SignedModerationEvent>,
    pub persisted_signal_id: Option<String>,
    pub verdict_id: Option<String>,
    /// provider を呼んだか、保存済み verdict を再利用したか（#1050）。
    pub disposition: ScanDisposition,
    /// この subject 自身の content advisory（ADR 0028 §8.6。ラベル付き allow のみ非空）。
    ///
    /// 再利用時は保存済み行から自 subject 分だけを復元する。post と参照 blob の和集合は
    /// indexer が `persist_advisories` で post 行へ確定させる。
    pub advisories: Vec<ContentAdvisory>,
}

pub struct SafetyScanService {
    content: Option<ContentScanCoordinator>,
    orchestrator: Arc<SafetyOrchestrator>,
    signer: Option<Arc<dyn ModerationEventSigner + Send + Sync>>,
    store: Arc<dyn SafetyArtifactStore>,
    issuer_node_id: String,
}

impl std::fmt::Debug for SafetyScanService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SafetyScanService")
            .field("issuer_node_id", &self.issuer_node_id)
            .field("has_signer", &self.signer.is_some())
            .finish_non_exhaustive()
    }
}

impl SafetyScanService {
    pub fn builder(
        orchestrator: Arc<SafetyOrchestrator>,
        store: Arc<dyn SafetyArtifactStore>,
    ) -> SafetyScanServiceBuilder {
        SafetyScanServiceBuilder {
            orchestrator,
            store,
            signer: None,
            unsigned_issuer: None,
        }
    }

    pub fn issuer_node_id(&self) -> &str {
        &self.issuer_node_id
    }

    pub fn moderation_metrics(&self) -> Option<Arc<kukuri_cn_safety::metrics::ModerationMetrics>> {
        self.orchestrator.moderation_metrics()
    }

    /// 構築時に確定した scan 構成の fingerprint（#1050）。
    pub fn scan_config_fingerprint(&self) -> &str {
        self.orchestrator.scan_config_fingerprint()
    }

    /// subject の verdict 行に content advisory の集合を確定させる（ADR 0028 §8.3）。
    ///
    /// indexer が post 本文と参照 blob の advisory の和集合を post 行へ書くために使う。
    pub async fn persist_advisories(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        advisories: &[ContentAdvisory],
    ) -> Result<()> {
        self.store
            .persist_advisories(subject_kind, subject_id, advisories)
            .await
            .context("failed to persist content advisories")
    }

    pub async fn scan_and_record(
        &self,
        request: &ProviderScanRequest,
    ) -> Result<SafetyScanOutcome> {
        self.scan_and_record_inner(request, None, None, None).await
    }

    pub async fn scan_and_record_for_author(
        &self,
        request: &ProviderScanRequest,
        subject_author: &str,
    ) -> Result<SafetyScanOutcome> {
        if subject_author.trim().is_empty() {
            bail!("scan subject author must not be empty");
        }
        self.scan_and_record_inner(request, Some(subject_author), None, None)
            .await
    }

    /// 保存済み verdict を再利用できるなら provider を呼ばずに返し、できなければ scan する
    /// （#1050）。
    ///
    /// `source_fingerprint` は subject の内容識別子（post = state レコードの content hash、
    /// blob = blob hash）。同subjectの再利用では保存済みverdictのidを返し、必要な著者関連付けを行う。
    /// 別subjectの共通内容cache hitではproviderを呼ばず、自subjectのartifactを生成する（#1060）。
    pub async fn scan_or_reuse(
        &self,
        request: &ProviderScanRequest,
        subject_author: Option<&str>,
        source_fingerprint: &str,
    ) -> Result<SafetyScanOutcome> {
        if subject_author.is_some_and(|author| author.trim().is_empty()) {
            bail!("scan subject author must not be empty");
        }
        if source_fingerprint.trim().is_empty() {
            bail!("scan source fingerprint must not be empty");
        }
        self.scan_and_record_inner(request, subject_author, Some(source_fingerprint), None)
            .await
    }

    pub async fn scan_or_reuse_guarded(
        &self,
        request: &ProviderScanRequest,
        subject_author: &str,
        source_fingerprint: &str,
        guard: &dyn crate::ScanReferenceGuard,
    ) -> Result<SafetyScanOutcome> {
        if subject_author.trim().is_empty() || source_fingerprint.trim().is_empty() {
            bail!("scan author and source fingerprint must be present");
        }
        self.scan_and_record_inner(
            request,
            Some(subject_author),
            Some(source_fingerprint),
            Some(guard),
        )
        .await
    }

    async fn scan_and_record_inner(
        &self,
        request: &ProviderScanRequest,
        subject_author: Option<&str>,
        source_fingerprint: Option<&str>,
        guard: Option<&dyn crate::ScanReferenceGuard>,
    ) -> Result<SafetyScanOutcome> {
        crate::reference_guard::check(guard).await?;
        let subject = match (request.subject_kind, request.subject_id.as_deref()) {
            (Some(kind), Some(subject_id)) if !subject_id.trim().is_empty() => {
                Some((kind, subject_id))
            }
            _ => None,
        };
        let stored = match subject {
            Some((kind, subject_id)) => self
                .store
                .load_verdict(kind, subject_id)
                .await
                .context("failed to load stored scan verdict")?,
            None => None,
        };

        if let (Some(source_fingerprint), Some((kind, subject_id))) = (source_fingerprint, subject)
        {
            let inputs = ReuseInputs {
                source_fingerprint,
                scan_config_fingerprint: self.orchestrator.scan_config_fingerprint(),
            };
            if let (ReuseDecision::Reuse, Some(stored)) =
                (decide(stored.as_ref(), &inputs), &stored)
            {
                crate::reference_guard::check(guard).await?;
                // risk signal を持つ verdict（非 allow、またはラベル付き allow）の再利用では、
                // 共有 subject の 2 人目以降の著者も trust 入力へ関連付ける（#1050 TR-9 / #1054）。
                if (!stored.verdict.is_indexable() || stored.verdict.is_labeled_allow())
                    && let Some(author) = subject_author
                {
                    self.store
                        .attribute_subject_author(risk_target_for(kind), subject_id, author)
                        .await
                        .context("failed to attribute reused risk signal to author")?;
                }
                return Ok(SafetyScanOutcome {
                    report: SafetyScanReport {
                        verdict: stored.verdict.clone(),
                        scan_results: Vec::new(),
                        moderation_event: None,
                        risk_signal: None,
                        derived_tags: stored.derived_tags.clone(),
                    },
                    signed_event: None,
                    persisted_signal_id: None,
                    verdict_id: Some(stored.id.clone()),
                    disposition: ScanDisposition::Reused,
                    advisories: own_advisories(&stored.advisories, kind, subject_id),
                });
            }
        }

        let key = source_fingerprint
            .filter(|_| self.orchestrator.supports_content_reuse())
            .and_then(|_| {
                content_scan_key(
                    request,
                    &self.issuer_node_id,
                    self.scan_config_fingerprint(),
                )
            });
        let (report, content_reused) = match (&self.content, key) {
            (Some(content), Some(key)) => {
                match content.scan(key, request, &self.orchestrator, guard).await {
                    Ok(result) => result,
                    Err(_) => (
                        self.orchestrator.failed_report(
                            request,
                            &kukuri_cn_safety::ScanError::Unavailable(
                                "shared content scan could not complete".into(),
                            ),
                        ),
                        false,
                    ),
                }
            }
            _ => (
                self.orchestrator.scan_subject_guarded(request, guard).await,
                false,
            ),
        };
        // risk signal を verdict より先に永続化する。ラベル付き allow の content advisory は
        // signal id（appeal の入口）を持つため、verdict 行へ書く前に id が要る（ADR 0028 §8.6）。
        crate::reference_guard::check(guard).await?;
        let recorded = crate::recording::record_signals(
            self.store.as_ref(),
            &report,
            request,
            &self.issuer_node_id,
            subject_author,
            guard,
        )
        .await?;
        crate::recording::expire_superseded_advisories(
            self.store.as_ref(),
            &report,
            subject,
            &self.issuer_node_id,
            guard,
        )
        .await?;
        let (verdict_id, advisories) = match subject {
            Some((kind, subject_id)) => {
                let advisories = recorded.advisories;
                let meta = VerdictPersistMeta {
                    source_fingerprint: source_fingerprint.map(str::to_string),
                    scan_config_fingerprint: Some(
                        self.orchestrator.scan_config_fingerprint().to_string(),
                    ),
                    derived_tags: report.derived_tags.clone(),
                    advisories: advisories.clone(),
                };
                crate::reference_guard::check(guard).await?;
                let verdict_id = self
                    .store
                    .persist_verdict(kind, subject_id, &report.verdict, &meta)
                    .await
                    .context("failed to persist scan verdict state")?;
                (Some(verdict_id), advisories)
            }
            None => (None, Vec::new()),
        };
        // signed moderation event は「新しい判定」の記録。signal を既存行へ集約し verdict も
        // 変わらない再 scan では発行しない（#1050 AC-2）。
        let should_emit_event = recorded.any_new
            || verdict_changed(stored.as_ref().map(|s| &s.verdict), &report.verdict);
        let signed_event = match (report.moderation_event.as_ref(), self.signer.as_ref()) {
            (Some(body), Some(signer)) if should_emit_event => {
                crate::reference_guard::check(guard).await?;
                let event = issue_signed_event(body.clone(), signer.as_ref());
                self.store
                    .persist_event(&event)
                    .await
                    .context("failed to persist signed moderation event")?;
                Some(event)
            }
            _ => None,
        };
        Ok(SafetyScanOutcome {
            report,
            signed_event,
            persisted_signal_id: recorded.primary.map(|signal| signal.id),
            verdict_id,
            disposition: if content_reused {
                ScanDisposition::Reused
            } else {
                ScanDisposition::Fresh
            },
            advisories,
        })
    }
}

/// 保存済み advisory（post 行は和集合）から、この subject 自身の分だけを取り出す。
fn own_advisories(
    stored: &[ContentAdvisory],
    subject_kind: SubjectKind,
    subject_id: &str,
) -> Vec<ContentAdvisory> {
    let own_kind = match subject_kind {
        SubjectKind::Post => AdvisorySubjectKind::PostId,
        SubjectKind::Blob => AdvisorySubjectKind::BlobCid,
        SubjectKind::User | SubjectKind::Peer => return Vec::new(),
    };
    stored
        .iter()
        .filter(|advisory| advisory.subject_kind == own_kind && advisory.subject_id == subject_id)
        .cloned()
        .collect()
}

pub struct SafetyScanServiceBuilder {
    orchestrator: Arc<SafetyOrchestrator>,
    store: Arc<dyn SafetyArtifactStore>,
    signer: Option<Arc<dyn ModerationEventSigner + Send + Sync>>,
    unsigned_issuer: Option<String>,
}

impl SafetyScanServiceBuilder {
    pub fn signer(mut self, signer: Arc<dyn ModerationEventSigner + Send + Sync>) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn without_signed_events(mut self, issuer_node_id: impl Into<String>) -> Self {
        self.unsigned_issuer = Some(issuer_node_id.into());
        self
    }

    pub fn build(self) -> Result<SafetyScanService> {
        let (signer, issuer_node_id) = match (self.signer, self.unsigned_issuer) {
            (Some(signer), None) => {
                let issuer = signer.issuer_node_id().to_string();
                (Some(signer), issuer)
            }
            (None, Some(issuer)) => {
                let issuer = issuer.trim().to_string();
                if issuer.is_empty() {
                    bail!("safety scan service issuer_node_id must not be empty");
                }
                (None, issuer)
            }
            (None, None) => bail!(
                "signed moderation events are enabled but no signer is configured (set \
                 {SAFETY_SIGNING_KEY_ENV}, or disable emission explicitly with \
                 without_signed_events)"
            ),
            (Some(_), Some(_)) => {
                bail!("safety scan service cannot both sign moderation events and disable emission")
            }
        };
        let content = self.store.content_store().map(ContentScanCoordinator::new);
        Ok(SafetyScanService {
            content,
            orchestrator: self.orchestrator,
            signer,
            store: self.store,
            issuer_node_id,
        })
    }
}

pub fn build_safety_scan_service(
    config: &SafetyRuntimeConfig,
    providers: Vec<Arc<dyn SafetyProvider>>,
    store: Arc<dyn SafetyArtifactStore>,
) -> Result<Option<SafetyScanService>> {
    if config.providers.is_empty() {
        if !providers.is_empty() {
            bail!("resolved safety providers do not match an empty runtime configuration");
        }
        return Ok(None);
    }
    if providers.is_empty() {
        bail!("no resolved safety providers; refusing to build a scan service (fail-closed)");
    }

    let signer = match config.signing_key.as_deref() {
        Some(secret) => Some(
            Secp256k1ModerationEventSigner::from_secret(secret)
                .context("invalid moderation event signing key")?,
        ),
        None => None,
    };
    let issuer = match (
        &signer,
        config.emit_signed_events,
        config.issuer_node_id.as_deref(),
    ) {
        (Some(signer), _, _) => signer.issuer_node_id().to_string(),
        (None, true, _) => bail!(
            "signed moderation events are enabled but no signing key is configured (set \
             {SAFETY_SIGNING_KEY_ENV} from Secret Manager, or disable \
             safety.events.emit_signed_moderation_events)"
        ),
        (None, false, Some(issuer)) if !issuer.trim().is_empty() => issuer.trim().to_string(),
        (None, false, _) => bail!(
            "signed moderation events are disabled and no issuer node id is available (set a \
             signing key or an explicit issuer node id)"
        ),
    };

    let policy = resolve_safety_policy(config)?;
    let mut orchestrator = SafetyOrchestrator::builder(
        &issuer,
        Arc::new(SystemScanClock::new()),
        Arc::new(UuidEventIdGenerator::new()),
    )
    .policy(policy);
    for provider in providers {
        orchestrator = orchestrator.provider(provider);
    }
    let orchestrator = Arc::new(
        orchestrator
            .build()
            .context("failed to build safety orchestrator")?,
    );
    let builder = SafetyScanService::builder(orchestrator, store);
    let service = match signer {
        Some(signer) if config.emit_signed_events => builder.signer(Arc::new(signer)).build()?,
        _ => builder.without_signed_events(issuer).build()?,
    };
    Ok(Some(service))
}
