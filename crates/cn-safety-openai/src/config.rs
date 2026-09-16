use std::time::Duration;

use kukuri_cn_safety::ScanError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::BudgetConfig;

pub const API_KEY_ENV: &str = "COMMUNITY_NODE_VLM_API_KEY";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModerationConfig {
    pub api_base_url: String,
    pub model: String,
    pub configuration_version: String,
    pub http_timeout: Duration,
    pub scan_timeout: Duration,
    pub queue_capacity: usize,
    pub request_max_bytes: usize,
    pub response_max_bytes: usize,
    pub image_token_reservation: u32,
    pub budget: BudgetConfig,
}

impl Default for ModerationConfig {
    fn default() -> Self {
        Self {
            api_base_url: "https://api.openai.com/v1".into(),
            model: "omni-moderation-latest".into(),
            configuration_version: "1".into(),
            http_timeout: Duration::from_secs(30),
            scan_timeout: Duration::from_secs(300),
            queue_capacity: 16,
            request_max_bytes: 384 * 1024,
            response_max_bytes: 64 * 1024,
            image_token_reservation: 2048,
            budget: BudgetConfig::default(),
        }
    }
}

impl ModerationConfig {
    pub fn from_env() -> Result<Self, ScanError> {
        let mut config = Self::default();
        for (name, target) in [
            (
                "COMMUNITY_NODE_MODERATION_API_BASE_URL",
                &mut config.api_base_url,
            ),
            ("COMMUNITY_NODE_MODERATION_MODEL", &mut config.model),
            (
                "COMMUNITY_NODE_MODERATION_CONFIG_VERSION",
                &mut config.configuration_version,
            ),
        ] {
            if let Ok(value) = std::env::var(name) {
                *target = value.trim().to_owned();
            }
        }
        config.budget.requests_per_minute = env_u32("COMMUNITY_NODE_MODERATION_RPM", 400)?;
        config.budget.requests_per_day = env_u32("COMMUNITY_NODE_MODERATION_RPD", 8000)?;
        config.budget.tokens_per_minute = env_u32("COMMUNITY_NODE_MODERATION_TPM", 8000)?;
        config.image_token_reservation = env_u32("COMMUNITY_NODE_MODERATION_IMAGE_TOKENS", 2048)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ScanError> {
        let url = url::Url::parse(&self.api_base_url)
            .map_err(|_| invalid("invalid moderation API base URL"))?;
        let local = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
        if !(url.scheme() == "https" || (local && url.scheme() == "http"))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(invalid(
                "moderation endpoint requires HTTPS without embedded credentials",
            ));
        }
        if self.model.trim().is_empty()
            || self.configuration_version.trim().is_empty()
            || self.queue_capacity == 0
            || self.queue_capacity > 64
            || self.http_timeout.is_zero()
            || self.scan_timeout.is_zero()
            || self.http_timeout > Duration::from_secs(30)
            || self.scan_timeout > Duration::from_secs(300)
            || self.http_timeout > self.scan_timeout
            || self.request_max_bytes == 0
            || self.request_max_bytes > 384 * 1024
            || self.response_max_bytes == 0
            || self.response_max_bytes > 64 * 1024
            || self.image_token_reservation == 0
            || self.image_token_reservation > self.budget.tokens_per_minute
        {
            return Err(invalid("invalid moderation configuration limits"));
        }
        self.budget.validate()
    }

    pub fn fingerprint(&self) -> String {
        // Only the digest is stored; credentials are never part of this configuration.
        let bytes = serde_json::to_vec(self).expect("serializable moderation config");
        format!(
            "{}:{}:{}",
            crate::PROVIDER_NAME,
            crate::PREPROCESSING_VERSION,
            hex::encode(Sha256::digest(bytes))
        )
    }
}

pub struct ModerationCredentials(String);

impl ModerationCredentials {
    pub fn from_env() -> Result<Self, ScanError> {
        Self::new(std::env::var(API_KEY_ENV).unwrap_or_default())
    }

    pub fn new(key: impl Into<String>) -> Result<Self, ScanError> {
        let key = key.into();
        let key = key.trim();
        if key.is_empty() || key.contains(['\r', '\n']) {
            return Err(invalid("COMMUNITY_NODE_VLM_API_KEY is missing or invalid"));
        }
        Ok(Self(key.to_owned()))
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ModerationCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ModerationCredentials([redacted])")
    }
}

fn env_u32(name: &str, default: u32) -> Result<u32, ScanError> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| invalid(&format!("invalid {name}"))),
        Err(_) => Ok(default),
    }
}

pub(crate) fn invalid(reason: &str) -> ScanError {
    ScanError::Protocol(reason.to_owned())
}
