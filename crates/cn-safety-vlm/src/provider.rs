//! `SafetyProvider` 実装（#420）。
//!
//! OpenAI-compatible VLM の moderation 結果を domain の `ProviderScanResult` に写像する。
//!
//! 不変条件（ADR 0028 §2.1）:
//! - **basis は常に `ClassifierScore`**: `known_hash_match` は構造的に常に `false`（設定する
//!   経路が存在しない）。router の confirmed（`CsamConfirmed`）へ到達する写像は無い。
//! - 確信度は `ProviderScanResult.score`（0-100）に載せ、suspected 判定は router の
//!   `suspected_threshold` が行う。
//! - 入力は最小参照（`media_hint` / `text`）。media は scan のための一時 fetch のみで、
//!   この provider は bytes を保持しない。
//! - critical（CSAM / CSE / grooming）を検知した scan では `derived_tags` を空にする
//!   （タグ収集側 `derived_tags_for_index` の遮断と合わせた二重防御）。

use std::sync::Arc;

use async_trait::async_trait;

use kukuri_cn_safety::provider::{
    MediaFetcher, ProviderScanRequest, ProviderScanResult, ScanError, ScanOutcome,
};
use kukuri_cn_safety::{SafetyCategory, SafetyLabel, SafetyProvider, SafetyProviderCapability};

use crate::client::{VlmClient, VlmModeration, VlmScanInput};
use crate::config::{VlmConfigError, VlmCredentials, VlmProviderConfig};

/// provider の安定識別子。config（provider slot）の値・verdict の `provider` に使う。
pub const PROVIDER_NAME: &str = "openai-compatible-vlm";

const UNKNOWN_CSAM_CAPABILITIES: [SafetyProviderCapability; 4] = [
    SafetyProviderCapability::NovelCsamImageClassifier,
    SafetyProviderCapability::NovelCsamVideoClassifier,
    SafetyProviderCapability::CseTextClassifier,
    SafetyProviderCapability::GroomingTextClassifier,
];

const GENERAL_CAPABILITIES: [SafetyProviderCapability; 2] = [
    SafetyProviderCapability::GeneralMediaModeration,
    SafetyProviderCapability::SpamAbuseModeration,
];

/// provider slot ごとの capability プロファイル。
///
/// 同一実装を `unknown_csam` slot（critical classifier 系）と `general` slot
/// （general moderation 系）のどちらにも差せるようにする。`known_csam` slot には使えない
/// （known-match 系 capability を持たない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityProfile {
    /// 未知 CSAM / CSE / grooming の classifier（critical route の suspected 入力）。
    UnknownCsam,
    /// 一般 media / spam moderation（general route の入力）。
    General,
}

impl CapabilityProfile {
    fn capabilities(self) -> &'static [SafetyProviderCapability] {
        match self {
            CapabilityProfile::UnknownCsam => &UNKNOWN_CSAM_CAPABILITIES,
            CapabilityProfile::General => &GENERAL_CAPABILITIES,
        }
    }

    /// 検知が無いときに結果へ刻む代表 capability。
    fn primary(self) -> SafetyProviderCapability {
        match self {
            CapabilityProfile::UnknownCsam => SafetyProviderCapability::NovelCsamImageClassifier,
            CapabilityProfile::General => SafetyProviderCapability::GeneralMediaModeration,
        }
    }
}

/// OpenAI-compatible VLM を非決定論的 moderation provider として実装する。
pub struct VlmModerationProvider {
    client: VlmClient,
    profile: CapabilityProfile,
    /// media_hint → 一時 bytes の取得手段。未構成なら media scan は `Unavailable`（fail-closed）。
    fetcher: Option<Arc<dyn MediaFetcher>>,
}

impl std::fmt::Debug for VlmModerationProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VlmModerationProvider")
            .field("name", &PROVIDER_NAME)
            .field("profile", &self.profile)
            .field("client", &self.client)
            .field("fetcher_configured", &self.fetcher.is_some())
            .finish()
    }
}

impl VlmModerationProvider {
    /// 設定から provider を組み立てる。credentials は設定が指す env から読む
    /// （API key env 未設定は無認証として許容する）。
    pub fn new(
        config: &VlmProviderConfig,
        profile: CapabilityProfile,
    ) -> Result<Self, VlmConfigError> {
        Ok(Self {
            client: VlmClient::from_config(config)?,
            profile,
            fetcher: None,
        })
    }

    /// env（base URL / model / 応答形式 / timeout）から provider を組み立てる。
    pub fn from_env(profile: CapabilityProfile) -> Result<Self, VlmConfigError> {
        Self::new(&VlmProviderConfig::from_env()?, profile)
    }

    /// 明示的な credentials から組み立てる（テスト / env 以外の secret 供給経路用）。
    pub fn with_credentials(
        config: &VlmProviderConfig,
        credentials: VlmCredentials,
        profile: CapabilityProfile,
    ) -> Result<Self, VlmConfigError> {
        Ok(Self {
            client: VlmClient::new(config, credentials)?,
            profile,
            fetcher: None,
        })
    }

    /// media_hint から一時 bytes を取得する fetcher を接続する（media scan pipeline 側が注入）。
    pub fn with_media_fetcher(mut self, fetcher: Arc<dyn MediaFetcher>) -> Self {
        self.fetcher = Some(fetcher);
        self
    }
}

#[async_trait]
impl SafetyProvider for VlmModerationProvider {
    fn name(&self) -> &str {
        PROVIDER_NAME
    }

    fn capabilities(&self) -> &[SafetyProviderCapability] {
        self.profile.capabilities()
    }

    async fn scan(&self, request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        let text = request
            .text
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty());
        let media_hint = request
            .media_hint
            .as_deref()
            .map(str::trim)
            .filter(|hint| !hint.is_empty());

        // 分類対象が無い（text も media も無い）scan は「検知なし」ではなく「未照合」。
        // NoKnownMatch（safe の証明ではない）として scan 実施の記録だけ残す。
        if text.is_none() && media_hint.is_none() {
            let mut result = ProviderScanResult::completed(PROVIDER_NAME, self.profile.primary());
            result.outcome = ScanOutcome::NoKnownMatch;
            return Ok(result);
        }

        // media_hint があるのに fetcher が未構成なら scan 不能 → fail-closed。
        let media = match media_hint {
            Some(hint) => {
                let Some(fetcher) = &self.fetcher else {
                    return Err(ScanError::Unavailable(
                        "openai-compatible-vlm has no media fetcher configured; \
                         cannot scan referenced media (fail-closed)"
                            .to_string(),
                    ));
                };
                Some(fetcher.fetch(hint, request.media_mime.as_deref()).await?)
            }
            None => None,
        };

        let moderation = self
            .client
            .moderate(&VlmScanInput {
                text,
                media: media.as_ref(),
            })
            .await
            .map_err(ScanError::from)?;
        let media_content_type = media.as_ref().map(|m| m.content_type.clone());
        // media bytes はここでスコープを抜けて破棄される（恒久保存しない）。
        drop(media);
        Ok(self.scan_result_from(moderation, media_content_type.as_deref()))
    }
}

impl VlmModerationProvider {
    /// moderation 応答を domain の `ProviderScanResult` に写像する。
    fn scan_result_from(
        &self,
        moderation: VlmModeration,
        media_content_type: Option<&str>,
    ) -> ProviderScanResult {
        let mut labels = Vec::new();
        let mut max_score: Option<u8> = None;
        let mut result_capability: Option<SafetyProviderCapability> = None;
        let mut has_critical = false;
        for detection in &moderation.detections {
            let capability = capability_for(detection.category, media_content_type);
            labels.push(
                SafetyLabel::new(detection.category)
                    .with_confidence(detection.score)
                    .with_provider_capability(capability),
            );
            if detection.category.is_critical_safety() {
                has_critical = true;
            }
            if max_score.is_none_or(|current| detection.score > current) {
                max_score = Some(detection.score);
                result_capability = Some(capability);
            }
        }
        let mut result = ProviderScanResult::completed(
            PROVIDER_NAME,
            result_capability.unwrap_or_else(|| self.profile.primary()),
        );
        // known_hash_match は設定しない（常に false = basis が confirmed に昇格しない）。
        result.score = max_score;
        result.labels = labels;
        // critical 検知時はタグを源流で遮断する（二重防御。収集側にも同じガードがある）。
        result.derived_tags = if has_critical {
            Vec::new()
        } else {
            moderation.tags
        };
        result
    }
}

/// 検知 category から、その検知を生んだとみなす capability を導く。
///
/// CSAM は入力が video なら video classifier、それ以外は image classifier として刻む。
/// general slot の provider が critical category を返した場合も忠実に critical capability を
/// 刻む（router の critical route に入る = fail-closed 方向。安全側に握りつぶさない）。
fn capability_for(
    category: SafetyCategory,
    media_content_type: Option<&str>,
) -> SafetyProviderCapability {
    match category {
        SafetyCategory::Csam => {
            if media_content_type.is_some_and(|ct| ct.starts_with("video/")) {
                SafetyProviderCapability::NovelCsamVideoClassifier
            } else {
                SafetyProviderCapability::NovelCsamImageClassifier
            }
        }
        SafetyCategory::Cse => SafetyProviderCapability::CseTextClassifier,
        SafetyCategory::Grooming => SafetyProviderCapability::GroomingTextClassifier,
        SafetyCategory::Spam => SafetyProviderCapability::SpamAbuseModeration,
        _ => SafetyProviderCapability::GeneralMediaModeration,
    }
}
