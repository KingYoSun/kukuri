//! verdict model（#353）。検知ラベルと最終 action を分離する。
//!
//! `docs/safety/community-node-critical-safety.md` §7 に従い:
//! - action（`allow` / `hold` / `quarantine` / `exclude`）と label を分ける。
//! - `csam_confirmed`（known hash match / provider confirmed）と
//!   `csam_suspected`（classifier high score / CSE 疑い）を区別する。
//!
//! confidence / score は等値比較とテストの決定性のため浮動小数ではなく整数 `u8`(0-100)。

use serde::{Deserialize, Serialize};

use crate::capability::SafetyProviderCapability;

/// scan 後の最終 action。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyAction {
    /// index / surfacing を許可する唯一の action。
    Allow,
    /// 保留。index しない。
    Hold,
    /// 隔離。index しない。
    Quarantine,
    /// 排除。index しない。
    Exclude,
}

impl SafetyAction {
    /// この action が index / discovery / recommendation への surfacing を許すか。
    ///
    /// `Allow` のときだけ true。fail-closed の単一判定点。
    pub fn allows_indexing(self) -> bool {
        matches!(self, SafetyAction::Allow)
    }
}

/// risk signal / moderation のカテゴリ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyCategory {
    Csam,
    Cse,
    Grooming,
    Nsfw,
    Spam,
    Malware,
    Phishing,
}

impl SafetyCategory {
    /// critical safety（CSAM / CSE / grooming）か。general moderation と区別する。
    pub fn is_critical_safety(self) -> bool {
        matches!(
            self,
            SafetyCategory::Csam | SafetyCategory::Cse | SafetyCategory::Grooming
        )
    }
}

/// severity。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// 判定の根拠。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// 既知 hash 一致（confirmed の根拠）。
    KnownHashMatch,
    /// provider が confirmed と返した。
    ProviderVerdict,
    /// classifier スコア（suspected の根拠）。
    ClassifierScore,
    /// ローカルポリシー判断。
    LocalPolicy,
}

/// signal / event の配布範囲。
///
/// suspected unknown CSAM / CSE は既定 `Local`。known hash match / provider confirmed の場合のみ
/// `SubscribedNodes` 以上を検討する（誤検知を public advisory として拡散しない）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// issuer node 内のローカル判断にのみ使う（既定・最も安全側）。
    #[default]
    Local,
    /// この node を trust input として購読している node にのみ配布。
    SubscribedNodes,
    /// 公開 advisory。
    Public,
}

/// reason code。`csam_confirmed` と `csam_suspected` を型で分離する。
///
/// **重要**: `NoKnownMatch` を `Clean` / safe と同一視しないこと。known-hash provider のみの
/// 場合、「未知 CSAM は未検査」であり、no match は安全の証明ではない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    /// 既知 CSAM の confirmed（known hash match / provider confirmed）。
    CsamConfirmed,
    /// 未知 CSAM の suspected（classifier high score）。
    CsamSuspected,
    /// CSE の suspected。
    CseSuspected,
    /// 一般 moderation（nsfw / violence / hate / harassment / spam / malware / phishing）。
    GeneralModeration,
    /// scan が失敗した（fail-closed の根拠）。
    ScanFailed,
    /// provider が利用不可だった（fail-closed の根拠）。
    ProviderUnavailable,
    /// scan 要求がそもそも無い（unscanned。fail-closed の根拠）。
    Unscanned,
    /// 既知 hash には一致しなかった（**safe の証明ではない**）。
    NoKnownMatch,
    /// 明示的に clean と判定できた（全 capability で問題なし）。
    Clean,
}

/// 検知ラベル。action とは独立に「何を検知したか」を表す。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SafetyLabel {
    pub category: SafetyCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_capability: Option<SafetyProviderCapability>,
}

impl SafetyLabel {
    pub fn new(category: SafetyCategory) -> Self {
        Self {
            category,
            confidence: None,
            provider_capability: None,
        }
    }

    pub fn with_confidence(mut self, confidence: u8) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn with_provider_capability(mut self, capability: SafetyProviderCapability) -> Self {
        self.provider_capability = Some(capability);
        self
    }
}

/// policy router が返す最終 verdict。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SafetyVerdict {
    pub action: SafetyAction,
    #[serde(default)]
    pub labels: Vec<SafetyLabel>,
    pub critical: bool,
    pub reason_code: ReasonCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_capability: Option<SafetyProviderCapability>,
    pub policy_version: String,
    /// scan 時刻（RFC3339）。この crate では時計を持たず、呼び出し側が与える。
    pub scanned_at: String,
}

impl SafetyVerdict {
    /// この verdict が index / discovery / recommendation への surfacing を許すか。
    ///
    /// `action == Allow` のときだけ true。`hold` / `quarantine` / `exclude` と
    /// すべての fail-closed 経路（scan failure / provider unavailable / unscanned）は false。
    pub fn is_indexable(&self) -> bool {
        self.action.allows_indexing()
    }
}

/// moderation event の action。risk_label を含むため `SafetyAction` とは別 enum。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationAction {
    Exclude,
    Hold,
    Quarantine,
    RiskLabel,
}
