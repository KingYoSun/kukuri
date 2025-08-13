use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub npub: String,
    pub pubkey: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl User {
    pub fn new(npub: String, pubkey: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            npub,
            pubkey,
            name: None,
            display_name: None,
            about: None,
            picture: None,
            banner: None,
            nip05: None,
            lud16: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_profile(mut self, name: Option<String>, display_name: Option<String>, about: Option<String>) -> Self {
        self.name = name;
        self.display_name = display_name;
        self.about = about;
        self.updated_at = chrono::Utc::now().timestamp();
        self
    }

    pub fn update_metadata(&mut self, metadata: UserMetadata) {
        if let Some(name) = metadata.name {
            self.name = Some(name);
        }
        if let Some(display_name) = metadata.display_name {
            self.display_name = Some(display_name);
        }
        if let Some(about) = metadata.about {
            self.about = Some(about);
        }
        if let Some(picture) = metadata.picture {
            self.picture = Some(picture);
        }
        if let Some(banner) = metadata.banner {
            self.banner = Some(banner);
        }
        if let Some(nip05) = metadata.nip05 {
            self.nip05 = Some(nip05);
        }
        if let Some(lud16) = metadata.lud16 {
            self.lud16 = Some(lud16);
        }
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetadata {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
}