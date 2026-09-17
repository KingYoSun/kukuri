//! Non-secret deployment settings for the dedicated moderation provider (#1060).
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModerationDeployConfig {
    pub api_base_url: String,
    pub model: String,
    pub config_version: String,
    pub rpm: u32,
    pub rpd: u32,
    pub tpm: u32,
    pub image_tokens: u32,
}
impl Default for ModerationDeployConfig {
    fn default() -> Self {
        Self {
            api_base_url: "https://api.openai.com/v1".into(),
            model: "omni-moderation-latest".into(),
            config_version: "1".into(),
            rpm: 400,
            rpd: 8000,
            tpm: 8000,
            image_tokens: 2048,
        }
    }
}
impl ModerationDeployConfig {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.api_base_url.chars().any(char::is_control),
            "invalid moderation endpoint characters"
        );
        let url = url::Url::parse(&self.api_base_url)
            .map_err(|_| anyhow::anyhow!("deploy.moderation.api_base_url is invalid"))?;
        ensure!(
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "deploy.moderation requires HTTPS without credentials or query parameters"
        );
        ensure!(
            !self.model.trim().is_empty()
                && !self.config_version.trim().is_empty()
                && !self.model.chars().any(char::is_control)
                && !self.config_version.chars().any(char::is_control),
            "invalid moderation model/config version"
        );
        ensure!(
            (1..=500).contains(&self.rpm)
                && (1..=10000).contains(&self.rpd)
                && (1..=10000).contains(&self.tpm)
                && self.image_tokens > 0
                && self.image_tokens <= self.tpm,
            "moderation budget exceeds the initial Tier 1 bounds"
        );
        Ok(())
    }
}
