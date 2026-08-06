//! DB非依存の safety scan service と構築境界。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;

use kukuri_cn_safety::provider::{ProviderScanRequest, SubjectKind};
use kukuri_cn_safety::{
    ModerationEventSigner, RiskSignalTarget, SafetyPolicy, SafetyProvider, SafetyRiskSignal,
    SafetyVerdict, SignedModerationEvent, Visibility, issue_signed_event,
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
    Ok(policy)
}

#[async_trait]
pub trait SafetyArtifactStore: Send + Sync {
    async fn persist_event(&self, event: &SignedModerationEvent) -> Result<()>;

    async fn persist_signal(
        &self,
        issuer_node_id: &str,
        signal: &SafetyRiskSignal,
        subject_author: Option<&str>,
    ) -> Result<String>;

    async fn persist_verdict(
        &self,
        subject_kind: SubjectKind,
        subject_id: &str,
        verdict: &SafetyVerdict,
    ) -> Result<String>;
}

#[derive(Debug, Default)]
pub struct MemorySafetyArtifactStore {
    events: Mutex<Vec<SignedModerationEvent>>,
    signals: Mutex<Vec<(String, SafetyRiskSignal)>>,
    signal_subject_authors: Mutex<Vec<(RiskSignalTarget, String, String)>>,
    verdicts: Mutex<HashMap<(String, String), (String, SafetyVerdict)>>,
}

impl MemorySafetyArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<SignedModerationEvent> {
        self.events.lock().expect("events mutex poisoned").clone()
    }

    pub fn signals(&self) -> Vec<(String, SafetyRiskSignal)> {
        self.signals.lock().expect("signals mutex poisoned").clone()
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
            .cloned()
    }

    pub fn verdict_by_id(&self, verdict_id: &str) -> Option<SafetyVerdict> {
        self.verdicts
            .lock()
            .expect("verdicts mutex poisoned")
            .values()
            .find(|(id, _)| id == verdict_id)
            .map(|(_, verdict)| verdict.clone())
    }
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
    ) -> Result<String> {
        let mut signals = self.signals.lock().expect("signals mutex poisoned");
        let id = format!("memory-signal-{}", signals.len() + 1);
        signals.push((issuer_node_id.to_string(), signal.clone()));
        if let Some(author) = subject_author {
            self.signal_subject_authors
                .lock()
                .expect("signal subject authors mutex poisoned")
                .push((signal.target, signal.target_id.clone(), author.to_string()));
        }
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
        let key = (subject_kind_key(subject_kind), subject_id.to_string());
        let next_id = format!("memory-verdict-{}", verdicts.len() + 1);
        let (id, stored) = verdicts
            .entry(key)
            .or_insert_with(|| (next_id, verdict.clone()));
        *stored = verdict.clone();
        Ok(id.clone())
    }
}

#[derive(Clone, Debug)]
pub struct SafetyScanOutcome {
    pub report: SafetyScanReport,
    pub signed_event: Option<SignedModerationEvent>,
    pub persisted_signal_id: Option<String>,
    pub verdict_id: Option<String>,
}

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

    pub async fn scan_and_record(
        &self,
        request: &ProviderScanRequest,
    ) -> Result<SafetyScanOutcome> {
        self.scan_and_record_inner(request, None).await
    }

    pub async fn scan_and_record_for_author(
        &self,
        request: &ProviderScanRequest,
        subject_author: &str,
    ) -> Result<SafetyScanOutcome> {
        if subject_author.trim().is_empty() {
            bail!("scan subject author must not be empty");
        }
        self.scan_and_record_inner(request, Some(subject_author))
            .await
    }

    async fn scan_and_record_inner(
        &self,
        request: &ProviderScanRequest,
        subject_author: Option<&str>,
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
                    .persist_signal(&self.issuer_node_id, signal, subject_author)
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
        Ok(SafetyScanService {
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
