use serde::{Deserialize, Serialize};

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
}

impl ProfileMetadata {
    const NAME_LIMIT: usize = 100;
    const DISPLAY_NAME_LIMIT: usize = 100;
    const ABOUT_LIMIT: usize = 1_000;
    const URL_LIMIT: usize = 1_024;

    pub fn new(
        name: Option<String>,
        display_name: Option<String>,
        about: Option<String>,
        picture: Option<String>,
        banner: Option<String>,
        nip05: Option<String>,
        lud16: Option<String>,
        website: Option<String>,
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
        };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn validate(&self) -> Result<(), String> {
        if let Some(name) = self.name.as_ref() {
            if name.chars().count() > Self::NAME_LIMIT {
                return Err(format!(
                    "Name is too long (max {} characters)",
                    Self::NAME_LIMIT
                ));
            }
        }

        if let Some(display_name) = self.display_name.as_ref() {
            if display_name.chars().count() > Self::DISPLAY_NAME_LIMIT {
                return Err(format!(
                    "Display name is too long (max {} characters)",
                    Self::DISPLAY_NAME_LIMIT
                ));
            }
        }

        if let Some(about) = self.about.as_ref() {
            if about.chars().count() > Self::ABOUT_LIMIT {
                return Err(format!(
                    "About is too long (max {} characters)",
                    Self::ABOUT_LIMIT
                ));
            }
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
    }
}
