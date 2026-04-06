use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::normalize::{normalize_http_url, normalize_http_url_list};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeResolvedUrls {
    pub public_base_url: String,
    pub connectivity_urls: Vec<String>,
    #[serde(default)]
    pub seed_peers: Vec<CommunityNodeSeedPeer>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeSeedPeer {
    pub endpoint_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr_hint: Option<String>,
}

impl CommunityNodeSeedPeer {
    pub fn new(endpoint_id: impl Into<String>, addr_hint: Option<String>) -> Result<Self> {
        let endpoint_id = endpoint_id.into().trim().to_string();
        if endpoint_id.is_empty() {
            bail!("community-node seed peer endpoint id must not be empty");
        }
        let addr_hint = addr_hint
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Ok(Self {
            endpoint_id,
            addr_hint,
        })
    }

    pub fn display(&self) -> String {
        match self.addr_hint.as_deref() {
            Some(addr_hint) => format!("{}@{}", self.endpoint_id, addr_hint),
            None => self.endpoint_id.clone(),
        }
    }
}

pub(crate) fn normalize_seed_peers(
    values: Vec<CommunityNodeSeedPeer>,
) -> Result<Vec<CommunityNodeSeedPeer>> {
    let mut deduped = BTreeMap::new();
    for value in values {
        let normalized = CommunityNodeSeedPeer::new(value.endpoint_id, value.addr_hint)?;
        deduped.insert(normalized.display(), normalized);
    }
    Ok(deduped.into_values().collect())
}

impl CommunityNodeResolvedUrls {
    pub fn new(
        public_base_url: impl Into<String>,
        connectivity_urls: Vec<String>,
        seed_peers: Vec<CommunityNodeSeedPeer>,
    ) -> Result<Self> {
        let public_base_url = normalize_http_url(public_base_url.into().as_str())?;
        let connectivity_urls = normalize_http_url_list(connectivity_urls)?;
        let seed_peers = normalize_seed_peers(seed_peers)?;
        Ok(Self {
            public_base_url,
            connectivity_urls,
            seed_peers,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeBootstrapNode {
    pub base_url: String,
    pub resolved_urls: CommunityNodeResolvedUrls,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthChallengeResponse {
    pub challenge: String,
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthVerifyResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: i64,
    pub pubkey: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapHeartbeatResponse {
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeConsentItem {
    pub policy_slug: String,
    pub policy_version: i32,
    pub title: String,
    pub required: bool,
    pub accepted_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityNodeConsentStatus {
    pub all_required_accepted: bool,
    pub items: Vec<CommunityNodeConsentItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BearerIdentity {
    pub pubkey: String,
    pub endpoint_id: Option<String>,
}
