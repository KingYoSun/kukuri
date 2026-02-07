use crate::application::ports::group_key_store::{GroupKeyRecord, GroupKeyStore};
use crate::application::ports::join_request_store::{
    InviteUsageRecord, JoinRequestRecord, JoinRequestStore,
};
use crate::application::ports::key_manager::KeyManager;
use crate::application::ports::repositories::UserRepository;
use crate::application::shared::nostr::to_nostr_event;
use crate::domain::entities::Event;
use crate::domain::p2p::user_topic_id;
use crate::infrastructure::crypto::SignatureService;
use crate::infrastructure::p2p::GossipService;
use crate::shared::{AppError, RateLimiter, ValidationFailureKind};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use nostr_sdk::prelude::{PublicKey, SecretKey, nip44};
use rand::rngs::OsRng;
use rand_core::TryRngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const KIP_NAMESPACE: &str = "kukuri";
const KIP_VERSION: &str = "1";
const KIND_KEY_ENVELOPE: u32 = 39020;
const KIND_INVITE_CAPABILITY: u32 = 39021;
const KIND_JOIN_REQUEST: u32 = 39022;
const SCHEMA_KEY_ENVELOPE: &str = "kukuri-key-envelope-v1";
const SCHEMA_INVITE_CAPABILITY: &str = "kukuri-invite-v1";
const SCHEMA_JOIN_REQUEST: &str = "kukuri-join-request-v1";
const JOIN_REQUEST_RATE_LIMIT_MAX: usize = 3;
const JOIN_REQUEST_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct JoinRequestInput {
    pub topic_id: Option<String>,
    pub scope: Option<String>,
    pub invite_event_json: Option<serde_json::Value>,
    pub target_pubkey: Option<String>,
    pub broadcast_to_topic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequestResult {
    pub event_id: String,
    pub sent_topics: Vec<String>,
    pub event_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequestApprovalResult {
    pub event_id: String,
    pub key_envelope_event_id: String,
    pub key_envelope_event_json: serde_json::Value,
    pub recipient_pubkey: String,
    pub topic_id: String,
    pub scope: String,
}

pub struct AccessControlService {
    key_manager: Arc<dyn KeyManager>,
    group_key_store: Arc<dyn GroupKeyStore>,
    join_request_store: Arc<dyn JoinRequestStore>,
    user_repository: Arc<dyn UserRepository>,
    signature_service: Arc<dyn SignatureService>,
    gossip_service: Arc<dyn GossipService>,
    join_request_rate_limiter: RateLimiter,
}

impl AccessControlService {
    pub fn new(
        key_manager: Arc<dyn KeyManager>,
        group_key_store: Arc<dyn GroupKeyStore>,
        join_request_store: Arc<dyn JoinRequestStore>,
        user_repository: Arc<dyn UserRepository>,
        signature_service: Arc<dyn SignatureService>,
        gossip_service: Arc<dyn GossipService>,
    ) -> Self {
        Self {
            key_manager,
            group_key_store,
            join_request_store,
            user_repository,
            signature_service,
            gossip_service,
            join_request_rate_limiter: RateLimiter::new(
                JOIN_REQUEST_RATE_LIMIT_MAX,
                JOIN_REQUEST_RATE_LIMIT_WINDOW,
            ),
        }
    }

    pub async fn issue_invite(
        &self,
        topic_id: &str,
        expires_in: Option<i64>,
        max_uses: Option<i64>,
        nonce: Option<String>,
    ) -> Result<serde_json::Value, AppError> {
        if topic_id.trim().is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Topic ID is required",
            ));
        }

        let keypair = self.current_keypair().await?;
        let now = Utc::now().timestamp();
        let expires_in = expires_in.unwrap_or(86_400).max(60);
        let expires_at = now.saturating_add(expires_in);
        let max_uses = max_uses.unwrap_or(1).max(1);
        let nonce = nonce.unwrap_or_else(|| Uuid::new_v4().to_string());

        let _ = self.ensure_group_key(topic_id, "invite").await?;

        let content = json!({
            "schema": SCHEMA_INVITE_CAPABILITY,
            "topic": topic_id,
            "scope": "invite",
            "expires": expires_at,
            "max_uses": max_uses,
            "nonce": nonce,
            "issuer": format!("pubkey:{}", keypair.public_key),
        })
        .to_string();

        let tags = vec![
            vec!["t".to_string(), topic_id.to_string()],
            vec!["scope".to_string(), "invite".to_string()],
            vec!["d".to_string(), format!("invite:{nonce}")],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["exp".to_string(), expires_at.to_string()],
        ];

        let event = self
            .build_signed_event(KIND_INVITE_CAPABILITY, content, tags)
            .await?;
        let nostr_event = to_nostr_event(&event)?;
        serde_json::to_value(nostr_event)
            .map_err(|err| AppError::SerializationError(err.to_string()))
    }

    pub async fn request_join(
        &self,
        input: JoinRequestInput,
    ) -> Result<JoinRequestResult, AppError> {
        let keypair = self.current_keypair().await?;
        let requester_pubkey = keypair.public_key.clone();
        let now = Utc::now().timestamp();
        let broadcast_to_topic = input.broadcast_to_topic;

        let (topic_id, scope, invite_event_json, issuer_pubkey) =
            self.resolve_join_request_input(&input)?;
        if let Some(issuer) = issuer_pubkey.as_ref() {
            PublicKey::from_hex(issuer).map_err(|err| {
                AppError::validation(
                    ValidationFailureKind::Generic,
                    format!("Invalid target pubkey: {err}"),
                )
            })?;
        }

        let mut tags = vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["scope".to_string(), scope.clone()],
            vec![
                "d".to_string(),
                format!("join:{topic_id}:{}:{requester_pubkey}", Uuid::new_v4()),
            ],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
        ];

        if let Some(invite_event) = invite_event_json.as_ref() {
            if let Some(invite_id) = invite_event.get("id").and_then(|v| v.as_str()) {
                tags.push(vec!["e".to_string(), invite_id.to_string()]);
            }
        }
        if let Some(issuer) = issuer_pubkey.as_ref() {
            tags.push(vec!["p".to_string(), issuer.to_string()]);
        }

        let mut content = json!({
            "schema": SCHEMA_JOIN_REQUEST,
            "topic": topic_id,
            "scope": scope,
            "requester": format!("pubkey:{requester_pubkey}"),
            "requested_at": now,
        });
        if let Some(invite_event) = invite_event_json {
            content["invite_event_json"] = invite_event;
        }

        let event = self
            .build_signed_event(KIND_JOIN_REQUEST, content.to_string(), tags)
            .await?;
        let event_json = serde_json::to_value(to_nostr_event(&event)?)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;

        let mut topics = HashSet::new();
        if let Some(issuer) = issuer_pubkey {
            topics.insert(user_topic_id(&issuer));
        }
        if broadcast_to_topic {
            topics.insert(topic_id.clone());
        }

        if topics.is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Join request target is missing",
            ));
        }

        let sent_topics: Vec<String> = topics.into_iter().collect();
        self.broadcast_event(&event, &sent_topics).await?;

        Ok(JoinRequestResult {
            event_id: event.id.clone(),
            sent_topics,
            event_json,
        })
    }

    pub async fn list_pending_join_requests(&self) -> Result<Vec<JoinRequestRecord>, AppError> {
        let keypair = self.current_keypair().await?;
        self.join_request_store
            .list_requests(&keypair.public_key)
            .await
    }

    pub async fn approve_join_request(
        &self,
        event_id: &str,
    ) -> Result<JoinRequestApprovalResult, AppError> {
        let keypair = self.current_keypair().await?;
        if event_id.trim().is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Join request event_id is required",
            ));
        }

        let Some(record) = self
            .join_request_store
            .get_request(&keypair.public_key, event_id)
            .await?
        else {
            return Err(AppError::NotFound("Join request not found".to_string()));
        };

        let Some(context) = self
            .validate_join_request_event(&record.event, &keypair.public_key)
            .await?
        else {
            return Err(AppError::NotFound(
                "Join request is not available".to_string(),
            ));
        };

        let group_key = self
            .ensure_group_key(&context.topic_id, &context.scope)
            .await?;
        let envelope_event = self
            .build_key_envelope_event(&context.requester_pubkey, &group_key)
            .await?;
        let key_envelope_event_json = serde_json::to_value(to_nostr_event(&envelope_event)?)
            .map_err(|err| AppError::SerializationError(err.to_string()))?;
        let topics = vec![user_topic_id(&context.requester_pubkey)];
        self.broadcast_event(&envelope_event, &topics).await?;

        self.join_request_store
            .delete_request(&keypair.public_key, event_id)
            .await?;

        Ok(JoinRequestApprovalResult {
            event_id: record.event.id,
            key_envelope_event_id: envelope_event.id,
            key_envelope_event_json,
            recipient_pubkey: context.requester_pubkey,
            topic_id: context.topic_id,
            scope: context.scope,
        })
    }

    pub async fn reject_join_request(&self, event_id: &str) -> Result<(), AppError> {
        let keypair = self.current_keypair().await?;
        if event_id.trim().is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Join request event_id is required",
            ));
        }
        self.join_request_store
            .delete_request(&keypair.public_key, event_id)
            .await?;
        Ok(())
    }

    pub async fn handle_incoming_event(&self, event: &Event) -> Result<(), AppError> {
        match event.kind {
            KIND_JOIN_REQUEST => self.handle_join_request(event).await?,
            KIND_KEY_ENVELOPE => self.handle_key_envelope(event).await?,
            _ => {}
        }
        Ok(())
    }

    async fn handle_join_request(&self, event: &Event) -> Result<(), AppError> {
        let keypair = match self.key_manager.current_keypair().await {
            Ok(pair) => pair,
            Err(_) => return Ok(()),
        };
        let Some(context) = self
            .validate_join_request_event(event, &keypair.public_key)
            .await?
        else {
            return Ok(());
        };

        if self
            .join_request_store
            .get_request(&keypair.public_key, &event.id)
            .await?
            .is_some()
        {
            return Ok(());
        }

        let rate_key = format!(
            "join:{}:{}:{}",
            context.topic_id, context.scope, context.requester_pubkey
        );
        self.join_request_rate_limiter
            .check_and_record(
                &rate_key,
                "join.request の受信が多すぎます。しばらく待ってください",
            )
            .await?;

        if context.scope == "invite" {
            let invite_event_id = context.invite_event_id.as_deref().ok_or_else(|| {
                AppError::validation(ValidationFailureKind::Generic, "Invite event_id is missing")
            })?;
            let max_uses = context.invite_max_uses.unwrap_or(1);
            self.consume_invite_usage(&keypair.public_key, invite_event_id, max_uses)
                .await?;
        }

        let record = JoinRequestRecord {
            event: event.clone(),
            topic_id: context.topic_id,
            scope: context.scope,
            requester_pubkey: context.requester_pubkey,
            target_pubkey: context.target_pubkey,
            requested_at: context.requested_at,
            received_at: Utc::now().timestamp(),
            invite_event_json: context.invite_event_json,
        };

        self.join_request_store
            .upsert_request(&keypair.public_key, record)
            .await?;
        Ok(())
    }

    async fn validate_join_request_event(
        &self,
        event: &Event,
        current_pubkey: &str,
    ) -> Result<Option<JoinRequestContext>, AppError> {
        if event.pubkey == current_pubkey {
            return Ok(None);
        }
        if let Ok(nostr_event) = to_nostr_event(event) {
            if nostr_event.verify().is_err() {
                return Ok(None);
            }
        }

        let tags = event.tags.clone();
        let topic_id = require_tag_value(&tags, "t")?;
        let scope = require_tag_value(&tags, "scope")?;
        let _ = require_tag_value(&tags, "d")?;
        validate_kip_tags(&tags)?;
        match scope.as_str() {
            "invite" | "friend" => {}
            "friend_plus" => {}
            _ => {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    format!("Invalid join scope: {scope}"),
                ));
            }
        }
        let target_pubkey = tag_value(&tags, "p");
        if let Some(target) = target_pubkey.as_ref() {
            if target != current_pubkey {
                return Ok(None);
            }
        }

        let content: JoinRequestPayload = serde_json::from_str(&event.content)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;
        if content.schema != SCHEMA_JOIN_REQUEST {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invalid join.request schema",
            ));
        }
        if let Some(topic) = content.topic.as_ref() {
            if topic != &topic_id {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    "Join.request topic mismatch",
                ));
            }
        }
        if let Some(scope_value) = content.scope.as_ref() {
            if scope_value != &scope {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    "Join.request scope mismatch",
                ));
            }
        }
        if let Some(requester) = content.requester.as_ref() {
            if let Some(hex) = requester.strip_prefix("pubkey:") {
                if hex != event.pubkey {
                    return Err(AppError::validation(
                        ValidationFailureKind::Generic,
                        "Join.request requester mismatch",
                    ));
                }
            }
        }
        if let Some(requested_at) = content.requested_at {
            if requested_at < 0 {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    "Join.request requested_at is invalid",
                ));
            }
        }

        let mut invite_event_id = None;
        let mut invite_max_uses = None;
        if scope == "invite" {
            let invite_json = content.invite_event_json.clone().ok_or_else(|| {
                AppError::validation(ValidationFailureKind::Generic, "Invite payload is missing")
            })?;
            let invite = validate_invite_event(&invite_json, Some(&topic_id))?;
            invite_event_id = Some(invite.event_id.clone());
            invite_max_uses = Some(invite.max_uses);
            if let Some(target) = target_pubkey.as_ref() {
                if invite.issuer_pubkey != *target {
                    return Ok(None);
                }
            }
        }
        if scope == "friend_plus" {
            let is_fof = self
                .is_friend_of_friend(current_pubkey, &event.pubkey)
                .await?;
            if !is_fof {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    "friend_plus join.request requires FoF",
                ));
            }
        }

        Ok(Some(JoinRequestContext {
            topic_id,
            scope,
            target_pubkey,
            requester_pubkey: event.pubkey.clone(),
            invite_event_json: content.invite_event_json,
            invite_event_id,
            invite_max_uses,
            requested_at: content.requested_at,
        }))
    }

    async fn handle_key_envelope(&self, event: &Event) -> Result<(), AppError> {
        let keypair = match self.key_manager.current_keypair().await {
            Ok(pair) => pair,
            Err(_) => return Ok(()),
        };
        if let Ok(nostr_event) = to_nostr_event(event) {
            if nostr_event.verify().is_err() {
                return Ok(());
            }
        }

        let tags = event.tags.clone();
        validate_kip_tags(&tags)?;
        let recipient = require_tag_value(&tags, "p")?;
        if recipient != keypair.public_key {
            return Ok(());
        }
        let topic_tag = require_tag_value(&tags, "t")?;
        let scope_tag = require_tag_value(&tags, "scope")?;
        let epoch_tag = require_tag_value(&tags, "epoch")?
            .parse::<i64>()
            .map_err(|_| {
                AppError::validation(ValidationFailureKind::Generic, "Invalid epoch tag")
            })?;
        if epoch_tag <= 0 {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invalid epoch tag",
            ));
        }

        let secret_key = SecretKey::from_hex(&keypair.private_key)
            .map_err(|err| AppError::Crypto(format!("Invalid private key: {err}")))?;
        let sender_pubkey = PublicKey::from_hex(&event.pubkey)
            .map_err(|err| AppError::Crypto(format!("Invalid sender pubkey: {err}")))?;
        let decrypted = nip44::decrypt(&secret_key, &sender_pubkey, event.content.clone())
            .map_err(|err| AppError::Crypto(format!("NIP-44 decrypt failed: {err}")))?;
        let payload: KeyEnvelopePayload = serde_json::from_str(&decrypted)
            .map_err(|err| AppError::DeserializationError(err.to_string()))?;

        if payload.schema != SCHEMA_KEY_ENVELOPE {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invalid key envelope schema",
            ));
        }
        if payload.topic != topic_tag || payload.scope != scope_tag || payload.epoch != epoch_tag {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Key envelope payload mismatch",
            ));
        }

        let stored_at = payload.issued_at.unwrap_or_else(|| Utc::now().timestamp());
        let record = GroupKeyRecord {
            topic_id: payload.topic,
            scope: payload.scope,
            epoch: payload.epoch,
            key_b64: payload.key_b64,
            stored_at,
        };
        self.group_key_store.store_key(record).await?;
        Ok(())
    }

    async fn ensure_group_key(
        &self,
        topic_id: &str,
        scope: &str,
    ) -> Result<GroupKeyRecord, AppError> {
        if let Some(record) = self.group_key_store.get_latest_key(topic_id, scope).await? {
            return Ok(record);
        }

        let mut key_bytes = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut key_bytes)
            .map_err(|err| AppError::Crypto(format!("Failed to generate group key: {err}")))?;
        let key_b64 = BASE64_STANDARD.encode(key_bytes);
        let record = GroupKeyRecord {
            topic_id: topic_id.to_string(),
            scope: scope.to_string(),
            epoch: 1,
            key_b64,
            stored_at: Utc::now().timestamp(),
        };
        self.group_key_store.store_key(record.clone()).await?;
        Ok(record)
    }

    async fn build_key_envelope_event(
        &self,
        recipient_pubkey: &str,
        record: &GroupKeyRecord,
    ) -> Result<Event, AppError> {
        let keypair = self.current_keypair().await?;
        let secret_key = SecretKey::from_hex(&keypair.private_key)
            .map_err(|err| AppError::Crypto(format!("Invalid private key: {err}")))?;
        let recipient = PublicKey::from_hex(recipient_pubkey)
            .map_err(|err| AppError::Crypto(format!("Invalid recipient pubkey: {err}")))?;

        let payload = json!({
            "schema": SCHEMA_KEY_ENVELOPE,
            "topic": record.topic_id.clone(),
            "scope": record.scope.clone(),
            "epoch": record.epoch,
            "key_b64": record.key_b64.clone(),
            "issued_at": Utc::now().timestamp()
        });
        let encrypted = nip44::encrypt(
            &secret_key,
            &recipient,
            payload.to_string(),
            nip44::Version::V2,
        )
        .map_err(|err| AppError::Crypto(format!("NIP-44 encrypt failed: {err}")))?;

        let d_tag = format!(
            "keyenv:{}:{}:{}:{}",
            record.topic_id.as_str(),
            record.scope.as_str(),
            record.epoch,
            recipient_pubkey
        );
        let tags = vec![
            vec!["p".to_string(), recipient_pubkey.to_string()],
            vec!["t".to_string(), record.topic_id.clone()],
            vec!["scope".to_string(), record.scope.clone()],
            vec!["epoch".to_string(), record.epoch.to_string()],
            vec!["k".to_string(), KIP_NAMESPACE.to_string()],
            vec!["ver".to_string(), KIP_VERSION.to_string()],
            vec!["d".to_string(), d_tag],
        ];

        self.build_signed_event(KIND_KEY_ENVELOPE, encrypted, tags)
            .await
    }

    async fn build_signed_event(
        &self,
        kind: u32,
        content: String,
        tags: Vec<Vec<String>>,
    ) -> Result<Event, AppError> {
        let keypair = self.current_keypair().await?;
        let mut event = Event::new(kind, content, keypair.public_key.clone()).with_tags(tags);
        self.signature_service
            .sign_event(&mut event, &keypair.private_key)
            .await
            .map_err(|err| AppError::Crypto(err.to_string()))?;
        Ok(event)
    }

    async fn broadcast_event(&self, event: &Event, topics: &[String]) -> Result<(), AppError> {
        let mut uniq: HashSet<String> = HashSet::new();
        for topic in topics {
            let trimmed = topic.trim();
            if !trimmed.is_empty() {
                uniq.insert(trimmed.to_string());
            }
        }

        for topic in uniq {
            self.gossip_service.join_topic(&topic, Vec::new()).await?;
            self.gossip_service.broadcast(&topic, event).await?;
        }
        Ok(())
    }

    async fn collect_mutual_follow_pubkeys(
        &self,
        pubkey: &str,
    ) -> Result<HashSet<String>, AppError> {
        let following = self.user_repository.list_following_pubkeys(pubkey).await?;
        let followers = self.user_repository.list_follower_pubkeys(pubkey).await?;
        let following_set: HashSet<String> = following.into_iter().collect();
        let mut mutual = HashSet::new();
        for follower in followers {
            if following_set.contains(&follower) {
                mutual.insert(follower);
            }
        }
        Ok(mutual)
    }

    async fn is_friend_of_friend(
        &self,
        current_pubkey: &str,
        requester_pubkey: &str,
    ) -> Result<bool, AppError> {
        if current_pubkey == requester_pubkey {
            return Ok(false);
        }
        let current_friends = self.collect_mutual_follow_pubkeys(current_pubkey).await?;
        if current_friends.is_empty() {
            return Ok(false);
        }
        let requester_friends = self.collect_mutual_follow_pubkeys(requester_pubkey).await?;
        if requester_friends.is_empty() {
            return Ok(false);
        }
        Ok(current_friends
            .intersection(&requester_friends)
            .next()
            .is_some())
    }

    async fn consume_invite_usage(
        &self,
        owner_pubkey: &str,
        invite_event_id: &str,
        max_uses: i64,
    ) -> Result<(), AppError> {
        let now = Utc::now().timestamp();
        let incoming_max = max_uses.max(1);
        let mut record = match self
            .join_request_store
            .get_invite_usage(owner_pubkey, invite_event_id)
            .await?
        {
            Some(record) => record,
            None => InviteUsageRecord {
                invite_event_id: invite_event_id.to_string(),
                max_uses: incoming_max,
                used_count: 0,
                last_used_at: now,
            },
        };

        let existing_max = record.max_uses.max(1);
        let effective_max = existing_max.min(incoming_max);
        if record.used_count >= effective_max {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite max_uses exceeded",
            ));
        }

        record.used_count += 1;
        record.max_uses = effective_max;
        record.last_used_at = now;
        self.join_request_store
            .upsert_invite_usage(owner_pubkey, record)
            .await?;
        Ok(())
    }

    fn resolve_join_request_input(
        &self,
        input: &JoinRequestInput,
    ) -> Result<(String, String, Option<serde_json::Value>, Option<String>), AppError> {
        if let Some(invite_json) = input.invite_event_json.as_ref() {
            let invite = validate_invite_event(invite_json, None)?;
            let topic_id = invite.topic_id;
            let scope = "invite".to_string();
            let issuer_pubkey = Some(invite.issuer_pubkey);
            return Ok((topic_id, scope, Some(invite_json.clone()), issuer_pubkey));
        }

        let topic_id = input.topic_id.clone().ok_or_else(|| {
            AppError::validation(ValidationFailureKind::Generic, "Topic ID is required")
        })?;
        let scope = input.scope.clone().unwrap_or_else(|| "friend".to_string());
        if scope == "invite" {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite JSON is required for invite scope",
            ));
        }
        if scope != "friend" && scope != "friend_plus" {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                format!("Unsupported join scope: {scope}"),
            ));
        }

        let issuer_pubkey = input
            .target_pubkey
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if issuer_pubkey.is_none() && !input.broadcast_to_topic {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Target pubkey or broadcast_to_topic is required",
            ));
        }

        Ok((topic_id, scope, None, issuer_pubkey))
    }

    async fn current_keypair(
        &self,
    ) -> Result<crate::application::ports::key_manager::KeyPair, AppError> {
        self.key_manager.current_keypair().await
    }
}

#[derive(Debug, Clone)]
struct JoinRequestContext {
    topic_id: String,
    scope: String,
    target_pubkey: Option<String>,
    requester_pubkey: String,
    invite_event_json: Option<serde_json::Value>,
    invite_event_id: Option<String>,
    invite_max_uses: Option<i64>,
    requested_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct JoinRequestPayload {
    schema: String,
    topic: Option<String>,
    scope: Option<String>,
    invite_event_json: Option<serde_json::Value>,
    requester: Option<String>,
    requested_at: Option<i64>,
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

struct InviteValidation {
    topic_id: String,
    issuer_pubkey: String,
    event_id: String,
    max_uses: i64,
}

fn validate_invite_event(
    value: &serde_json::Value,
    expected_topic: Option<&str>,
) -> Result<InviteValidation, AppError> {
    let event: nostr_sdk::Event = serde_json::from_value(value.clone())
        .map_err(|err| AppError::DeserializationError(err.to_string()))?;

    if event.kind.as_u16() as u32 != KIND_INVITE_CAPABILITY {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invalid invite event kind",
        ));
    }
    let tags = event
        .tags
        .iter()
        .map(|tag| tag.clone().to_vec())
        .collect::<Vec<_>>();
    validate_kip_tags(&tags)?;
    let topic_id = require_tag_value(&tags, "t")?;
    if let Some(expected) = expected_topic {
        if expected != topic_id {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite topic mismatch",
            ));
        }
    }
    let scope = require_tag_value(&tags, "scope")?;
    if scope != "invite" {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invite scope must be invite",
        ));
    }
    let invite_d = require_tag_value(&tags, "d")?;
    if !invite_d.starts_with("invite:") {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invite d tag is invalid",
        ));
    }
    if event.verify().is_err() {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invite signature is invalid",
        ));
    }

    let payload: InvitePayload = serde_json::from_str(&event.content)
        .map_err(|err| AppError::DeserializationError(err.to_string()))?;
    if payload.schema != SCHEMA_INVITE_CAPABILITY {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invalid invite schema",
        ));
    }
    if let Some(topic) = payload.topic.as_ref() {
        if topic != &topic_id {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite topic mismatch",
            ));
        }
    }
    if let Some(scope) = payload.scope.as_ref() {
        if scope != "invite" {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite scope mismatch",
            ));
        }
    }
    if let Some(expires) = payload.expires {
        if expires <= Utc::now().timestamp() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite has expired",
            ));
        }
    }
    let max_uses = payload.max_uses.unwrap_or(1);
    if max_uses <= 0 {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invite max_uses must be positive",
        ));
    }
    if let Some(nonce) = payload.nonce.as_ref() {
        if nonce.trim().is_empty() {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite nonce is empty",
            ));
        }
        let expected = format!("invite:{nonce}");
        if invite_d != expected {
            return Err(AppError::validation(
                ValidationFailureKind::Generic,
                "Invite nonce mismatch",
            ));
        }
    }

    if let Some(issuer) = payload.issuer.as_ref() {
        if let Some(hex) = issuer.strip_prefix("pubkey:") {
            if hex != event.pubkey.to_string() {
                return Err(AppError::validation(
                    ValidationFailureKind::Generic,
                    "Invite issuer mismatch",
                ));
            }
        }
    }

    Ok(InviteValidation {
        topic_id,
        issuer_pubkey: event.pubkey.to_string(),
        event_id: event.id.to_string(),
        max_uses,
    })
}

#[derive(Debug, Deserialize)]
struct InvitePayload {
    schema: String,
    topic: Option<String>,
    scope: Option<String>,
    expires: Option<i64>,
    max_uses: Option<i64>,
    nonce: Option<String>,
    issuer: Option<String>,
}

fn validate_kip_tags(tags: &[Vec<String>]) -> Result<(), AppError> {
    let namespace = require_tag_value(tags, "k")?;
    if namespace != KIP_NAMESPACE {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invalid k tag",
        ));
    }
    let ver = require_tag_value(tags, "ver")?;
    if ver != KIP_VERSION {
        return Err(AppError::validation(
            ValidationFailureKind::Generic,
            "Invalid ver tag",
        ));
    }
    Ok(())
}

fn tag_value(tags: &[Vec<String>], name: &str) -> Option<String> {
    tags.iter()
        .find(|tag| tag.first().map(|v| v.as_str()) == Some(name))
        .and_then(|tag| tag.get(1))
        .cloned()
}

fn require_tag_value(tags: &[Vec<String>], name: &str) -> Result<String, AppError> {
    tag_value(tags, name).ok_or_else(|| {
        AppError::validation(
            ValidationFailureKind::Generic,
            format!("Missing {name} tag"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::group_key_store::GroupKeyEntry;
    use crate::application::ports::join_request_store::{InviteUsageRecord, JoinRequestStore};
    use crate::application::ports::key_manager::KeyPair;
    use crate::application::ports::repositories::{FollowListSort, UserCursorPage, UserRepository};
    use crate::domain::entities::User;
    use crate::infrastructure::crypto::DefaultSignatureService;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use nostr_sdk::ToBech32;
    use nostr_sdk::prelude::{EventBuilder, Keys, Kind, Tag};
    use std::collections::{HashMap, HashSet};
    use tokio::sync::RwLock;

    #[derive(Clone)]
    struct TestKeyManager {
        keypair: KeyPair,
    }

    impl TestKeyManager {
        fn new(keypair: KeyPair) -> Self {
            Self { keypair }
        }
    }

    #[async_trait]
    impl KeyManager for TestKeyManager {
        async fn generate_keypair(&self) -> Result<KeyPair, AppError> {
            Err(AppError::NotImplemented("generate_keypair".into()))
        }

        async fn import_private_key(&self, _nsec: &str) -> Result<KeyPair, AppError> {
            Err(AppError::NotImplemented("import_private_key".into()))
        }

        async fn export_private_key(&self, _npub: &str) -> Result<String, AppError> {
            Err(AppError::NotImplemented("export_private_key".into()))
        }

        async fn get_public_key(&self, _npub: &str) -> Result<String, AppError> {
            Err(AppError::NotImplemented("get_public_key".into()))
        }

        async fn store_keypair(&self, _keypair: &KeyPair) -> Result<(), AppError> {
            Err(AppError::NotImplemented("store_keypair".into()))
        }

        async fn delete_keypair(&self, _npub: &str) -> Result<(), AppError> {
            Err(AppError::NotImplemented("delete_keypair".into()))
        }

        async fn list_npubs(&self) -> Result<Vec<String>, AppError> {
            Ok(vec![self.keypair.npub.clone()])
        }

        async fn current_keypair(&self) -> Result<KeyPair, AppError> {
            Ok(self.keypair.clone())
        }
    }

    #[derive(Clone, Default)]
    struct TestGroupKeyStore {
        records: Arc<RwLock<Vec<GroupKeyRecord>>>,
    }

    impl TestGroupKeyStore {
        async fn latest_key(&self, topic_id: &str, scope: &str) -> Option<GroupKeyRecord> {
            let records = self.records.read().await;
            records
                .iter()
                .filter(|record| record.topic_id == topic_id && record.scope == scope)
                .max_by_key(|record| record.epoch)
                .cloned()
        }
    }

    #[async_trait]
    impl GroupKeyStore for TestGroupKeyStore {
        async fn store_key(&self, record: GroupKeyRecord) -> Result<(), AppError> {
            let mut records = self.records.write().await;
            records.retain(|existing| {
                !(existing.topic_id == record.topic_id
                    && existing.scope == record.scope
                    && existing.epoch == record.epoch)
            });
            records.push(record);
            Ok(())
        }

        async fn get_key(
            &self,
            topic_id: &str,
            scope: &str,
            epoch: i64,
        ) -> Result<Option<GroupKeyRecord>, AppError> {
            let records = self.records.read().await;
            Ok(records
                .iter()
                .find(|record| {
                    record.topic_id == topic_id && record.scope == scope && record.epoch == epoch
                })
                .cloned())
        }

        async fn get_latest_key(
            &self,
            topic_id: &str,
            scope: &str,
        ) -> Result<Option<GroupKeyRecord>, AppError> {
            Ok(self.latest_key(topic_id, scope).await)
        }

        async fn list_keys(&self) -> Result<Vec<GroupKeyEntry>, AppError> {
            let records = self.records.read().await;
            Ok(records
                .iter()
                .map(|record| GroupKeyEntry {
                    topic_id: record.topic_id.clone(),
                    scope: record.scope.clone(),
                    epoch: record.epoch,
                    stored_at: record.stored_at,
                })
                .collect())
        }
    }

    #[derive(Clone, Default)]
    struct TestJoinRequestStore {
        records: Arc<RwLock<HashMap<String, HashMap<String, JoinRequestRecord>>>>,
        invite_usage: Arc<RwLock<HashMap<String, HashMap<String, InviteUsageRecord>>>>,
    }

    #[async_trait]
    impl JoinRequestStore for TestJoinRequestStore {
        async fn upsert_request(
            &self,
            owner_pubkey: &str,
            record: JoinRequestRecord,
        ) -> Result<(), AppError> {
            let mut records = self.records.write().await;
            let owner = records.entry(owner_pubkey.to_string()).or_default();
            owner.insert(record.event.id.clone(), record);
            Ok(())
        }

        async fn list_requests(
            &self,
            owner_pubkey: &str,
        ) -> Result<Vec<JoinRequestRecord>, AppError> {
            let records = self.records.read().await;
            Ok(records
                .get(owner_pubkey)
                .map(|owner| owner.values().cloned().collect())
                .unwrap_or_default())
        }

        async fn get_request(
            &self,
            owner_pubkey: &str,
            event_id: &str,
        ) -> Result<Option<JoinRequestRecord>, AppError> {
            let records = self.records.read().await;
            Ok(records
                .get(owner_pubkey)
                .and_then(|owner| owner.get(event_id).cloned()))
        }

        async fn delete_request(&self, owner_pubkey: &str, event_id: &str) -> Result<(), AppError> {
            let mut records = self.records.write().await;
            if let Some(owner) = records.get_mut(owner_pubkey) {
                owner.remove(event_id);
            }
            Ok(())
        }

        async fn get_invite_usage(
            &self,
            owner_pubkey: &str,
            invite_event_id: &str,
        ) -> Result<Option<InviteUsageRecord>, AppError> {
            let records = self.invite_usage.read().await;
            Ok(records
                .get(owner_pubkey)
                .and_then(|owner| owner.get(invite_event_id).cloned()))
        }

        async fn upsert_invite_usage(
            &self,
            owner_pubkey: &str,
            record: InviteUsageRecord,
        ) -> Result<(), AppError> {
            let mut records = self.invite_usage.write().await;
            let owner = records.entry(owner_pubkey.to_string()).or_default();
            owner.insert(record.invite_event_id.clone(), record);
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct TestUserRepository {
        follows: Arc<RwLock<HashSet<(String, String)>>>,
    }

    impl TestUserRepository {
        async fn seed_follow(&self, follower: &str, followed: &str) {
            let mut follows = self.follows.write().await;
            follows.insert((follower.to_string(), followed.to_string()));
        }
    }

    #[async_trait]
    impl UserRepository for TestUserRepository {
        async fn create_user(&self, _user: &User) -> Result<(), AppError> {
            Err(AppError::NotImplemented("create_user".into()))
        }

        async fn get_user(&self, _npub: &str) -> Result<Option<User>, AppError> {
            Err(AppError::NotImplemented("get_user".into()))
        }

        async fn get_user_by_pubkey(&self, _pubkey: &str) -> Result<Option<User>, AppError> {
            Err(AppError::NotImplemented("get_user_by_pubkey".into()))
        }

        async fn search_users(&self, _query: &str, _limit: usize) -> Result<Vec<User>, AppError> {
            Err(AppError::NotImplemented("search_users".into()))
        }

        async fn update_user(&self, _user: &User) -> Result<(), AppError> {
            Err(AppError::NotImplemented("update_user".into()))
        }

        async fn delete_user(&self, _npub: &str) -> Result<(), AppError> {
            Err(AppError::NotImplemented("delete_user".into()))
        }

        async fn get_followers_paginated(
            &self,
            _npub: &str,
            _cursor: Option<&str>,
            _limit: usize,
            _sort: FollowListSort,
            _search: Option<&str>,
        ) -> Result<UserCursorPage, AppError> {
            Err(AppError::NotImplemented("get_followers_paginated".into()))
        }

        async fn get_following_paginated(
            &self,
            _npub: &str,
            _cursor: Option<&str>,
            _limit: usize,
            _sort: FollowListSort,
            _search: Option<&str>,
        ) -> Result<UserCursorPage, AppError> {
            Err(AppError::NotImplemented("get_following_paginated".into()))
        }

        async fn add_follow_relation(
            &self,
            follower_pubkey: &str,
            followed_pubkey: &str,
        ) -> Result<bool, AppError> {
            let mut follows = self.follows.write().await;
            Ok(follows.insert((follower_pubkey.to_string(), followed_pubkey.to_string())))
        }

        async fn remove_follow_relation(
            &self,
            follower_pubkey: &str,
            followed_pubkey: &str,
        ) -> Result<bool, AppError> {
            let mut follows = self.follows.write().await;
            Ok(follows.remove(&(follower_pubkey.to_string(), followed_pubkey.to_string())))
        }

        async fn list_following_pubkeys(
            &self,
            follower_pubkey: &str,
        ) -> Result<Vec<String>, AppError> {
            let follows = self.follows.read().await;
            Ok(follows
                .iter()
                .filter_map(|(follower, followed)| {
                    if follower == follower_pubkey {
                        Some(followed.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }

        async fn list_follower_pubkeys(
            &self,
            followed_pubkey: &str,
        ) -> Result<Vec<String>, AppError> {
            let follows = self.follows.read().await;
            Ok(follows
                .iter()
                .filter_map(|(follower, followed)| {
                    if followed == followed_pubkey {
                        Some(follower.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }
    }

    #[derive(Clone, Default)]
    struct TestGossipService {
        joined: Arc<RwLock<HashSet<String>>>,
        broadcasts: Arc<RwLock<Vec<(String, Event)>>>,
    }

    impl TestGossipService {
        async fn broadcasts(&self) -> Vec<(String, Event)> {
            self.broadcasts.read().await.clone()
        }
    }

    #[async_trait]
    impl GossipService for TestGossipService {
        async fn join_topic(
            &self,
            topic: &str,
            _initial_peers: Vec<String>,
        ) -> Result<(), AppError> {
            self.joined.write().await.insert(topic.to_string());
            Ok(())
        }

        async fn leave_topic(&self, topic: &str) -> Result<(), AppError> {
            self.joined.write().await.remove(topic);
            Ok(())
        }

        async fn broadcast(&self, topic: &str, event: &Event) -> Result<(), AppError> {
            self.broadcasts
                .write()
                .await
                .push((topic.to_string(), event.clone()));
            Ok(())
        }

        async fn subscribe(
            &self,
            _topic: &str,
        ) -> Result<tokio::sync::mpsc::Receiver<Event>, AppError> {
            Err(AppError::NotImplemented("subscribe".into()))
        }

        async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
            Ok(self.joined.read().await.iter().cloned().collect())
        }

        async fn get_topic_peers(&self, _topic: &str) -> Result<Vec<String>, AppError> {
            Err(AppError::NotImplemented("get_topic_peers".into()))
        }

        async fn get_topic_stats(
            &self,
            _topic: &str,
        ) -> Result<Option<crate::domain::p2p::TopicStats>, AppError> {
            Err(AppError::NotImplemented("get_topic_stats".into()))
        }

        async fn broadcast_message(&self, _topic: &str, _message: &[u8]) -> Result<(), AppError> {
            Err(AppError::NotImplemented("broadcast_message".into()))
        }
    }

    fn make_keypair() -> (Keys, KeyPair) {
        let keys = Keys::generate();
        let public_key = keys.public_key().to_string();
        let private_key = keys.secret_key().display_secret().to_string();
        let npub = keys
            .public_key()
            .to_bech32()
            .unwrap_or_else(|_| public_key.clone());
        let nsec = format!("nsec1{private_key}");
        let keypair = KeyPair {
            public_key,
            private_key,
            npub,
            nsec,
        };
        (keys, keypair)
    }

    fn domain_event_from_nostr(event: &nostr_sdk::Event) -> Event {
        let created_at = Utc
            .timestamp_opt(event.created_at.as_secs() as i64, 0)
            .single()
            .expect("timestamp");
        Event {
            id: event.id.to_string(),
            pubkey: event.pubkey.to_string(),
            created_at,
            kind: event.kind.as_u16() as u32,
            tags: event.tags.iter().map(|tag| tag.clone().to_vec()).collect(),
            content: event.content.clone(),
            sig: event.sig.to_string(),
        }
    }

    #[tokio::test]
    async fn issue_invite_creates_event_and_group_key() {
        let (_keys, keypair) = make_keypair();
        let key_manager = Arc::new(TestKeyManager::new(keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let invite_json = service
            .issue_invite("kukuri:topic1", Some(600), Some(2), Some("nonce-1".into()))
            .await
            .expect("invite");

        let event: nostr_sdk::Event = serde_json::from_value(invite_json).expect("nostr event");
        assert_eq!(event.kind.as_u16() as u32, KIND_INVITE_CAPABILITY);

        let tags = event
            .tags
            .iter()
            .map(|tag| tag.clone().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(tag_value(&tags, "t"), Some("kukuri:topic1".to_string()));
        assert_eq!(tag_value(&tags, "scope"), Some("invite".to_string()));
        assert_eq!(tag_value(&tags, "k"), Some(KIP_NAMESPACE.to_string()));
        assert_eq!(tag_value(&tags, "ver"), Some(KIP_VERSION.to_string()));
        assert_eq!(tag_value(&tags, "d"), Some("invite:nonce-1".to_string()));

        let stored = group_key_store
            .get_latest_key("kukuri:topic1", "invite")
            .await
            .expect("store")
            .expect("group key");
        assert_eq!(stored.scope, "invite");
    }

    #[tokio::test]
    async fn request_join_with_invite_broadcasts_to_issuer_topic() {
        let (_keys, keypair) = make_keypair();
        let key_manager = Arc::new(TestKeyManager::new(keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let invite_json = service
            .issue_invite("kukuri:topic1", Some(600), Some(1), Some("nonce-2".into()))
            .await
            .expect("invite");
        let invite_event: nostr_sdk::Event =
            serde_json::from_value(invite_json.clone()).expect("nostr event");

        let result = service
            .request_join(JoinRequestInput {
                topic_id: None,
                scope: None,
                invite_event_json: Some(invite_json),
                target_pubkey: None,
                broadcast_to_topic: false,
            })
            .await
            .expect("join request");

        let issuer_topic = user_topic_id(&invite_event.pubkey.to_string());
        assert!(result.sent_topics.contains(&issuer_topic));

        let broadcasts = gossip_service.broadcasts().await;
        assert_eq!(broadcasts.len(), 1);
        let (topic, event) = &broadcasts[0];
        assert_eq!(topic, &issuer_topic);
        assert_eq!(event.kind, KIND_JOIN_REQUEST);

        let tags = event.tags.clone();
        assert_eq!(tag_value(&tags, "e"), Some(invite_event.id.to_string()));
        assert_eq!(tag_value(&tags, "p"), Some(invite_event.pubkey.to_string()));
    }

    #[tokio::test]
    async fn validate_invite_event_rejects_nonce_mismatch() {
        let (keys, keypair) = make_keypair();
        let topic_id = "kukuri:topic1";

        let content = json!({
            "schema": SCHEMA_INVITE_CAPABILITY,
            "topic": topic_id,
            "scope": "invite",
            "expires": Utc::now().timestamp() + 600,
            "max_uses": 1,
            "nonce": "nonce-1",
            "issuer": format!("pubkey:{}", keypair.public_key),
        })
        .to_string();

        let tags = vec![
            Tag::parse(["t", topic_id]).expect("tag"),
            Tag::parse(["scope", "invite"]).expect("tag"),
            Tag::parse(["d", "invite:nonce-2"]).expect("tag"),
            Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
            Tag::parse(["ver", KIP_VERSION]).expect("tag"),
        ];

        let nostr_event = EventBuilder::new(Kind::from(KIND_INVITE_CAPABILITY as u16), content)
            .tags(tags)
            .sign_with_keys(&keys)
            .expect("signed");

        let value = serde_json::to_value(nostr_event).expect("value");
        let err = match validate_invite_event(&value, Some(topic_id)) {
            Ok(_) => panic!("should fail"),
            Err(err) => err,
        };
        assert_eq!(err.validation_message(), Some("Invite nonce mismatch"));
    }

    #[tokio::test]
    async fn validate_invite_event_rejects_invalid_schema() {
        let (keys, keypair) = make_keypair();
        let topic_id = "kukuri:topic1";

        let content = json!({
            "schema": "kukuri-invite-legacy-v1",
            "topic": topic_id,
            "scope": "invite",
            "expires": Utc::now().timestamp() + 600,
            "max_uses": 1,
            "nonce": "nonce-1",
            "issuer": format!("pubkey:{}", keypair.public_key),
        })
        .to_string();

        let tags = vec![
            Tag::parse(["t", topic_id]).expect("tag"),
            Tag::parse(["scope", "invite"]).expect("tag"),
            Tag::parse(["d", "invite:nonce-1"]).expect("tag"),
            Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
            Tag::parse(["ver", KIP_VERSION]).expect("tag"),
        ];

        let nostr_event = EventBuilder::new(Kind::from(KIND_INVITE_CAPABILITY as u16), content)
            .tags(tags)
            .sign_with_keys(&keys)
            .expect("signed");

        let value = serde_json::to_value(nostr_event).expect("value");
        let err = match validate_invite_event(&value, Some(topic_id)) {
            Ok(_) => panic!("should fail"),
            Err(err) => err,
        };
        assert_eq!(err.validation_message(), Some("Invalid invite schema"));
    }

    #[tokio::test]
    async fn handle_join_request_stores_pending_request() {
        let (_member_keys, member_keypair) = make_keypair();
        let (requester_keys, requester_keypair) = make_keypair();

        let key_manager = Arc::new(TestKeyManager::new(member_keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let topic_id = "kukuri:topic1";
        group_key_store
            .store_key(GroupKeyRecord {
                topic_id: topic_id.to_string(),
                scope: "friend".to_string(),
                epoch: 1,
                key_b64: "aGVsbG8=".to_string(),
                stored_at: Utc::now().timestamp(),
            })
            .await
            .expect("store");

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let content = json!({
            "schema": SCHEMA_JOIN_REQUEST,
            "topic": topic_id,
            "scope": "friend",
            "requester": format!("pubkey:{}", requester_keypair.public_key),
            "requested_at": Utc::now().timestamp(),
        })
        .to_string();

        let tags = vec![
            Tag::parse(["t", topic_id]).expect("tag"),
            Tag::parse(["scope", "friend"]).expect("tag"),
            Tag::parse(["d", "join:kukuri:topic1:nonce:requester"]).expect("tag"),
            Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
            Tag::parse(["ver", KIP_VERSION]).expect("tag"),
        ];

        let nostr_event = EventBuilder::new(Kind::from(KIND_JOIN_REQUEST as u16), content)
            .tags(tags)
            .sign_with_keys(&requester_keys)
            .expect("signed");

        let event = domain_event_from_nostr(&nostr_event);
        service.handle_incoming_event(&event).await.expect("handle");

        let broadcasts = gossip_service.broadcasts().await;
        assert!(
            broadcasts
                .iter()
                .all(|(_, event)| event.kind != KIND_KEY_ENVELOPE)
        );

        let pending = join_request_store
            .list_requests(&member_keypair.public_key)
            .await
            .expect("list");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].requester_pubkey, requester_keypair.public_key);
    }

    #[tokio::test]
    async fn handle_join_request_accepts_friend_plus_with_fof() {
        let (_member_keys, member_keypair) = make_keypair();
        let (_friend_keys, friend_keypair) = make_keypair();
        let (requester_keys, requester_keypair) = make_keypair();

        let key_manager = Arc::new(TestKeyManager::new(member_keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        user_repository
            .seed_follow(&member_keypair.public_key, &friend_keypair.public_key)
            .await;
        user_repository
            .seed_follow(&friend_keypair.public_key, &member_keypair.public_key)
            .await;
        user_repository
            .seed_follow(&friend_keypair.public_key, &requester_keypair.public_key)
            .await;
        user_repository
            .seed_follow(&requester_keypair.public_key, &friend_keypair.public_key)
            .await;
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let topic_id = "kukuri:topic1";
        let content = json!({
            "schema": SCHEMA_JOIN_REQUEST,
            "topic": topic_id,
            "scope": "friend_plus",
            "requester": format!("pubkey:{}", requester_keypair.public_key),
            "requested_at": Utc::now().timestamp(),
        })
        .to_string();
        let tags = vec![
            Tag::parse(["t", topic_id]).expect("tag"),
            Tag::parse(["scope", "friend_plus"]).expect("tag"),
            Tag::parse(["d", "join:kukuri:topic1:nonce:requester"]).expect("tag"),
            Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
            Tag::parse(["ver", KIP_VERSION]).expect("tag"),
        ];

        let nostr_event = EventBuilder::new(Kind::from(KIND_JOIN_REQUEST as u16), content)
            .tags(tags)
            .sign_with_keys(&requester_keys)
            .expect("signed");

        let event = domain_event_from_nostr(&nostr_event);
        service.handle_incoming_event(&event).await.expect("handle");

        let pending = join_request_store
            .list_requests(&member_keypair.public_key)
            .await
            .expect("list");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].scope, "friend_plus");
    }

    #[tokio::test]
    async fn friend_plus_detects_two_hop_mutual_follow() {
        let (_keys, keypair) = make_keypair();
        let key_manager = Arc::new(TestKeyManager::new(keypair));
        let group_key_store: Arc<dyn GroupKeyStore> = Arc::new(TestGroupKeyStore::default());
        let join_request_store: Arc<dyn JoinRequestStore> =
            Arc::new(TestJoinRequestStore::default());
        let user_repository = Arc::new(TestUserRepository::default());
        let user_repository_trait: Arc<dyn UserRepository> = user_repository.clone();
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service: Arc<dyn GossipService> = Arc::new(TestGossipService::default());

        let owner_pubkey = "owner_pubkey";
        let friend_pubkey = "friend_pubkey";
        let requester_pubkey = "requester_pubkey";

        user_repository
            .seed_follow(owner_pubkey, friend_pubkey)
            .await;
        user_repository
            .seed_follow(friend_pubkey, owner_pubkey)
            .await;
        user_repository
            .seed_follow(friend_pubkey, requester_pubkey)
            .await;
        user_repository
            .seed_follow(requester_pubkey, friend_pubkey)
            .await;

        let service = AccessControlService::new(
            key_manager,
            group_key_store,
            join_request_store,
            user_repository_trait,
            signature_service,
            gossip_service,
        );

        let is_fof = service
            .is_friend_of_friend(owner_pubkey, requester_pubkey)
            .await
            .expect("fof check");
        assert!(is_fof);
    }

    #[tokio::test]
    async fn friend_plus_rejects_direct_mutual_follow_only() {
        let (_keys, keypair) = make_keypair();
        let key_manager = Arc::new(TestKeyManager::new(keypair));
        let group_key_store: Arc<dyn GroupKeyStore> = Arc::new(TestGroupKeyStore::default());
        let join_request_store: Arc<dyn JoinRequestStore> =
            Arc::new(TestJoinRequestStore::default());
        let user_repository = Arc::new(TestUserRepository::default());
        let user_repository_trait: Arc<dyn UserRepository> = user_repository.clone();
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service: Arc<dyn GossipService> = Arc::new(TestGossipService::default());

        let owner_pubkey = "owner_pubkey";
        let requester_pubkey = "requester_pubkey";

        user_repository
            .seed_follow(owner_pubkey, requester_pubkey)
            .await;
        user_repository
            .seed_follow(requester_pubkey, owner_pubkey)
            .await;

        let service = AccessControlService::new(
            key_manager,
            group_key_store,
            join_request_store,
            user_repository_trait,
            signature_service,
            gossip_service,
        );

        let is_fof = service
            .is_friend_of_friend(owner_pubkey, requester_pubkey)
            .await
            .expect("fof check");
        assert!(!is_fof);
    }

    #[tokio::test]
    async fn friend_plus_rejects_when_no_mutual_friend_exists() {
        let (_keys, keypair) = make_keypair();
        let key_manager = Arc::new(TestKeyManager::new(keypair));
        let group_key_store: Arc<dyn GroupKeyStore> = Arc::new(TestGroupKeyStore::default());
        let join_request_store: Arc<dyn JoinRequestStore> =
            Arc::new(TestJoinRequestStore::default());
        let user_repository = Arc::new(TestUserRepository::default());
        let user_repository_trait: Arc<dyn UserRepository> = user_repository.clone();
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service: Arc<dyn GossipService> = Arc::new(TestGossipService::default());

        let owner_pubkey = "owner_pubkey";
        let requester_pubkey = "requester_pubkey";

        let service = AccessControlService::new(
            key_manager,
            group_key_store,
            join_request_store,
            user_repository_trait,
            signature_service,
            gossip_service,
        );

        let is_fof = service
            .is_friend_of_friend(owner_pubkey, requester_pubkey)
            .await
            .expect("fof check");
        assert!(!is_fof);
    }

    #[tokio::test]
    async fn handle_join_request_rejects_invite_reuse() {
        let (_member_keys, member_keypair) = make_keypair();
        let (requester_keys, requester_keypair) = make_keypair();
        let (requester_keys_2, requester_keypair_2) = make_keypair();

        let key_manager = Arc::new(TestKeyManager::new(member_keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let topic_id = "kukuri:topic1";
        let invite_json = service
            .issue_invite(topic_id, Some(600), Some(1), Some("nonce-reuse".into()))
            .await
            .expect("invite");
        let invite_event: nostr_sdk::Event =
            serde_json::from_value(invite_json.clone()).expect("nostr event");
        let invite_id = invite_event.id.to_string();
        let issuer_pubkey = invite_event.pubkey.to_string();

        let build_join_request = |keys: &Keys, requester_pubkey: &str, nonce: &str| {
            let content = json!({
                "schema": SCHEMA_JOIN_REQUEST,
                "topic": topic_id,
                "scope": "invite",
                "requester": format!("pubkey:{requester_pubkey}"),
                "requested_at": Utc::now().timestamp(),
                "invite_event_json": invite_json.clone(),
            })
            .to_string();

            let d_tag = format!("join:{topic_id}:{nonce}:{requester_pubkey}");
            let tags = vec![
                Tag::parse(["t", topic_id]).expect("tag"),
                Tag::parse(["scope", "invite"]).expect("tag"),
                Tag::parse(["d", d_tag.as_str()]).expect("tag"),
                Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
                Tag::parse(["ver", KIP_VERSION]).expect("tag"),
                Tag::parse(["e", invite_id.as_str()]).expect("tag"),
                Tag::parse(["p", issuer_pubkey.as_str()]).expect("tag"),
            ];

            EventBuilder::new(Kind::from(KIND_JOIN_REQUEST as u16), content)
                .tags(tags)
                .sign_with_keys(keys)
                .expect("signed")
        };

        let event_first = domain_event_from_nostr(&build_join_request(
            &requester_keys,
            &requester_keypair.public_key,
            "nonce-1",
        ));
        service
            .handle_incoming_event(&event_first)
            .await
            .expect("first join");

        let event_second = domain_event_from_nostr(&build_join_request(
            &requester_keys_2,
            &requester_keypair_2.public_key,
            "nonce-2",
        ));
        let err = service
            .handle_incoming_event(&event_second)
            .await
            .expect_err("should reject reuse");
        assert_eq!(err.validation_message(), Some("Invite max_uses exceeded"));

        let pending = join_request_store
            .list_requests(&member_keypair.public_key)
            .await
            .expect("list");
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn approve_join_request_sends_key_envelope_and_clears_pending() {
        let (_member_keys, member_keypair) = make_keypair();
        let (requester_keys, requester_keypair) = make_keypair();

        let key_manager = Arc::new(TestKeyManager::new(member_keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let topic_id = "kukuri:topic1";

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let content = json!({
            "schema": SCHEMA_JOIN_REQUEST,
            "topic": topic_id,
            "scope": "friend",
            "requester": format!("pubkey:{}", requester_keypair.public_key),
            "requested_at": Utc::now().timestamp(),
        })
        .to_string();

        let tags = vec![
            Tag::parse(["t", topic_id]).expect("tag"),
            Tag::parse(["scope", "friend"]).expect("tag"),
            Tag::parse(["d", "join:kukuri:topic1:nonce:requester"]).expect("tag"),
            Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
            Tag::parse(["ver", KIP_VERSION]).expect("tag"),
        ];

        let nostr_event = EventBuilder::new(Kind::from(KIND_JOIN_REQUEST as u16), content)
            .tags(tags)
            .sign_with_keys(&requester_keys)
            .expect("signed");

        let event = domain_event_from_nostr(&nostr_event);
        service.handle_incoming_event(&event).await.expect("handle");

        let approval = service
            .approve_join_request(&event.id)
            .await
            .expect("approve");
        assert_eq!(approval.event_id, event.id);

        let broadcasts = gossip_service.broadcasts().await;
        let expected_topic = user_topic_id(&requester_keypair.public_key);
        assert!(
            broadcasts
                .iter()
                .any(|(topic, event)| *topic == expected_topic && event.kind == KIND_KEY_ENVELOPE)
        );

        let pending = join_request_store
            .list_requests(&member_keypair.public_key)
            .await
            .expect("list");
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn handle_key_envelope_rejects_legacy_schema_name() {
        let (_recipient_keys, recipient_keypair) = make_keypair();
        let (sender_keys, sender_keypair) = make_keypair();

        let key_manager = Arc::new(TestKeyManager::new(recipient_keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let payload = json!({
            "schema": "kukuri-keyenv-v1",
            "topic": "kukuri:topic1",
            "scope": "friend",
            "epoch": 1,
            "key_b64": "aGVsbG8=",
            "issued_at": Utc::now().timestamp(),
        });
        let sender_secret = SecretKey::from_hex(&sender_keypair.private_key).expect("secret");
        let recipient_pubkey = PublicKey::from_hex(&recipient_keypair.public_key).expect("pubkey");
        let encrypted = nip44::encrypt(
            &sender_secret,
            &recipient_pubkey,
            payload.to_string(),
            nip44::Version::V2,
        )
        .expect("encrypt");
        let tags = vec![
            Tag::parse(["p", recipient_keypair.public_key.as_str()]).expect("tag"),
            Tag::parse(["t", "kukuri:topic1"]).expect("tag"),
            Tag::parse(["scope", "friend"]).expect("tag"),
            Tag::parse(["epoch", "1"]).expect("tag"),
            Tag::parse(["d", "keyenv:kukuri:topic1:friend:1:legacy"]).expect("tag"),
            Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
            Tag::parse(["ver", KIP_VERSION]).expect("tag"),
        ];
        let nostr_event = EventBuilder::new(Kind::from(KIND_KEY_ENVELOPE as u16), encrypted)
            .tags(tags)
            .sign_with_keys(&sender_keys)
            .expect("signed");

        let event = domain_event_from_nostr(&nostr_event);
        let err = service
            .handle_incoming_event(&event)
            .await
            .expect_err("should reject legacy schema");
        assert_eq!(
            err.validation_message(),
            Some("Invalid key envelope schema")
        );

        let keys = group_key_store.list_keys().await.expect("keys");
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn handle_join_request_rejects_negative_requested_at() {
        let (_member_keys, member_keypair) = make_keypair();
        let (requester_keys, requester_keypair) = make_keypair();

        let key_manager = Arc::new(TestKeyManager::new(member_keypair.clone()));
        let group_key_store = Arc::new(TestGroupKeyStore::default());
        let group_key_store_trait: Arc<dyn GroupKeyStore> = group_key_store.clone();
        let join_request_store = Arc::new(TestJoinRequestStore::default());
        let join_request_store_trait: Arc<dyn JoinRequestStore> = join_request_store.clone();
        let user_repository = Arc::new(TestUserRepository::default());
        let signature_service = Arc::new(DefaultSignatureService::new());
        let gossip_service = Arc::new(TestGossipService::default());
        let gossip_service_trait: Arc<dyn GossipService> = gossip_service.clone();

        let service = AccessControlService::new(
            key_manager,
            group_key_store_trait,
            join_request_store_trait,
            Arc::clone(&user_repository) as Arc<dyn UserRepository>,
            signature_service,
            gossip_service_trait,
        );

        let content = json!({
            "schema": SCHEMA_JOIN_REQUEST,
            "topic": "kukuri:topic1",
            "scope": "friend",
            "requester": format!("pubkey:{}", requester_keypair.public_key),
            "requested_at": -1,
        })
        .to_string();

        let tags = vec![
            Tag::parse(["t", "kukuri:topic1"]).expect("tag"),
            Tag::parse(["scope", "friend"]).expect("tag"),
            Tag::parse(["d", "join:kukuri:topic1:nonce:requester"]).expect("tag"),
            Tag::parse(["k", KIP_NAMESPACE]).expect("tag"),
            Tag::parse(["ver", KIP_VERSION]).expect("tag"),
        ];

        let nostr_event = EventBuilder::new(Kind::from(KIND_JOIN_REQUEST as u16), content)
            .tags(tags)
            .sign_with_keys(&requester_keys)
            .expect("signed");

        let event = domain_event_from_nostr(&nostr_event);
        let err = service
            .handle_incoming_event(&event)
            .await
            .expect_err("should fail");
        assert_eq!(
            err.validation_message(),
            Some("Join.request requested_at is invalid")
        );
    }
}
