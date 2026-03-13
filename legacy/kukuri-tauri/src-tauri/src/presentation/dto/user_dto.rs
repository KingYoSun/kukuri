use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub npub: String,
    pub pubkey: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub website: Option<String>,
    pub nip05: Option<String>,
    pub is_profile_public: Option<bool>,
    pub show_online_status: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginatedUserProfiles {
    pub items: Vec<UserProfile>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchUsersRequest {
    pub query: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub allow_incomplete: Option<bool>,
    pub viewer_npub: Option<String>,
}

impl crate::presentation::dto::Validate for SearchUsersRequest {
    fn validate(&self) -> Result<(), String> {
        if self.query.trim().is_empty() {
            return Err("検索キーワードを入力してください".to_string());
        }
        if let Some(limit) = self.limit
            && (limit == 0 || limit > 50)
        {
            return Err("limit は 1〜50 の範囲で指定してください".to_string());
        }
        if let Some(sort) = self.sort.as_deref()
            && sort != "relevance"
            && sort != "recency"
        {
            return Err("sort は relevance または recency を指定してください".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchUsersResponse {
    pub items: Vec<UserProfile>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_count: u64,
    pub took_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetFollowersRequest {
    pub npub: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub search: Option<String>,
    pub viewer_npub: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetFollowingRequest {
    pub npub: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub search: Option<String>,
    pub viewer_npub: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdatePrivacySettingsRequest {
    pub npub: String,
    pub public_profile: bool,
    pub show_online_status: bool,
}

impl crate::presentation::dto::Validate for UpdatePrivacySettingsRequest {
    fn validate(&self) -> Result<(), String> {
        if self.npub.trim().is_empty() {
            return Err("npub is required".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateUserProfileRequest {
    pub npub: String,
    pub name: String,
    pub display_name: String,
    pub about: String,
    pub picture: String,
    pub nip05: String,
}

impl crate::presentation::dto::Validate for UpdateUserProfileRequest {
    fn validate(&self) -> Result<(), String> {
        if self.npub.trim().is_empty() {
            return Err("npub is required".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("name is required".to_string());
        }
        if self.name.chars().count() > 100 {
            return Err("name is too long (max 100 characters)".to_string());
        }
        if self.display_name.chars().count() > 100 {
            return Err("display_name is too long (max 100 characters)".to_string());
        }
        if self.about.chars().count() > 1_000 {
            return Err("about is too long (max 1000 characters)".to_string());
        }
        if self.picture.chars().count() > 1_024 {
            return Err("picture is too long (max 1024 characters)".to_string());
        }
        if self.nip05.chars().count() > 255 {
            return Err("nip05 is too long (max 255 characters)".to_string());
        }
        Ok(())
    }
}
