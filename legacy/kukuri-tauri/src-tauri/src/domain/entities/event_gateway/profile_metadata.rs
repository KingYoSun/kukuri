use nostr_sdk::prelude::Url;
use serde::{Deserialize, Serialize};

/// NIP-65 Relay Listエントリ。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayEndpoint {
    pub url: String,
    pub read: bool,
    pub write: bool,
}

impl RelayEndpoint {
    pub fn new(url: String, read: bool, write: bool) -> Result<Self, String> {
        Self::validate_url(&url)?;
        Ok(Self { url, read, write })
    }

    pub fn validate(&self) -> Result<(), String> {
        Self::validate_url(&self.url)
    }

    fn validate_url(value: &str) -> Result<(), String> {
        if value.is_empty() {
            return Err("Relay URL must not be empty".to_string());
        }
        let parsed =
            Url::parse(value).map_err(|_| "Relay URL must be a valid websocket URL".to_string())?;
        match parsed.scheme() {
            "ws" | "wss" => Ok(()),
            _ => Err("Relay URL must use ws:// or wss://".to_string()),
        }
    }
}

/// プロフィール更新時に利用するメタデータ。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileMetadata {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
    pub website: Option<String>,
    pub relays: Option<Vec<RelayEndpoint>>,
    pub privacy: Option<PrivacyPreferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyPreferences {
    pub public_profile: bool,
    pub show_online_status: bool,
}

impl ProfileMetadata {
    const NAME_LIMIT: usize = 100;
    const DISPLAY_NAME_LIMIT: usize = 100;
    const ABOUT_LIMIT: usize = 1_000;
    const URL_LIMIT: usize = 1_024;
    const MAX_RELAYS: usize = 64;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Option<String>,
        display_name: Option<String>,
        about: Option<String>,
        picture: Option<String>,
        banner: Option<String>,
        nip05: Option<String>,
        lud16: Option<String>,
        website: Option<String>,
        relays: Option<Vec<RelayEndpoint>>,
        privacy: Option<PrivacyPreferences>,
    ) -> Result<Self, String> {
        let metadata = Self {
            name,
            display_name,
            about,
            picture,
            banner,
            nip05,
            lud16,
            website,
            relays,
            privacy,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn validate(&self) -> Result<(), String> {
        if let Some(name) = self.name.as_ref()
            && name.chars().count() > Self::NAME_LIMIT
        {
            return Err(format!(
                "Name is too long (max {} characters)",
                Self::NAME_LIMIT
            ));
        }

        if let Some(display_name) = self.display_name.as_ref()
            && display_name.chars().count() > Self::DISPLAY_NAME_LIMIT
        {
            return Err(format!(
                "Display name is too long (max {} characters)",
                Self::DISPLAY_NAME_LIMIT
            ));
        }

        if let Some(about) = self.about.as_ref()
            && about.chars().count() > Self::ABOUT_LIMIT
        {
            return Err(format!(
                "About is too long (max {} characters)",
                Self::ABOUT_LIMIT
            ));
        }

        if let Some(picture) = self.picture.as_ref() {
            Self::validate_url_length(picture, "Picture")?;
        }

        if let Some(banner) = self.banner.as_ref() {
            Self::validate_url_length(banner, "Banner")?;
        }

        if let Some(website) = self.website.as_ref() {
            Self::validate_url_length(website, "Website")?;
        }

        if let Some(relays) = self.relays.as_ref()
            && relays.len() > Self::MAX_RELAYS
        {
            return Err(format!(
                "Relay list is too long (max {} entries)",
                Self::MAX_RELAYS
            ));
        }
        if let Some(relays) = self.relays.as_ref() {
            for relay in relays {
                relay.validate()?;
            }
        }

        Ok(())
    }

    fn validate_url_length(value: &str, field: &str) -> Result<(), String> {
        if value.chars().count() > Self::URL_LIMIT {
            return Err(format!(
                "{} URL is too long (max {} characters)",
                field,
                Self::URL_LIMIT
            ));
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.display_name.is_none()
            && self.about.is_none()
            && self.picture.is_none()
            && self.banner.is_none()
            && self.nip05.is_none()
            && self.lud16.is_none()
            && self.website.is_none()
            && self.relays.as_ref().is_none_or(|relays| relays.is_empty())
            && self.privacy.is_none()
    }
}
