use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

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
        }
    }
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
    /// high-confidence 検知時の action 宣言。
    ///
    /// 注意: 現段階（readiness + config schema）では宣言として受理・検証するのみで、
    /// readiness 判定には使用しない。実際の効果は後続の runtime scan orchestration で適用する。
    #[serde(default)]
    pub on_high_confidence: Option<SafetyErrorAction>,
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
