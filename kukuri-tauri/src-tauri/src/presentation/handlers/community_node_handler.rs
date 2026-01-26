use crate::application::ports::group_key_store::{GroupKeyEntry, GroupKeyRecord, GroupKeyStore};
use crate::application::ports::key_manager::KeyManager;
use crate::infrastructure::storage::SecureStorage;
use crate::presentation::dto::community_node_dto::{
    CommunityNodeAuthResponse, CommunityNodeBootstrapServicesRequest, CommunityNodeConfigRequest,
    CommunityNodeConfigResponse, CommunityNodeConsentRequest, CommunityNodeKeyEnvelopeRequest,
    CommunityNodeKeyEnvelopeResponse, CommunityNodeLabelsRequest, CommunityNodeRedeemInviteRequest,
    CommunityNodeRedeemInviteResponse, CommunityNodeReportRequest, CommunityNodeSearchRequest,
    CommunityNodeTrustRequest,
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
use std::sync::Arc;

const COMMUNITY_NODE_CONFIG_KEY: &str = "community_node_config_v1";
const AUTH_KIND: u16 = 22242;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CommunityNodeConfig {
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
        let base_url = normalize_base_url(&request.base_url)?;
        let mut config = self.load_config().await?.unwrap_or_default();
        if config.base_url != base_url {
            config.access_token = None;
            config.token_expires_at = None;
            config.pubkey = None;
        }
        config.base_url = base_url;
        self.save_config(&config).await?;
        Ok(config_response(&config))
    }

    pub async fn get_config(&self) -> Result<Option<CommunityNodeConfigResponse>, AppError> {
        let Some(config) = self.load_config().await? else {
            return Ok(None);
        };
        if config.base_url.is_empty() {
            return Ok(None);
        }
        Ok(Some(config_response(&config)))
    }

    pub async fn clear_config(&self) -> Result<(), AppError> {
        self.secure_storage
            .delete(COMMUNITY_NODE_CONFIG_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        Ok(())
    }

    pub async fn clear_token(&self) -> Result<(), AppError> {
        let mut config = self.require_config().await?;
        config.access_token = None;
        config.token_expires_at = None;
        config.pubkey = None;
        self.save_config(&config).await
    }

    pub async fn authenticate(&self) -> Result<CommunityNodeAuthResponse, AppError> {
        let mut config = self.require_config().await?;
        let keypair = self.key_manager.current_keypair().await?;
        let challenge = self
            .request_auth_challenge(&config.base_url, &keypair.public_key)
            .await?;
        let auth_event = build_auth_event(&config.base_url, &challenge.challenge, &keypair.nsec)?;
        let verified = self.verify_auth(&config.base_url, &auth_event).await?;
        config.access_token = Some(verified.access_token.clone());
        config.token_expires_at = Some(verified.expires_at);
        config.pubkey = Some(verified.pubkey.clone());
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
        let url = build_url(&config.base_url, "/v1/keys/envelopes");
        let mut builder = self
            .authorized_request(&config, Method::GET, url, true)
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
        let url = build_url(&config.base_url, "/v1/invite/redeem");
        let builder = self
            .authorized_request(&config, Method::POST, url, true)
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
        let url = build_url(&config.base_url, "/v1/labels");
        let mut builder = self
            .authorized_request(&config, Method::GET, url, true)
            .await?;
        builder = builder.query(&[
            ("target", request.target),
            ("limit", request.limit.unwrap_or(50).to_string()),
        ]);
        if let Some(topic) = request.topic {
            builder = builder.query(&[("topic", topic)]);
        }
        if let Some(cursor) = request.cursor {
            builder = builder.query(&[("cursor", cursor)]);
        }
        request_json(builder).await
    }

    pub async fn submit_report(
        &self,
        request: CommunityNodeReportRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let url = build_url(&config.base_url, "/v1/reports");
        let builder = self
            .authorized_request(&config, Method::POST, url, true)
            .await?
            .json(&request);
        request_json(builder).await
    }

    pub async fn trust_report_based(
        &self,
        request: CommunityNodeTrustRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let url = build_url(&config.base_url, "/v1/trust/report-based");
        let builder = self
            .authorized_request(&config, Method::GET, url, true)
            .await?
            .query(&[("subject", request.subject)]);
        request_json(builder).await
    }

    pub async fn trust_communication_density(
        &self,
        request: CommunityNodeTrustRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let url = build_url(&config.base_url, "/v1/trust/communication-density");
        let builder = self
            .authorized_request(&config, Method::GET, url, true)
            .await?
            .query(&[("subject", request.subject)]);
        request_json(builder).await
    }

    pub async fn search(
        &self,
        request: CommunityNodeSearchRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let url = build_url(&config.base_url, "/v1/search");
        let mut builder = self
            .authorized_request(&config, Method::GET, url, true)
            .await?
            .query(&[("topic", request.topic)]);
        if let Some(query) = request.q {
            builder = builder.query(&[("q", query)]);
        }
        if let Some(limit) = request.limit {
            builder = builder.query(&[("limit", limit.to_string())]);
        }
        if let Some(cursor) = request.cursor {
            builder = builder.query(&[("cursor", cursor)]);
        }
        request_json(builder).await
    }

    pub async fn list_bootstrap_nodes(&self) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let url = build_url(&config.base_url, "/v1/bootstrap/nodes");
        let builder = self
            .authorized_request(&config, Method::GET, url, false)
            .await?;
        request_json(builder).await
    }

    pub async fn list_bootstrap_services(
        &self,
        request: CommunityNodeBootstrapServicesRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let path = format!("/v1/bootstrap/topics/{}/services", request.topic_id);
        let url = build_url(&config.base_url, &path);
        let builder = self
            .authorized_request(&config, Method::GET, url, false)
            .await?;
        request_json(builder).await
    }

    pub async fn get_consent_status(&self) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let url = build_url(&config.base_url, "/v1/consents/status");
        let builder = self
            .authorized_request(&config, Method::GET, url, true)
            .await?;
        request_json(builder).await
    }

    pub async fn accept_consents(
        &self,
        request: CommunityNodeConsentRequest,
    ) -> Result<serde_json::Value, AppError> {
        let config = self.require_config().await?;
        let url = build_url(&config.base_url, "/v1/consents");
        let builder = self
            .authorized_request(&config, Method::POST, url, true)
            .await?
            .json(&request);
        request_json(builder).await
    }

    async fn require_config(&self) -> Result<CommunityNodeConfig, AppError> {
        self.load_config()
            .await?
            .filter(|cfg| !cfg.base_url.is_empty())
            .ok_or_else(|| AppError::NotFound("Community node is not configured".to_string()))
    }

    async fn load_config(&self) -> Result<Option<CommunityNodeConfig>, AppError> {
        let raw = self
            .secure_storage
            .retrieve(COMMUNITY_NODE_CONFIG_KEY)
            .await
            .map_err(|err| AppError::Storage(err.to_string()))?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let parsed = serde_json::from_str(&raw)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        Ok(Some(parsed))
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
        config: &CommunityNodeConfig,
        method: Method,
        url: String,
        require_auth: bool,
    ) -> Result<reqwest::RequestBuilder, AppError> {
        let builder = self.client.request(method, url);
        let Some(token) = config.access_token.as_ref() else {
            if require_auth {
                return Err(AppError::Unauthorized(
                    "Community node token is missing".to_string(),
                ));
            }
            return Ok(builder);
        };
        if let Some(exp) = config.token_expires_at {
            if exp <= Utc::now().timestamp() {
                return Err(AppError::Unauthorized(
                    "Community node token has expired".to_string(),
                ));
            }
        }
        Ok(builder.bearer_auth(token))
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
        base_url: config.base_url.clone(),
        has_token: config.access_token.is_some(),
        token_expires_at: config.token_expires_at,
        pubkey: config.pubkey.clone(),
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
