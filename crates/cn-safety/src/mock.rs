//! deterministic な mock 実装（#353）。
//!
//! production provider / credentials に依存せず、policy router と fail-closed 挙動を
//! 決定論的にテストするための mock。
//!
//! - [`MockSafetyProvider`] — subject_id ごとに返す scan 結果を事前設定できる provider。
//! - [`MockSigner`] — canonical body の決定論的ハッシュを署名として返す signer。実鍵（secp256k1）は
//!   使わない。

use std::collections::HashMap;

use async_trait::async_trait;

use crate::capability::SafetyProviderCapability;
use crate::event::{ModerationEventBody, ModerationEventSigner};
use crate::provider::{
    ProviderScanRequest, ProviderScanResult, SafetyProvider, ScanError, ScanOutcome,
};
use crate::verdict::SafetyLabel;

/// subject_id ごとに決定論的な結果を返す mock provider。
///
/// 設定が無い subject_id に対しては既定の振る舞い（`default_outcome`）を返す。
#[derive(Clone, Debug)]
pub struct MockSafetyProvider {
    name: String,
    capabilities: Vec<SafetyProviderCapability>,
    results: HashMap<String, ProviderScanResult>,
    /// 設定外 subject に対する既定の outcome。fail-closed テストのため `Unavailable` 等も選べる。
    default_outcome: DefaultOutcome,
}

#[derive(Clone, Debug)]
enum DefaultOutcome {
    /// 既知一致なし（safe ではない）。
    NoKnownMatch,
    /// scan 失敗。
    Failed,
    /// provider 利用不可。
    Unavailable,
    /// 呼び出し自体をエラーにする。
    Error(ScanError),
}

impl MockSafetyProvider {
    /// known CSAM hash match provider の mock。既定は「一致なし」。
    pub fn known_csam(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capabilities: vec![SafetyProviderCapability::KnownCsamHashMatch],
            results: HashMap::new(),
            default_outcome: DefaultOutcome::NoKnownMatch,
        }
    }

    /// 任意 capability の mock。既定は「一致なし」。
    pub fn with_capabilities(
        name: impl Into<String>,
        capabilities: Vec<SafetyProviderCapability>,
    ) -> Self {
        Self {
            name: name.into(),
            capabilities,
            results: HashMap::new(),
            default_outcome: DefaultOutcome::NoKnownMatch,
        }
    }

    /// 設定外 subject を「scan 失敗」にする（fail-closed テスト用）。
    pub fn default_failed(mut self) -> Self {
        self.default_outcome = DefaultOutcome::Failed;
        self
    }

    /// 設定外 subject を「provider 利用不可」にする（fail-closed テスト用）。
    pub fn default_unavailable(mut self) -> Self {
        self.default_outcome = DefaultOutcome::Unavailable;
        self
    }

    /// 設定外 subject で `scan` 自体をエラーにする（呼び出し側の fail-closed 写像テスト用）。
    pub fn default_error(mut self, error: ScanError) -> Self {
        self.default_outcome = DefaultOutcome::Error(error);
        self
    }

    /// subject_id を既知 CSAM hash 一致にする。
    pub fn with_known_hash_match(mut self, subject_id: impl Into<String>) -> Self {
        let capability = self
            .capabilities
            .first()
            .copied()
            .unwrap_or(SafetyProviderCapability::KnownCsamHashMatch);
        let result = ProviderScanResult {
            provider: self.name.clone(),
            capability,
            outcome: ScanOutcome::Completed,
            known_hash_match: true,
            score: None,
            labels: vec![
                SafetyLabel::new(crate::verdict::SafetyCategory::Csam)
                    .with_provider_capability(capability),
            ],
        };
        self.results.insert(subject_id.into(), result);
        self
    }

    /// subject_id に classifier スコアを与える（suspected 判定の入力）。
    pub fn with_score(
        mut self,
        subject_id: impl Into<String>,
        capability: SafetyProviderCapability,
        category: crate::verdict::SafetyCategory,
        score: u8,
    ) -> Self {
        let result = ProviderScanResult {
            provider: self.name.clone(),
            capability,
            outcome: ScanOutcome::Completed,
            known_hash_match: false,
            score: Some(score),
            labels: vec![
                SafetyLabel::new(category)
                    .with_confidence(score)
                    .with_provider_capability(capability),
            ],
        };
        self.results.insert(subject_id.into(), result);
        self
    }

    /// subject_id を明示的に scan 失敗にする。
    pub fn with_failure(mut self, subject_id: impl Into<String>) -> Self {
        let capability = self
            .capabilities
            .first()
            .copied()
            .unwrap_or(SafetyProviderCapability::KnownCsamHashMatch);
        let result = ProviderScanResult {
            provider: self.name.clone(),
            capability,
            outcome: ScanOutcome::Failed,
            known_hash_match: false,
            score: None,
            labels: Vec::new(),
        };
        self.results.insert(subject_id.into(), result);
        self
    }

    /// subject_id を「既知一致なし（completed だが NoKnownMatch）」にする。
    pub fn with_no_known_match(mut self, subject_id: impl Into<String>) -> Self {
        let capability = self
            .capabilities
            .first()
            .copied()
            .unwrap_or(SafetyProviderCapability::KnownCsamHashMatch);
        let mut result = ProviderScanResult::completed(self.name.clone(), capability);
        result.outcome = ScanOutcome::NoKnownMatch;
        self.results.insert(subject_id.into(), result);
        self
    }

    fn default_result(&self) -> Result<ProviderScanResult, ScanError> {
        let capability = self
            .capabilities
            .first()
            .copied()
            .unwrap_or(SafetyProviderCapability::KnownCsamHashMatch);
        let outcome = match &self.default_outcome {
            DefaultOutcome::NoKnownMatch => ScanOutcome::NoKnownMatch,
            DefaultOutcome::Failed => ScanOutcome::Failed,
            DefaultOutcome::Unavailable => ScanOutcome::Unavailable,
            DefaultOutcome::Error(err) => return Err(err.clone()),
        };
        let mut result = ProviderScanResult::completed(self.name.clone(), capability);
        result.outcome = outcome;
        Ok(result)
    }
}

#[async_trait]
impl SafetyProvider for MockSafetyProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &[SafetyProviderCapability] {
        &self.capabilities
    }

    async fn scan(&self, request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        match request.subject_id.as_deref() {
            Some(id) => match self.results.get(id) {
                Some(result) => Ok(result.clone()),
                None => self.default_result(),
            },
            None => self.default_result(),
        }
    }
}

/// 決定論的な mock signer。canonical body の安定ハッシュを署名として返す。
///
/// 実鍵署名（secp256k1）ではない。署名の決定性のみを保証する。
#[derive(Clone, Debug)]
pub struct MockSigner {
    issuer_node_id: String,
}

impl MockSigner {
    pub fn new(issuer_node_id: impl Into<String>) -> Self {
        Self {
            issuer_node_id: issuer_node_id.into(),
        }
    }
}

impl ModerationEventSigner for MockSigner {
    fn issuer_node_id(&self) -> &str {
        &self.issuer_node_id
    }

    fn sign(&self, body: &ModerationEventBody) -> String {
        let digest = fnv1a_64(&body.canonical_bytes());
        format!("mock:{}:{:016x}", self.issuer_node_id, digest)
    }
}

/// 依存を増やさない決定論的な 64bit ハッシュ（FNV-1a）。暗号学的強度は無く、mock 署名専用。
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
