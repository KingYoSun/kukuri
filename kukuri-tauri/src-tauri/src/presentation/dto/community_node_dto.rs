use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeRoleConfig {
    pub labels: bool,
    pub trust: bool,
    pub search: bool,
    pub bootstrap: bool,
}

impl Default for CommunityNodeRoleConfig {
    fn default() -> Self {
        Self {
            labels: true,
            trust: true,
            search: false,
            bootstrap: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeConfigNodeRequest {
    pub base_url: String,
    pub roles: Option<CommunityNodeRoleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeConfigRequest {
    pub nodes: Vec<CommunityNodeConfigNodeRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeConfigNodeResponse {
    pub base_url: String,
    pub roles: CommunityNodeRoleConfig,
    pub has_token: bool,
    pub token_expires_at: Option<i64>,
    pub pubkey: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeConfigResponse {
    pub nodes: Vec<CommunityNodeConfigNodeResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeAuthResponse {
    pub expires_at: i64,
    pub pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeLabelsRequest {
    pub base_url: Option<String>,
    pub target: String,
    pub topic: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeTrustRequest {
    pub base_url: Option<String>,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeTrustProviderRequest {
    pub provider_pubkey: String,
    pub assertion_kind: Option<u16>,
    pub relay_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeTrustProviderState {
    pub provider_pubkey: String,
    pub assertion_kind: u16,
    pub relay_url: Option<String>,
    pub issued_at: i64,
    pub event_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeSearchRequest {
    pub base_url: Option<String>,
    pub topic: String,
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeConsentRequest {
    pub base_url: Option<String>,
    pub policy_ids: Option<Vec<String>>,
    pub accept_all_current: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeReportRequest {
    pub base_url: Option<String>,
    pub report_event_json: Option<serde_json::Value>,
    pub target: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeAuthRequest {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeTokenRequest {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNodeBootstrapServicesRequest {
    pub base_url: Option<String>,
    pub topic_id: String,
}
