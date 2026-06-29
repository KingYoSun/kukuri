//! provider abstraction（#353）。
//!
//! moderation server は provider credentials を保持し、必要に応じて blob を一時 fetch して
//! scan provider を実行する。この crate はその abstraction（trait）と、provider が返す
//! scan 結果の型のみを定義する。実際の I/O・credentials・本番 provider（#391）はこの trait の
//! 実装として後続段階で追加する。
//!
//! trait は async。本番 provider は HTTP API を叩くため本質的に async であり、mock も async で
//! 実装することで trait を変えずに差し替えられる。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::capability::SafetyProviderCapability;
use crate::verdict::SafetyLabel;

/// scan 対象の種別。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Post,
    Blob,
    User,
    Peer,
}

/// provider への scan 要求。
///
/// 生コンテンツそのものではなく、provider が必要とする最小の参照（hash / CID / text）を渡す。
/// community node は blob 本体を恒久保存しない前提のため、media は参照（hint）で表す。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderScanRequest {
    pub subject_kind: Option<SubjectKind>,
    /// 対象の識別子（post id / blob CID / pubkey / node id 等）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
    /// メディアの参照ヒント（hash / CID 等）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_hint: Option<String>,
    /// テキスト本文（text moderation / grooming classifier 用）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl ProviderScanRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_subject(kind: SubjectKind, subject_id: impl Into<String>) -> Self {
        Self {
            subject_kind: Some(kind),
            subject_id: Some(subject_id.into()),
            media_hint: None,
            text: None,
        }
    }

    pub fn with_media_hint(mut self, media_hint: impl Into<String>) -> Self {
        self.media_hint = Some(media_hint.into());
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

/// scan の完了状態。policy router の fail-closed 判定の入力になる。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanOutcome {
    /// scan が完了し、何らかの検知結果（labels / hash match / score）がある。
    Completed,
    /// scan は完了したが、この provider の既知データには一致しなかった。
    ///
    /// **safe / clean の証明ではない**（その provider の検査範囲で一致しなかっただけ）。
    NoKnownMatch,
    /// scan が失敗した（fail-closed の対象）。
    Failed,
    /// provider が利用不可だった（fail-closed の対象）。
    Unavailable,
}

impl ScanOutcome {
    /// この結果が fail-closed の対象か（scan failure / provider unavailable）。
    pub fn is_fail_closed(self) -> bool {
        matches!(self, ScanOutcome::Failed | ScanOutcome::Unavailable)
    }
}

/// provider 1 つの scan 結果。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderScanResult {
    /// provider 名（verdict の `provider` フィールドに反映され得る）。
    pub provider: String,
    /// この結果を生んだ capability。
    pub capability: SafetyProviderCapability,
    pub outcome: ScanOutcome,
    /// 既知 hash 一致（confirmed の根拠）。
    #[serde(default)]
    pub known_hash_match: bool,
    /// classifier スコア（0-100。suspected 判定に使う）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<u8>,
    /// 検知ラベル。
    #[serde(default)]
    pub labels: Vec<SafetyLabel>,
}

impl ProviderScanResult {
    /// completed かつ何も検知していない素の結果。
    pub fn completed(provider: impl Into<String>, capability: SafetyProviderCapability) -> Self {
        Self {
            provider: provider.into(),
            capability,
            outcome: ScanOutcome::Completed,
            known_hash_match: false,
            score: None,
            labels: Vec::new(),
        }
    }
}

/// provider 呼び出しのエラー。
///
/// 呼び出し側（moderation server / policy 適用層）は、このエラーを `ScanOutcome::Failed` /
/// `Unavailable` の結果へ写像し、fail-closed に倒す。allow へ落とさない。
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ScanError {
    #[error("safety provider unavailable: {0}")]
    Unavailable(String),
    #[error("safety provider timed out: {0}")]
    Timeout(String),
    #[error("safety provider protocol error: {0}")]
    Protocol(String),
}

/// safety / moderation provider の抽象。
///
/// 実装例: mock provider（本 crate）、#391 Project Arachnid Shield、一般 moderation provider。
#[async_trait]
pub trait SafetyProvider: Send + Sync {
    /// provider 名（安定識別子）。
    fn name(&self) -> &str;

    /// この provider が提供する capability。
    fn capabilities(&self) -> &[SafetyProviderCapability];

    /// 対象を scan する。I/O 失敗は `ScanError` で返し、呼び出し側が fail-closed に写像する。
    async fn scan(&self, request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError>;
}
