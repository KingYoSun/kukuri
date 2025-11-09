use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// UserProfile for compatibility with SqliteRepository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub npub: String,
    pub pubkey: String,
    pub profile: UserProfile,
    pub name: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
    pub public_profile: bool,
    pub show_online_status: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(npub: String, pubkey: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            npub,
            pubkey,
            profile: UserProfile {
                display_name: String::new(),
                bio: String::new(),
                avatar_url: None,
            },
            name: None,
            nip05: None,
            lud16: None,
            public_profile: true,
            show_online_status: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_profile(
        mut self,
        name: Option<String>,
        display_name: Option<String>,
        about: Option<String>,
    ) -> Self {
        self.name = name;
        self.profile.display_name = display_name.unwrap_or_default();
        self.profile.bio = about.unwrap_or_default();
        self.updated_at = chrono::Utc::now();
        self
    }

    pub fn update_metadata(&mut self, metadata: UserMetadata) {
        if let Some(name) = metadata.name {
            self.name = Some(name);
        }
        if let Some(display_name) = metadata.display_name {
            self.profile.display_name = display_name;
        }
        if let Some(about) = metadata.about {
            self.profile.bio = about;
        }
        if let Some(picture) = metadata.picture {
            self.profile.avatar_url = Some(picture);
        }
        if let Some(nip05) = metadata.nip05 {
            self.nip05 = Some(nip05);
        }
        if let Some(lud16) = metadata.lud16 {
            self.lud16 = Some(lud16);
        }
        if let Some(public_profile) = metadata.public_profile {
            self.public_profile = public_profile;
        }
        if let Some(show_online_status) = metadata.show_online_status {
            self.show_online_status = show_online_status;
        }
        self.updated_at = chrono::Utc::now();
    }

    pub fn pubkey(&self) -> &str {
        &self.pubkey
    }

    pub fn npub(&self) -> &str {
        &self.npub
    }

    pub fn from_pubkey(pubkey: &str) -> Self {
        use nostr_sdk::prelude::*;

        let npub = PublicKey::from_hex(pubkey)
            .ok()
            .and_then(|pk| pk.to_bech32().ok())
            .unwrap_or_else(|| pubkey.to_string());

        Self::new(npub, pubkey.to_string())
    }

    pub fn new_with_profile(npub: String, profile: UserProfile) -> Self {
        let mut user = Self::new(npub.clone(), String::new());
        user.profile = profile;
        user
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
    pub public_profile: Option<bool>,
    pub show_online_status: Option<bool>,
}
