use crate::application::ports::group_key_store::{GroupKeyEntry, GroupKeyRecord, GroupKeyStore};
use crate::application::ports::key_manager::KeyManager;
use crate::infrastructure::storage::SecureStorage;
use crate::presentation::dto::community_node_dto::{
    CommunityNodeAuthRequest, CommunityNodeAuthResponse, CommunityNodeBootstrapServicesRequest,
    CommunityNodeConfigRequest, CommunityNodeConfigResponse, CommunityNodeConsentRequest,
    CommunityNodeKeyEnvelopeRequest, CommunityNodeKeyEnvelopeResponse, CommunityNodeLabelsRequest,
    CommunityNodeRedeemInviteRequest, CommunityNodeRedeemInviteResponse,
    CommunityNodeReportRequest, CommunityNodeRoleConfig, CommunityNodeSearchRequest,
    CommunityNodeTokenRequest, CommunityNodeTrustRequest,
};
use crate::shared::{AppError, ValidationFailureKind};
use chrono::Utc;
use nostr_sdk::prelude::{
    Event as NostrEvent, EventBuilder, FromBech32, Keys, Kind, SecretKey, Tag, nip44,
};
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

const COMMUNITY_NODE_CONFIG_KEY: &str = "community_node_config_v2";
const COMMUNITY_NODE_CONFIG_LEGACY_KEY: &str = "community_node_config_v1";
const AUTH_KIND: u16 = 22242;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CommunityNodeConfig {
    #[serde(default)]
    nodes: Vec<CommunityNodeConfigNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommunityNodeConfigNode {
    base_url: String,
    #[serde(default)]
    roles: CommunityNodeRoleConfig,
    access_token: Option<String>,
    token_expires_at: Option<i64>,
    pubkey: Option<String>,
}

impl CommunityNodeConfigNode {
    fn new(base_url: String, roles: CommunityNodeRoleConfig) -> Self {
        Self {
            base_url,
            roles,
            access_token: None,
            token_expires_at: None,
            pubkey: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyCommunityNodeConfig {
    base_url: String,
    access_token: Option<String>,
    token_expires_at: Option<i64>,
    pubkey: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthChallengeResponse {
    challenge: String,
    #[serde(rename = "expires_at")]
    _expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct AuthVerifyResponse {
    access_token: String,
    #[serde(rename = "token_type")]
    _token_type: String,
    expires_at: i64,
    pubkey: String,
}

#[derive(Debug, Deserialize)]
struct KeyEnvelopeListResponse {
    items: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CommunityNodeSearchPayload {
    items: Vec<serde_json::Value>,
    next_cursor: Option<String>,
    total: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InviteRedeemResponse {
    topic_id: String,
    scope: String,
    epoch: i64,
    key_envelope_event: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct KeyEnvelopePayload {
    schema: String,
    topic: String,
    scope: String,
    epoch: i64,
    key_b64: String,
    issued_at: Option<i64>,
}

pub struct CommunityNodeHandler {
    key_manager: Arc<dyn KeyManager>,
    secure_storage: Arc<dyn SecureStorage>,
    group_key_store: Arc<dyn GroupKeyStore>,
    client: Client,
}

impl CommunityNodeHandler {
    pub fn new(
        key_manager: Arc<dyn KeyManager>,
        secure_storage: Arc<dyn SecureStorage>,
        group_key_store: Arc<dyn GroupKeyStore>,
    ) -> Self {
        Self {
            key_manager,
            secure_storage,
            group_key_store,
            client: Client::new(),
        }
    }

    pub async fn set_config(
        &self,
        request: CommunityNodeConfigRequest,
    ) -> Result<CommunityNodeConfigResponse, AppError> {
        let mut config = self.load_config().await?.unwrap_or_default();
        let mut existing = HashMap::new();
        for node in config.nodes.drain(..) {
            existing.insert(node.base_url.clone(), node);
        }

        let mut next_nodes = Vec::new();
        let mut seen = HashMap::new();
        for node_request in request.nodes {
            let base_url = normalize_base_url(&node_request.base_url)?;
            if seen.contains_key(&base_url) {
                continue;
            }
            seen.insert(base_url.clone(), true);
            let roles = node_request.roles.unwrap_or_default();
            let mut node = existing
                .remove(&base_url)
                .unwrap_or_else(|| CommunityNodeConfigNode::new(base_url.clone(), roles.clone()));
            if node.base_url != base_url {
                node.access_token = None;
                node.token_expires_at = None;
                node.pubkey = None;
            }
            node.base_url = base_url;
            node.roles = roles;
            next_nodes.push(node);
        }
        config.nodes = next_nodes;
        self.save_config(&config).await?;
        Ok(config_response(&config))
    }

    pub async fn get_config(&self) -> Result<Option<CommunityNodeConfigResponse>, AppError> {
        let Some(config) = self.load_config().await? else {
            return Ok(None);
        };
        if config.nodes.is_empty() {
            return Ok(None);
        }
        Ok(Some(config_response(&config)))
    }

    pub async fn clear_config(&self) -> Result<(), AppError> {
        self.secure_storage
            .delete(COMMUNITY_NODE_CONFIG_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let _ = self
            .secure_storage
            .delete(COMMUNITY_NODE_CONFIG_LEGACY_KEY)
            .await;
        Ok(())
    }

    pub async fn clear_token(&self, request: CommunityNodeTokenRequest) -> Result<(), AppError> {
        let base_url = normalize_base_url(&request.base_url)?;
        let mut config = self.require_config().await?;
        let node = find_node_mut(&mut config, &base_url)?;
        node.access_token = None;
        node.token_expires_at = None;
        node.pubkey = None;
        self.save_config(&config).await
    }

    pub async fn authenticate(
        &self,
        request: CommunityNodeAuthRequest,
    ) -> Result<CommunityNodeAuthResponse, AppError> {
        let base_url = normalize_base_url(&request.base_url)?;
        let mut config = self.require_config().await?;
        let node = find_node_mut(&mut config, &base_url)?;
        let keypair = self.key_manager.current_keypair().await?;
        let challenge = self
            .request_auth_challenge(&node.base_url, &keypair.public_key)
            .await?;
        let auth_event = build_auth_event(&node.base_url, &challenge.challenge, &keypair.nsec)?;
        let verified = self.verify_auth(&node.base_url, &auth_event).await?;
        node.access_token = Some(verified.access_token.clone());
        node.token_expires_at = Some(verified.expires_at);
        node.pubkey = Some(verified.pubkey.clone());
        self.save_config(&config).await?;

        Ok(CommunityNodeAuthResponse {
            expires_at: verified.expires_at,
            pubkey: verified.pubkey,
        })
    }

    pub async fn list_group_keys(&self) -> Result<Vec<GroupKeyEntry>, AppError> {
        self.group_key_store.list_keys().await
    }

    pub async fn sync_key_envelopes(
        &self,
        request: CommunityNodeKeyEnvelopeRequest,
    ) -> Result<CommunityNodeKeyEnvelopeResponse, AppError> {
        let config = self.require_config().await?;
        let node = select_node(&config, request.base_url.as_deref())?;
        let url = build_url(&node.base_url, "/v1/keys/envelopes");
        let mut builder = self
            .authorized_request(node, Method::GET, url, true)
            .await?;
        builder = builder.query(&[
            ("topic_id", request.topic_id.clone()),
            (
                "scope",
                request
                    .scope
                    .clone()
                    .unwrap_or_else(|| "invite".to_string()),
            ),
        ]);
        if let Some(after_epoch) = request.after_epoch {
            builder = builder.query(&[("after_epoch", after_epoch.to_string())]);
        }
        let response: KeyEnvelopeListResponse = request_json(builder).await?;
        let mut stored = Vec::new();
        for value in response.items {
            let entry = self.store_key_envelope(value).await?;
            stored.push(entry);
        }
        Ok(CommunityNodeKeyEnvelopeResponse { stored })
    }

    pub async fn redeem_invite(
        &self,
        request: CommunityNodeRedeemInviteRequest,
    ) -> Result<CommunityNodeRedeemInviteResponse, AppError> {
        let config = self.require_config().await?;
        let node = select_node(&config, request.base_url.as_deref())?;
        let url = build_url(&node.base_url, "/v1/invite/redeem");
        let builder = self
            .authorized_request(node, Method::POST, url, true)
            .await?
            .json(&json!({ "capability_event_json": request.capability_event_json }));
        let response: InviteRedeemResponse = request_json(builder).await?;
        let _ = self.store_key_envelope(response.key_envelope_event).await?;
        Ok(CommunityNodeRedeemInviteResponse {
            topic_id: response.topic_id,
            scope: response.scope,
            epoch: response.epoch,
        })
    }

    pub async fn list_labels(
        &self,
        request: CommunityNodeLabelsRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let nodes = select_nodes_for_role(
            &config,
            request.base_url.as_deref(),
            CommunityNodeRole::Labels,
        )?;
        let mut items: Vec<serde_json::Value> = Vec::new();
        let mut last_error: Option<AppError> = None;

        for node in nodes {
            let url = build_url(&node.base_url, "/v1/labels");
            let mut builder = self
                .authorized_request(node, Method::GET, url, true)
                .await?;
            builder = builder.query(&[
                ("target", request.target.clone()),
                ("limit", request.limit.unwrap_or(50).to_string()),
            ]);
            if let Some(topic) = request.topic.clone() {
                builder = builder.query(&[("topic", topic)]);
            }
            if let Some(cursor) = request.cursor.clone() {
                builder = builder.query(&[("cursor", cursor)]);
            }
            match request_json::<serde_json::Value>(builder).await {
                Ok(response) => {
                    if let Some(list) = response.get("items").and_then(|value| value.as_array()) {
                        items.extend(list.iter().cloned());
                    }
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        if items.is_empty() {
            return Err(last_error.unwrap_or_else(|| {
                AppError::NotFound("Community node labels are unavailable".to_string())
            }));
        }

        Ok(json!({ "items": items }))
    }

    pub async fn submit_report(
        &self,
        request: CommunityNodeReportRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let node = select_node(&config, request.base_url.as_deref())?;
        let url = build_url(&node.base_url, "/v1/reports");
        let builder = self
            .authorized_request(node, Method::POST, url, true)
            .await?
            .json(&request);
        request_json(builder).await
    }

    pub async fn trust_report_based(
        &self,
        request: CommunityNodeTrustRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let nodes = select_nodes_for_role(
            &config,
            request.base_url.as_deref(),
            CommunityNodeRole::Trust,
        )?;
        self.aggregate_trust_scores(nodes, "/v1/trust/report-based", &request.subject)
            .await
    }

    pub async fn trust_communication_density(
        &self,
        request: CommunityNodeTrustRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let nodes = select_nodes_for_role(
            &config,
            request.base_url.as_deref(),
            CommunityNodeRole::Trust,
        )?;
        self.aggregate_trust_scores(nodes, "/v1/trust/communication-density", &request.subject)
            .await
    }

    pub async fn search(
        &self,
        request: CommunityNodeSearchRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let nodes = select_nodes_for_role(
            &config,
            request.base_url.as_deref(),
            CommunityNodeRole::Search,
        )?;
        self.aggregate_search(nodes, request).await
    }

    pub async fn list_bootstrap_nodes(&self) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let nodes = select_nodes_for_role(&config, None, CommunityNodeRole::Bootstrap)?;
        self.aggregate_bootstrap(nodes, "/v1/bootstrap/nodes", None)
            .await
    }

    pub async fn list_bootstrap_services(
        &self,
        request: CommunityNodeBootstrapServicesRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let nodes = select_nodes_for_role(
            &config,
            request.base_url.as_deref(),
            CommunityNodeRole::Bootstrap,
        )?;
        let path = format!("/v1/bootstrap/topics/{}/services", request.topic_id);
        self.aggregate_bootstrap(nodes, &path, Some(request.topic_id))
            .await
    }

    pub async fn get_consent_status(
        &self,
        request: CommunityNodeTokenRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let node = select_node(&config, Some(&request.base_url))?;
        let url = build_url(&node.base_url, "/v1/consents/status");
        let builder = self
            .authorized_request(node, Method::GET, url, true)
            .await?;
        request_json(builder).await
    }

    pub async fn accept_consents(
        &self,
        request: CommunityNodeConsentRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let node = select_node(&config, request.base_url.as_deref())?;
        let url = build_url(&node.base_url, "/v1/consents");
        let builder = self
            .authorized_request(node, Method::POST, url, true)
            .await?
            .json(&request);
        request_json(builder).await
    }

    async fn require_config(&self) -> Result<CommunityNodeConfig, AppError> {
        self.load_config()
            .await?
            .filter(|cfg| !cfg.nodes.is_empty())
            .ok_or_else(|| AppError::NotFound("Community node is not configured".to_string()))
    }

    async fn load_config(&self) -> Result<Option<CommunityNodeConfig>, AppError> {
        let raw = self
            .secure_storage
            .retrieve(COMMUNITY_NODE_CONFIG_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let Some(raw) = raw else {
            return self.load_legacy_config().await;
        };
        let parsed = serde_json::from_str(&raw)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        Ok(Some(parsed))
    }

    async fn load_legacy_config(&self) -> Result<Option<CommunityNodeConfig>, AppError> {
        let raw = self
            .secure_storage
            .retrieve(COMMUNITY_NODE_CONFIG_LEGACY_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let legacy: LegacyCommunityNodeConfig = serde_json::from_str(&raw)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        if legacy.base_url.trim().is_empty() {
            return Ok(None);
        }
        let base_url = normalize_base_url(&legacy.base_url)?;
        let mut node = CommunityNodeConfigNode::new(base_url, CommunityNodeRoleConfig::default());
        node.access_token = legacy.access_token;
        node.token_expires_at = legacy.token_expires_at;
        node.pubkey = legacy.pubkey;
        let config = CommunityNodeConfig { nodes: vec![node] };
        self.save_config(&config).await?;
        let _ = self
            .secure_storage
            .delete(COMMUNITY_NODE_CONFIG_LEGACY_KEY)
            .await;
        Ok(Some(config))
    }

    async fn save_config(&self, config: &CommunityNodeConfig) -> Result<(), AppError> {
        let json = serde_json::to_string(config)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        self.secure_storage
            .store(COMMUNITY_NODE_CONFIG_KEY, &json)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        Ok(())
    }

    async fn authorized_request(
        &self,
        node: &CommunityNodeConfigNode,
        method: Method,
        url: String,
        require_auth: bool,
    ) -> Result<reqwest::RequestBuilder, AppError> {
        let builder = self.client.request(method, url);
        let Some(token) = node.access_token.as_ref() else {
            if require_auth {
                return Err(AppError::Unauthorized(
                    "Community node token is missing".to_string(),
                ));
            }
            return Ok(builder);
        };
        if let Some(exp) = node.token_expires_at {
            if exp <= Utc::now().timestamp() {
                if !require_auth {
                    return Ok(builder);
                }
                return Err(AppError::Unauthorized(
                    "Community node token has expired".to_string(),
                ));
            }
        }
        Ok(builder.bearer_auth(token))
    }

    async fn aggregate_trust_scores(
        &self,
        nodes: Vec<&CommunityNodeConfigNode>,
        path: &str,
        subject: &str,
    ) -> Result<serde_json::Value, AppError> {
        let mut scores: Vec<f64> = Vec::new();
        let mut sources: Vec<serde_json::Value> = Vec::new();
        let mut last_error: Option<AppError> = None;

        for node in nodes {
            let url = build_url(&node.base_url, path);
            let builder = match self.authorized_request(node, Method::GET, url, true).await {
                Ok(builder) => builder,
                Err(err) => {
                    last_error = Some(err);
                    continue;
                }
            };
            let builder = builder.query(&[("subject", subject.to_string())]);
            match request_json::<serde_json::Value>(builder).await {
                Ok(response) => {
                    if let Some(score) = response.get("score").and_then(|value| value.as_f64()) {
                        scores.push(score);
                        sources.push(json!({
                            "base_url": node.base_url.clone(),
                            "score": score,
                        }));
                    }
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        if scores.is_empty() {
            return Err(last_error.unwrap_or_else(|| {
                AppError::NotFound("Community node trust score is unavailable".to_string())
            }));
        }

        let sum: f64 = scores.iter().sum();
        let avg = sum / scores.len() as f64;

        Ok(json!({
            "score": avg,
            "sources": sources,
        }))
    }

    async fn aggregate_search(
        &self,
        nodes: Vec<&CommunityNodeConfigNode>,
        request: CommunityNodeSearchRequest,
    ) -> Result<serde_json::Value, AppError> {
        let use_composite_cursor = nodes.len() > 1;
        let cursor_map = request
            .cursor
            .as_ref()
            .and_then(|raw| parse_cursor_map(raw, use_composite_cursor));
        let mut items: Vec<serde_json::Value> = Vec::new();
        let mut next_cursor_map: HashMap<String, String> = HashMap::new();
        let mut total: i64 = 0;
        let mut last_error: Option<AppError> = None;

        for node in nodes {
            let url = build_url(&node.base_url, "/v1/search");
            let builder = match self.authorized_request(node, Method::GET, url, true).await {
                Ok(builder) => builder,
                Err(err) => {
                    last_error = Some(err);
                    continue;
                }
            };

            let mut builder = builder.query(&[("topic", request.topic.clone())]);
            if let Some(query) = request.q.clone() {
                builder = builder.query(&[("q", query)]);
            }
            if let Some(limit) = request.limit {
                builder = builder.query(&[("limit", limit.to_string())]);
            }
            let node_cursor = if use_composite_cursor {
                cursor_map
                    .as_ref()
                    .and_then(|map| map.get(&node.base_url).cloned())
            } else {
                request.cursor.clone()
            };
            if let Some(cursor) = node_cursor {
                builder = builder.query(&[("cursor", cursor)]);
            }

            match request_json::<CommunityNodeSearchPayload>(builder).await {
                Ok(response) => {
                    total += response.total.unwrap_or(response.items.len() as i64);
                    if let Some(next_cursor) = response.next_cursor {
                        next_cursor_map.insert(node.base_url.clone(), next_cursor);
                    }
                    items.extend(response.items);
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        if items.is_empty() {
            return Err(last_error.unwrap_or_else(|| {
                AppError::NotFound("Community node search is unavailable".to_string())
            }));
        }

        let next_cursor = if next_cursor_map.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&next_cursor_map)
                    .map_err(|err| AppError::SerializationError(err.to_string()))?,
            )
        };

        Ok(json!({
            "topic": request.topic,
            "query": request.q,
            "items": items,
            "next_cursor": next_cursor,
            "total": total,
        }))
    }

    async fn aggregate_bootstrap(
        &self,
        nodes: Vec<&CommunityNodeConfigNode>,
        path: &str,
        _topic_id: Option<String>,
    ) -> Result<serde_json::Value, AppError> {
        let mut items: Vec<serde_json::Value> = Vec::new();
        let mut last_error: Option<AppError> = None;

        for node in nodes {
            let url = build_url(&node.base_url, path);
            let builder = match self.authorized_request(node, Method::GET, url, false).await {
                Ok(builder) => builder,
                Err(err) => {
                    last_error = Some(err);
                    continue;
                }
            };
            match request_json::<serde_json::Value>(builder).await {
                Ok(response) => {
                    if let Some(list) = response.get("items").and_then(|value| value.as_array()) {
                        items.extend(list.iter().cloned());
                    }
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        if items.is_empty() {
            return Err(last_error.unwrap_or_else(|| {
                AppError::NotFound("Community node bootstrap data is unavailable".to_string())
            }));
        }

        Ok(json!({ "items": items }))
    }

    async fn request_auth_challenge(
        &self,
        base_url: &str,
        pubkey: &str,
    ) -> Result<AuthChallengeResponse, AppError> {
        let url = build_url(base_url, "/v1/auth/challenge");
        let builder = self.client.post(url).json(&json!({ "pubkey": pubkey }));
        request_json(builder).await
    }

    async fn verify_auth(
        &self,
        base_url: &str,
        auth_event: &NostrEvent,
    ) -> Result<AuthVerifyResponse, AppError> {
        let url = build_url(base_url, "/v1/auth/verify");
        let payload = json!({ "auth_event_json": auth_event });
        let builder = self.client.post(url).json(&payload);
        request_json(builder).await
    }

    async fn store_key_envelope(
        &self,
        value: serde_json::Value,
    ) -> Result<GroupKeyEntry, AppError> {
        let event: NostrEvent = serde_json::from_value(value)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        let keypair = self.key_manager.current_keypair().await?;
        let secret_key = SecretKey::from_bech32(&keypair.nsec)
            .map_err(|err| AppError::Crypto(format!("Invalid nsec: {err}")))?;
        let decrypted = nip44::decrypt(&secret_key, &event.pubkey, event.content)
            .map_err(|err| AppError::Crypto(format!("NIP-44 decrypt failed: {err}")))?;
        let payload: KeyEnvelopePayload = serde_json::from_str(&decrypted)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        if payload.schema != "kukuri-key-envelope-v1" && payload.schema != "kukuri-keyenv-v1" {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invalid key envelope schema",
            ));
        }
        let stored_at = payload.issued_at.unwrap_or_else(|| Utc::now().timestamp());
        let record = GroupKeyRecord {
            topic_id: payload.topic.clone(),
            scope: payload.scope.clone(),
            epoch: payload.epoch,
            key_b64: payload.key_b64.clone(),
            stored_at,
        };
        self.group_key_store.store_key(record).await?;
        Ok(GroupKeyEntry {
            topic_id: payload.topic,
            scope: payload.scope,
            epoch: payload.epoch,
            stored_at,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum CommunityNodeRole {
    Labels,
    Trust,
    Search,
    Bootstrap,
}

fn role_enabled(roles: &CommunityNodeRoleConfig, role: CommunityNodeRole) -> bool {
    match role {
        CommunityNodeRole::Labels => roles.labels,
        CommunityNodeRole::Trust => roles.trust,
        CommunityNodeRole::Search => roles.search,
        CommunityNodeRole::Bootstrap => roles.bootstrap,
    }
}

fn find_node<'a>(
    config: &'a CommunityNodeConfig,
    base_url: &str,
) -> Result<&'a CommunityNodeConfigNode, AppError> {
    config
        .nodes
        .iter()
        .find(|node| node.base_url == base_url)
        .ok_or_else(|| AppError::NotFound("Community node is not configured".to_string()))
}

fn find_node_mut<'a>(
    config: &'a mut CommunityNodeConfig,
    base_url: &str,
) -> Result<&'a mut CommunityNodeConfigNode, AppError> {
    config
        .nodes
        .iter_mut()
        .find(|node| node.base_url == base_url)
        .ok_or_else(|| AppError::NotFound("Community node is not configured".to_string()))
}

fn select_node<'a>(
    config: &'a CommunityNodeConfig,
    base_url: Option<&str>,
) -> Result<&'a CommunityNodeConfigNode, AppError> {
    if let Some(raw) = base_url {
        let base_url = normalize_base_url(raw)?;
        return find_node(config, &base_url);
    }
    config
        .nodes
        .first()
        .ok_or_else(|| AppError::NotFound("Community node is not configured".to_string()))
}

fn select_nodes_for_role<'a>(
    config: &'a CommunityNodeConfig,
    base_url: Option<&str>,
    role: CommunityNodeRole,
) -> Result<Vec<&'a CommunityNodeConfigNode>, AppError> {
    if let Some(raw) = base_url {
        let base_url = normalize_base_url(raw)?;
        let node = find_node(config, &base_url)?;
        return Ok(vec![node]);
    }

    let nodes: Vec<_> = config
        .nodes
        .iter()
        .filter(|node| role_enabled(&node.roles, role))
        .collect();
    if nodes.is_empty() {
        return Err(AppError::NotFound(
            "Community node role is not configured".to_string(),
        ));
    }
    Ok(nodes)
}

fn parse_cursor_map(raw: &str, enable: bool) -> Option<HashMap<String, String>> {
    if !enable {
        return None;
    }
    serde_json::from_str::<HashMap<String, String>>(raw).ok()
}

fn normalize_base_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim().trim_end_matches('/').to_string();
    let url = Url::parse(&trimmed).map_err(|err| {
        AppError::validation(
            ValidationFailureKind::Generic,
            format!("Invalid URL: {err}"),
        )
    })?;
    match url.scheme() {
        "http" | "https" => Ok(trimmed),
        _ => Err(AppError::validation(
            ValidationFailureKind::Generic,
            "URL scheme must be http or https",
        )),
    }
}

fn build_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

fn config_response(config: &CommunityNodeConfig) -> CommunityNodeConfigResponse {
    CommunityNodeConfigResponse {
        nodes: config
            .nodes
            .iter()
            .map(|node| {
                crate::presentation::dto::community_node_dto::CommunityNodeConfigNodeResponse {
                    base_url: node.base_url.clone(),
                    roles: node.roles.clone(),
                    has_token: node.access_token.is_some(),
                    token_expires_at: node.token_expires_at,
                    pubkey: node.pubkey.clone(),
                }
            })
            .collect(),
    }
}

fn build_auth_event(base_url: &str, challenge: &str, nsec: &str) -> Result<NostrEvent, AppError> {
    let secret_key =
        SecretKey::from_bech32(nsec).map_err(|err| AppError::Crypto(err.to_string()))?;
    let keys = Keys::new(secret_key);
    let tags = vec![
        Tag::parse(["relay", base_url]).map_err(|err| AppError::NostrError(err.to_string()))?,
        Tag::parse(["challenge", challenge])
            .map_err(|err| AppError::NostrError(err.to_string()))?,
    ];
    let event = EventBuilder::new(Kind::Custom(AUTH_KIND), "")
        .tags(tags)
        .sign_with_keys(&keys)?;
    Ok(event)
}

async fn request_json<T: DeserializeOwned>(
    builder: reqwest::RequestBuilder,
) -> Result<T, AppError> {
    let response = builder
        .send()
        .await
        .map_err(|err| AppError::Network(err.to_string()))?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .text()
        .await
        .map_err(|err| AppError::Network(err.to_string()))?;
    if !status.is_success() {
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = headers
                .get("Retry-After")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(60);
            return Err(AppError::rate_limited(
                "Community node rate limited",
                retry_after,
            ));
        }
        return Err(AppError::Network(format!(
            "Community node error ({status}): {body}"
        )));
    }
    serde_json::from_str(&body).map_err(|err| AppError::DeserializationError(err.to_string()))
}
