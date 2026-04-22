pub(crate) use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
pub(crate) use std::sync::Arc;

pub(crate) use anyhow::{Context, Result};
pub(crate) use base64::Engine;
pub(crate) use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
pub(crate) use chrono::Utc;
pub(crate) use futures_util::StreamExt;
pub(crate) use kukuri_blob_service::{BlobService, BlobStatus, MemoryBlobService, StoredBlob};
pub(crate) use kukuri_core::{
    AssetRole, AuthorProfileDocV1, AuthorProfilePostDocV1, AuthorProfileRepostDocV1,
    CanonicalPostHeader, ChannelAudienceKind, ChannelId, ChannelRef, ChannelSharingState,
    CreatePrivateChannelInput, CustomReactionAssetDocV1, CustomReactionAssetSnapshotV1,
    DirectMessageAttachmentKind, DirectMessageAttachmentManifestV1,
    DirectMessageEncryptedAttachmentV1, DirectMessageEncryptedBlobRefV1, DirectMessageFrameV1,
    DirectMessagePayloadV1, EnvelopeId, FollowEdge, FollowEdgeDocV1, FollowEdgeStatus,
    FriendOnlyGrantPreview, FriendPlusSharePreview, GAME_MANIFEST_MIME, GameParticipant,
    GameRoomManifestBlobV1, GameRoomStateDocV1, GameRoomStatus, GameScoreEntry, GossipHint,
    HintObjectRef, KukuriEnvelope, KukuriKeys, KukuriMediaManifestV1,
    KukuriProfileEnvelopeContentV1, KukuriProfilePostEnvelopeContentV1,
    KukuriProfileRepostEnvelopeContentV1, LIVE_MANIFEST_MIME, LiveSessionManifestBlobV1,
    LiveSessionStateDocV1, LiveSessionStatus, ManifestBlobRef, MediaManifestItem, ObjectStatus,
    ObjectVisibility, PayloadRef, PrivateChannelEpochHandoffGrantDocV1,
    PrivateChannelEpochHandoffGrantPayloadV1, PrivateChannelInvitePreview,
    PrivateChannelInviteTokenParams, PrivateChannelJoinMode, PrivateChannelMetadataDocV1,
    PrivateChannelParticipantDocV1, PrivateChannelPolicyDocV1, Profile, ProfilePost, ProfileRepost,
    Pubkey, ReactionDocV1, ReactionKeyKind, ReactionKeyV1, ReplicaId, RepostSourceSnapshotV1,
    TimelineScope, TopicId, author_profile_topic_id, build_custom_reaction_asset_envelope,
    build_direct_message_ack, build_follow_edge_envelope, build_friend_only_grant_token,
    build_friend_plus_share_token, build_game_session_envelope, build_live_session_envelope,
    build_media_manifest_envelope, build_post_envelope_with_payload_in_channel,
    build_private_channel_epoch_handoff_grant_envelope, build_private_channel_invite_token,
    build_private_channel_participant_envelope, build_private_channel_policy_envelope,
    build_profile_envelope, build_profile_post_envelope, build_profile_repost_envelope,
    build_reaction_envelope, build_repost_envelope, decrypt_direct_message_attachment,
    decrypt_direct_message_frame, decrypt_private_channel_epoch_handoff_grant,
    derive_direct_message_topic, deterministic_reaction_id, direct_message_id_for_participants,
    encrypt_direct_message_attachment, encrypt_direct_message_frame,
    encrypt_private_channel_epoch_handoff_grant, generate_keys, parse_custom_reaction_asset,
    parse_follow_edge, parse_friend_only_grant_token, parse_friend_plus_share_token,
    parse_private_channel_epoch_handoff_grant, parse_private_channel_invite_token,
    parse_private_channel_participant, parse_private_channel_policy, parse_profile,
    parse_profile_post, parse_profile_repost, parse_reaction, timeline_sort_key,
};
pub(crate) use kukuri_docs_sync::{
    DocEvent, DocOp, DocQuery, DocRecord, DocsSync, MemoryDocsSync, author_replica_id,
    private_channel_epoch_replica_id, private_channel_hint_topic, private_channel_replica_id,
    stable_key, topic_replica_id,
};
pub(crate) use kukuri_store::{
    AuthorRelationshipProjectionRow, BlobCacheStatus, BookmarkedCustomReactionRow,
    BookmarkedPostRow, DirectMessageConversationRow, DirectMessageMessageRow,
    DirectMessageOutboxRow, DirectMessageTombstoneRow, GameRoomProjectionRow,
    LiveSessionProjectionRow, MutedAuthorRow, NotificationKind, NotificationRow,
    ObjectProjectionRow, Page, ProjectionStore, ReactionProjectionRow, Store, TimelineCursor,
};
pub(crate) use kukuri_transport::{
    DiscoveryMode, DiscoverySnapshot, HintTransport, PeerSnapshot, SeedPeer, TopicPeerSnapshot,
    Transport,
};
pub(crate) use serde::{Serialize, de::DeserializeOwned};
pub(crate) use tokio::sync::Mutex;
pub(crate) use tokio::task::JoinHandle;
pub(crate) use tracing::{info, warn};

pub(crate) const REPLICA_SYNC_RESTART_RETRY_SECONDS: i64 = 5;
pub(crate) const DIRECT_MESSAGE_SUBSCRIPTION_RESTART_RETRY_SECONDS: i64 = 5;
pub(crate) const PUBLIC_TOPIC_RECOVERY_GRACE_MS: i64 = 3_000;
pub(crate) const PUBLIC_TOPIC_RECOVERY_BACKOFF_MS: [i64; 3] = [3_000, 10_000, 30_000];
pub(crate) const PUBLIC_CHANNEL_ID: &str = "public";
pub(crate) const DIRECT_MESSAGE_FRAME_MIME: &str =
    "application/vnd.kukuri.direct-message-frame+json";
pub(crate) const DIRECT_MESSAGE_ATTACHMENT_MIME: &str =
    "application/vnd.kukuri.direct-message-attachment+json";
pub(crate) const DIRECT_MESSAGE_RETRY_INTERVAL_MS: u64 = 2_000;
pub(crate) const NOTIFICATION_PREVIEW_LIMIT: usize = 80;

pub(crate) use crate::views::*;

mod attachment_support;
mod direct_messages_delivery_support;
mod direct_messages_subscription_support;
mod hydration_support;
mod live_game_support;
mod notifications_support;
mod object_persistence_support;
mod private_channels_support;
mod profile_docs_support;
mod projection_support;
mod social_helpers;
mod social_runtime_support;
mod timeline_runtime_support;

pub(crate) use attachment_support::*;
pub(crate) use hydration_support::*;
pub(crate) use notifications_support::*;
pub(crate) use object_persistence_support::*;
pub(crate) use profile_docs_support::*;
pub(crate) use projection_support::*;
pub(crate) use social_helpers::*;
pub(crate) use timeline_runtime_support::*;

pub(crate) async fn maybe_restart_replica_sync_with_cooldown(
    docs_sync: &dyn DocsSync,
    deadlines: &Arc<Mutex<HashMap<String, i64>>>,
    topic_id: &str,
    replica: &ReplicaId,
) {
    let key = replica.as_str().to_string();
    let now = Utc::now().timestamp();
    {
        let mut guard = deadlines.lock().await;
        let next_due_at = guard.get(key.as_str()).copied().unwrap_or_default();
        if next_due_at > now {
            return;
        }
        guard.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
    }
    if let Err(error) = docs_sync.restart_replica_sync(replica).await {
        warn!(
            topic = %topic_id,
            replica = %replica.as_str(),
            error = %error,
            "failed to restart replica sync"
        );
    }
}

pub(crate) async fn record_public_topic_docs_activity_if_current(
    delivery: &Arc<Mutex<HashMap<String, PublicTopicDeliveryStatus>>>,
    topic_id: &str,
    generation: u64,
    at_ms: i64,
) {
    let mut guard = delivery.lock().await;
    if let Some(entry) = guard.get_mut(topic_id)
        && entry.generation == generation
    {
        entry.last_docs_activity_at = Some(at_ms);
    }
}

pub(crate) async fn restart_replica_sync_with_backoff(
    docs_sync: &dyn DocsSync,
    topic_id: &str,
    replica: &ReplicaId,
    backoff: &mut SubscriptionRecoveryBackoff,
) {
    let now_ms = Utc::now().timestamp_millis();
    if !backoff.ready(now_ms) {
        return;
    }
    if let Err(error) = docs_sync.restart_replica_sync(replica).await {
        warn!(
            topic = %topic_id,
            replica = %replica.as_str(),
            error = %error,
            "failed to restart replica sync"
        );
    }
    backoff.schedule(now_ms);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProfileTimelineItem {
    Post(ProfilePost),
    Repost(ProfileRepost),
}

impl ProfileTimelineItem {
    pub(crate) fn created_at(&self) -> i64 {
        match self {
            Self::Post(post) => post.created_at,
            Self::Repost(repost) => repost.created_at,
        }
    }

    pub(crate) fn object_id(&self) -> &EnvelopeId {
        match self {
            Self::Post(post) => &post.object_id,
            Self::Repost(repost) => &repost.object_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedRepostSource {
    pub(crate) repost_of: RepostSourceSnapshotV1,
}

pub struct AppService {
    pub(crate) store: Arc<dyn Store>,
    pub(crate) projection_store: Arc<dyn ProjectionStore>,
    pub(crate) transport: Arc<dyn Transport>,
    pub(crate) hint_transport: Arc<dyn HintTransport>,
    pub(crate) docs_sync: Arc<dyn DocsSync>,
    pub(crate) blob_service: Arc<dyn BlobService>,
    pub(crate) keys: Arc<KukuriKeys>,
    pub(crate) subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub(crate) direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub(crate) private_channel_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub(crate) author_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub(crate) joined_private_channels: Arc<Mutex<HashMap<String, JoinedPrivateChannelState>>>,
    pub(crate) live_presence_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    pub(crate) last_sync_ts: Arc<Mutex<Option<i64>>>,
    pub(crate) subscription_generations: Arc<Mutex<HashMap<String, u64>>>,
    pub(crate) public_topic_delivery: Arc<Mutex<HashMap<String, PublicTopicDeliveryStatus>>>,
    pub(crate) direct_message_subscription_restart_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    pub(crate) replica_sync_restart_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    pub(crate) empty_recovery_candidates: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PublicTopicDeliveryStatus {
    pub(crate) generation: u64,
    pub(crate) last_docs_activity_at: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SubscriptionRecoveryBackoff {
    pub(crate) next_retry_at_ms: i64,
    pub(crate) step: usize,
}

impl SubscriptionRecoveryBackoff {
    pub(crate) fn reset(&mut self) {
        self.next_retry_at_ms = 0;
        self.step = 0;
    }

    pub(crate) fn ready(&self, now_ms: i64) -> bool {
        self.next_retry_at_ms <= now_ms
    }

    pub(crate) fn schedule(&mut self, now_ms: i64) {
        let delay_ms = PUBLIC_TOPIC_RECOVERY_BACKOFF_MS[self
            .step
            .min(PUBLIC_TOPIC_RECOVERY_BACKOFF_MS.len().saturating_sub(1))];
        self.next_retry_at_ms = now_ms.saturating_add(delay_ms);
        if self.step + 1 < PUBLIC_TOPIC_RECOVERY_BACKOFF_MS.len() {
            self.step += 1;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JoinedPrivateChannelState {
    pub(crate) topic_id: String,
    pub(crate) channel_id: ChannelId,
    pub(crate) label: String,
    pub(crate) creator_pubkey: String,
    pub(crate) owner_pubkey: String,
    pub(crate) joined_via_pubkey: Option<String>,
    pub(crate) audience_kind: ChannelAudienceKind,
    pub(crate) current_epoch_id: String,
    pub(crate) current_epoch_secret_hex: String,
    pub(crate) archived_epochs: Vec<PrivateChannelEpochCapability>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrivateChannelDiagnostics {
    pub(crate) sharing_state: ChannelSharingState,
    pub(crate) participant_count: usize,
    pub(crate) stale_participant_count: usize,
    pub(crate) rotation_required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrivateChannelOwnerAction {
    Write,
    Share,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NotificationCandidate {
    pub(crate) kind: NotificationKind,
    pub(crate) actor_pubkey: String,
    pub(crate) source_envelope_id: Option<EnvelopeId>,
    pub(crate) source_replica_id: Option<ReplicaId>,
    pub(crate) topic_id: Option<String>,
    pub(crate) channel_id: Option<String>,
    pub(crate) object_id: Option<EnvelopeId>,
    pub(crate) dm_id: Option<String>,
    pub(crate) message_id: Option<String>,
    pub(crate) preview_text: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) received_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NotificationDocEventBaseline {
    fingerprints: BTreeSet<String>,
}

impl NotificationDocEventBaseline {
    pub(crate) fn from_records(records: &[DocRecord]) -> Self {
        Self {
            fingerprints: records
                .iter()
                .map(|record| {
                    notification_doc_event_fingerprint_parts(&record.key, &record.content_hash)
                })
                .collect(),
        }
    }

    pub(crate) fn contains(&self, event: &DocEvent) -> bool {
        self.fingerprints
            .contains(notification_doc_event_fingerprint(event).as_str())
    }
}

impl AppService {
    pub fn new<S, T>(store: Arc<S>, transport: Arc<T>) -> Self
    where
        S: Store + ProjectionStore + 'static,
        T: Transport + HintTransport + 'static,
    {
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        Self::new_with_services(
            store.clone() as Arc<dyn Store>,
            store as Arc<dyn ProjectionStore>,
            transport.clone(),
            transport as Arc<dyn HintTransport>,
            docs_sync,
            blob_service,
            generate_keys(),
        )
    }

    pub fn new_with_services(
        store: Arc<dyn Store>,
        projection_store: Arc<dyn ProjectionStore>,
        transport: Arc<dyn Transport>,
        hint_transport: Arc<dyn HintTransport>,
        docs_sync: Arc<dyn DocsSync>,
        blob_service: Arc<dyn BlobService>,
        keys: KukuriKeys,
    ) -> Self {
        Self {
            store,
            transport,
            projection_store,
            hint_transport,
            docs_sync,
            blob_service,
            keys: Arc::new(keys),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            direct_message_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            private_channel_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            author_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            joined_private_channels: Arc::new(Mutex::new(HashMap::new())),
            live_presence_tasks: Arc::new(Mutex::new(HashMap::new())),
            last_sync_ts: Arc::new(Mutex::new(None)),
            subscription_generations: Arc::new(Mutex::new(HashMap::new())),
            public_topic_delivery: Arc::new(Mutex::new(HashMap::new())),
            direct_message_subscription_restart_deadlines: Arc::new(Mutex::new(HashMap::new())),
            replica_sync_restart_deadlines: Arc::new(Mutex::new(HashMap::new())),
            empty_recovery_candidates: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub(crate) async fn resolve_repost_source(
        &self,
        source_topic_id: &str,
        source_object_id: &str,
    ) -> Result<ResolvedRepostSource> {
        let source_object_id = EnvelopeId::from(source_object_id);
        if ProjectionStore::get_object_projection(self.projection_store.as_ref(), &source_object_id)
            .await?
            .is_none()
        {
            let _ = hydrate_topic_state_with_services(
                self.docs_sync.as_ref(),
                self.blob_service.as_ref(),
                self.projection_store.as_ref(),
                source_topic_id,
            )
            .await?;
        }
        let projection = ProjectionStore::get_object_projection(
            self.projection_store.as_ref(),
            &source_object_id,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("repost source object not found"))?;
        if projection.topic_id != source_topic_id {
            anyhow::bail!("repost source topic does not match");
        }
        if projection.channel_id != PUBLIC_CHANNEL_ID {
            anyhow::bail!("only public posts and comments can be reposted");
        }
        if !matches!(projection.object_kind.as_str(), "post" | "comment") {
            anyhow::bail!("only public posts and comments can be reposted");
        }

        let header = fetch_post_object_for_projection(
            self.docs_sync.as_ref(),
            &projection.source_replica_id,
            projection.source_key.as_str(),
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("repost source header not found"))?;
        let content = match &projection.payload_ref {
            PayloadRef::InlineText { text } => text.clone(),
            PayloadRef::BlobText { hash, .. } => {
                fetch_projection_blob_text(self.blob_service.as_ref(), hash)
                    .await
                    .ok_or_else(|| anyhow::anyhow!("repost source content is unavailable"))?
            }
        };
        Ok(ResolvedRepostSource {
            repost_of: RepostSourceSnapshotV1 {
                source_object_id: header.object_id,
                source_topic_id: header.topic_id,
                source_author_pubkey: header.author,
                source_object_kind: header.object_kind,
                content,
                attachments: header.attachments,
                reply_to_object_id: header.reply_to,
                root_id: header.root,
            },
        })
    }

    pub(crate) async fn find_existing_simple_repost(
        &self,
        target_topic_id: &str,
        source_object_id: &str,
        commentary: Option<&str>,
    ) -> Result<Option<String>> {
        if commentary.is_some() {
            return Ok(None);
        }
        let target_replica = topic_replica_id(target_topic_id);
        let local_author_pubkey = self.current_author_pubkey();
        for record in self
            .docs_sync
            .query_replica(&target_replica, DocQuery::Prefix("objects/".into()))
            .await?
        {
            if !record.key.ends_with("/state") {
                continue;
            }
            let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
            if header.object_kind != "repost"
                || header.author.as_str() != local_author_pubkey
                || header.channel_id.is_some()
            {
                continue;
            }
            let Some(repost_of) = header.repost_of.as_ref() else {
                continue;
            };
            if repost_of.source_object_id.as_str() != source_object_id {
                continue;
            }
            let commentary = match &header.payload_ref {
                PayloadRef::InlineText { text } => normalize_repost_commentary(Some(text.clone())),
                PayloadRef::BlobText { .. } => None,
            };
            if commentary.is_none() {
                return Ok(Some(header.object_id.as_str().to_string()));
            }
        }
        Ok(None)
    }

    pub(crate) async fn docs_assisted_peer_ids(&self) -> Result<Vec<String>> {
        self.docs_sync.assist_peer_ids().await
    }

    pub(crate) async fn blob_assisted_peer_ids(&self) -> Result<Vec<String>> {
        self.blob_service.assist_peer_ids().await
    }

    pub(crate) async fn next_subscription_generation(&self, key: &str) -> u64 {
        let mut generations = self.subscription_generations.lock().await;
        let generation = generations
            .get(key)
            .copied()
            .unwrap_or_default()
            .saturating_add(1);
        generations.insert(key.to_string(), generation);
        generation
    }

    pub(crate) async fn reset_public_topic_delivery_generation(
        &self,
        topic_id: &str,
        generation: u64,
    ) {
        self.public_topic_delivery.lock().await.insert(
            topic_id.to_string(),
            PublicTopicDeliveryStatus {
                generation,
                last_docs_activity_at: None,
            },
        );
    }

    pub(crate) async fn clear_public_topic_delivery(&self, topic_id: &str) {
        self.public_topic_delivery.lock().await.remove(topic_id);
    }

    pub(crate) async fn public_topic_delivery_status(
        &self,
        topic_id: &str,
    ) -> Option<PublicTopicDeliveryStatus> {
        self.public_topic_delivery
            .lock()
            .await
            .get(topic_id)
            .copied()
    }

    pub(crate) async fn restart_active_subscriptions(&self) -> Result<()> {
        let topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }

        let private_channels = self
            .joined_private_channels
            .lock()
            .await
            .values()
            .map(|state| {
                (
                    state.topic_id.clone(),
                    state.channel_id.as_str().to_string(),
                )
            })
            .collect::<Vec<_>>();
        for (topic_id, channel_id) in private_channels {
            self.restart_private_channel_subscription(topic_id.as_str(), channel_id.as_str())
                .await?;
        }

        let authors = self
            .author_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for author in authors {
            self.restart_author_subscription(author.as_str()).await?;
        }
        self.restart_direct_message_subscriptions().await?;
        Ok(())
    }

    pub async fn shutdown(&self) {
        let topics_to_unsubscribe = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let private_channels_to_unsubscribe = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter_map(|key| key.split("::").nth(1).map(str::to_owned))
            .collect::<BTreeSet<_>>();
        let handles = {
            let mut subscriptions = self.subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        let private_handles = {
            let mut subscriptions = self.private_channel_subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in private_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        for channel_id in private_channels_to_unsubscribe {
            let _ = self
                .hint_transport
                .unsubscribe_hints(&private_channel_hint_topic(channel_id.as_str()))
                .await;
        }
        for topic_id in topics_to_unsubscribe {
            let _ = self
                .hint_transport
                .unsubscribe_hints(&TopicId::new(topic_id))
                .await;
        }
        let dm_peers_to_unsubscribe = self
            .direct_message_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let dm_handles = {
            let mut subscriptions = self.direct_message_subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in dm_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        for peer_pubkey in dm_peers_to_unsubscribe {
            if let Ok(topic) =
                derive_direct_message_topic(self.keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))
            {
                let _ = self.hint_transport.unsubscribe_hints(&topic).await;
            }
        }
        let author_handles = {
            let mut subscriptions = self.author_subscriptions.lock().await;
            subscriptions
                .drain()
                .map(|(_, handle)| handle)
                .collect::<Vec<_>>()
        };
        for handle in author_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        let presence_handles = {
            let mut tasks = self.live_presence_tasks.lock().await;
            tasks.drain().map(|(_, handle)| handle).collect::<Vec<_>>()
        };
        for handle in presence_handles {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
    }

    pub(crate) fn current_author_pubkey(&self) -> String {
        self.keys.public_key_hex()
    }

    pub(crate) async fn reaction_state_for_target(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
    ) -> Result<ReactionStateView> {
        let rows = self
            .projection_store
            .list_reaction_cache_for_target(source_replica_id, target_object_id)
            .await?;
        Ok(reaction_state_view_from_rows(
            source_replica_id,
            target_object_id,
            rows,
            self.current_author_pubkey().as_str(),
        ))
    }
}

#[cfg(test)]
#[path = "../tests/mod.rs"]
mod tests;
