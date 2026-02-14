use crate::application::ports::group_key_store::{GroupKeyEntry, GroupKeyStore};
use crate::application::ports::key_manager::KeyManager;
use crate::infrastructure::storage::SecureStorage;
use crate::presentation::dto::community_node_dto::{
    CommunityNodeAuthRequest, CommunityNodeAuthResponse, CommunityNodeBootstrapServicesRequest,
    CommunityNodeConfigRequest, CommunityNodeConfigResponse, CommunityNodeConsentRequest,
    CommunityNodeLabelsRequest, CommunityNodeReportRequest, CommunityNodeRoleConfig,
    CommunityNodeSearchRequest, CommunityNodeTokenRequest, CommunityNodeTrustAnchorRequest,
    CommunityNodeTrustAnchorState, CommunityNodeTrustRequest,
};
use crate::shared::{AppError, ValidationFailureKind};
use chrono::Utc;
use nostr_sdk::prelude::{
    Event as NostrEvent, EventBuilder, FromBech32, Keys, Kind, PublicKey, SecretKey, Tag,
};
use reqwest::{Client, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

const COMMUNITY_NODE_CONFIG_KEY: &str = "community_node_config_v2";
const COMMUNITY_NODE_CONFIG_LEGACY_KEY: &str = "community_node_config_v1";
const COMMUNITY_NODE_TRUST_ANCHOR_KEY: &str = "community_node_trust_anchor_v1";
const COMMUNITY_NODE_BOOTSTRAP_CACHE_KEY: &str = "community_node_bootstrap_cache_v1";
const AUTH_KIND: u16 = 22242;
const NODE_DESCRIPTOR_KIND: u16 = 39000;
const TOPIC_SERVICE_KIND: u16 = 39001;
const LABEL_KIND: u16 = 39006;
const ATTESTATION_KIND: u16 = 39010;
const TRUST_ANCHOR_KIND: u16 = 39011;
const KIP_NAMESPACE: &str = "kukuri";
const KIP_VERSION: &str = "1";
const KIP_NODE_DESCRIPTOR_SCHEMA: &str = "kukuri-node-desc-v1";
const KIP_TOPIC_SERVICE_SCHEMA: &str = "kukuri-topic-service-v1";
const KIP_ATTESTATION_SCHEMA: &str = "kukuri-attest-v1";
const KIP_BOOTSTRAP_HINT_SCHEMA: &str = "kukuri-bootstrap-update-hint-v1";

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTrustAnchor {
    attester: String,
    claim: Option<String>,
    topic: Option<String>,
    weight: f64,
    issued_at: i64,
    event_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BootstrapCache {
    #[serde(default)]
    nodes: BootstrapCacheEntry,
    #[serde(default)]
    services: HashMap<String, BootstrapCacheEntry>,
    #[serde(default)]
    hint_cursors: HashMap<String, BootstrapHintCursorEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BootstrapCacheEntry {
    #[serde(default)]
    items: Vec<serde_json::Value>,
    #[serde(default)]
    next_refresh_at: Option<i64>,
    #[serde(default)]
    updated_at: Option<i64>,
    #[serde(default)]
    stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BootstrapHintCursorEntry {
    #[serde(default)]
    last_seq: u64,
}

impl From<StoredTrustAnchor> for CommunityNodeTrustAnchorState {
    fn from(anchor: StoredTrustAnchor) -> Self {
        Self {
            attester: anchor.attester,
            claim: anchor.claim,
            topic: anchor.topic,
            weight: anchor.weight,
            issued_at: anchor.issued_at,
            event_json: anchor.event_json,
        }
    }
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
struct CommunityNodeSearchPayload {
    items: Vec<serde_json::Value>,
    next_cursor: Option<String>,
    total: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BootstrapHttpResponse {
    items: Vec<serde_json::Value>,
    #[serde(default)]
    next_refresh_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BootstrapHintHttpResponse {
    seq: u64,
    #[allow(dead_code)]
    #[serde(default)]
    received_at: Option<i64>,
    hint: BootstrapHintPayload,
}

#[derive(Debug, Deserialize, Default)]
struct BootstrapHintPayload {
    #[serde(default)]
    schema: Option<String>,
    #[serde(default)]
    descriptor_changed: bool,
    #[serde(default)]
    changed_topic_ids: Vec<String>,
    #[serde(default)]
    refresh_paths: Vec<String>,
}

#[derive(Debug, Clone)]
struct BootstrapAggregateResult {
    items: Vec<serde_json::Value>,
    next_refresh_at: Option<i64>,
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

    pub async fn get_trust_anchor(
        &self,
    ) -> Result<Option<CommunityNodeTrustAnchorState>, AppError> {
        let raw = self
            .secure_storage
            .retrieve(COMMUNITY_NODE_TRUST_ANCHOR_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let stored: StoredTrustAnchor = serde_json::from_str(&raw)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        Ok(Some(stored.into()))
    }

    pub async fn set_trust_anchor(
        &self,
        request: CommunityNodeTrustAnchorRequest,
    ) -> Result<CommunityNodeTrustAnchorState, AppError> {
        let attester = request.attester.trim().to_string();
        if attester.is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Attester is required",
            ));
        }
        PublicKey::from_hex(&attester).map_err(|err| {
            AppError::validation(
                ValidationFailureKind::Generic,
                format!("Invalid attester pubkey: {err}"),
            )
        })?;

        let weight = request.weight.unwrap_or(1.0);
        if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Weight must be between 0 and 1",
            ));
        }

        let claim = normalize_optional_value(request.claim);
        let topic = normalize_optional_value(request.topic);

        let keypair = self.key_manager.current_keypair().await?;
        let secret_key = SecretKey::from_bech32(&keypair.nsec)
            .map_err(|err| AppError::Crypto(format!("Invalid nsec: {err}")))?;
        let keys = Keys::new(secret_key);
        let mut tags = vec![
            Tag::parse(["k", KIP_NAMESPACE])
                .map_err(|err| AppError::NostrError(err.to_string()))?,
            Tag::parse(["ver", KIP_VERSION])
                .map_err(|err| AppError::NostrError(err.to_string()))?,
            Tag::parse(["attester", &attester])
                .map_err(|err| AppError::NostrError(err.to_string()))?,
            Tag::parse(["weight", &weight.to_string()])
                .map_err(|err| AppError::NostrError(err.to_string()))?,
        ];
        if let Some(value) = claim.as_ref() {
            tags.push(
                Tag::parse(["claim", value])
                    .map_err(|err| AppError::NostrError(err.to_string()))?,
            );
        }
        if let Some(value) = topic.as_ref() {
            tags.push(
                Tag::parse(["t", value]).map_err(|err| AppError::NostrError(err.to_string()))?,
            );
        }

        let event = EventBuilder::new(Kind::Custom(TRUST_ANCHOR_KIND), "")
            .tags(tags)
            .sign_with_keys(&keys)?;
        let event_json = serde_json::to_value(&event)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        let issued_at = event.created_at.as_secs() as i64;

        let stored = StoredTrustAnchor {
            attester,
            claim,
            topic,
            weight,
            issued_at,
            event_json,
        };
        let raw = serde_json::to_string(&stored)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        self.secure_storage
            .store(COMMUNITY_NODE_TRUST_ANCHOR_KEY, &raw)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;

        Ok(stored.into())
    }

    pub async fn clear_trust_anchor(&self) -> Result<(), AppError> {
        self.secure_storage
            .delete(COMMUNITY_NODE_TRUST_ANCHOR_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        Ok(())
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
        let now = Utc::now().timestamp();
        let current_pubkey = self
            .key_manager
            .current_keypair()
            .await
            .ok()
            .map(|pair| pair.public_key);

        for node in nodes {
            let expected_pubkey =
                resolve_expected_pubkey(node.pubkey.as_deref(), current_pubkey.as_deref());

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
                        for item in list {
                            if validate_kip_event_json(item, LABEL_KIND, expected_pubkey, now)
                                .is_some()
                            {
                                items.push(item.clone());
                            }
                        }
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
        let config = self.load_config().await?.unwrap_or_default();
        let nodes =
            select_nodes_for_role(&config, None, CommunityNodeRole::Bootstrap).unwrap_or_default();
        let now = Utc::now().timestamp();
        let mut cache = self.load_bootstrap_cache().await?;
        self.refresh_bootstrap_cache_from_hints(&mut cache, &nodes, None)
            .await;
        let mut entry = cache.nodes.clone();
        let mut items = sanitize_bootstrap_items(NODE_DESCRIPTOR_KIND, &entry.items, now, None);
        entry.items = items.clone();
        let mut last_error: Option<AppError> = None;

        if !nodes.is_empty() && (items.is_empty() || should_refresh_bootstrap(&entry, now)) {
            match self.aggregate_bootstrap(nodes, "/v1/bootstrap/nodes").await {
                Ok(result) => {
                    let fetched =
                        sanitize_bootstrap_items(NODE_DESCRIPTOR_KIND, &result.items, now, None);
                    entry.items = fetched.clone();
                    entry.next_refresh_at = result.next_refresh_at;
                    entry.updated_at = Some(now);
                    entry.stale = false;
                    items = fetched;
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        entry.updated_at = entry.updated_at.or(Some(now));
        cache.nodes = entry;
        self.save_bootstrap_cache(&cache).await?;

        if items.is_empty() {
            if let Some(err) = last_error {
                return Err(err);
            }
            return Err(AppError::NotFound(
                "Community node bootstrap data is unavailable".to_string(),
            ));
        }

        Ok(json!({
            "items": items,
            "next_refresh_at": cache.nodes.next_refresh_at
        }))
    }

    pub async fn list_bootstrap_services(
        &self,
        request: CommunityNodeBootstrapServicesRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.load_config().await?.unwrap_or_default();
        let nodes = if request.base_url.is_some() {
            select_nodes_for_role(
                &config,
                request.base_url.as_deref(),
                CommunityNodeRole::Bootstrap,
            )?
        } else {
            select_nodes_for_role(&config, None, CommunityNodeRole::Bootstrap).unwrap_or_default()
        };

        let topic_id = request.topic_id;
        let path = format!("/v1/bootstrap/topics/{}/services", topic_id);
        let now = Utc::now().timestamp();
        let mut cache = self.load_bootstrap_cache().await?;
        self.refresh_bootstrap_cache_from_hints(&mut cache, &nodes, Some(&topic_id))
            .await;
        let mut entry = cache.services.get(&topic_id).cloned().unwrap_or_default();
        let mut items =
            sanitize_bootstrap_items(TOPIC_SERVICE_KIND, &entry.items, now, Some(&topic_id));
        entry.items = items.clone();
        let mut last_error: Option<AppError> = None;

        if !nodes.is_empty() && (items.is_empty() || should_refresh_bootstrap(&entry, now)) {
            match self.aggregate_bootstrap(nodes, &path).await {
                Ok(result) => {
                    let fetched = sanitize_bootstrap_items(
                        TOPIC_SERVICE_KIND,
                        &result.items,
                        now,
                        Some(&topic_id),
                    );
                    entry.items = fetched.clone();
                    entry.next_refresh_at = result.next_refresh_at;
                    entry.updated_at = Some(now);
                    entry.stale = false;
                    items = fetched;
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        entry.updated_at = entry.updated_at.or(Some(now));
        cache.services.insert(topic_id.clone(), entry);
        self.save_bootstrap_cache(&cache).await?;

        if items.is_empty() {
            if let Some(err) = last_error {
                return Err(err);
            }
            return Err(AppError::NotFound(
                "Community node bootstrap data is unavailable".to_string(),
            ));
        }

        Ok(json!({
            "items": items,
            "next_refresh_at": cache
                .services
                .get(&topic_id)
                .and_then(|entry| entry.next_refresh_at)
        }))
    }

    pub async fn refresh_bootstrap_from_hint(
        &self,
        topic_id: Option<&str>,
    ) -> Result<(), AppError> {
        let Some(config) = self.load_config().await? else {
            return Ok(());
        };
        let nodes =
            select_nodes_for_role(&config, None, CommunityNodeRole::Bootstrap).unwrap_or_default();
        if nodes.is_empty() {
            return Ok(());
        }

        let now = Utc::now().timestamp();
        let mut cache = self.load_bootstrap_cache().await?;

        if let Ok(result) = self
            .aggregate_bootstrap(nodes.clone(), "/v1/bootstrap/nodes")
            .await
        {
            let mut entry = cache.nodes.clone();
            entry.items = sanitize_bootstrap_items(NODE_DESCRIPTOR_KIND, &result.items, now, None);
            entry.next_refresh_at = result.next_refresh_at;
            entry.updated_at = Some(now);
            entry.stale = false;
            cache.nodes = entry;
        }

        if let Some(topic_id) = topic_id {
            let path = format!("/v1/bootstrap/topics/{}/services", topic_id);
            if let Ok(result) = self.aggregate_bootstrap(nodes, &path).await {
                let mut entry = cache.services.get(topic_id).cloned().unwrap_or_default();
                entry.items = sanitize_bootstrap_items(
                    TOPIC_SERVICE_KIND,
                    &result.items,
                    now,
                    Some(topic_id),
                );
                entry.next_refresh_at = result.next_refresh_at;
                entry.updated_at = Some(now);
                entry.stale = false;
                cache.services.insert(topic_id.to_string(), entry);
            }
        }

        self.save_bootstrap_cache(&cache).await?;
        Ok(())
    }

    pub async fn ingest_bootstrap_event(
        &self,
        event: &crate::domain::entities::Event,
    ) -> Result<(), AppError> {
        let kind = match u16::try_from(event.kind) {
            Ok(kind) => kind,
            Err(_) => return Ok(()),
        };
        if kind != NODE_DESCRIPTOR_KIND && kind != TOPIC_SERVICE_KIND {
            return Ok(());
        }

        let event_json = json!({
            "id": event.id,
            "pubkey": event.pubkey,
            "created_at": event.created_at.timestamp(),
            "kind": event.kind,
            "tags": event.tags,
            "content": event.content,
            "sig": event.sig,
        });

        let now = Utc::now().timestamp();
        let Some(nostr_event) = validate_kip_event_json(&event_json, kind, None, now) else {
            return Ok(());
        };

        let mut cache = self.load_bootstrap_cache().await?;
        let exp = event_tag_value(&nostr_event, "exp").and_then(|value| value.parse::<i64>().ok());

        if kind == NODE_DESCRIPTOR_KIND {
            merge_bootstrap_entry(
                &mut cache.nodes,
                kind,
                &nostr_event,
                event_json,
                exp,
                now,
                None,
            );
        } else {
            let Some(topic_id) = event_tag_value(&nostr_event, "t").map(|value| value.to_string())
            else {
                return Ok(());
            };
            let entry = cache.services.entry(topic_id.clone()).or_default();
            merge_bootstrap_entry(
                entry,
                kind,
                &nostr_event,
                event_json,
                exp,
                now,
                Some(topic_id.as_str()),
            );
        }

        self.save_bootstrap_cache(&cache).await?;
        Ok(())
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

    async fn load_bootstrap_cache(&self) -> Result<BootstrapCache, AppError> {
        let raw = self
            .secure_storage
            .retrieve(COMMUNITY_NODE_BOOTSTRAP_CACHE_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let Some(raw) = raw else {
            return Ok(BootstrapCache::default());
        };
        serde_json::from_str(&raw).map_err(|err| AppError::DeserializationError(err.to_string()))
    }

    async fn save_bootstrap_cache(&self, cache: &BootstrapCache) -> Result<(), AppError> {
        let json = serde_json::to_string(cache)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        self.secure_storage
            .store(COMMUNITY_NODE_BOOTSTRAP_CACHE_KEY, &json)
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
        let now = Utc::now().timestamp();
        let current_pubkey = self
            .key_manager
            .current_keypair()
            .await
            .ok()
            .map(|pair| pair.public_key);

        for node in nodes {
            let expected_pubkey =
                resolve_expected_pubkey(node.pubkey.as_deref(), current_pubkey.as_deref());
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
                        if !validate_attestation_payload(&response, expected_pubkey, now) {
                            last_error = Some(AppError::validation(
                                ValidationFailureKind::Generic,
                                "Community node attestation is invalid",
                            ));
                            continue;
                        }
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
            if let Some(err) = last_error {
                return Err(err);
            }
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
    ) -> Result<BootstrapAggregateResult, AppError> {
        let mut items: Vec<serde_json::Value> = Vec::new();
        let mut next_refresh_at: Option<i64> = None;
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
            match request_json::<BootstrapHttpResponse>(builder).await {
                Ok(response) => {
                    items.extend(response.items);
                    if let Some(refresh) = response.next_refresh_at {
                        next_refresh_at = Some(match next_refresh_at {
                            Some(current) => current.min(refresh),
                            None => refresh,
                        });
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

        Ok(BootstrapAggregateResult {
            items,
            next_refresh_at,
        })
    }

    async fn refresh_bootstrap_cache_from_hints(
        &self,
        cache: &mut BootstrapCache,
        nodes: &[&CommunityNodeConfigNode],
        requested_topic_id: Option<&str>,
    ) {
        for node in nodes {
            let since = cache
                .hint_cursors
                .get(&node.base_url)
                .map(|cursor| cursor.last_seq)
                .unwrap_or(0);
            let hint_response = match self.fetch_bootstrap_hint(node, since).await {
                Ok(response) => response,
                Err(_) => continue,
            };
            let Some(hint_response) = hint_response else {
                continue;
            };

            apply_bootstrap_hint_to_cache(cache, &hint_response.hint, requested_topic_id);
            cache.hint_cursors.insert(
                node.base_url.clone(),
                BootstrapHintCursorEntry {
                    last_seq: hint_response.seq,
                },
            );
        }
    }

    async fn fetch_bootstrap_hint(
        &self,
        node: &CommunityNodeConfigNode,
        since: u64,
    ) -> Result<Option<BootstrapHintHttpResponse>, AppError> {
        let path = format!("/v1/bootstrap/hints/latest?since={since}");
        let url = build_url(&node.base_url, &path);
        let builder = self
            .authorized_request(node, Method::GET, url, false)
            .await?;

        let response = builder
            .send()
            .await
            .map_err(|err| AppError::Network(err.to_string()))?;
        let status = response.status();
        if status == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let body = response
            .text()
            .await
            .map_err(|err| AppError::Network(err.to_string()))?;

        if !status.is_success() {
            return Err(AppError::Network(format!(
                "Community node error ({status}): {body}"
            )));
        }

        let parsed = serde_json::from_str::<BootstrapHintHttpResponse>(&body)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        Ok(Some(parsed))
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

fn normalize_optional_value(value: Option<String>) -> Option<String> {
    value
        .map(|val| val.trim().to_string())
        .filter(|val| !val.is_empty())
}

fn build_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

fn validate_kip_event_json(
    event_json: &serde_json::Value,
    expected_kind: u16,
    expected_pubkey: Option<&str>,
    now: i64,
) -> Option<NostrEvent> {
    let event: NostrEvent = serde_json::from_value(event_json.clone()).ok()?;
    if event.kind.as_u16() != expected_kind {
        return None;
    }
    if let Some(expected_pubkey) = expected_pubkey {
        let expected_pubkey = expected_pubkey.trim();
        if expected_pubkey.is_empty() {
            return None;
        }
        if event.pubkey.to_string() != expected_pubkey {
            return None;
        }
    }
    if event.verify().is_err() {
        return None;
    }
    if !validate_kip_tags(&event) {
        return None;
    }
    if !validate_kip_requirements(&event, expected_kind, now) {
        return None;
    }
    Some(event)
}

fn validate_attestation_payload(
    response: &serde_json::Value,
    expected_pubkey: Option<&str>,
    now: i64,
) -> bool {
    let Some(attestation) = response.get("attestation") else {
        return false;
    };
    if attestation.is_null() {
        return false;
    }
    let exp = match attestation.get("exp").and_then(|value| value.as_i64()) {
        Some(exp) => exp,
        None => return false,
    };
    if exp <= now {
        return false;
    }
    let Some(event_json) = attestation.get("event_json") else {
        return false;
    };
    validate_kip_event_json(event_json, ATTESTATION_KIND, expected_pubkey, now).is_some()
}

fn resolve_expected_pubkey<'a>(
    node_pubkey: Option<&'a str>,
    current_pubkey: Option<&str>,
) -> Option<&'a str> {
    let node_pubkey = node_pubkey
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if let Some(current_pubkey) = current_pubkey {
        if node_pubkey.eq_ignore_ascii_case(current_pubkey) {
            return None;
        }
    }
    Some(node_pubkey)
}

fn event_tag_value<'a>(event: &'a NostrEvent, name: &str) -> Option<&'a str> {
    event.tags.iter().find_map(|tag| {
        let values = tag.as_slice();
        if values.first().map(|value| value.as_str()) == Some(name) {
            values.get(1).map(|value| value.as_str())
        } else {
            None
        }
    })
}

fn event_tag(event: &NostrEvent, name: &str) -> Option<Vec<String>> {
    event.tags.iter().find_map(|tag| {
        let values = tag.as_slice();
        if values.first().map(|value| value.as_str()) == Some(name) {
            Some(values.to_vec())
        } else {
            None
        }
    })
}

fn has_tag(event: &NostrEvent, name: &str) -> bool {
    event
        .tags
        .iter()
        .any(|tag| tag.as_slice().first().map(|value| value.as_str()) == Some(name))
}

fn require_tag_value<'a>(event: &'a NostrEvent, name: &str) -> Option<&'a str> {
    let value = event_tag_value(event, name)?;
    if value.trim().is_empty() {
        return None;
    }
    Some(value)
}

fn require_exp_tag(event: &NostrEvent, now: i64) -> Option<i64> {
    let exp = require_tag_value(event, "exp")?
        .trim()
        .parse::<i64>()
        .ok()?;
    if exp <= now {
        return None;
    }
    Some(exp)
}

fn validate_schema(event: &NostrEvent, expected: &str) -> bool {
    let content = event.content.trim();
    let parsed: serde_json::Value = match serde_json::from_str(content) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let schema = match parsed.get("schema").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => return false,
    };
    schema == expected
}

fn validate_scope(scope: &str, allow_public: bool) -> bool {
    match scope {
        "friend_plus" | "friend" | "invite" => true,
        "public" => allow_public,
        _ => false,
    }
}

fn validate_kip_tags(event: &NostrEvent) -> bool {
    let namespace = match require_tag_value(event, "k") {
        Some(value) => value.trim(),
        None => return false,
    };
    if namespace != KIP_NAMESPACE {
        return false;
    }
    let version = match require_tag_value(event, "ver") {
        Some(value) => value.trim(),
        None => return false,
    };
    if version != KIP_VERSION {
        return false;
    }
    true
}

fn validate_kip_requirements(event: &NostrEvent, expected_kind: u16, now: i64) -> bool {
    match expected_kind {
        NODE_DESCRIPTOR_KIND => {
            if require_tag_value(event, "d").is_none() {
                return false;
            }
            if require_exp_tag(event, now).is_none() {
                return false;
            }
            if !validate_schema(event, KIP_NODE_DESCRIPTOR_SCHEMA) {
                return false;
            }
        }
        TOPIC_SERVICE_KIND => {
            if require_tag_value(event, "d").is_none() {
                return false;
            }
            if require_tag_value(event, "t").is_none() {
                return false;
            }
            if require_tag_value(event, "role").is_none() {
                return false;
            }
            let scope = match require_tag_value(event, "scope") {
                Some(value) => value.trim(),
                None => return false,
            };
            if !validate_scope(scope.trim(), true) {
                return false;
            }
            if require_exp_tag(event, now).is_none() {
                return false;
            }
            if !validate_schema(event, KIP_TOPIC_SERVICE_SCHEMA) {
                return false;
            }
        }
        LABEL_KIND => {
            if require_tag_value(event, "target").is_none() {
                return false;
            }
            if require_tag_value(event, "label").is_none() {
                return false;
            }
            if require_exp_tag(event, now).is_none() {
                return false;
            }
            if require_tag_value(event, "policy_url").is_none()
                && require_tag_value(event, "policy").is_none()
            {
                return false;
            }
            if has_tag(event, "policy_ref") && require_tag_value(event, "policy_ref").is_none() {
                return false;
            }
        }
        ATTESTATION_KIND => {
            let sub_tag = match event_tag(event, "sub") {
                Some(tag) => tag,
                None => return false,
            };
            if sub_tag.len() < 3 {
                return false;
            }
            if sub_tag
                .get(1)
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
                || sub_tag
                    .get(2)
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
            {
                return false;
            }
            if require_tag_value(event, "claim").is_none() {
                return false;
            }
            if require_exp_tag(event, now).is_none() {
                return false;
            }
            if !validate_schema(event, KIP_ATTESTATION_SCHEMA) {
                return false;
            }
        }
        TRUST_ANCHOR_KIND => {
            if require_tag_value(event, "attester").is_none() {
                return false;
            }
            if require_tag_value(event, "weight").is_none() {
                return false;
            }
        }
        _ => return false,
    }
    true
}

fn should_refresh_bootstrap(entry: &BootstrapCacheEntry, now: i64) -> bool {
    if entry.stale {
        return true;
    }
    if let Some(next_refresh_at) = entry.next_refresh_at {
        if next_refresh_at <= now {
            return true;
        }
    }
    false
}

fn apply_bootstrap_hint_to_cache(
    cache: &mut BootstrapCache,
    hint: &BootstrapHintPayload,
    requested_topic_id: Option<&str>,
) {
    if hint.schema.as_deref() != Some(KIP_BOOTSTRAP_HINT_SCHEMA) {
        return;
    }

    let refresh_nodes = hint.descriptor_changed
        || hint
            .refresh_paths
            .iter()
            .any(|path| path == "/v1/bootstrap/nodes");
    if refresh_nodes {
        cache.nodes.stale = true;
    }

    let refresh_services = hint
        .refresh_paths
        .iter()
        .any(|path| path == "/v1/bootstrap/topics/{topic_id}/services");
    if !refresh_services {
        return;
    }

    if hint.changed_topic_ids.is_empty() {
        for entry in cache.services.values_mut() {
            entry.stale = true;
        }
        if let Some(topic_id) = requested_topic_id {
            cache
                .services
                .entry(topic_id.to_string())
                .or_default()
                .stale = true;
        }
        return;
    }

    for topic_id in &hint.changed_topic_ids {
        cache.services.entry(topic_id.clone()).or_default().stale = true;
    }
}

fn sanitize_bootstrap_items(
    expected_kind: u16,
    items: &[serde_json::Value],
    now: i64,
    topic_filter: Option<&str>,
) -> Vec<serde_json::Value> {
    let mut map: HashMap<String, (NostrEvent, serde_json::Value)> = HashMap::new();

    for item in items {
        let Some(event) = validate_kip_event_json(item, expected_kind, None, now) else {
            continue;
        };
        if let Some(topic) = topic_filter {
            if event_tag_value(&event, "t") != Some(topic) {
                continue;
            }
        }
        let Some(key) = addressable_key(&event) else {
            continue;
        };
        let replace = match map.get(&key) {
            Some((existing, _)) => is_newer_addressable(&event, existing),
            None => true,
        };
        if replace {
            map.insert(key, (event, item.clone()));
        }
    }

    let mut entries: Vec<(NostrEvent, serde_json::Value)> = map.into_values().collect();
    entries.sort_by(|(left, _), (right, _)| {
        let left_ts = left.created_at.as_secs();
        let right_ts = right.created_at.as_secs();
        match right_ts.cmp(&left_ts) {
            Ordering::Equal => left.id.to_string().cmp(&right.id.to_string()),
            other => other,
        }
    });

    entries.into_iter().map(|(_, value)| value).collect()
}

fn merge_bootstrap_entry(
    entry: &mut BootstrapCacheEntry,
    expected_kind: u16,
    event: &NostrEvent,
    event_json: serde_json::Value,
    exp: Option<i64>,
    now: i64,
    topic_filter: Option<&str>,
) {
    let mut items = sanitize_bootstrap_items(expected_kind, &entry.items, now, topic_filter);
    insert_or_replace_addressable(&mut items, event, event_json);
    entry.items = items;
    entry.updated_at = Some(now);
    entry.stale = true;
    if let Some(exp) = exp {
        entry.next_refresh_at = Some(match entry.next_refresh_at {
            Some(current) => current.min(exp),
            None => exp,
        });
    }
}

fn insert_or_replace_addressable(
    items: &mut Vec<serde_json::Value>,
    event: &NostrEvent,
    event_json: serde_json::Value,
) {
    let Some(key) = addressable_key(event) else {
        return;
    };

    for item in items.iter_mut() {
        let Ok(existing) = serde_json::from_value::<NostrEvent>(item.clone()) else {
            continue;
        };
        if addressable_key(&existing).as_deref() != Some(&key) {
            continue;
        }
        if is_newer_addressable(event, &existing) {
            *item = event_json;
        }
        return;
    }

    items.push(event_json);
}

fn addressable_key(event: &NostrEvent) -> Option<String> {
    let d_tag = event_tag_value(event, "d")?;
    Some(format!(
        "{}:{}:{}",
        event.kind.as_u16(),
        event.pubkey,
        d_tag
    ))
}

fn is_newer_addressable(candidate: &NostrEvent, existing: &NostrEvent) -> bool {
    let candidate_ts = candidate.created_at.as_secs();
    let existing_ts = existing.created_at.as_secs();
    match candidate_ts.cmp(&existing_ts) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => candidate.id.to_string() < existing.id.to_string(),
    }
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

#[cfg(test)]
mod community_node_validation_tests {
    use super::*;
    use nostr_sdk::prelude::{EventBuilder, Keys, Kind, Tag};

    fn build_event_json(
        keys: &Keys,
        kind: u16,
        tags: Vec<Tag>,
        content: &str,
    ) -> serde_json::Value {
        let event = EventBuilder::new(Kind::Custom(kind), content)
            .tags(tags)
            .sign_with_keys(keys)
            .expect("signed");
        serde_json::to_value(event).expect("event json")
    }

    fn build_label_tags(exp: i64) -> Vec<Tag> {
        let exp_str = exp.to_string();
        vec![
            Tag::parse(["k", KIP_NAMESPACE]).expect("k"),
            Tag::parse(["ver", KIP_VERSION]).expect("ver"),
            Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
            Tag::parse(["target", "event:deadbeef"]).expect("target"),
            Tag::parse(["label", "spam"]).expect("label"),
            Tag::parse(["policy_url", "https://example.com/policy"]).expect("policy_url"),
        ]
    }

    fn build_label_event_json(keys: &Keys, exp: i64) -> serde_json::Value {
        build_event_json(keys, LABEL_KIND, build_label_tags(exp), "")
    }

    fn build_node_descriptor_event_json(keys: &Keys, exp: i64, schema: &str) -> serde_json::Value {
        let exp_str = exp.to_string();
        let tags = vec![
            Tag::parse(["k", KIP_NAMESPACE]).expect("k"),
            Tag::parse(["ver", KIP_VERSION]).expect("ver"),
            Tag::parse(["d", "descriptor"]).expect("d"),
            Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
        ];
        let content = json!({ "schema": schema }).to_string();
        build_event_json(keys, NODE_DESCRIPTOR_KIND, tags, content.as_str())
    }

    #[test]
    fn validate_kip_event_json_accepts_valid_label() {
        let keys = Keys::generate();
        let pubkey = keys.public_key().to_string();
        let now = 1000;
        let event_json = build_label_event_json(&keys, now + 60);
        let validated =
            validate_kip_event_json(&event_json, LABEL_KIND, Some(pubkey.as_str()), now);
        assert!(validated.is_some());
    }

    #[test]
    fn validate_kip_event_json_accepts_without_expected_pubkey() {
        let keys = Keys::generate();
        let now = 1000;
        let event_json = build_label_event_json(&keys, now + 60);
        let validated = validate_kip_event_json(&event_json, LABEL_KIND, None, now);
        assert!(validated.is_some());
    }

    #[test]
    fn validate_kip_event_json_rejects_expired_event() {
        let keys = Keys::generate();
        let pubkey = keys.public_key().to_string();
        let now = 1000;
        let event_json = build_label_event_json(&keys, now - 1);
        let validated =
            validate_kip_event_json(&event_json, LABEL_KIND, Some(pubkey.as_str()), now);
        assert!(validated.is_none());
    }

    #[test]
    fn validate_kip_event_json_rejects_wrong_pubkey() {
        let keys = Keys::generate();
        let other_keys = Keys::generate();
        let other_pubkey = other_keys.public_key().to_string();
        let now = 1000;
        let event_json = build_label_event_json(&keys, now + 60);
        let validated =
            validate_kip_event_json(&event_json, LABEL_KIND, Some(other_pubkey.as_str()), now);
        assert!(validated.is_none());
    }

    #[test]
    fn validate_kip_event_json_rejects_missing_k_tag() {
        let keys = Keys::generate();
        let now = 1000;
        let mut tags = build_label_tags(now + 60);
        tags.retain(|tag| tag.as_slice().first().map(|value| value.as_str()) != Some("k"));
        let event_json = build_event_json(&keys, LABEL_KIND, tags, "");
        let validated = validate_kip_event_json(&event_json, LABEL_KIND, None, now);
        assert!(validated.is_none());
    }

    #[test]
    fn validate_kip_event_json_rejects_missing_ver_tag() {
        let keys = Keys::generate();
        let now = 1000;
        let mut tags = build_label_tags(now + 60);
        tags.retain(|tag| tag.as_slice().first().map(|value| value.as_str()) != Some("ver"));
        let event_json = build_event_json(&keys, LABEL_KIND, tags, "");
        let validated = validate_kip_event_json(&event_json, LABEL_KIND, None, now);
        assert!(validated.is_none());
    }

    #[test]
    fn validate_kip_event_json_rejects_missing_policy_tag() {
        let keys = Keys::generate();
        let now = 1000;
        let mut tags = build_label_tags(now + 60);
        tags.retain(|tag| tag.as_slice().first().map(|value| value.as_str()) != Some("policy_url"));
        let event_json = build_event_json(&keys, LABEL_KIND, tags, "");
        let validated = validate_kip_event_json(&event_json, LABEL_KIND, None, now);
        assert!(validated.is_none());
    }

    #[test]
    fn validate_kip_event_json_rejects_invalid_schema() {
        let keys = Keys::generate();
        let now = 1000;
        let event_json = build_node_descriptor_event_json(&keys, now + 60, "invalid-schema");
        let validated = validate_kip_event_json(&event_json, NODE_DESCRIPTOR_KIND, None, now);
        assert!(validated.is_none());
    }
}

#[cfg(test)]
mod community_node_handler_tests {
    use super::*;
    use crate::application::ports::group_key_store::GroupKeyStore;
    use crate::infrastructure::crypto::DefaultKeyManager;
    use crate::infrastructure::storage::{SecureGroupKeyStore, SecureStorage};
    use crate::presentation::dto::community_node_dto::CommunityNodeConfigNodeRequest;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use tiny_http::{Header, Response, Server};
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct InMemorySecureStorage {
        entries: Mutex<HashMap<String, String>>,
    }

    #[async_trait]
    impl SecureStorage for InMemorySecureStorage {
        async fn store(
            &self,
            key: &str,
            value: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.entries
                .lock()
                .await
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn retrieve(
            &self,
            key: &str,
        ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.entries.lock().await.get(key).cloned())
        }

        async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.entries.lock().await.remove(key);
            Ok(())
        }

        async fn exists(
            &self,
            key: &str,
        ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.entries.lock().await.contains_key(key))
        }

        async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.entries.lock().await.keys().cloned().collect())
        }

        async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.entries.lock().await.clear();
            Ok(())
        }
    }

    fn test_handler() -> CommunityNodeHandler {
        let key_manager = Arc::new(DefaultKeyManager::new());
        let secure_storage = Arc::new(InMemorySecureStorage::default());
        let group_key_store =
            Arc::new(SecureGroupKeyStore::new(secure_storage.clone())) as Arc<dyn GroupKeyStore>;
        CommunityNodeHandler::new(key_manager, secure_storage, group_key_store)
    }

    fn to_domain_event(event: &NostrEvent) -> crate::domain::entities::Event {
        let created_at =
            chrono::DateTime::<chrono::Utc>::from_timestamp(event.created_at.as_secs() as i64, 0)
                .expect("timestamp");
        crate::domain::entities::Event {
            id: event.id.to_string(),
            pubkey: event.pubkey.to_string(),
            created_at,
            kind: event.kind.as_u16() as u32,
            tags: event
                .tags
                .iter()
                .map(|tag| tag.as_slice().to_vec())
                .collect(),
            content: event.content.clone(),
            sig: event.sig.to_string(),
        }
    }

    #[derive(Debug)]
    struct CapturedRequest {
        path: String,
        params: HashMap<String, String>,
    }

    #[derive(Debug)]
    struct MockHttpResponse {
        status: u16,
        body: Option<serde_json::Value>,
    }

    impl MockHttpResponse {
        fn json(status: u16, body: serde_json::Value) -> Self {
            Self {
                status,
                body: Some(body),
            }
        }

        fn empty(status: u16) -> Self {
            Self { status, body: None }
        }
    }

    fn spawn_json_server(
        response_body: serde_json::Value,
    ) -> (
        String,
        mpsc::Receiver<CapturedRequest>,
        thread::JoinHandle<()>,
    ) {
        let server = Server::http("127.0.0.1:0").expect("server");
        let base_url = format!("http://{}", server.server_addr());
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            for request in server.incoming_requests().take(1) {
                let url = request.url();
                let parsed = Url::parse(&format!("http://localhost{url}")).expect("request url");
                let params = parsed
                    .query_pairs()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect();
                let captured = CapturedRequest {
                    path: parsed.path().to_string(),
                    params,
                };
                let _ = tx.send(captured);

                let mut response = Response::from_string(response_body.to_string());
                response.add_header(
                    Header::from_bytes("Content-Type", "application/json").expect("header"),
                );
                let _ = request.respond(response);
            }
        });
        (base_url, rx, handle)
    }

    fn spawn_json_sequence_server(
        responses: Vec<MockHttpResponse>,
    ) -> (
        String,
        mpsc::Receiver<CapturedRequest>,
        thread::JoinHandle<()>,
    ) {
        let expected_requests = responses.len();
        let server = Server::http("127.0.0.1:0").expect("server");
        let base_url = format!("http://{}", server.server_addr());
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            for (request, response_spec) in server
                .incoming_requests()
                .take(expected_requests)
                .zip(responses.into_iter())
            {
                let url = request.url();
                let parsed = Url::parse(&format!("http://localhost{url}")).expect("request url");
                let params = parsed
                    .query_pairs()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect();
                let captured = CapturedRequest {
                    path: parsed.path().to_string(),
                    params,
                };
                let _ = tx.send(captured);

                let mut response = match response_spec.body {
                    Some(body) => {
                        let mut response = Response::from_string(body.to_string());
                        response.add_header(
                            Header::from_bytes("Content-Type", "application/json")
                                .expect("json content-type header"),
                        );
                        response
                    }
                    None => Response::from_string(String::new()),
                };
                response = response.with_status_code(response_spec.status);
                let _ = request.respond(response);
            }
        });
        (base_url, rx, handle)
    }

    fn build_config_node(
        base_url: String,
        roles: CommunityNodeRoleConfig,
    ) -> CommunityNodeConfigNode {
        let mut node = CommunityNodeConfigNode::new(base_url, roles);
        node.access_token = Some("test-token".to_string());
        node.token_expires_at = Some(Utc::now().timestamp() + 600);
        node
    }

    fn build_attestation_event(exp: i64) -> serde_json::Value {
        let keys = Keys::generate();
        let exp_str = exp.to_string();
        let subject = keys.public_key().to_string();
        let tags = vec![
            Tag::parse(["k", KIP_NAMESPACE]).expect("k"),
            Tag::parse(["ver", KIP_VERSION]).expect("ver"),
            Tag::parse(["sub", "pubkey", subject.as_str()]).expect("sub"),
            Tag::parse(["claim", "reputation"]).expect("claim"),
            Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
        ];
        let content = json!({
            "schema": KIP_ATTESTATION_SCHEMA,
            "subject": format!("pubkey:{subject}"),
            "claim": "reputation",
            "value": { "score": 0.5 },
            "expires": exp
        })
        .to_string();
        let event = EventBuilder::new(Kind::Custom(ATTESTATION_KIND), content)
            .tags(tags)
            .sign_with_keys(&keys)
            .expect("sign");
        serde_json::to_value(event).expect("event json")
    }

    fn build_bootstrap_topic_service_event(topic_id: &str, marker: &str) -> serde_json::Value {
        let keys = Keys::generate();
        let exp = Utc::now().timestamp() + 600;
        let d_tag = format!("topic_service:{topic_id}:bootstrap:public");
        let exp_str = exp.to_string();
        let tags = vec![
            Tag::parse(["d", d_tag.as_str()]).expect("d"),
            Tag::parse(["t", topic_id]).expect("t"),
            Tag::parse(["role", "bootstrap"]).expect("role"),
            Tag::parse(["scope", "public"]).expect("scope"),
            Tag::parse(["k", "kukuri"]).expect("k"),
            Tag::parse(["ver", "1"]).expect("ver"),
            Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
        ];
        let content = json!({
            "schema": "kukuri-topic-service-v1",
            "topic": topic_id,
            "role": "bootstrap",
            "scope": "public",
            "marker": marker,
        })
        .to_string();
        let event = EventBuilder::new(Kind::Custom(TOPIC_SERVICE_KIND), content)
            .tags(tags)
            .sign_with_keys(&keys)
            .expect("sign");
        serde_json::to_value(event).expect("bootstrap topic service event json")
    }

    #[tokio::test]
    async fn set_config_normalizes_and_deduplicates_nodes() {
        let handler = test_handler();
        let request = CommunityNodeConfigRequest {
            nodes: vec![
                CommunityNodeConfigNodeRequest {
                    base_url: "https://example.com/".to_string(),
                    roles: Some(CommunityNodeRoleConfig {
                        labels: true,
                        trust: false,
                        search: true,
                        bootstrap: false,
                    }),
                },
                CommunityNodeConfigNodeRequest {
                    base_url: "https://example.com".to_string(),
                    roles: None,
                },
                CommunityNodeConfigNodeRequest {
                    base_url: "https://node2.example.com".to_string(),
                    roles: None,
                },
            ],
        };

        let response = handler.set_config(request).await.expect("set config");
        assert_eq!(response.nodes.len(), 2);
        assert_eq!(response.nodes[0].base_url, "https://example.com");
        assert_eq!(response.nodes[1].base_url, "https://node2.example.com");
        assert!(response.nodes[0].roles.search);
        assert!(!response.nodes[0].roles.bootstrap);

        let loaded = handler.get_config().await.expect("get config");
        assert!(loaded.is_some());
    }

    #[tokio::test]
    async fn trust_anchor_roundtrip() {
        let key_manager = Arc::new(DefaultKeyManager::new());
        let keypair = key_manager.generate_keypair().await.expect("keypair");
        let secure_storage = Arc::new(InMemorySecureStorage::default());
        let group_key_store =
            Arc::new(SecureGroupKeyStore::new(secure_storage.clone())) as Arc<dyn GroupKeyStore>;
        let handler = CommunityNodeHandler::new(key_manager, secure_storage, group_key_store);

        let request = CommunityNodeTrustAnchorRequest {
            attester: keypair.public_key.clone(),
            claim: Some("trust:v1".to_string()),
            topic: Some("kukuri:topic1".to_string()),
            weight: Some(0.6),
        };
        let stored = handler.set_trust_anchor(request).await.expect("set");
        let loaded = handler
            .get_trust_anchor()
            .await
            .expect("get")
            .expect("stored");
        assert_eq!(loaded.attester, stored.attester);
        assert_eq!(loaded.weight, 0.6);
        assert_eq!(loaded.claim, Some("trust:v1".to_string()));
        assert_eq!(loaded.topic, Some("kukuri:topic1".to_string()));
    }

    #[tokio::test]
    async fn trust_anchor_rejects_invalid_weight() {
        let key_manager = Arc::new(DefaultKeyManager::new());
        let keypair = key_manager.generate_keypair().await.expect("keypair");
        let secure_storage = Arc::new(InMemorySecureStorage::default());
        let group_key_store =
            Arc::new(SecureGroupKeyStore::new(secure_storage.clone())) as Arc<dyn GroupKeyStore>;
        let handler = CommunityNodeHandler::new(key_manager, secure_storage, group_key_store);

        let request = CommunityNodeTrustAnchorRequest {
            attester: keypair.public_key.clone(),
            claim: None,
            topic: None,
            weight: Some(1.5),
        };
        assert!(handler.set_trust_anchor(request).await.is_err());
    }

    #[tokio::test]
    async fn ingest_bootstrap_descriptor_populates_cache() {
        let handler = test_handler();
        let keys = Keys::generate();
        let now = Utc::now().timestamp();
        let exp = now + 600;

        let exp_str = exp.to_string();
        let tags = vec![
            Tag::parse(["d", "descriptor"]).expect("d"),
            Tag::parse(["k", "kukuri"]).expect("k"),
            Tag::parse(["ver", "1"]).expect("ver"),
            Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
            Tag::parse(["role", "bootstrap"]).expect("role"),
        ];
        let content = json!({
            "schema": "kukuri-node-desc-v1",
            "name": "Test Node",
            "roles": ["bootstrap"],
            "endpoints": { "http": "https://node.example" }
        })
        .to_string();
        let event = EventBuilder::new(Kind::Custom(NODE_DESCRIPTOR_KIND), content)
            .tags(tags)
            .sign_with_keys(&keys)
            .expect("sign");
        let domain_event = to_domain_event(&event);

        handler
            .ingest_bootstrap_event(&domain_event)
            .await
            .expect("ingest");

        let response = handler.list_bootstrap_nodes().await.expect("list");
        let items = response
            .get("items")
            .and_then(|value| value.as_array())
            .expect("items array");
        assert!(!items.is_empty());
    }

    #[tokio::test]
    async fn ingest_bootstrap_topic_service_populates_cache() {
        let handler = test_handler();
        let keys = Keys::generate();
        let now = Utc::now().timestamp();
        let exp = now + 600;
        let topic_id = "kukuri:topic1";
        let d_tag = format!("topic_service:{topic_id}:bootstrap:public");

        let exp_str = exp.to_string();
        let tags = vec![
            Tag::parse(["d", d_tag.as_str()]).expect("d"),
            Tag::parse(["t", topic_id]).expect("t"),
            Tag::parse(["role", "bootstrap"]).expect("role"),
            Tag::parse(["scope", "public"]).expect("scope"),
            Tag::parse(["k", "kukuri"]).expect("k"),
            Tag::parse(["ver", "1"]).expect("ver"),
            Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
        ];
        let content = json!({
            "schema": "kukuri-topic-service-v1",
            "topic": topic_id,
            "role": "bootstrap",
            "scope": "public"
        })
        .to_string();
        let event = EventBuilder::new(Kind::Custom(TOPIC_SERVICE_KIND), content)
            .tags(tags)
            .sign_with_keys(&keys)
            .expect("sign");
        let domain_event = to_domain_event(&event);

        handler
            .ingest_bootstrap_event(&domain_event)
            .await
            .expect("ingest");

        let response = handler
            .list_bootstrap_services(CommunityNodeBootstrapServicesRequest {
                base_url: None,
                topic_id: topic_id.to_string(),
            })
            .await
            .expect("list services");
        let items = response
            .get("items")
            .and_then(|value| value.as_array())
            .expect("items array");
        assert!(!items.is_empty());
    }

    #[tokio::test]
    async fn refresh_bootstrap_from_hint_refetches_nodes_and_topic_services() {
        let handler = test_handler();
        let topic_id = format!(
            "kukuri:hint-bridge-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let now = Utc::now().timestamp();

        let keys = Keys::generate();
        let exp_str = (now + 3600).to_string();
        let descriptor = EventBuilder::new(
            Kind::Custom(NODE_DESCRIPTOR_KIND),
            json!({
                "schema": "kukuri-node-desc-v1",
                "name": "Hint Bridge Node",
                "roles": ["bootstrap"],
                "endpoints": {"http": "https://node.example"}
            })
            .to_string(),
        )
        .tags(vec![
            Tag::parse(["d", "descriptor"]).expect("d"),
            Tag::parse(["k", "kukuri"]).expect("k"),
            Tag::parse(["ver", "1"]).expect("ver"),
            Tag::parse(["exp", exp_str.as_str()]).expect("exp"),
            Tag::parse(["role", "bootstrap"]).expect("role"),
        ])
        .sign_with_keys(&keys)
        .expect("sign descriptor");

        let topic_service = build_bootstrap_topic_service_event(&topic_id, "hint-bridge");
        let service_path = format!("/v1/bootstrap/topics/{topic_id}/services");
        let responses = vec![
            MockHttpResponse::json(
                200,
                json!({
                    "items": [serde_json::to_value(&descriptor).expect("descriptor json")],
                    "next_refresh_at": now + 3600,
                }),
            ),
            MockHttpResponse::json(
                200,
                json!({
                    "items": [topic_service],
                    "next_refresh_at": now + 3600,
                }),
            ),
        ];
        let (base_url, rx, handle) = spawn_json_sequence_server(responses);
        let config = CommunityNodeConfig {
            nodes: vec![build_config_node(
                base_url.clone(),
                CommunityNodeRoleConfig {
                    labels: false,
                    trust: false,
                    search: false,
                    bootstrap: true,
                },
            )],
        };
        handler.save_config(&config).await.expect("save config");

        handler
            .refresh_bootstrap_from_hint(Some(&topic_id))
            .await
            .expect("refresh from hint");

        let nodes = handler.list_bootstrap_nodes().await.expect("list nodes");
        let node_items = nodes
            .get("items")
            .and_then(|value| value.as_array())
            .expect("node items");
        assert_eq!(node_items.len(), 1);

        let services = handler
            .list_bootstrap_services(CommunityNodeBootstrapServicesRequest {
                base_url: None,
                topic_id: topic_id.clone(),
            })
            .await
            .expect("list services");
        let service_items = services
            .get("items")
            .and_then(|value| value.as_array())
            .expect("service items");
        assert_eq!(service_items.len(), 1);

        let req1 = rx.recv_timeout(Duration::from_secs(2)).expect("request 1");
        assert_eq!(req1.path, "/v1/bootstrap/nodes");

        let req2 = rx.recv_timeout(Duration::from_secs(2)).expect("request 2");
        assert_eq!(req2.path, service_path);

        handle.join().expect("server");
    }

    #[tokio::test]
    async fn list_bootstrap_services_refetches_when_hint_endpoint_reports_update() {
        let handler = test_handler();
        let topic_id = format!(
            "kukuri:hint-refresh-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let service_path = format!("/v1/bootstrap/topics/{topic_id}/services");
        let initial_event = build_bootstrap_topic_service_event(&topic_id, "initial");
        let refreshed_event = build_bootstrap_topic_service_event(&topic_id, "refreshed");
        let now = Utc::now().timestamp();
        let responses = vec![
            MockHttpResponse::empty(204),
            MockHttpResponse::json(
                200,
                json!({
                    "items": [initial_event],
                    "next_refresh_at": now + 3600,
                }),
            ),
            MockHttpResponse::json(
                200,
                json!({
                    "seq": 1,
                    "received_at": now,
                    "hint": {
                        "schema": KIP_BOOTSTRAP_HINT_SCHEMA,
                        "descriptor_changed": false,
                        "changed_topic_ids": [topic_id.clone()],
                        "refresh_paths": [
                            "/v1/bootstrap/nodes",
                            "/v1/bootstrap/topics/{topic_id}/services"
                        ]
                    }
                }),
            ),
            MockHttpResponse::json(
                200,
                json!({
                    "items": [refreshed_event],
                    "next_refresh_at": now + 3600,
                }),
            ),
        ];
        let (base_url, rx, handle) = spawn_json_sequence_server(responses);
        let config = CommunityNodeConfig {
            nodes: vec![build_config_node(
                base_url.clone(),
                CommunityNodeRoleConfig {
                    labels: false,
                    trust: false,
                    search: false,
                    bootstrap: true,
                },
            )],
        };
        handler.save_config(&config).await.expect("save config");

        let first = handler
            .list_bootstrap_services(CommunityNodeBootstrapServicesRequest {
                base_url: None,
                topic_id: topic_id.clone(),
            })
            .await
            .expect("first bootstrap services");
        let first_id = first
            .get("items")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|item| item.get("id"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .expect("first event id");

        let second = handler
            .list_bootstrap_services(CommunityNodeBootstrapServicesRequest {
                base_url: None,
                topic_id: topic_id.clone(),
            })
            .await
            .expect("second bootstrap services");
        let second_id = second
            .get("items")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|item| item.get("id"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .expect("second event id");

        assert_ne!(first_id, second_id);

        let req1 = rx.recv_timeout(Duration::from_secs(2)).expect("request 1");
        assert_eq!(req1.path, "/v1/bootstrap/hints/latest");
        assert_eq!(req1.params.get("since"), Some(&"0".to_string()));

        let req2 = rx.recv_timeout(Duration::from_secs(2)).expect("request 2");
        assert_eq!(req2.path, service_path);

        let req3 = rx.recv_timeout(Duration::from_secs(2)).expect("request 3");
        assert_eq!(req3.path, "/v1/bootstrap/hints/latest");
        assert_eq!(req3.params.get("since"), Some(&"0".to_string()));

        let req4 = rx.recv_timeout(Duration::from_secs(2)).expect("request 4");
        assert_eq!(req4.path, service_path);

        handle.join().expect("server");
    }

    #[tokio::test]
    async fn trust_report_based_aggregates_scores_across_nodes() {
        let exp = Utc::now().timestamp() + 600;
        let event_json = build_attestation_event(exp);
        let response1 = json!({
            "score": 0.2,
            "attestation": { "exp": exp, "event_json": event_json.clone() }
        });
        let response2 = json!({
            "score": 0.6,
            "attestation": { "exp": exp, "event_json": event_json }
        });

        let (base_url1, rx1, handle1) = spawn_json_server(response1);
        let (base_url2, rx2, handle2) = spawn_json_server(response2);

        let handler = test_handler();
        let roles = CommunityNodeRoleConfig {
            labels: false,
            trust: true,
            search: false,
            bootstrap: false,
        };
        let config = CommunityNodeConfig {
            nodes: vec![
                build_config_node(base_url1.clone(), roles.clone()),
                build_config_node(base_url2.clone(), roles),
            ],
        };
        handler.save_config(&config).await.expect("save config");

        let response = handler
            .trust_report_based(CommunityNodeTrustRequest {
                base_url: None,
                subject: "npub1testsubject".to_string(),
            })
            .await
            .expect("trust response");

        let score = response
            .get("score")
            .and_then(|value| value.as_f64())
            .expect("score");
        assert!((score - 0.4).abs() < 1e-9);

        let sources = response
            .get("sources")
            .and_then(|value| value.as_array())
            .expect("sources");
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|value| {
            value.get("base_url") == Some(&serde_json::Value::String(base_url1.clone()))
                && value.get("score") == Some(&serde_json::Value::from(0.2))
        }));
        assert!(sources.iter().any(|value| {
            value.get("base_url") == Some(&serde_json::Value::String(base_url2.clone()))
                && value.get("score") == Some(&serde_json::Value::from(0.6))
        }));

        let req1 = rx1.recv_timeout(Duration::from_secs(2)).expect("request 1");
        assert_eq!(req1.path, "/v1/trust/report-based");
        assert_eq!(
            req1.params.get("subject"),
            Some(&"npub1testsubject".to_string())
        );

        let req2 = rx2.recv_timeout(Duration::from_secs(2)).expect("request 2");
        assert_eq!(req2.path, "/v1/trust/report-based");
        assert_eq!(
            req2.params.get("subject"),
            Some(&"npub1testsubject".to_string())
        );

        handle1.join().expect("server1");
        handle2.join().expect("server2");
    }

    #[tokio::test]
    async fn search_aggregates_items_and_composes_cursor_for_multiple_nodes() {
        let response1 = json!({
            "items": [ { "id": "a" } ],
            "next_cursor": "next-1",
            "total": 2
        });
        let response2 = json!({
            "items": [ { "id": "b" }, { "id": "c" } ],
            "next_cursor": "next-2",
            "total": 3
        });

        let (base_url1, rx1, handle1) = spawn_json_server(response1);
        let (base_url2, rx2, handle2) = spawn_json_server(response2);

        let handler = test_handler();
        let roles = CommunityNodeRoleConfig {
            labels: false,
            trust: false,
            search: true,
            bootstrap: false,
        };
        let config = CommunityNodeConfig {
            nodes: vec![
                build_config_node(base_url1.clone(), roles.clone()),
                build_config_node(base_url2.clone(), roles),
            ],
        };
        handler.save_config(&config).await.expect("save config");

        let mut cursor_map = HashMap::new();
        cursor_map.insert(base_url1.clone(), "cursor-1".to_string());
        cursor_map.insert(base_url2.clone(), "cursor-2".to_string());
        let cursor = serde_json::to_string(&cursor_map).expect("cursor map");
        let response = handler
            .search(CommunityNodeSearchRequest {
                base_url: None,
                topic: "kukuri:topic1".to_string(),
                q: Some("rust".to_string()),
                limit: Some(5),
                cursor: Some(cursor),
            })
            .await
            .expect("search");

        let items = response
            .get("items")
            .and_then(|value| value.as_array())
            .expect("items");
        assert_eq!(items.len(), 3);

        let total = response
            .get("total")
            .and_then(|value| value.as_i64())
            .expect("total");
        assert_eq!(total, 5);

        assert_eq!(
            response.get("topic"),
            Some(&serde_json::Value::String("kukuri:topic1".to_string()))
        );
        assert_eq!(
            response.get("query"),
            Some(&serde_json::Value::String("rust".to_string()))
        );

        let next_cursor = response
            .get("next_cursor")
            .and_then(|value| value.as_str())
            .expect("next_cursor");
        let cursor_map: HashMap<String, String> =
            serde_json::from_str(next_cursor).expect("cursor map");
        assert_eq!(cursor_map.get(&base_url1), Some(&"next-1".to_string()));
        assert_eq!(cursor_map.get(&base_url2), Some(&"next-2".to_string()));

        let req1 = rx1.recv_timeout(Duration::from_secs(2)).expect("request 1");
        assert_eq!(req1.path, "/v1/search");
        assert_eq!(req1.params.get("topic"), Some(&"kukuri:topic1".to_string()));
        assert_eq!(req1.params.get("q"), Some(&"rust".to_string()));
        assert_eq!(req1.params.get("limit"), Some(&"5".to_string()));
        assert_eq!(req1.params.get("cursor"), Some(&"cursor-1".to_string()));

        let req2 = rx2.recv_timeout(Duration::from_secs(2)).expect("request 2");
        assert_eq!(req2.path, "/v1/search");
        assert_eq!(req2.params.get("topic"), Some(&"kukuri:topic1".to_string()));
        assert_eq!(req2.params.get("q"), Some(&"rust".to_string()));
        assert_eq!(req2.params.get("limit"), Some(&"5".to_string()));
        assert_eq!(req2.params.get("cursor"), Some(&"cursor-2".to_string()));

        handle1.join().expect("server1");
        handle2.join().expect("server2");
    }
}
