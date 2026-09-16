//! #1060: share completed content assessments, never subject-specific artifacts.
use anyhow::{Result, bail};
use async_trait::async_trait;
use kukuri_cn_safety::{ProviderScanRequest, ProviderScanResult, SubjectKind};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::Semaphore, time::Instant};

use crate::{SafetyOrchestrator, SafetyScanReport, is_reusable_verdict};

pub const CONTENT_SCAN_LEASE: Duration = Duration::from_secs(320);
const SCAN_DEADLINE: Duration = Duration::from_secs(300);

#[async_trait]
pub trait ContentScanStore: Send + Sync {
    async fn load(&self, key: &str) -> Result<Option<Vec<ProviderScanResult>>>;
    async fn claim(&self, key: &str, owner: &str, lease: Duration) -> Result<bool>;
    /// Atomically save a completed result only while this owner still owns the lease.
    async fn complete(
        &self,
        key: &str,
        owner: &str,
        results: &[ProviderScanResult],
    ) -> Result<bool>;
    async fn release(&self, key: &str, owner: &str) -> Result<()>;
}

#[derive(Default, Debug)]
pub struct MemoryContentScanStore {
    state: Mutex<MemoryState>,
}
#[derive(Default, Debug)]
struct MemoryState {
    results: HashMap<String, Vec<ProviderScanResult>>,
    claims: HashMap<String, (String, Instant)>,
}

#[async_trait]
impl ContentScanStore for MemoryContentScanStore {
    async fn load(&self, key: &str) -> Result<Option<Vec<ProviderScanResult>>> {
        Ok(self
            .state
            .lock()
            .expect("content cache mutex")
            .results
            .get(key)
            .cloned())
    }
    async fn claim(&self, key: &str, owner: &str, lease: Duration) -> Result<bool> {
        let mut state = self.state.lock().expect("content cache mutex");
        let now = Instant::now();
        state.claims.retain(|_, (_, expires)| *expires > now);
        if state.claims.contains_key(key) {
            return Ok(false);
        }
        state
            .claims
            .insert(key.to_owned(), (owner.to_owned(), now + lease));
        Ok(true)
    }
    async fn complete(
        &self,
        key: &str,
        owner: &str,
        results: &[ProviderScanResult],
    ) -> Result<bool> {
        let mut state = self.state.lock().expect("content cache mutex");
        if !state
            .claims
            .get(key)
            .is_some_and(|(current, expires)| current == owner && *expires > Instant::now())
        {
            return Ok(false);
        }
        state.results.insert(key.to_owned(), results.to_vec());
        state.claims.remove(key);
        Ok(true)
    }
    async fn release(&self, key: &str, owner: &str) -> Result<()> {
        let mut state = self.state.lock().expect("content cache mutex");
        if state
            .claims
            .get(key)
            .is_some_and(|(current, _)| current == owner)
        {
            state.claims.remove(key);
        }
        Ok(())
    }
}

/// IDs, authors, scopes and appeal status are intentionally not part of shared content.
/// The issuer and full scan configuration isolate nodes and classifier generations.
pub fn content_scan_key(
    request: &ProviderScanRequest,
    issuer: &str,
    configuration: &str,
) -> Option<String> {
    let (kind, identity) = match request.subject_kind? {
        SubjectKind::Post if request.media_hint.is_none() => (
            "text",
            hex::encode(Sha256::digest(request.text.as_deref()?.as_bytes())),
        ),
        SubjectKind::Blob if request.text.as_ref().is_none_or(|s| s.is_empty()) => {
            let hash = request.media_hint.as_deref()?;
            if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            ("blob", hash.to_ascii_lowercase())
        }
        _ => return None,
    };
    let fields = ["content-scan-v1", issuer, configuration, kind, &identity];
    Some(hex::encode(Sha256::digest(
        serde_json::to_vec(&fields).expect("content key fields"),
    )))
}

pub struct ContentScanCoordinator {
    store: Arc<dyn ContentScanStore>,
    queue: Semaphore,
}

impl ContentScanCoordinator {
    pub fn new(store: Arc<dyn ContentScanStore>) -> Self {
        Self {
            store,
            queue: Semaphore::new(16),
        }
    }

    /// The bool reports content reuse; subject-specific artifacts are rebuilt by the caller.
    pub async fn scan(
        &self,
        key: String,
        request: &ProviderScanRequest,
        orchestrator: &SafetyOrchestrator,
        guard: Option<&dyn crate::ScanReferenceGuard>,
    ) -> Result<(SafetyScanReport, bool)> {
        let _permit = self
            .queue
            .try_acquire()
            .map_err(|_| anyhow::anyhow!("content scan queue is full"))?;
        tokio::time::timeout(
            SCAN_DEADLINE,
            self.scan_inner(key, request, orchestrator, guard),
        )
        .await
        .map_err(|_| anyhow::anyhow!("content scan deadline exceeded"))?
    }

    async fn cached(
        &self,
        key: &str,
        request: &ProviderScanRequest,
        orchestrator: &SafetyOrchestrator,
    ) -> Result<Option<SafetyScanReport>> {
        let Some(results) = self.store.load(key).await? else {
            return Ok(None);
        };
        if !orchestrator.cached_results_match(request, &results) {
            return Ok(None);
        }
        let report = orchestrator.report_from_results(request, results);
        Ok(is_reusable_verdict(&report.verdict).then_some(report))
    }

    async fn scan_inner(
        &self,
        key: String,
        request: &ProviderScanRequest,
        orchestrator: &SafetyOrchestrator,
        guard: Option<&dyn crate::ScanReferenceGuard>,
    ) -> Result<(SafetyScanReport, bool)> {
        let owner = uuid::Uuid::new_v4().to_string();
        loop {
            crate::reference_guard::check(guard).await?;
            if let Some(report) = self.cached(&key, request, orchestrator).await? {
                return Ok((report, true));
            }
            if !self.store.claim(&key, &owner, CONTENT_SCAN_LEASE).await? {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            let mut claim = ClaimGuard {
                store: self.store.clone(),
                key: key.clone(),
                owner: owner.clone(),
                armed: true,
            };
            if let Some(report) = self.cached(&key, request, orchestrator).await? {
                claim.release().await?;
                return Ok((report, true));
            }
            let report = orchestrator.scan_subject_guarded(request, guard).await;
            if orchestrator.cached_results_match(request, &report.scan_results)
                && is_reusable_verdict(&report.verdict)
            {
                if !self
                    .store
                    .complete(&key, &owner, &report.scan_results)
                    .await?
                {
                    bail!("content scan lease expired before completion");
                }
                claim.armed = false;
            } else {
                claim.release().await?;
            }
            return Ok((report, false));
        }
    }
}

struct ClaimGuard {
    store: Arc<dyn ContentScanStore>,
    key: String,
    owner: String,
    armed: bool,
}
impl ClaimGuard {
    async fn release(&mut self) -> Result<()> {
        self.store.release(&self.key, &self.owner).await?;
        self.armed = false;
        Ok(())
    }
}
impl Drop for ClaimGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let (store, key, owner) = (self.store.clone(), self.key.clone(), self.owner.clone());
            runtime.spawn(async move {
                let _ = store.release(&key, &owner).await;
            });
        }
        // Process crashes/runtime shutdown are recovered by the persistent lease expiry.
    }
}
