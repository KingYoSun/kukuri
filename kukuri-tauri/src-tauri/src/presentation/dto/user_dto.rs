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
        if let Some(limit) = self.limit {
            if limit == 0 || limit > 50 {
                return Err("limit は 1〜50 の範囲で指定してください".to_string());
            }
        }
        if let Some(sort) = self.sort.as_deref() {
            if sort != "relevance" && sort != "recency" {
                return Err("sort は relevance または recency を指定してください".to_string());
            }
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
