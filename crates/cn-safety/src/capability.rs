//! safety / moderation provider の capability モデル（#353）。
//!
//! provider は単一の boolean ではなく capability として扱う。これにより、known CSAM hash
//! matching のみを行う provider と、unknown CSAM classifier / 一般 moderation を行う provider を
//! 役割で区別できる。public-node の readiness（後続段階）は「known CSAM hash match capability を
//! 持つ provider が設定されているか」を capability 単位で検査できる。

use serde::{Deserialize, Serialize};

/// safety / moderation provider が提供し得る capability。
///
/// ADR 0027 `docs/adr/0027-deterministic-moderation-critical-safety.md` §2.7 の provider capability と、
/// Issue #353 の `SafetyProviderCapability` に対応する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyProviderCapability {
    /// 既知 CSAM の hash 一致（confirmed 判定の根拠になり得る）。
    KnownCsamHashMatch,
    /// 知覚ハッシュ（perceptual hash）一致。
    PerceptualHashMatch,
    /// 未知 CSAM 画像の classifier（suspected 判定）。
    NovelCsamImageClassifier,
    /// 未知 CSAM 動画の classifier（suspected 判定）。
    NovelCsamVideoClassifier,
    /// CSE（child sexual exploitation）テキスト classifier。
    CseTextClassifier,
    /// grooming テキスト classifier。
    GroomingTextClassifier,
    /// 一般メディア moderation（nsfw / violence 等）。
    GeneralMediaModeration,
    /// spam / abuse moderation。
    SpamAbuseModeration,
    /// malware / phishing 検出。
    MalwarePhishingDetection,
    /// 通報ワークフロー連携。
    ReportingWorkflow,
}

impl SafetyProviderCapability {
    /// この capability が critical safety（CSAM / CSE）に直接関与するか。
    ///
    /// general moderation route（nsfw / spam / malware 等）と critical route を
    /// 取り違えないための補助。route の最終判定は policy router が行う。
    pub fn is_critical_safety(self) -> bool {
        matches!(
            self,
            SafetyProviderCapability::KnownCsamHashMatch
                | SafetyProviderCapability::PerceptualHashMatch
                | SafetyProviderCapability::NovelCsamImageClassifier
                | SafetyProviderCapability::NovelCsamVideoClassifier
                | SafetyProviderCapability::CseTextClassifier
                | SafetyProviderCapability::GroomingTextClassifier
        )
    }

    /// この capability が「既知 CSAM の confirmed 判定」を生み得るか。
    ///
    /// known hash match のみ confirmed の根拠になり、classifier 系は suspected に留める。
    pub fn can_confirm_known_csam(self) -> bool {
        matches!(self, SafetyProviderCapability::KnownCsamHashMatch)
    }
}
