//! community node runtime の safety scan 結線（#406）。
//!
//! `cn-safety-runtime` の `SafetyOrchestrator`（scan → verdict → 未署名 artifact）と、
//! `cn-core` の永続化（`safety_events`、#405）を合成する runtime 境界。ここまでの実装では
//! orchestrator は verdict gate（`cn-indexer`）としてのみ使われ、`SafetyScanReport` の
//! moderation event / risk signal は破棄されていた。本モジュールが scan と同時に
//! artifact を署名・永続化し、risk signal を trust / relation reads（`trust_inputs`、#415）が
//! 消費できる状態にする。
//!
//! 構成要素:
//! - [`SafetyArtifactStore`] — artifact 永続化の抽象。本番は Postgres
//!   （[`PgSafetyArtifactStore`]）、contract test は in-memory
//!   （[`MemorySafetyArtifactStore`]）。
//! - [`SafetyScanService`] — scan → risk signal persist → moderation event 署名 / persist を
//!   合成する service。verdict の判定（allow / fail-closed / artifact 生成可否）は
//!   `cn-safety-runtime` の artifacts ガードレールに委ね、ここで再判断しない（単一判定点）。
//! - [`build_safety_orchestrator`] / [`build_safety_scan_service`] — 設定から
//!   `SystemScanClock` + `UuidEventIdGenerator` + provider 群で orchestrator / service を
//!   組み立てる構築境界。
//!
//! fail-closed の要:
//! - 未知の provider 名は `required` の真偽に依らず Err（黙って skip すると「unknown_csam を
//!   設定したつもりで未検査が素通りする」誤構成を隠すため）。
//! - signed moderation event の emit が有効（既定）なのに signer が無い構成は build Err
//!   （実行時 skip にしない）。
//! - provider が 1 つも無い構成では service を構築しない（unscanned を index しない現状と整合）。
//!
//! 設計の真実源:
//! - `docs/adr/0026-community-node-trust-relation-foundation.md` §2.7（risk signal の反映先契約）
//! - `docs/adr/0027-deterministic-moderation-critical-safety.md` §2.5 / §2.6（signed event / risk
//!   signal / visibility）

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sqlx::PgPool;

use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety::{
    ModerationEventSigner, SafetyPolicy, SafetyProvider, SafetyRiskSignal, SafetyVerdict,
    SignedModerationEvent, issue_signed_event,
};
use kukuri_cn_safety_runtime::{
    SAFETY_SIGNING_KEY_ENV, SafetyOrchestrator, SafetyScanReport, Secp256k1ModerationEventSigner,
    SystemScanClock, UuidEventIdGenerator,
};

use crate::safety_events::{persist_risk_signal, persist_signed_moderation_event, to_db_enum};
use crate::scan_verdicts::upsert_scan_verdict;

/// runtime 側の safety provider slot 設定。
///
/// operator config（`cn-operator` の `SafetyProvidersConfig`）と slot が 1:1 対応する。
/// operator → env 注入 → runtime 消費という既存の流儀（`COMMUNITY_NODE_SAFETY_SIGNING_KEY` と
/// 同じ経路）に合わせ、cn-operator には依存しない。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SafetyRuntimeProvidersConfig {
    /// 既知 CSAM hash match provider（ADR 0027 §2.7）。
    pub known_csam: Option<SafetyRuntimeProviderEntry>,
    /// 一般 moderation provider（nsfw / spam 等）。
    pub general: Option<SafetyRuntimeProviderEntry>,
    /// 未知 CSAM / CSE classifier provider（#411）。
    pub unknown_csam: Option<SafetyRuntimeProviderEntry>,
}

impl SafetyRuntimeProvidersConfig {
    /// provider が 1 つも設定されていないか。
    pub fn is_empty(&self) -> bool {
        self.known_csam.is_none() && self.general.is_none() && self.unknown_csam.is_none()
    }
}

/// provider slot 1 つ分の設定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafetyRuntimeProviderEntry {
    /// provider 実装名。現状解決できるのは `mock`（`safety-mock-provider` feature）のみで、
    /// 本番 provider 実装名は #391 / #411 で追加する。未知の名前は fail-closed で Err。
    pub provider: String,
    /// operator が readiness 上必須と宣言した provider か。現段階の runtime 構築では挙動を
    /// 変えない（未知名は required に依らず Err）が、operator config との 1:1 対応のため保持する。
    pub required: bool,
}

/// safety scan runtime の構成。
///
/// env の読み込みは呼び出し側（`cn-indexer` の config 層）が行い、ここには値を渡す。
/// `Debug` は手動実装で `signing_key` を秘匿する（誤ってログへ署名鍵を出さない）。
#[derive(Clone, PartialEq, Eq)]
pub struct SafetyRuntimeConfig {
    pub providers: SafetyRuntimeProvidersConfig,
    /// moderation event 署名鍵の値（`COMMUNITY_NODE_SAFETY_SIGNING_KEY` の中身）。
    pub signing_key: Option<String>,
    /// signed moderation event を発行するか（operator config の
    /// `safety.events.emit_signed_moderation_events` に対応。既定 true）。
    pub emit_signed_events: bool,
    /// `emit_signed_events = false` のときに risk signal の issuer として使う node id。
    /// 署名鍵がある場合は鍵から導出した公開鍵 hex を優先する。
    pub issuer_node_id: Option<String>,
}

impl Default for SafetyRuntimeConfig {
    fn default() -> Self {
        Self {
            providers: SafetyRuntimeProvidersConfig::default(),
            signing_key: None,
            emit_signed_events: true,
            issuer_node_id: None,
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
            .finish()
    }
}

/// safety artifact（signed moderation event / risk signal）永続化の抽象。
///
/// 本番は Postgres（`cn_safety` schema、#405）、contract test は in-memory 実装を差す。
#[async_trait]
pub trait SafetyArtifactStore: Send + Sync {
    /// 署名済み moderation event を保存する。
    async fn persist_event(&self, event: &SignedModerationEvent) -> Result<()>;

    /// risk signal を保存し、採番した id を返す。
    async fn persist_signal(
        &self,
        issuer_node_id: &str,
        signal: &SafetyRiskSignal,
    ) -> Result<String>;

    /// scan 対象の最新 verdict state を upsert し、verdict record id を返す（#404）。
    ///
    /// `allow` を含む全 verdict を対象ごとに 1 行で保持する。id は初回採番のまま据え置かれ、
    /// index 真実源（`cn_index.index_entries`）の FK 参照先になる。
    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
    ) -> Result<String>;
}

/// Postgres 実装。`safety_events`（#405）の persist API に委譲する。
#[derive(Clone, Debug)]
pub struct PgSafetyArtifactStore {
    pool: PgPool,
}

impl PgSafetyArtifactStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SafetyArtifactStore for PgSafetyArtifactStore {
    async fn persist_event(&self, event: &SignedModerationEvent) -> Result<()> {
        persist_signed_moderation_event(&self.pool, event)
            .await
            .map(|_| ())
    }

    async fn persist_signal(
        &self,
        issuer_node_id: &str,
        signal: &SafetyRiskSignal,
    ) -> Result<String> {
        persist_risk_signal(&self.pool, issuer_node_id, signal)
            .await
            .map(|stored| stored.id)
    }

    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
    ) -> Result<String> {
        upsert_scan_verdict(&self.pool, subject_kind, subject_id, verdict)
            .await
            .map(|stored| stored.id)
    }
}

/// contract test 用の in-memory 実装（`MemoryDocsSync` / `MemoryIndexProjection` と同じ流儀）。
///
/// 署名検証は行わない（Postgres 実装は persist 時に検証する）。テストは保存された event を
/// 読み出して `verify_signed_event` で検証する。
#[derive(Debug, Default)]
pub struct MemorySafetyArtifactStore {
    events: Mutex<Vec<SignedModerationEvent>>,
    signals: Mutex<Vec<(String, SafetyRiskSignal)>>,
    /// (subject_kind の snake_case, subject_id) → (verdict record id, 最新 verdict)。
    /// Postgres 実装と同じく id は初回採番のまま据え置く。
    verdicts: Mutex<HashMap<(String, String), (String, SafetyVerdict)>>,
}

impl MemorySafetyArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 保存済み signed moderation event のスナップショット。
    pub fn events(&self) -> Vec<SignedModerationEvent> {
        self.events.lock().expect("events mutex poisoned").clone()
    }

    /// 保存済み risk signal（issuer_node_id, signal）のスナップショット。
    pub fn signals(&self) -> Vec<(String, SafetyRiskSignal)> {
        self.signals.lock().expect("signals mutex poisoned").clone()
    }

    /// 保存済みの対象別最新 verdict（verdict record id, verdict）のスナップショット。
    pub fn verdict_for(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
    ) -> Option<(String, SafetyVerdict)> {
        let key = (
            memory_subject_kind_key(subject_kind),
            subject_id.to_string(),
        );
        self.verdicts
            .lock()
            .expect("verdicts mutex poisoned")
            .get(&key)
            .cloned()
    }

    /// verdict record id から最新 verdict を引く（Postgres 実装の verdict_id join に対応）。
    pub fn verdict_by_id(&self, verdict_id: &str) -> Option<SafetyVerdict> {
        self.verdicts
            .lock()
            .expect("verdicts mutex poisoned")
            .values()
            .find(|(id, _)| id == verdict_id)
            .map(|(_, verdict)| verdict.clone())
    }
}

/// `SubjectKind` を in-memory map の鍵（snake_case 文字列）に写す。
fn memory_subject_kind_key(subject_kind: SubjectKind) -> String {
    to_db_enum(&subject_kind).expect("SubjectKind serializes to a snake_case string")
}

#[async_trait]
impl SafetyArtifactStore for MemorySafetyArtifactStore {
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
    ) -> Result<String> {
        let mut signals = self.signals.lock().expect("signals mutex poisoned");
        let id = format!("memory-signal-{}", signals.len() + 1);
        signals.push((issuer_node_id.to_string(), signal.clone()));
        Ok(id)
    }

    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
    ) -> Result<String> {
        if subject_id.trim().is_empty() {
            bail!("scan verdict subject_id must not be empty");
        }
        let mut verdicts = self.verdicts.lock().expect("verdicts mutex poisoned");
        let key = (
            memory_subject_kind_key(subject_kind),
            subject_id.to_string(),
        );
        let next_id = format!("memory-verdict-{}", verdicts.len() + 1);
        let entry = verdicts.entry(key);
        let (id, stored) = entry.or_insert_with(|| (next_id, verdict.clone()));
        *stored = verdict.clone();
        Ok(id.clone())
    }
}

/// [`SafetyScanService::scan_and_record`] の結果。
///
/// verdict の判定（`is_indexable()` 等）は `report` を、永続化の結果は残りのフィールドを見る。
#[derive(Clone, Debug)]
pub struct SafetyScanOutcome {
    /// orchestrator の生 scan 結果（verdict / scan_results / 未署名 artifact）。
    pub report: SafetyScanReport,
    /// 署名して store に保存した moderation event（signer 未構成の場合は None）。
    pub signed_event: Option<SignedModerationEvent>,
    /// store が採番した risk signal id（signal が生成されなかった場合は None）。
    pub persisted_signal_id: Option<String>,
    /// store が upsert した対象別最新 verdict の record id（#404）。
    ///
    /// request に subject_kind / subject_id が無い場合は None。index 真実源
    /// （`cn_index.index_entries`）はこの id を FK で参照する。
    pub verdict_id: Option<String>,
}

/// scan → verdict → artifact 署名 / 永続化を合成する runtime 境界（#406）。
pub struct SafetyScanService {
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
    /// builder を開始する。signed moderation event の emit は既定で有効（signer 必須）。
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

    /// この service が risk signal / moderation event の issuer として使う node id。
    pub fn issuer_node_id(&self) -> &str {
        &self.issuer_node_id
    }

    /// scan して、生成された artifact を署名・永続化する。
    ///
    /// 1. `SafetyOrchestrator::scan_subject`（provider 失敗は fail-closed に写像済み）。
    /// 2. request に subject があれば、対象別の最新 verdict state を store へ upsert する（#404。
    ///    `allow` を含む。index 真実源の FK 参照先になる）。
    /// 3. risk signal があれば store へ保存する（署名対象ではないため signer の有無に依らない）。
    /// 4. moderation event body があり signer が構成されていれば、署名して store へ保存する。
    ///
    /// allow verdict / target 欠落 / operational fail-closed（scan_failed 等）で artifact を
    /// 作らない判断は `cn-safety-runtime` の artifacts ガードレールに委ねる（ここで再判断しない）。
    /// 永続化失敗は Err で返し、呼び出し側（ingest 等）の per-entry fail-closed に乗せる。
    pub async fn scan_and_record(
        &self,
        request: &ProviderScanRequest,
    ) -> Result<SafetyScanOutcome> {
        let report = self.orchestrator.scan_subject(request).await;

        let verdict_id = match (request.subject_kind, request.subject_id.as_deref()) {
            (Some(kind), Some(subject_id)) if !subject_id.trim().is_empty() => Some(
                self.store
                    .persist_verdict(kind, subject_id, &report.verdict)
                    .await
                    .context("failed to persist scan verdict state")?,
            ),
            _ => None,
        };

        let persisted_signal_id = match report.risk_signal.as_ref() {
            Some(signal) => Some(
                self.store
                    .persist_signal(&self.issuer_node_id, signal)
                    .await
                    .context("failed to persist safety risk signal")?,
            ),
            None => None,
        };

        let signed_event = match (report.moderation_event.as_ref(), self.signer.as_ref()) {
            (Some(body), Some(signer)) => {
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
            persisted_signal_id,
            verdict_id,
        })
    }
}

/// [`SafetyScanService`] の builder。
pub struct SafetyScanServiceBuilder {
    orchestrator: Arc<SafetyOrchestrator>,
    store: Arc<dyn SafetyArtifactStore>,
    signer: Option<Arc<dyn ModerationEventSigner + Send + Sync>>,
    unsigned_issuer: Option<String>,
}

impl SafetyScanServiceBuilder {
    /// moderation event の signer を設定する。issuer node id は signer から導出する。
    ///
    /// 注意: orchestrator の issuer_node_id（`ModerationEventBody.issuer_node_id` になる）と
    /// signer の公開鍵は一致している必要がある（`verify_signed_event` / persist 時の署名検証が
    /// 一致を要求する）。[`build_safety_scan_service`] は signer から issuer を導出して
    /// orchestrator を組むため、構造的に一致する。
    pub fn signer(mut self, signer: Arc<dyn ModerationEventSigner + Send + Sync>) -> Self {
        self.signer = Some(signer);
        self
    }

    /// signed moderation event の emit を明示的に無効化する（operator config
    /// `safety.events.emit_signed_moderation_events = false` に対応）。
    ///
    /// moderation event は署名・永続化されない（report には未署名 body が残る）。risk signal は
    /// 署名対象ではないため引き続き永続化され、その issuer として `issuer_node_id` を使う。
    pub fn without_signed_events(mut self, issuer_node_id: impl Into<String>) -> Self {
        self.unsigned_issuer = Some(issuer_node_id.into());
        self
    }

    /// 構成を検証して [`SafetyScanService`] を構築する。
    ///
    /// fail-closed: signed event emit が有効（既定）なのに signer が無い構成は Err にする
    /// （実行時に黙って署名を skip しない）。
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
                "signed moderation events are enabled but no signer is configured \
                 (set {SAFETY_SIGNING_KEY_ENV}, or disable emission explicitly with \
                 without_signed_events)"
            ),
            (Some(_), Some(_)) => {
                bail!("safety scan service cannot both sign moderation events and disable emission")
            }
        };

        Ok(SafetyScanService {
            orchestrator: self.orchestrator,
            signer,
            store: self.store,
            issuer_node_id,
        })
    }
}

/// 設定から `SystemScanClock` + `UuidEventIdGenerator` + provider 群で
/// [`SafetyOrchestrator`] を構築する（#406）。
///
/// fail-closed:
/// - 未知の provider 名は `required` の真偽に依らず Err（黙って skip しない）。
/// - provider が 1 つも無い構成は Err（orchestrator builder の NoProviders と同じ）。
///
/// policy 未指定なら `SafetyPolicy::public_node_default()`（orchestrator builder の既定）。
pub fn build_safety_orchestrator(
    issuer_node_id: &str,
    providers: &SafetyRuntimeProvidersConfig,
    policy: Option<SafetyPolicy>,
) -> Result<SafetyOrchestrator> {
    if providers.is_empty() {
        bail!(
            "no safety providers configured; refusing to build a scan orchestrator (fail-closed)"
        );
    }

    let mut builder = SafetyOrchestrator::builder(
        issuer_node_id,
        Arc::new(SystemScanClock::new()),
        Arc::new(UuidEventIdGenerator::new()),
    );
    let slots: [(&'static str, Option<&SafetyRuntimeProviderEntry>); 3] = [
        ("known_csam", providers.known_csam.as_ref()),
        ("general", providers.general.as_ref()),
        ("unknown_csam", providers.unknown_csam.as_ref()),
    ];
    for (slot, entry) in slots {
        let Some(entry) = entry else {
            continue;
        };
        builder = builder.provider(resolve_provider(slot, entry)?);
    }
    if let Some(policy) = policy {
        builder = builder.policy(policy);
    }
    builder
        .build()
        .context("failed to build safety orchestrator")
}

/// provider 実装名を実装に解決する。
///
/// 現状解決できるのは `mock`（`safety-mock-provider` feature 有効時）のみ。本番 provider
/// 実装名（#391 Project Arachnid Shield 等）はここに match arm を追加する。未知の名前は
/// `required` に依らず Err（fail-closed）。
fn resolve_provider(
    slot: &'static str,
    entry: &SafetyRuntimeProviderEntry,
) -> Result<Arc<dyn SafetyProvider>> {
    match entry.provider.trim() {
        #[cfg(feature = "safety-mock-provider")]
        "mock" => Ok(mock_provider_for_slot(slot)),
        other => bail!(
            "unknown safety provider `{other}` for slot `{slot}` \
             (fail-closed; production providers are added in #391 / #411)"
        ),
    }
}

/// slot に対応する capability を持つ mock provider を作る（test / 構成検証用）。
#[cfg(feature = "safety-mock-provider")]
fn mock_provider_for_slot(slot: &'static str) -> Arc<dyn SafetyProvider> {
    use kukuri_cn_safety::MockSafetyProvider;
    use kukuri_cn_safety::SafetyProviderCapability;

    let name = format!("mock-{slot}");
    let provider = match slot {
        "known_csam" => MockSafetyProvider::known_csam(name),
        "general" => MockSafetyProvider::with_capabilities(
            name,
            vec![
                SafetyProviderCapability::GeneralMediaModeration,
                SafetyProviderCapability::SpamAbuseModeration,
            ],
        ),
        // unknown_csam slot は未知 CSAM / CSE classifier（suspected 判定、#411）。
        _ => MockSafetyProvider::with_capabilities(
            name,
            vec![
                SafetyProviderCapability::NovelCsamImageClassifier,
                SafetyProviderCapability::CseTextClassifier,
            ],
        ),
    };
    Arc::new(provider)
}

/// 設定から [`SafetyScanService`] を構築する runtime 構築境界（#406）。
///
/// 戻り値:
/// - `Ok(None)` — safety provider が未構成。scan orchestration は構成されず、呼び出し側は
///   ingest を起動しない（unscanned を index しない fail-closed と整合）。
/// - `Ok(Some(service))` — 構成が有効で service を構築した。
/// - `Err` — provider があるのに構成が不正（未知 provider 名 / emit 有効なのに署名鍵なし等）。
///   黙って縮退せず起動を失敗させる。
pub fn build_safety_scan_service(
    config: &SafetyRuntimeConfig,
    store: Arc<dyn SafetyArtifactStore>,
) -> Result<Option<SafetyScanService>> {
    if config.providers.is_empty() {
        return Ok(None);
    }

    // 署名鍵があれば issuer は常に鍵の公開鍵 hex（emit 無効でも issuer 詐称を避ける）。
    let signer = match config.signing_key.as_deref() {
        Some(secret) => Some(
            Secp256k1ModerationEventSigner::from_secret(secret)
                .context("invalid moderation event signing key")?,
        ),
        None => None,
    };

    if config.emit_signed_events {
        let Some(signer) = signer else {
            bail!(
                "signed moderation events are enabled but no signing key is configured \
                 (set {SAFETY_SIGNING_KEY_ENV} from Secret Manager, or disable \
                 safety.events.emit_signed_moderation_events)"
            );
        };
        let issuer = signer.issuer_node_id().to_string();
        let orchestrator = build_safety_orchestrator(&issuer, &config.providers, None)?;
        let service = SafetyScanService::builder(Arc::new(orchestrator), store)
            .signer(Arc::new(signer))
            .build()?;
        return Ok(Some(service));
    }

    let issuer = match (&signer, config.issuer_node_id.as_deref()) {
        (Some(signer), _) => signer.issuer_node_id().to_string(),
        (None, Some(issuer)) if !issuer.trim().is_empty() => issuer.trim().to_string(),
        _ => bail!(
            "signed moderation events are disabled and no issuer node id is available \
             (set a signing key or an explicit issuer node id)"
        ),
    };
    let orchestrator = build_safety_orchestrator(&issuer, &config.providers, None)?;
    let service = SafetyScanService::builder(Arc::new(orchestrator), store)
        .without_signed_events(issuer)
        .build()?;
    Ok(Some(service))
}
