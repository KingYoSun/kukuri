use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub use kukuri_cn_safety::GeneralAction;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyConfig {
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default = "default_policy_version")]
    pub policy_version: String,
    #[serde(default)]
    pub indexing: SafetyIndexingConfig,
    #[serde(default)]
    pub storage: SafetyStorageConfig,
    #[serde(default)]
    pub events: SafetyEventsConfig,
    #[serde(default)]
    pub providers: SafetyProvidersConfig,
    #[serde(default)]
    pub moderation: SafetyModerationConfig,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            profile: None,
            policy_version: default_policy_version(),
            indexing: SafetyIndexingConfig::default(),
            storage: SafetyStorageConfig::default(),
            events: SafetyEventsConfig::default(),
            providers: SafetyProvidersConfig::default(),
            moderation: SafetyModerationConfig::default(),
        }
    }
}

/// 非決定論的 moderation（VLM。#420 / ADR 0028）の operator 設定。
///
/// runtime へは env（`COMMUNITY_NODE_SAFETY_SUSPECTED_THRESHOLD` /
/// `COMMUNITY_NODE_SAFETY_SUSPECTED_SIGNAL_VISIBILITY` /
/// `COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW` / `COMMUNITY_NODE_SAFETY_GENERAL_ACTION`）として
/// 注入される宣言。
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyModerationConfig {
    /// suspected 判定の classifier スコア閾値（1-100。未指定なら policy 既定 70 = 0.7）。
    #[serde(default)]
    pub suspected_threshold: Option<u8>,
    /// suspected（classifier_score）advisory の配布 visibility。
    /// 未指定なら既定 `local`（安全側。hard cap ではない。ADR 0028 §2.4 / §2.7）。
    #[serde(default)]
    pub suspected_signal_visibility: Option<SignalVisibility>,
    /// operator レビュー（検知メタデータの直接編集）を有効化するか（既定 true。ADR 0028 §2.3）。
    #[serde(default = "default_operator_review")]
    pub operator_review: bool,
    /// nsfw / objectionable の suspected に対する action（ADR 0028 §8.7）。
    ///
    /// `label`（既定。content advisory 付きで index）/ `hold` / `exclude`。ラベル無しの `allow` は
    /// 値域に無く、parse で拒否される（`general_action_operator_tunable_stricter_only`）。
    /// 未指定は法務 snapshot（`policy_snapshot_revision`）の canonical 入力に現れないため、
    /// 既定のまま運用する限り再同意は発生しない。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_action: Option<GeneralAction>,
}

impl Default for SafetyModerationConfig {
    fn default() -> Self {
        Self {
            suspected_threshold: None,
            suspected_signal_visibility: None,
            operator_review: default_operator_review(),
            general_action: None,
        }
    }
}

const fn default_operator_review() -> bool {
    true
}

/// risk signal の配布 visibility（`cn-safety` の `Visibility` と同じ語彙）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalVisibility {
    Local,
    SubscribedNodes,
    Public,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyIndexingConfig {
    #[serde(default)]
    pub index_before_scan: bool,
    #[serde(default)]
    pub on_scan_error: SafetyErrorAction,
}

impl Default for SafetyIndexingConfig {
    fn default() -> Self {
        Self {
            index_before_scan: false,
            on_scan_error: SafetyErrorAction::Hold,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyStorageConfig {
    #[serde(default)]
    pub permanent_blob_storage: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyEventsConfig {
    #[serde(default = "default_emit_signed_moderation_events")]
    pub emit_signed_moderation_events: bool,
    /// moderation event の実鍵署名（secp256k1）に使う signing key を保持する
    /// Secret Manager secret ID（値ではない）。runtime はこの secret を
    /// `COMMUNITY_NODE_SAFETY_SIGNING_KEY` env として注入され、署名鍵を読み込む。
    #[serde(default)]
    pub signing_key_secret_id: Option<String>,
}

impl Default for SafetyEventsConfig {
    fn default() -> Self {
        Self {
            emit_signed_moderation_events: default_emit_signed_moderation_events(),
            signing_key_secret_id: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyProvidersConfig {
    #[serde(default)]
    pub known_csam: Option<SafetyProviderEntry>,
    #[serde(default)]
    pub general: Option<SafetyProviderEntry>,
    #[serde(default)]
    pub unknown_csam: Option<SafetyProviderEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyProviderEntry {
    pub provider: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub credential_secret_id: Option<String>,
    /// high-confidence 検知時の action 宣言（**deprecated**。#1051 / ADR 0028 §8.7）。
    ///
    /// 宣言のみで runtime に実効性が無かったため、受理はするが読み捨てて警告する
    /// （`ResolvedConfig::warnings`）。nsfw / objectionable の扱いは
    /// `safety.moderation.general_action` で指定する。法務 snapshot の canonical 入力には
    /// 従来どおり含めたままにし、既存 config の snapshot を変えない。
    #[serde(default)]
    pub on_high_confidence: Option<SafetyErrorAction>,
    /// プロバイダ基盤の区分（外部送信表示の生成に使う。#617）。
    ///
    /// - `self_host`: 運営者が管理する基盤（第三者への外部送信ではない）
    /// - `external`: 第三者の API（第三者への外部送信として開示する）
    ///
    /// 未指定は保守側（`external` 相当 = 第三者への外部送信として開示）で扱う。
    /// 開示にはこの区分のみを使い、接続先 URL・内部アドレスは公開資料へ出さない。
    #[serde(default)]
    pub hosting: Option<ProviderHosting>,
}

/// 安全性プロバイダの基盤区分（開示用）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHosting {
    SelfHost,
    External,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyErrorAction {
    Allow,
    #[default]
    Hold,
    Quarantine,
    Exclude,
}

impl SafetyErrorAction {
    pub fn allows_indexing(self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn key(self) -> &'static str {
        match self {
            SafetyErrorAction::Allow => "allow",
            SafetyErrorAction::Hold => "hold",
            SafetyErrorAction::Quarantine => "quarantine",
            SafetyErrorAction::Exclude => "exclude",
        }
    }
}

/// 起動を止めない注意事項（deprecated key の使用など）。`validate-config` が表示する。
pub fn safety_config_warnings(config: &SafetyConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (field, entry) in [
        (
            "safety.providers.known_csam",
            config.providers.known_csam.as_ref(),
        ),
        (
            "safety.providers.general",
            config.providers.general.as_ref(),
        ),
        (
            "safety.providers.unknown_csam",
            config.providers.unknown_csam.as_ref(),
        ),
    ] {
        if let Some(action) = entry.and_then(|entry| entry.on_high_confidence) {
            warnings.push(format!(
                "{field}.on_high_confidence（{}）は deprecated です。runtime には反映されず読み捨てます。\
                 nsfw / objectionable の扱いは safety.moderation.general_action（label / hold / exclude）で\
                 指定してください（ADR 0028 §8.7）",
                action.key()
            ));
        }
    }
    warnings
}

pub fn validate_safety_config(config: &SafetyConfig) -> Result<()> {
    validate_config_string("safety.policy_version", config.policy_version.as_str())?;
    if let Some(profile) = config.profile.as_deref() {
        validate_config_string("safety.profile", profile)?;
    }
    validate_provider_entry(
        "safety.providers.known_csam",
        config.providers.known_csam.as_ref(),
    )?;
    validate_provider_entry(
        "safety.providers.general",
        config.providers.general.as_ref(),
    )?;
    validate_provider_entry(
        "safety.providers.unknown_csam",
        config.providers.unknown_csam.as_ref(),
    )?;
    if let Some(secret_id) = config.events.signing_key_secret_id.as_deref() {
        validate_secret_id("safety.events.signing_key_secret_id", secret_id)?;
    }
    if let Some(threshold) = config.moderation.suspected_threshold
        && (threshold == 0 || threshold > 100)
    {
        bail!(
            "safety.moderation.suspected_threshold は 1-100 の整数で指定してください \
                 (got {threshold})"
        );
    }
    Ok(())
}

fn validate_provider_entry(field: &str, entry: Option<&SafetyProviderEntry>) -> Result<()> {
    let Some(entry) = entry else {
        return Ok(());
    };
    validate_config_string(
        format!("{field}.provider").as_str(),
        entry.provider.as_str(),
    )?;
    if entry.provider.trim().is_empty() {
        bail!("{field}.provider は必須です");
    }
    if let Some(secret_id) = entry.credential_secret_id.as_deref() {
        validate_secret_id(format!("{field}.credential_secret_id").as_str(), secret_id)?;
    }
    Ok(())
}

fn validate_config_string(field: &str, value: &str) -> Result<()> {
    if value.chars().any(char::is_control) {
        bail!("{field} に制御文字は指定できません");
    }
    Ok(())
}

fn validate_secret_id(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 255
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!(
            "{field} は Secret Manager secret ID 形式（英数字、hyphen、underscore）で指定してください"
        );
    }
    Ok(())
}

fn default_policy_version() -> String {
    "2026-06-public-node-v1".to_string()
}

fn default_emit_signed_moderation_events() -> bool {
    true
}

impl crate::config::ResolvedConfig {
    /// 起動を止めない注意事項（deprecated key の使用など）。CLI が表示する。
    pub fn warnings(&self) -> Vec<String> {
        self.raw
            .safety
            .as_ref()
            .map(safety_config_warnings)
            .unwrap_or_default()
    }
}
