pub(crate) use std::collections::{BTreeMap, BTreeSet, HashMap};
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
pub(crate) const PUBLIC_CHANNEL_ID: &str = "public";
pub(crate) const DIRECT_MESSAGE_FRAME_MIME: &str =
    "application/vnd.kukuri.direct-message-frame+json";
pub(crate) const DIRECT_MESSAGE_ATTACHMENT_MIME: &str =
    "application/vnd.kukuri.direct-message-attachment+json";
pub(crate) const DIRECT_MESSAGE_RETRY_INTERVAL_MS: u64 = 2_000;
pub(crate) const NOTIFICATION_PREVIEW_LIMIT: usize = 80;

pub(crate) use crate::views::*;

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
    pub(crate) direct_message_subscription_restart_deadlines: Arc<Mutex<HashMap<String, i64>>>,
    pub(crate) replica_sync_restart_deadlines: Arc<Mutex<HashMap<String, i64>>>,
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
            direct_message_subscription_restart_deadlines: Arc::new(Mutex::new(HashMap::new())),
            replica_sync_restart_deadlines: Arc::new(Mutex::new(HashMap::new())),
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

    pub(crate) async fn assisted_peer_ids(&self) -> Result<Vec<String>> {
        Ok(merge_peer_ids(
            self.docs_sync.assist_peer_ids().await?,
            self.blob_service.assist_peer_ids().await?,
        ))
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

    pub(crate) async fn maybe_redeem_rotation_grants_for_scope(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<()> {
        match scope {
            TimelineScope::Public => Ok(()),
            TimelineScope::AllJoined => self.maybe_redeem_rotation_grants_for_topic(topic_id).await,
            TimelineScope::Channel { channel_id } => self
                .maybe_redeem_rotation_grants_for_channel(topic_id, channel_id.as_str())
                .await
                .map(|_| ()),
        }
    }

    pub(crate) async fn maybe_redeem_rotation_grants_for_topic(
        &self,
        topic_id: &str,
    ) -> Result<()> {
        for state in self.joined_private_channel_states_for_topic(topic_id).await {
            self.maybe_redeem_rotation_grants_for_channel(topic_id, state.channel_id.as_str())
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn maybe_redeem_rotation_grants_for_channel(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<bool> {
        let mut redeemed_any = false;
        loop {
            let Some(state) = self
                .joined_private_channel_state(topic_id, channel_id)
                .await
            else {
                return Ok(redeemed_any);
            };
            let local_author = self.current_author_pubkey();
            let replica = current_private_channel_replica_id(&state);
            let grant_doc = fetch_private_channel_rotation_grant_from_replica(
                self.docs_sync.as_ref(),
                &replica,
                local_author.as_str(),
            )
            .await?;
            let grant_doc = if let Some(grant_doc) = grant_doc {
                Some(grant_doc)
            } else {
                if let Err(error) = self.docs_sync.restart_replica_sync(&replica).await {
                    warn!(
                        topic = %topic_id,
                        channel_id = %channel_id,
                        epoch_id = %state.current_epoch_id,
                        error = %error,
                        "failed to restart private channel replica sync while polling epoch handoff"
                    );
                }
                fetch_private_channel_rotation_grant_from_replica(
                    self.docs_sync.as_ref(),
                    &replica,
                    local_author.as_str(),
                )
                .await?
            };
            let Some(grant_doc) = grant_doc else {
                return Ok(redeemed_any);
            };
            let payload =
                match decrypt_private_channel_epoch_handoff_grant(self.keys.as_ref(), &grant_doc) {
                    Ok(payload) => payload,
                    Err(error) => {
                        warn!(
                            topic = %topic_id,
                            channel_id = %channel_id,
                            epoch_id = %state.current_epoch_id,
                            error = %error,
                            "failed to decrypt private channel epoch handoff grant"
                        );
                        return Ok(redeemed_any);
                    }
                };
            if payload.old_epoch_id != state.current_epoch_id
                || private_channel_epoch_capabilities(&state)
                    .iter()
                    .any(|known_epoch| known_epoch.epoch_id == payload.new_epoch_id)
            {
                return Ok(redeemed_any);
            }
            let next_replica =
                private_channel_epoch_replica_id(channel_id, payload.new_epoch_id.as_str());
            self.docs_sync
                .register_private_replica_secret(
                    &next_replica,
                    payload.new_namespace_secret_hex.as_str(),
                )
                .await?;
            if let Err(error) = self.docs_sync.restart_replica_sync(&next_replica).await {
                warn!(
                    topic = %topic_id,
                    channel_id = %channel_id,
                    epoch_id = %payload.new_epoch_id,
                    error = %error,
                    "failed to restart rotated private channel replica sync"
                );
            }
            let (metadata, policy, participants) = match wait_for_private_channel_epoch_snapshot(
                self.docs_sync.as_ref(),
                &next_replica,
                "private channel epoch handoff sync",
            )
            .await
            {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    warn!(
                        topic = %topic_id,
                        channel_id = %channel_id,
                        epoch_id = %payload.new_epoch_id,
                        error = %error,
                        "failed to load rotated private channel replica"
                    );
                    return Ok(redeemed_any);
                }
            };
            if policy.audience_kind != state.audience_kind
                || policy.epoch_id != payload.new_epoch_id
                || policy.previous_epoch_id.as_deref() != Some(payload.old_epoch_id.as_str())
            {
                warn!(
                    topic = %topic_id,
                    channel_id = %channel_id,
                    epoch_id = %payload.new_epoch_id,
                    audience_kind = ?policy.audience_kind,
                    "private channel epoch handoff payload does not match rotated policy"
                );
                return Ok(redeemed_any);
            }
            let local_pubkey = Pubkey::from(local_author.clone());
            if !participants.iter().any(|participant| {
                participant.participant_pubkey == local_pubkey
                    && participant.epoch_id == policy.epoch_id
            }) {
                persist_private_channel_participant(
                    self.docs_sync.as_ref(),
                    self.keys.as_ref(),
                    &PrivateChannelParticipantDocV1 {
                        channel_id: metadata.channel_id.clone(),
                        topic_id: metadata.topic_id.clone(),
                        epoch_id: policy.epoch_id.clone(),
                        participant_pubkey: local_pubkey,
                        joined_at: Utc::now().timestamp_millis(),
                        is_owner: false,
                        join_mode: Some(PrivateChannelJoinMode::RotationRedeem),
                        sponsor_pubkey: Some(policy.owner_pubkey.clone()),
                        share_token_id: None,
                    },
                    &next_replica,
                )
                .await?;
            }
            let next_state = merged_private_channel_state_from_epoch_join(
                Some(state.clone()),
                metadata.topic_id.as_str(),
                metadata.channel_id.clone(),
                metadata.label.as_str(),
                metadata.creator_pubkey.as_str(),
                policy.owner_pubkey.as_str(),
                state.joined_via_pubkey.as_deref(),
                policy.audience_kind.clone(),
                payload.new_epoch_id.as_str(),
                payload.new_namespace_secret_hex.as_str(),
            );
            self.register_joined_private_channel(next_state).await?;
            redeemed_any = true;
        }
    }

    pub(crate) async fn private_channel_diagnostics(
        &self,
        state: &JoinedPrivateChannelState,
    ) -> Result<PrivateChannelDiagnostics> {
        let replica = current_private_channel_replica_id(state);
        let sharing_state =
            fetch_private_channel_policy_from_replica(self.docs_sync.as_ref(), &replica)
                .await?
                .map(|policy| policy.sharing_state)
                .unwrap_or(ChannelSharingState::Open);
        let participants =
            fetch_private_channel_participants_from_replica(self.docs_sync.as_ref(), &replica)
                .await?;
        let participant_count = participants.len();
        let mut stale_participant_count = 0usize;
        if state.audience_kind == ChannelAudienceKind::FriendOnly
            && state.owner_pubkey == self.current_author_pubkey()
        {
            for participant in &participants {
                if participant.is_owner {
                    continue;
                }
                self.ensure_author_subscription(participant.participant_pubkey.as_str())
                    .await?;
                let relationship = self
                    .projection_store
                    .get_author_relationship(
                        self.current_author_pubkey().as_str(),
                        participant.participant_pubkey.as_str(),
                    )
                    .await?;
                if relationship.as_ref().is_some_and(|value| !value.mutual) {
                    stale_participant_count += 1;
                }
            }
        }
        Ok(PrivateChannelDiagnostics {
            sharing_state,
            participant_count,
            stale_participant_count,
            rotation_required: state.audience_kind == ChannelAudienceKind::FriendOnly
                && stale_participant_count > 0,
        })
    }

    pub(crate) async fn joined_private_channel_view_for_state(
        &self,
        state: &JoinedPrivateChannelState,
    ) -> Result<JoinedPrivateChannelView> {
        let diagnostics = self.private_channel_diagnostics(state).await?;
        Ok(JoinedPrivateChannelView {
            topic_id: state.topic_id.clone(),
            channel_id: state.channel_id.as_str().to_string(),
            label: state.label.clone(),
            creator_pubkey: state.creator_pubkey.clone(),
            owner_pubkey: state.owner_pubkey.clone(),
            joined_via_pubkey: state.joined_via_pubkey.clone(),
            audience_kind: state.audience_kind.clone(),
            is_owner: state.owner_pubkey == self.current_author_pubkey(),
            current_epoch_id: state.current_epoch_id.clone(),
            archived_epoch_ids: state
                .archived_epochs
                .iter()
                .map(|epoch| epoch.epoch_id.clone())
                .collect(),
            sharing_state: diagnostics.sharing_state,
            rotation_required: diagnostics.rotation_required,
            participant_count: diagnostics.participant_count,
            stale_participant_count: diagnostics.stale_participant_count,
        })
    }

    pub(crate) async fn private_channel_capability_from_state(
        &self,
        state: &JoinedPrivateChannelState,
    ) -> Result<PrivateChannelCapability> {
        let diagnostics = self.private_channel_diagnostics(state).await?;
        Ok(PrivateChannelCapability {
            topic_id: state.topic_id.clone(),
            channel_id: state.channel_id.as_str().to_string(),
            label: state.label.clone(),
            creator_pubkey: state.creator_pubkey.clone(),
            owner_pubkey: state.owner_pubkey.clone(),
            joined_via_pubkey: state.joined_via_pubkey.clone(),
            audience_kind: state.audience_kind.clone(),
            current_epoch_id: state.current_epoch_id.clone(),
            current_epoch_secret_hex: state.current_epoch_secret_hex.clone(),
            archived_epochs: state.archived_epochs.clone(),
            rotation_required: diagnostics.rotation_required,
            participant_count: diagnostics.participant_count,
            stale_participant_count: diagnostics.stale_participant_count,
            namespace_secret_hex: state.current_epoch_secret_hex.clone(),
        })
    }

    pub(crate) async fn audience_label_for_storage(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> String {
        if channel_id == PUBLIC_CHANNEL_ID {
            return "Public".to_string();
        }
        self.joined_private_channels
            .lock()
            .await
            .get(joined_private_channel_key(topic_id, channel_id).as_str())
            .map(|channel| channel.label.clone())
            .unwrap_or_else(|| "Private channel".to_string())
    }

    pub(crate) async fn joined_private_channel_states_for_topic(
        &self,
        topic_id: &str,
    ) -> Vec<JoinedPrivateChannelState> {
        self.joined_private_channels
            .lock()
            .await
            .values()
            .filter(|state| state.topic_id == topic_id)
            .cloned()
            .collect()
    }

    pub(crate) async fn joined_private_channel_state(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Option<JoinedPrivateChannelState> {
        self.joined_private_channels
            .lock()
            .await
            .get(joined_private_channel_key(topic_id, channel_id).as_str())
            .cloned()
    }

    pub(crate) async fn ensure_private_channel_access(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
    ) -> Result<()> {
        if self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
            .is_none()
        {
            anyhow::bail!("private channel is not joined");
        }
        Ok(())
    }

    pub(crate) async fn maybe_auto_rotate_private_channel_for_owner(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
        action: PrivateChannelOwnerAction,
    ) -> Result<()> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        if state.owner_pubkey != self.current_author_pubkey() {
            return Ok(());
        }
        match state.audience_kind {
            ChannelAudienceKind::InviteOnly | ChannelAudienceKind::FriendPlus => {
                if matches!(
                    action,
                    PrivateChannelOwnerAction::Write | PrivateChannelOwnerAction::Share
                ) {
                    let _ = self
                        .rotate_private_channel(topic_id, channel_id.as_str())
                        .await?;
                }
            }
            ChannelAudienceKind::FriendOnly => {
                let diagnostics = self.private_channel_diagnostics(&state).await?;
                if diagnostics.rotation_required {
                    let _ = self
                        .rotate_private_channel(topic_id, channel_id.as_str())
                        .await?;
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn private_channel_state_for_owner_action(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
        action: PrivateChannelOwnerAction,
    ) -> Result<JoinedPrivateChannelState> {
        self.maybe_redeem_rotation_grants_for_channel(topic_id, channel_id.as_str())
            .await?;
        self.ensure_private_channel_access(topic_id, channel_id)
            .await?;
        self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
            .await?;
        self.maybe_auto_rotate_private_channel_for_owner(topic_id, channel_id, action)
            .await?;
        self.maybe_redeem_rotation_grants_for_channel(topic_id, channel_id.as_str())
            .await?;
        self.ensure_private_channel_access(topic_id, channel_id)
            .await?;
        self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
            .await?;
        let state = self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
            .ok_or_else(|| anyhow::anyhow!("private channel is not joined"))?;
        if private_channel_rotation_is_pending(self.docs_sync.as_ref(), self.keys.as_ref(), &state)
            .await?
        {
            anyhow::bail!(
                "private channel epoch handoff is pending; wait for automatic redemption or use a fresh access token"
            );
        }
        Ok(state)
    }

    pub(crate) async fn private_channel_write_state(
        &self,
        topic_id: &str,
        channel_id: &ChannelId,
    ) -> Result<JoinedPrivateChannelState> {
        self.private_channel_state_for_owner_action(
            topic_id,
            channel_id,
            PrivateChannelOwnerAction::Write,
        )
        .await
    }

    pub(crate) async fn register_joined_private_channel(
        &self,
        state: JoinedPrivateChannelState,
    ) -> Result<()> {
        register_private_channel_replica_secrets(self.docs_sync.as_ref(), &state).await?;
        self.joined_private_channels.lock().await.insert(
            joined_private_channel_key(state.topic_id.as_str(), state.channel_id.as_str()),
            state.clone(),
        );
        self.ensure_private_channel_subscription(
            state.topic_id.as_str(),
            state.channel_id.as_str(),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn ensure_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        self.spawn_private_channel_subscription(state).await
    }

    pub(crate) async fn ensure_joined_private_channel_subscriptions(
        &self,
        topic_id: &str,
    ) -> Result<()> {
        for state in self.joined_private_channel_states_for_topic(topic_id).await {
            self.ensure_private_channel_subscription(topic_id, state.channel_id.as_str())
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn restart_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let prefix = joined_private_channel_subscription_prefix(topic_id, channel_id);
        let keys = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(prefix.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            if let Some(handle) = self
                .private_channel_subscriptions
                .lock()
                .await
                .remove(key.as_str())
            {
                handle.abort();
            }
        }
        self.hint_transport
            .unsubscribe_hints(&private_channel_hint_topic(channel_id))
            .await?;
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            return Ok(());
        };
        self.spawn_private_channel_subscription(state).await
    }

    pub(crate) async fn spawn_private_channel_subscription(
        &self,
        state: JoinedPrivateChannelState,
    ) -> Result<()> {
        let docs_sync = Arc::clone(&self.docs_sync);
        for epoch in private_channel_epoch_capabilities(&state) {
            let replica = private_channel_replica_for_epoch(
                state.channel_id.as_str(),
                epoch.epoch_id.as_str(),
            );
            let key = joined_private_channel_subscription_key(
                state.topic_id.as_str(),
                state.channel_id.as_str(),
                &replica,
            );
            if self
                .private_channel_subscriptions
                .lock()
                .await
                .contains_key(key.as_str())
            {
                continue;
            }
            docs_sync
                .register_private_replica_secret(&replica, epoch.namespace_secret_hex.as_str())
                .await?;
            self.spawn_subscription_task(
                state.topic_id.as_str(),
                Some(state.channel_id.clone()),
                replica,
                private_channel_hint_topic(state.channel_id.as_str()),
                Some(key),
            )
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn spawn_subscription_task(
        &self,
        topic_id: &str,
        channel_id: Option<ChannelId>,
        replica: ReplicaId,
        hint_topic: TopicId,
        private_key: Option<String>,
    ) -> Result<()> {
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let blob_service = Arc::clone(&self.blob_service);
        let hint_transport = Arc::clone(&self.hint_transport);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let topic = topic_id.to_string();
        let storage_channel_id = channel_storage_id(channel_id.as_ref());
        let local_author_pubkey = self.current_author_pubkey();
        docs_sync.open_replica(&replica).await?;
        let notification_baseline =
            snapshot_object_notification_baseline(docs_sync.as_ref(), &replica).await?;
        let mut doc_stream = docs_sync.subscribe_replica(&replica).await?;
        let mut hint_stream = hint_transport.subscribe_hints(&hint_topic).await?;
        let replica_for_task = replica.clone();
        let hint_topic_for_task = hint_topic.clone();
        let handle = tokio::spawn(async move {
            let _ = hydrate_subscription_state_with_services(
                docs_sync.as_ref(),
                blob_service.as_ref(),
                projection_store.as_ref(),
                topic.as_str(),
                &replica_for_task,
            )
            .await;
            loop {
                tokio::select! {
                    Some(event) = doc_stream.next() => {
                        if let Ok(event) = event {
                            if let Some(source_peer) = event.source_peer.as_deref()
                                && let Err(error) = blob_service.learn_peer(source_peer).await
                            {
                                warn!(
                                    topic = %topic,
                                    source_peer = %source_peer,
                                    error = %error,
                                    "failed to learn blob peer from docs sync event"
                                );
                            }
                            match AppService::maybe_create_notification_for_remote_object_event(
                                projection_store.as_ref(),
                                docs_sync.as_ref(),
                                blob_service.as_ref(),
                                local_author_pubkey.as_str(),
                                &notification_baseline,
                                &event,
                            ).await {
                                Ok(true) => {
                                    *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                                }
                                Ok(false) => {}
                                Err(error) => {
                                    warn!(
                                        topic = %topic,
                                        key = %event.key,
                                        error = %error,
                                        "failed to create notification from remote object event"
                                    );
                                }
                            }
                            if let Ok(count) = hydrate_subscription_event_with_services(
                                docs_sync.as_ref(),
                                blob_service.as_ref(),
                                projection_store.as_ref(),
                                topic.as_str(),
                                &replica_for_task,
                                event.key.as_str(),
                            ).await
                            && count > 0
                            {
                                *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            }
                        }
                    }
                    Some(event) = hint_stream.next() => {
                        if hint_targets_topic(&event.hint, topic.as_str()) {
                            match &event.hint {
                                GossipHint::LivePresence { session_id, author, ttl_ms, .. } => {
                                    let now = Utc::now().timestamp_millis();
                                    let _ = projection_store
                                        .upsert_live_presence(
                                            topic.as_str(),
                                            storage_channel_id.as_str(),
                                            session_id.as_str(),
                                            author.as_str(),
                                            now + i64::from(*ttl_ms),
                                            now,
                                        )
                                        .await;
                                    let _ = projection_store.clear_expired_live_presence(now).await;
                                    *last_sync.lock().await = Some(now);
                                }
                                _ => {
                                    if let Ok(count) = hydrate_subscription_hint_with_services(
                                        docs_sync.as_ref(),
                                        blob_service.as_ref(),
                                        projection_store.as_ref(),
                                        topic.as_str(),
                                        &replica_for_task,
                                        &event.hint,
                                    ).await
                                    && count > 0
                                    {
                                        *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                                    }
                                }
                            }
                        }
                    }
                    else => {
                        let _ = hint_transport.unsubscribe_hints(&hint_topic_for_task).await;
                        break;
                    },
                }
            }
        });

        if let Some(private_key) = private_key {
            self.private_channel_subscriptions
                .lock()
                .await
                .insert(private_key, handle);
        } else {
            self.subscriptions
                .lock()
                .await
                .insert(topic_id.to_string(), handle);
        }
        Ok(())
    }

    pub(crate) async fn stop_live_presence_task(
        &self,
        topic_id: &str,
        channel_id: &str,
        session_id: &str,
    ) {
        let key = live_presence_task_key(topic_id, channel_id, session_id);
        let handle = self.live_presence_tasks.lock().await.remove(key.as_str());
        if let Some(handle) = handle {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
    }

    pub(crate) async fn cleanup_ended_live_presence_tasks(
        &self,
        rows: &[LiveSessionProjectionRow],
    ) {
        for row in rows {
            if row.status == LiveSessionStatus::Ended {
                self.stop_live_presence_task(
                    row.topic_id.as_str(),
                    row.channel_id.as_str(),
                    row.session_id.as_str(),
                )
                .await;
            }
        }
    }

    pub(crate) async fn apply_live_presence(
        &self,
        topic_id: &str,
        channel_id: Option<&ChannelId>,
        session_id: &str,
        ttl_ms: u32,
    ) -> Result<()> {
        let now = Utc::now().timestamp_millis();
        let author = self.current_author_pubkey();
        self.projection_store
            .upsert_live_presence(
                topic_id,
                channel_storage_id(channel_id).as_str(),
                session_id,
                author.as_str(),
                now + i64::from(ttl_ms),
                now,
            )
            .await?;
        self.projection_store
            .clear_expired_live_presence(now)
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, channel_id),
                GossipHint::LivePresence {
                    topic_id: TopicId::new(topic_id),
                    session_id: session_id.to_string(),
                    author: Pubkey::from(author),
                    ttl_ms,
                },
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn persist_live_session_manifest(
        &self,
        replica: &ReplicaId,
        topic_id: &str,
        manifest: LiveSessionManifestBlobV1,
        created_at: i64,
        last_envelope_id: EnvelopeId,
    ) -> Result<LiveSessionStateDocV1> {
        let now = Utc::now().timestamp_millis();
        let stored =
            store_manifest_blob(self.blob_service.as_ref(), &manifest, LIVE_MANIFEST_MIME).await?;
        let state = LiveSessionStateDocV1 {
            session_id: manifest.session_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: manifest.channel_id.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            created_at,
            updated_at: now,
            status: manifest.status.clone(),
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            last_envelope_id,
        };
        persist_live_session_state(self.docs_sync.as_ref(), replica, &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    pub(crate) async fn persist_game_room_manifest(
        &self,
        replica: &ReplicaId,
        topic_id: &str,
        manifest: GameRoomManifestBlobV1,
        created_at: i64,
        last_envelope_id: EnvelopeId,
    ) -> Result<GameRoomStateDocV1> {
        let now = Utc::now().timestamp_millis();
        let stored =
            store_manifest_blob(self.blob_service.as_ref(), &manifest, GAME_MANIFEST_MIME).await?;
        let state = GameRoomStateDocV1 {
            room_id: manifest.room_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: manifest.channel_id.clone(),
            owner_pubkey: manifest.owner_pubkey.clone(),
            created_at,
            updated_at: now,
            status: manifest.status.clone(),
            current_manifest: ManifestBlobRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
            },
            last_envelope_id,
        };
        persist_game_room_state(self.docs_sync.as_ref(), replica, &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    pub(crate) async fn fetch_live_session_state_and_manifest(
        &self,
        topic_id: &str,
        session_id: &str,
    ) -> Result<Option<(ReplicaId, LiveSessionStateDocV1, LiveSessionManifestBlobV1)>> {
        for replica in subscription_replicas_for_topic(
            topic_id,
            self.joined_private_channel_states_for_topic(topic_id).await,
        ) {
            let Some(state) = fetch_live_session_state_from_replica(
                self.docs_sync.as_ref(),
                &replica,
                session_id,
            )
            .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<LiveSessionManifestBlobV1>(
                self.blob_service.as_ref(),
                &state.current_manifest,
            )
            .await?
            else {
                continue;
            };
            return Ok(Some((replica, state, manifest)));
        }
        Ok(None)
    }

    pub(crate) async fn fetch_game_room_state_and_manifest(
        &self,
        topic_id: &str,
        room_id: &str,
    ) -> Result<Option<(ReplicaId, GameRoomStateDocV1, GameRoomManifestBlobV1)>> {
        for replica in subscription_replicas_for_topic(
            topic_id,
            self.joined_private_channel_states_for_topic(topic_id).await,
        ) {
            let Some(state) =
                fetch_game_room_state_from_replica(self.docs_sync.as_ref(), &replica, room_id)
                    .await?
            else {
                continue;
            };
            let Some(manifest) = fetch_manifest_blob::<GameRoomManifestBlobV1>(
                self.blob_service.as_ref(),
                &state.current_manifest,
            )
            .await?
            else {
                continue;
            };
            return Ok(Some((replica, state, manifest)));
        }
        Ok(None)
    }

    pub(crate) async fn build_author_social_view(
        &self,
        author_pubkey: &str,
    ) -> Result<AuthorSocialView> {
        let profile = self.store.get_profile(author_pubkey).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), author_pubkey)
            .await?;
        let muted = self
            .projection_store
            .get_muted_author(author_pubkey)
            .await?
            .is_some();
        Ok(author_social_view_from_parts(
            author_pubkey,
            profile.as_ref(),
            relationship.as_ref(),
            muted,
        ))
    }

    pub(crate) async fn rebuild_author_relationships(&self) -> Result<()> {
        rebuild_author_relationships_with_services(
            self.store.as_ref(),
            self.projection_store.as_ref(),
            self.current_author_pubkey().as_str(),
        )
        .await?;
        self.reconcile_direct_message_subscriptions().await
    }

    pub(crate) async fn restart_direct_message_subscriptions(&self) -> Result<()> {
        let existing_peers = self
            .direct_message_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for peer_pubkey in existing_peers {
            stop_direct_message_subscription_with_services(
                self.direct_message_subscriptions.as_ref(),
                self.hint_transport.as_ref(),
                self.keys.as_ref(),
                peer_pubkey.as_str(),
            )
            .await?;
        }
        self.reconcile_direct_message_subscriptions().await
    }

    pub(crate) async fn current_muted_author_pubkeys(&self) -> Result<BTreeSet<String>> {
        Ok(self
            .projection_store
            .list_muted_authors()
            .await?
            .into_iter()
            .map(|row| row.author_pubkey)
            .collect())
    }

    pub(crate) async fn ensure_author_subscriptions_for_rows(
        &self,
        rows: &[ObjectProjectionRow],
    ) -> Result<()> {
        let mut author_pubkeys = BTreeSet::new();
        for row in rows {
            author_pubkeys.insert(row.author_pubkey.clone());
            if let Some(repost_of) = row.repost_of.as_ref() {
                author_pubkeys.insert(repost_of.source_author_pubkey.as_str().to_string());
            }
        }
        for author_pubkey in author_pubkeys {
            self.ensure_author_subscription(author_pubkey.as_str())
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn ensure_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        if self
            .author_subscriptions
            .lock()
            .await
            .contains_key(author_pubkey.as_str())
        {
            return Ok(());
        }

        self.spawn_author_subscription(author_pubkey.as_str()).await
    }

    pub(crate) async fn restart_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let author_pubkey = normalize_author_pubkey(author_pubkey)?;
        if let Some(handle) = self
            .author_subscriptions
            .lock()
            .await
            .remove(author_pubkey.as_str())
        {
            handle.abort();
        }
        self.spawn_author_subscription(author_pubkey.as_str()).await
    }

    pub(crate) async fn maybe_restart_author_subscription(&self, author_pubkey: &str) {
        let Ok(author_pubkey) = normalize_author_pubkey(author_pubkey) else {
            return;
        };
        let key = format!("author-subscription:{author_pubkey}");
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self
            .restart_author_subscription(author_pubkey.as_str())
            .await
        {
            warn!(
                author_pubkey = %author_pubkey,
                error = %error,
                "failed to restart author subscription"
            );
        }
    }

    pub(crate) async fn spawn_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let store = Arc::clone(&self.store);
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let blob_service = Arc::clone(&self.blob_service);
        let hint_transport = Arc::clone(&self.hint_transport);
        let transport = Arc::clone(&self.transport);
        let keys = Arc::clone(&self.keys);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let direct_message_subscriptions = Arc::clone(&self.direct_message_subscriptions);
        let author_key = normalize_author_pubkey(author_pubkey)?;
        let local_author_pubkey = self.current_author_pubkey();
        let replica = author_replica_id(author_key.as_str());
        docs_sync.open_replica(&replica).await?;
        let notification_baseline =
            snapshot_follow_notification_baseline(docs_sync.as_ref(), &replica).await?;
        let initial_count = hydrate_author_state_with_services(
            docs_sync.as_ref(),
            store.as_ref(),
            projection_store.as_ref(),
            local_author_pubkey.as_str(),
            author_key.as_str(),
        )
        .await?;
        if initial_count > 0 {
            *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
            schedule_direct_message_reconcile_with_services(
                Arc::clone(&store),
                Arc::clone(&projection_store),
                Arc::clone(&blob_service),
                Arc::clone(&hint_transport),
                Arc::clone(&transport),
                Arc::clone(&keys),
                Arc::clone(&last_sync),
                Arc::clone(&direct_message_subscriptions),
                local_author_pubkey.clone(),
                author_key.clone(),
            )
        }
        let mut doc_stream = docs_sync.subscribe_replica(&replica).await?;
        let author_key_for_task = author_key.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = doc_stream.next() => {
                        if event.is_err() {
                            continue;
                        }
                        if let Ok(event) = event.as_ref() {
                            match AppService::maybe_create_notification_for_remote_follow_event(
                                store.as_ref(),
                                projection_store.as_ref(),
                                docs_sync.as_ref(),
                                local_author_pubkey.as_str(),
                                author_key_for_task.as_str(),
                                &notification_baseline,
                                event,
                            ).await {
                                Ok(true) => {
                                    *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                                }
                                Ok(false) => {}
                                Err(error) => {
                                    warn!(
                                        author_pubkey = %author_key_for_task,
                                        key = %event.key,
                                        error = %error,
                                        "failed to create notification from remote follow event"
                                    );
                                }
                            }
                        }
                        if let Ok(count) = hydrate_author_state_with_services(
                            docs_sync.as_ref(),
                            store.as_ref(),
                            projection_store.as_ref(),
                            local_author_pubkey.as_str(),
                            author_key_for_task.as_str(),
                        ).await
                        && count > 0
                        {
                            *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            schedule_direct_message_reconcile_with_services(
                                Arc::clone(&store),
                                Arc::clone(&projection_store),
                                Arc::clone(&blob_service),
                                Arc::clone(&hint_transport),
                                Arc::clone(&transport),
                                Arc::clone(&keys),
                                Arc::clone(&last_sync),
                                Arc::clone(&direct_message_subscriptions),
                                local_author_pubkey.clone(),
                                author_key_for_task.clone(),
                            );
                        }
                    }
                    else => break,
                }
            }
        });
        self.author_subscriptions
            .lock()
            .await
            .insert(author_key, handle);
        Ok(())
    }

    pub(crate) async fn direct_message_send_enabled(&self, peer_pubkey: &str) -> Result<bool> {
        Ok(self
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), peer_pubkey)
            .await?
            .as_ref()
            .is_some_and(|relationship| relationship.mutual))
    }

    pub(crate) async fn reconcile_direct_message_subscriptions(&self) -> Result<()> {
        reconcile_direct_message_subscriptions_with_services(
            self.store.as_ref(),
            Arc::clone(&self.projection_store),
            Arc::clone(&self.blob_service),
            Arc::clone(&self.hint_transport),
            Arc::clone(&self.transport),
            Arc::clone(&self.keys),
            Arc::clone(&self.last_sync_ts),
            Arc::clone(&self.direct_message_subscriptions),
            self.current_author_pubkey().as_str(),
        )
        .await
    }

    pub(crate) async fn direct_message_status_view(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageStatusView> {
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        let send_enabled = self.direct_message_send_enabled(peer_pubkey).await?;
        let peer_count = if send_enabled {
            self.direct_message_topic_peer_count(peer_pubkey).await?
        } else {
            0
        };
        let pending_outbox_count = self
            .projection_store
            .list_direct_message_outbox()
            .await?
            .into_iter()
            .filter(|row| row.peer_pubkey == peer_pubkey)
            .count();
        Ok(DirectMessageStatusView {
            peer_pubkey: peer_pubkey.to_string(),
            dm_id,
            mutual: send_enabled,
            send_enabled,
            peer_count,
            pending_outbox_count,
        })
    }

    pub(crate) async fn ensure_direct_message_conversation_row(
        &self,
        peer_pubkey: &str,
    ) -> Result<()> {
        if self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?
            .is_some()
        {
            return Ok(());
        }
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        self.projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id,
                peer_pubkey: peer_pubkey.to_string(),
                updated_at: Utc::now().timestamp_millis(),
                last_message_at: None,
                last_message_id: None,
                last_message_preview: None,
            })
            .await
    }

    pub(crate) async fn refresh_direct_message_conversation(
        &self,
        peer_pubkey: &str,
    ) -> Result<()> {
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        let existing = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?;
        let page = self
            .projection_store
            .list_direct_message_messages(dm_id.as_str(), None, 1)
            .await?;
        let (updated_at, last_message_at, last_message_id, last_message_preview) =
            if let Some(message) = page.items.first() {
                (
                    message.created_at,
                    Some(message.created_at),
                    Some(message.message_id.clone()),
                    Some(direct_message_preview(message)),
                )
            } else if let Some(existing) = existing.as_ref() {
                (existing.updated_at, None, None, None)
            } else if self.direct_message_send_enabled(peer_pubkey).await? {
                (Utc::now().timestamp_millis(), None, None, None)
            } else {
                return Ok(());
            };
        self.projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id,
                peer_pubkey: peer_pubkey.to_string(),
                updated_at,
                last_message_at,
                last_message_id,
                last_message_preview,
            })
            .await
    }

    pub(crate) async fn direct_message_conversation_view(
        &self,
        peer_pubkey: &str,
    ) -> Result<DirectMessageConversationView> {
        let conversation = self
            .projection_store
            .get_direct_message_conversation_by_peer(peer_pubkey)
            .await?
            .ok_or_else(|| anyhow::anyhow!("direct message conversation is not initialized"))?;
        let profile = self.store.get_profile(peer_pubkey).await?;
        let status = self.direct_message_status_view(peer_pubkey).await?;
        Ok(DirectMessageConversationView {
            dm_id: conversation.dm_id,
            peer_pubkey: peer_pubkey.to_string(),
            peer_name: profile.as_ref().and_then(|value| value.name.clone()),
            peer_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            peer_picture: profile.as_ref().and_then(|value| value.picture.clone()),
            peer_picture_asset: profile_asset_view_from_ref(
                profile
                    .as_ref()
                    .and_then(|value| value.picture_asset.as_ref()),
            ),
            updated_at: conversation.updated_at,
            last_message_at: conversation.last_message_at,
            last_message_id: conversation.last_message_id,
            last_message_preview: conversation.last_message_preview,
            status,
        })
    }

    pub(crate) async fn direct_message_message_view(
        &self,
        row: DirectMessageMessageRow,
    ) -> Result<DirectMessageMessageView> {
        Ok(DirectMessageMessageView {
            dm_id: row.dm_id,
            message_id: row.message_id,
            sender_pubkey: row.sender_pubkey,
            recipient_pubkey: row.recipient_pubkey,
            created_at: row.created_at,
            text: row.text.unwrap_or_default(),
            reply_to_message_id: row.reply_to_message_id,
            attachments: direct_message_attachment_views(
                self.blob_service.as_ref(),
                row.attachment_manifest.as_ref(),
            )
            .await?,
            outgoing: row.outgoing,
            delivered: row.acked_at.is_some() || !row.outgoing,
        })
    }

    pub(crate) async fn notification_view_from_row(
        &self,
        row: NotificationRow,
    ) -> Result<NotificationView> {
        let object_id = row.object_id.clone();
        let thread_root_object_id = if let Some(object_id) = object_id.as_ref() {
            self.projection_store
                .get_object_projection(object_id)
                .await?
                .map(|projection| {
                    projection
                        .root_object_id
                        .unwrap_or(projection.object_id)
                        .as_str()
                        .to_string()
                })
        } else {
            None
        };
        let profile = self.store.get_profile(row.actor_pubkey.as_str()).await?;
        Ok(NotificationView {
            notification_id: row.notification_id,
            kind: row.kind,
            actor_pubkey: row.actor_pubkey,
            actor_name: profile.as_ref().and_then(|value| value.name.clone()),
            actor_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            actor_picture: profile.as_ref().and_then(|value| value.picture.clone()),
            actor_picture_asset: profile_asset_view_from_ref(
                profile
                    .as_ref()
                    .and_then(|value| value.picture_asset.as_ref()),
            ),
            source_envelope_id: row
                .source_envelope_id
                .map(|value| value.as_str().to_string()),
            source_replica_id: row
                .source_replica_id
                .map(|value| value.as_str().to_string()),
            topic_id: row.topic_id,
            channel_id: row.channel_id,
            object_id: object_id.map(|value| value.as_str().to_string()),
            thread_root_object_id,
            dm_id: row.dm_id,
            message_id: row.message_id,
            preview_text: row.preview_text,
            created_at: row.created_at,
            received_at: row.received_at,
            read_at: row.read_at,
        })
    }

    pub(crate) async fn notification_status_view(&self) -> Result<NotificationStatusView> {
        Ok(NotificationStatusView {
            unread_count: self.projection_store.count_unread_notifications().await?,
        })
    }

    pub(crate) async fn ensure_direct_message_subscription(&self, peer_pubkey: &str) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        if !self
            .direct_message_send_enabled(peer_pubkey.as_str())
            .await?
        {
            return Ok(());
        }
        let has_active_handle = self
            .direct_message_subscriptions
            .lock()
            .await
            .get(peer_pubkey.as_str())
            .is_some_and(|handle| !handle.is_finished());
        if has_active_handle {
            if self
                .should_restart_stale_direct_message_subscription(peer_pubkey.as_str())
                .await?
            {
                self.restart_direct_message_subscription(peer_pubkey.as_str())
                    .await?;
            }
            return Ok(());
        }
        Self::spawn_direct_message_subscription_with_services(
            Arc::clone(&self.direct_message_subscriptions),
            Arc::clone(&self.projection_store),
            Arc::clone(&self.blob_service),
            Arc::clone(&self.hint_transport),
            Arc::clone(&self.transport),
            Arc::clone(&self.keys),
            Arc::clone(&self.last_sync_ts),
            self.current_author_pubkey().as_str(),
            peer_pubkey.as_str(),
        )
        .await
    }

    pub(crate) async fn restart_direct_message_subscription(
        &self,
        peer_pubkey: &str,
    ) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        stop_direct_message_subscription_with_services(
            self.direct_message_subscriptions.as_ref(),
            self.hint_transport.as_ref(),
            self.keys.as_ref(),
            peer_pubkey.as_str(),
        )
        .await?;
        Self::spawn_direct_message_subscription_with_services(
            Arc::clone(&self.direct_message_subscriptions),
            Arc::clone(&self.projection_store),
            Arc::clone(&self.blob_service),
            Arc::clone(&self.hint_transport),
            Arc::clone(&self.transport),
            Arc::clone(&self.keys),
            Arc::clone(&self.last_sync_ts),
            self.current_author_pubkey().as_str(),
            peer_pubkey.as_str(),
        )
        .await
    }

    pub(crate) async fn direct_message_topic_snapshot(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<TopicPeerSnapshot>> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let topic =
            derive_direct_message_topic(self.keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))?;
        let hint_topic = format!("hint/{}", topic.as_str());
        Ok(self
            .transport
            .peers()
            .await?
            .topic_diagnostics
            .into_iter()
            .find(|diagnostic| {
                diagnostic.topic == hint_topic || diagnostic.topic == topic.as_str()
            }))
    }

    pub(crate) async fn should_restart_stale_direct_message_subscription(
        &self,
        peer_pubkey: &str,
    ) -> Result<bool> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        let Some(snapshot) = self
            .direct_message_topic_snapshot(peer_pubkey.as_str())
            .await?
        else {
            return Ok(false);
        };
        if snapshot.joined || snapshot.peer_count > 0 || snapshot.configured_peer_ids.is_empty() {
            self.direct_message_subscription_restart_deadlines
                .lock()
                .await
                .remove(peer_pubkey.as_str());
            return Ok(false);
        }
        let now = Utc::now().timestamp();
        let mut deadlines = self
            .direct_message_subscription_restart_deadlines
            .lock()
            .await;
        let next_due_at = deadlines
            .get(peer_pubkey.as_str())
            .copied()
            .unwrap_or_default();
        if now < next_due_at {
            return Ok(false);
        }
        deadlines.insert(
            peer_pubkey,
            now.saturating_add(DIRECT_MESSAGE_SUBSCRIPTION_RESTART_RETRY_SECONDS),
        );
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn spawn_direct_message_subscription_with_services(
        direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
        projection_store: Arc<dyn ProjectionStore>,
        blob_service: Arc<dyn BlobService>,
        hint_transport: Arc<dyn HintTransport>,
        transport: Arc<dyn Transport>,
        keys: Arc<KukuriKeys>,
        last_sync: Arc<Mutex<Option<i64>>>,
        local_author_pubkey: &str,
        peer_pubkey: &str,
    ) -> Result<()> {
        let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
        {
            let mut subscriptions = direct_message_subscriptions.lock().await;
            if subscriptions
                .get(peer_pubkey.as_str())
                .is_some_and(|handle| !handle.is_finished())
            {
                return Ok(());
            }
            subscriptions.remove(peer_pubkey.as_str());
        }
        let topic =
            derive_direct_message_topic(keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))?;
        let mut hint_stream = hint_transport.subscribe_hints(&topic).await?;
        let topic_for_task = topic.clone();
        let peer_for_task = peer_pubkey.clone();
        let local_author_pubkey = local_author_pubkey.to_string();
        let task_hint_transport = Arc::clone(&hint_transport);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(
                DIRECT_MESSAGE_RETRY_INTERVAL_MS,
            ));
            let _ = AppService::flush_direct_message_outbox_for_peer_with_services(
                projection_store.as_ref(),
                task_hint_transport.as_ref(),
                transport.as_ref(),
                local_author_pubkey.as_str(),
                keys.as_ref(),
                peer_for_task.as_str(),
            )
            .await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let _ = AppService::flush_direct_message_outbox_for_peer_with_services(
                            projection_store.as_ref(),
                            task_hint_transport.as_ref(),
                            transport.as_ref(),
                            local_author_pubkey.as_str(),
                            keys.as_ref(),
                            peer_for_task.as_str(),
                        ).await;
                    }
                    Some(event) = hint_stream.next() => {
                        if !matches!(
                            &event.hint,
                            GossipHint::DirectMessageFrame { topic_id, .. } | GossipHint::DirectMessageAck { topic_id, .. }
                            if topic_id.as_str() == topic_for_task.as_str()
                        ) {
                            continue;
                        }
                        if let Err(error) = blob_service.learn_peer(event.source_peer.as_str()).await {
                            warn!(
                                peer_pubkey = %peer_for_task,
                                source_peer = %event.source_peer,
                                error = %error,
                                "failed to learn direct message blob peer"
                            );
                        }
                        match AppService::handle_direct_message_hint_with_services(
                            projection_store.as_ref(),
                            blob_service.as_ref(),
                            task_hint_transport.as_ref(),
                            keys.as_ref(),
                            local_author_pubkey.as_str(),
                            peer_for_task.as_str(),
                            &topic_for_task,
                            &event.hint,
                        ).await {
                            Ok(true) => {
                                *last_sync.lock().await = Some(Utc::now().timestamp_millis());
                            }
                            Ok(false) => {}
                            Err(error) => {
                                warn!(
                                    peer_pubkey = %peer_for_task,
                                    error = %error,
                                    "failed to handle direct message hint"
                                );
                            }
                        }
                    }
                    else => {
                        let _ = task_hint_transport.unsubscribe_hints(&topic_for_task).await;
                        break;
                    }
                }
            }
        });
        let mut pending_handle = Some(handle);
        let should_abort_new_handle = {
            let mut subscriptions = direct_message_subscriptions.lock().await;
            if subscriptions
                .get(peer_pubkey.as_str())
                .is_some_and(|existing| !existing.is_finished())
            {
                true
            } else {
                subscriptions.insert(
                    peer_pubkey.clone(),
                    pending_handle
                        .take()
                        .expect("direct message subscription handle must be pending"),
                );
                false
            }
        };
        if should_abort_new_handle {
            pending_handle
                .expect("direct message subscription handle must remain pending")
                .abort();
            hint_transport.unsubscribe_hints(&topic).await?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn handle_direct_message_hint_with_services(
        projection_store: &dyn ProjectionStore,
        blob_service: &dyn BlobService,
        hint_transport: &dyn HintTransport,
        keys: &KukuriKeys,
        local_author_pubkey: &str,
        peer_pubkey: &str,
        topic: &TopicId,
        hint: &GossipHint,
    ) -> Result<bool> {
        match hint {
            GossipHint::DirectMessageFrame {
                dm_id,
                message_id,
                frame_hash,
                ..
            } => {
                AppService::ingest_direct_message_frame_with_services(
                    projection_store,
                    blob_service,
                    hint_transport,
                    keys,
                    local_author_pubkey,
                    peer_pubkey,
                    topic,
                    dm_id.as_str(),
                    message_id.as_str(),
                    frame_hash,
                )
                .await
            }
            GossipHint::DirectMessageAck { ack, .. } => {
                ack.verify()?;
                if ack.sender.as_str() != peer_pubkey
                    || ack.recipient.as_str() != local_author_pubkey
                {
                    return Ok(false);
                }
                projection_store
                    .set_direct_message_acked_at(
                        ack.dm_id.as_str(),
                        ack.message_id.as_str(),
                        ack.acked_at,
                    )
                    .await?;
                projection_store
                    .remove_direct_message_outbox(ack.dm_id.as_str(), ack.message_id.as_str())
                    .await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(crate) async fn maybe_create_notification_for_remote_object_event(
        projection_store: &dyn ProjectionStore,
        docs_sync: &dyn DocsSync,
        blob_service: &dyn BlobService,
        local_author_pubkey: &str,
        notification_baseline: &NotificationDocEventBaseline,
        event: &DocEvent,
    ) -> Result<bool> {
        if notification_baseline.contains(event) {
            return Ok(false);
        }
        let Some(candidate) = notification_candidate_from_object_event(
            projection_store,
            docs_sync,
            blob_service,
            local_author_pubkey,
            event,
        )
        .await?
        else {
            return Ok(false);
        };
        Self::put_notification_candidate(projection_store, local_author_pubkey, candidate).await
    }

    pub(crate) async fn maybe_create_notification_for_remote_follow_event(
        store: &dyn Store,
        projection_store: &dyn ProjectionStore,
        docs_sync: &dyn DocsSync,
        local_author_pubkey: &str,
        author_pubkey: &str,
        notification_baseline: &NotificationDocEventBaseline,
        event: &DocEvent,
    ) -> Result<bool> {
        if notification_baseline.contains(event) {
            return Ok(false);
        }
        let Some(candidate) = notification_candidate_from_follow_event(
            store,
            docs_sync,
            local_author_pubkey,
            author_pubkey,
            event,
        )
        .await?
        else {
            return Ok(false);
        };
        Self::put_notification_candidate(projection_store, local_author_pubkey, candidate).await
    }

    pub(crate) async fn put_notification_candidate(
        projection_store: &dyn ProjectionStore,
        recipient_pubkey: &str,
        candidate: NotificationCandidate,
    ) -> Result<bool> {
        let notification_id = if let (Some(dm_id), Some(message_id)) =
            (candidate.dm_id.as_deref(), candidate.message_id.as_deref())
        {
            direct_message_notification_id(recipient_pubkey, &candidate.kind, dm_id, message_id)
        } else {
            let source_envelope_id = candidate
                .source_envelope_id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("notification is missing source envelope id"))?;
            document_notification_id(recipient_pubkey, &candidate.kind, source_envelope_id)
        };
        projection_store
            .put_notification_if_absent(NotificationRow {
                notification_id,
                recipient_pubkey: recipient_pubkey.to_string(),
                kind: candidate.kind,
                actor_pubkey: candidate.actor_pubkey,
                source_envelope_id: candidate.source_envelope_id,
                source_replica_id: candidate.source_replica_id,
                topic_id: candidate.topic_id,
                channel_id: candidate.channel_id,
                object_id: candidate.object_id,
                dm_id: candidate.dm_id,
                message_id: candidate.message_id,
                preview_text: candidate.preview_text,
                created_at: candidate.created_at,
                received_at: candidate.received_at,
                read_at: None,
            })
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn ingest_direct_message_frame_with_services(
        projection_store: &dyn ProjectionStore,
        blob_service: &dyn BlobService,
        hint_transport: &dyn HintTransport,
        keys: &KukuriKeys,
        local_author_pubkey: &str,
        peer_pubkey: &str,
        topic: &TopicId,
        dm_id: &str,
        message_id: &str,
        frame_hash: &kukuri_core::BlobHash,
    ) -> Result<bool> {
        let expected_dm_id = direct_message_id_for_participants(
            &Pubkey::from(local_author_pubkey),
            &Pubkey::from(peer_pubkey),
        );
        if dm_id != expected_dm_id {
            return Ok(false);
        }
        let Some(frame_bytes) = blob_service.fetch_blob(frame_hash).await? else {
            return Ok(false);
        };
        let frame: DirectMessageFrameV1 = serde_json::from_slice(frame_bytes.as_slice())
            .context("failed to decode direct message frame blob")?;
        if frame.message_id != message_id || frame.dm_id != dm_id {
            return Ok(false);
        }
        if frame.sender.as_str() != peer_pubkey || frame.recipient.as_str() != local_author_pubkey {
            return Ok(false);
        }
        let payload = decrypt_direct_message_frame(keys, &frame)?;
        let ack = build_direct_message_ack(
            keys,
            dm_id,
            message_id,
            &frame.sender,
            Utc::now().timestamp_millis(),
        )?;
        if projection_store
            .has_direct_message_tombstone(dm_id, message_id)
            .await?
        {
            hint_transport
                .publish_hint(
                    topic,
                    GossipHint::DirectMessageAck {
                        topic_id: topic.clone(),
                        ack,
                    },
                )
                .await?;
            return Ok(false);
        }
        if projection_store
            .get_direct_message_message(dm_id, message_id)
            .await?
            .is_some()
        {
            hint_transport
                .publish_hint(
                    topic,
                    GossipHint::DirectMessageAck {
                        topic_id: topic.clone(),
                        ack,
                    },
                )
                .await?;
            return Ok(false);
        }
        let local_manifest = materialize_direct_message_manifest(
            blob_service,
            keys,
            &frame.sender,
            frame.message_id.as_str(),
            payload.attachment_manifest.as_ref(),
        )
        .await?;
        let message_row = DirectMessageMessageRow {
            dm_id: dm_id.to_string(),
            message_id: message_id.to_string(),
            sender_pubkey: frame.sender.as_str().to_string(),
            recipient_pubkey: frame.recipient.as_str().to_string(),
            created_at: frame.created_at,
            text: payload.text,
            reply_to_message_id: payload.reply_to,
            attachment_manifest: local_manifest,
            outgoing: false,
            acked_at: None,
        };
        let preview_text = notification_preview_text(Some(direct_message_preview(&message_row)));
        projection_store
            .put_direct_message_message(message_row)
            .await?;
        projection_store
            .upsert_direct_message_conversation(DirectMessageConversationRow {
                dm_id: dm_id.to_string(),
                peer_pubkey: peer_pubkey.to_string(),
                updated_at: frame.created_at,
                last_message_at: Some(frame.created_at),
                last_message_id: Some(message_id.to_string()),
                last_message_preview: preview_text.clone(),
            })
            .await?;
        Self::put_notification_candidate(
            projection_store,
            local_author_pubkey,
            NotificationCandidate {
                kind: NotificationKind::DirectMessage,
                actor_pubkey: peer_pubkey.to_string(),
                source_envelope_id: None,
                source_replica_id: None,
                topic_id: None,
                channel_id: None,
                object_id: None,
                dm_id: Some(dm_id.to_string()),
                message_id: Some(message_id.to_string()),
                preview_text,
                created_at: frame.created_at,
                received_at: Utc::now().timestamp_millis(),
            },
        )
        .await?;
        hint_transport
            .publish_hint(
                topic,
                GossipHint::DirectMessageAck {
                    topic_id: topic.clone(),
                    ack,
                },
            )
            .await?;
        Ok(true)
    }

    pub(crate) async fn flush_direct_message_outbox_for_peer_with_services(
        projection_store: &dyn ProjectionStore,
        hint_transport: &dyn HintTransport,
        transport: &dyn Transport,
        local_author_pubkey: &str,
        keys: &KukuriKeys,
        peer_pubkey: &str,
    ) -> Result<usize> {
        let relationship = projection_store
            .get_author_relationship(local_author_pubkey, peer_pubkey)
            .await?;
        if !relationship.as_ref().is_some_and(|value| value.mutual) {
            return Ok(0);
        }
        let topic = derive_direct_message_topic(keys, &Pubkey::from(peer_pubkey))?;
        let peer_count = direct_message_topic_peer_count(transport, &topic).await?;
        let topic_has_connected_peer = peer_count > 0;
        let mut published = 0usize;
        let attempted_at = Utc::now().timestamp_millis();
        for row in projection_store.list_direct_message_outbox().await? {
            if row.peer_pubkey != peer_pubkey {
                continue;
            }
            if topic_has_connected_peer {
                projection_store
                    .touch_direct_message_outbox_attempt(
                        row.dm_id.as_str(),
                        row.message_id.as_str(),
                        attempted_at,
                    )
                    .await?;
            }
            let publish_result = hint_transport
                .publish_hint(
                    &topic,
                    GossipHint::DirectMessageFrame {
                        topic_id: topic.clone(),
                        dm_id: row.dm_id.clone(),
                        message_id: row.message_id.clone(),
                        frame_hash: row.frame_blob_hash.clone(),
                    },
                )
                .await;
            if let Err(error) = publish_result {
                if topic_has_connected_peer {
                    return Err(error);
                }
                continue;
            }
            published += 1;
        }
        Ok(published)
    }

    pub(crate) async fn direct_message_topic_peer_count(&self, peer_pubkey: &str) -> Result<usize> {
        let topic = derive_direct_message_topic(self.keys.as_ref(), &Pubkey::from(peer_pubkey))?;
        direct_message_topic_peer_count(self.transport.as_ref(), &topic).await
    }

    pub(crate) async fn send_direct_message_internal(
        &self,
        peer_pubkey: &str,
        text: Option<&str>,
        reply_to_message_id: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        let text = normalize_optional_text(text.map(str::to_string));
        let dm_id = direct_message_id_for_participants(
            &Pubkey::from(self.current_author_pubkey()),
            &Pubkey::from(peer_pubkey),
        );
        if text.is_none() && attachments.is_empty() {
            anyhow::bail!("direct message text or attachment is required");
        }
        let message_id = format!(
            "dm-message-{}-{}",
            Utc::now().timestamp_millis(),
            short_id_suffix(self.current_author_pubkey().as_str())
        );
        if let Some(reply_to_message_id) = reply_to_message_id
            && self
                .projection_store
                .get_direct_message_message(dm_id.as_str(), reply_to_message_id.trim())
                .await?
                .is_none()
        {
            anyhow::bail!("direct message reply target was not found");
        }
        let (local_manifest, encrypted_manifest) = self
            .prepare_direct_message_manifests(peer_pubkey, message_id.as_str(), attachments)
            .await?;
        let created_at = Utc::now().timestamp_millis();
        let frame = encrypt_direct_message_frame(
            self.keys.as_ref(),
            &Pubkey::from(peer_pubkey),
            dm_id.as_str(),
            message_id.as_str(),
            created_at,
            &DirectMessagePayloadV1 {
                text: text.clone(),
                reply_to: normalize_optional_text(reply_to_message_id.map(str::to_string)),
                attachment_manifest: encrypted_manifest,
            },
        )?;
        let frame_bytes =
            serde_json::to_vec(&frame).context("failed to encode direct message frame blob")?;
        let frame_blob = self
            .blob_service
            .put_blob(frame_bytes, DIRECT_MESSAGE_FRAME_MIME)
            .await?;
        self.projection_store
            .put_direct_message_message(DirectMessageMessageRow {
                dm_id: dm_id.clone(),
                message_id: message_id.clone(),
                sender_pubkey: self.current_author_pubkey(),
                recipient_pubkey: peer_pubkey.to_string(),
                created_at,
                text,
                reply_to_message_id: normalize_optional_text(
                    reply_to_message_id.map(str::to_string),
                ),
                attachment_manifest: local_manifest,
                outgoing: true,
                acked_at: None,
            })
            .await?;
        self.projection_store
            .put_direct_message_outbox(DirectMessageOutboxRow {
                dm_id: dm_id.clone(),
                message_id: message_id.clone(),
                peer_pubkey: peer_pubkey.to_string(),
                frame_blob_hash: frame_blob.hash,
                created_at,
                last_attempt_at: None,
            })
            .await?;
        self.refresh_direct_message_conversation(peer_pubkey)
            .await?;
        let _ = Self::flush_direct_message_outbox_for_peer_with_services(
            self.projection_store.as_ref(),
            self.hint_transport.as_ref(),
            self.transport.as_ref(),
            self.current_author_pubkey().as_str(),
            self.keys.as_ref(),
            peer_pubkey,
        )
        .await?;
        Ok(message_id)
    }

    pub(crate) async fn prepare_direct_message_manifests(
        &self,
        peer_pubkey: &str,
        message_id: &str,
        attachments: Vec<PendingAttachment>,
    ) -> Result<(
        Option<DirectMessageAttachmentManifestV1>,
        Option<DirectMessageAttachmentManifestV1>,
    )> {
        if attachments.is_empty() {
            return Ok((None, None));
        }
        let image = attachments
            .iter()
            .find(|attachment| attachment.role == AssetRole::ImageOriginal);
        let video = attachments
            .iter()
            .find(|attachment| attachment.role == AssetRole::VideoManifest);
        let poster = attachments
            .iter()
            .find(|attachment| attachment.role == AssetRole::VideoPoster);
        match (image, video, poster) {
            (Some(image), None, None) => {
                if attachments.len() != 1 || !image.mime.starts_with("image/") {
                    anyhow::bail!(
                        "direct message image attachment must be a single image/* payload"
                    );
                }
                let local_blob = self
                    .blob_service
                    .put_blob(image.bytes.clone(), image.mime.as_str())
                    .await?;
                let encrypted = encrypt_direct_message_attachment(
                    self.keys.as_ref(),
                    &Pubkey::from(peer_pubkey),
                    message_id,
                    "original",
                    image.bytes.as_slice(),
                )?;
                let encrypted_blob = self
                    .blob_service
                    .put_blob(
                        serde_json::to_vec(&encrypted)
                            .context("failed to encode encrypted direct message attachment")?,
                        DIRECT_MESSAGE_ATTACHMENT_MIME,
                    )
                    .await?;
                Ok((
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Image,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: local_blob.hash,
                            mime: image.mime.clone(),
                            bytes: image.bytes.len() as u64,
                            nonce_hex: String::new(),
                        },
                        poster: None,
                    }),
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Image,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: encrypted_blob.hash,
                            mime: image.mime.clone(),
                            bytes: image.bytes.len() as u64,
                            nonce_hex: encrypted.nonce_hex,
                        },
                        poster: None,
                    }),
                ))
            }
            (None, Some(video), Some(poster)) => {
                if attachments.len() != 2
                    || !video.mime.starts_with("video/")
                    || !poster.mime.starts_with("image/")
                {
                    anyhow::bail!(
                        "direct message video attachment must contain one video/* payload and one image/* poster"
                    );
                }
                let local_video = self
                    .blob_service
                    .put_blob(video.bytes.clone(), video.mime.as_str())
                    .await?;
                let local_poster = self
                    .blob_service
                    .put_blob(poster.bytes.clone(), poster.mime.as_str())
                    .await?;
                let encrypted_video = encrypt_direct_message_attachment(
                    self.keys.as_ref(),
                    &Pubkey::from(peer_pubkey),
                    message_id,
                    "original",
                    video.bytes.as_slice(),
                )?;
                let encrypted_poster = encrypt_direct_message_attachment(
                    self.keys.as_ref(),
                    &Pubkey::from(peer_pubkey),
                    message_id,
                    "poster",
                    poster.bytes.as_slice(),
                )?;
                let encrypted_video_blob = self
                    .blob_service
                    .put_blob(
                        serde_json::to_vec(&encrypted_video)
                            .context("failed to encode encrypted direct message video")?,
                        DIRECT_MESSAGE_ATTACHMENT_MIME,
                    )
                    .await?;
                let encrypted_poster_blob = self
                    .blob_service
                    .put_blob(
                        serde_json::to_vec(&encrypted_poster)
                            .context("failed to encode encrypted direct message poster")?,
                        DIRECT_MESSAGE_ATTACHMENT_MIME,
                    )
                    .await?;
                Ok((
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Video,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: local_video.hash,
                            mime: video.mime.clone(),
                            bytes: video.bytes.len() as u64,
                            nonce_hex: String::new(),
                        },
                        poster: Some(DirectMessageEncryptedBlobRefV1 {
                            blob_id: "poster".into(),
                            hash: local_poster.hash,
                            mime: poster.mime.clone(),
                            bytes: poster.bytes.len() as u64,
                            nonce_hex: String::new(),
                        }),
                    }),
                    Some(DirectMessageAttachmentManifestV1 {
                        attachment_id: "attachment-1".into(),
                        kind: DirectMessageAttachmentKind::Video,
                        original: DirectMessageEncryptedBlobRefV1 {
                            blob_id: "original".into(),
                            hash: encrypted_video_blob.hash,
                            mime: video.mime.clone(),
                            bytes: video.bytes.len() as u64,
                            nonce_hex: encrypted_video.nonce_hex,
                        },
                        poster: Some(DirectMessageEncryptedBlobRefV1 {
                            blob_id: "poster".into(),
                            hash: encrypted_poster_blob.hash,
                            mime: poster.mime.clone(),
                            bytes: poster.bytes.len() as u64,
                            nonce_hex: encrypted_poster.nonce_hex,
                        }),
                    }),
                ))
            }
            _ => anyhow::bail!(
                "direct message attachment must be one image or one video with a poster"
            ),
        }
    }

    pub(crate) async fn ensure_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if self.subscriptions.lock().await.contains_key(topic_id) {
            return Ok(());
        }

        self.spawn_topic_subscription(topic_id).await
    }

    pub(crate) async fn restart_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await?;
        self.spawn_topic_subscription(topic_id).await
    }

    pub(crate) async fn spawn_topic_subscription(&self, topic_id: &str) -> Result<()> {
        self.spawn_subscription_task(
            topic_id,
            None,
            topic_replica_id(topic_id),
            TopicId::new(topic_id),
            None,
        )
        .await
    }

    pub(crate) async fn ingest_event(
        &self,
        replica: &ReplicaId,
        envelope: KukuriEnvelope,
        _stored_blob: Option<StoredBlob>,
        attachments: Vec<(AssetRole, StoredBlob)>,
    ) -> Result<()> {
        self.store.put_envelope(envelope.clone()).await?;
        let mut object = envelope
            .to_post_object()?
            .ok_or_else(|| anyhow::anyhow!("expected timeline envelope"))?;
        if object.object_kind != "repost" {
            object.attachments = attachments
                .iter()
                .map(|(role, stored)| kukuri_core::AssetRef {
                    hash: stored.hash.clone(),
                    mime: stored.mime.clone(),
                    bytes: stored.bytes,
                    role: role.clone(),
                })
                .collect();
        }
        let content = match &object.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => self
                .blob_service
                .fetch_blob(hash)
                .await?
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string()),
        };
        persist_post_object(
            self.docs_sync.as_ref(),
            replica,
            object.clone(),
            envelope.clone(),
        )
        .await?;
        ProjectionStore::put_object_projection(
            self.projection_store.as_ref(),
            projection_row_from_header(&object, content, replica),
        )
        .await?;
        if let PayloadRef::BlobText { hash, .. } = &object.payload_ref {
            ProjectionStore::mark_blob_status(
                self.projection_store.as_ref(),
                hash,
                BlobCacheStatus::Available,
            )
            .await?;
        }
        for (_, attachment) in attachments {
            ProjectionStore::mark_blob_status(
                self.projection_store.as_ref(),
                &attachment.hash,
                BlobCacheStatus::Available,
            )
            .await?;
        }
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    pub(crate) async fn resolve_parent_object(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<KukuriEnvelope>> {
        if let Some(envelope) = self.store.get_envelope(object_id).await? {
            return Ok(Some(envelope));
        }

        let Some(projection) =
            ProjectionStore::get_object_projection(self.projection_store.as_ref(), object_id)
                .await?
        else {
            return Ok(None);
        };

        let object_kind = projection.object_kind.as_str();
        let mut tags = vec![
            vec!["topic".into(), projection.topic_id.clone()],
            vec!["object".into(), object_kind.to_string()],
        ];
        if projection.channel_id != PUBLIC_CHANNEL_ID {
            tags.push(vec!["channel".into(), projection.channel_id.clone()]);
        }

        Ok(Some(KukuriEnvelope {
            id: projection.object_id,
            pubkey: projection.author_pubkey.into(),
            created_at: projection.created_at,
            kind: object_kind.into(),
            tags,
            content: serde_json::to_string(&kukuri_core::KukuriPostEnvelopeContentV1 {
                object_kind: object_kind.into(),
                topic_id: TopicId::new(projection.topic_id.clone()),
                channel_id: channel_id_from_storage(projection.channel_id.as_str()),
                payload_ref: projection.payload_ref.clone(),
                attachments: Vec::new(),
                media_manifest_refs: Vec::new(),
                visibility: if projection.channel_id == PUBLIC_CHANNEL_ID {
                    ObjectVisibility::Public
                } else {
                    ObjectVisibility::Private
                },
                reply_to: projection.reply_to_object_id.clone(),
                root_id: projection.root_object_id.clone(),
                repost_of: projection.repost_of.clone(),
            })?,
            sig: String::new(),
        }))
    }

    pub(crate) async fn ensure_scope_subscriptions(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<()> {
        self.maybe_redeem_rotation_grants_for_scope(topic_id, scope)
            .await?;
        self.ensure_topic_subscription(topic_id).await?;
        match scope {
            TimelineScope::Public => Ok(()),
            TimelineScope::AllJoined => {
                self.ensure_joined_private_channel_subscriptions(topic_id)
                    .await
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
                    .await
            }
        }
    }

    pub(crate) async fn scope_needs_current_private_epoch_hydration(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
        page: &Page<ObjectProjectionRow>,
    ) -> bool {
        let TimelineScope::Channel { channel_id } = scope else {
            return false;
        };
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id.as_str())
            .await
        else {
            return false;
        };
        if state.archived_epochs.is_empty() {
            return false;
        }
        let current_replica = current_private_channel_replica_id(&state);
        !page
            .items
            .iter()
            .any(|item| item.source_replica_id == current_replica)
    }

    pub(crate) async fn allowed_channel_ids_for_scope(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<BTreeSet<String>> {
        let mut allowed = BTreeSet::new();
        match scope {
            TimelineScope::Public => {
                allowed.insert(PUBLIC_CHANNEL_ID.to_string());
            }
            TimelineScope::AllJoined => {
                allowed.insert(PUBLIC_CHANNEL_ID.to_string());
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    allowed.insert(state.channel_id.as_str().to_string());
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                allowed.insert(channel_id.as_str().to_string());
            }
        }
        Ok(allowed)
    }

    pub(crate) async fn hydrate_scope_projection(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<usize> {
        let mut hydrated = hydrate_topic_state_with_services(
            self.docs_sync.as_ref(),
            self.blob_service.as_ref(),
            self.projection_store.as_ref(),
            topic_id,
        )
        .await?;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        hydrated += hydrate_subscription_state_with_services(
                            self.docs_sync.as_ref(),
                            self.blob_service.as_ref(),
                            self.projection_store.as_ref(),
                            topic_id,
                            &replica,
                        )
                        .await?;
                    }
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                if let Some(state) = self
                    .joined_private_channel_state(topic_id, channel_id.as_str())
                    .await
                {
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        hydrated += hydrate_subscription_state_with_services(
                            self.docs_sync.as_ref(),
                            self.blob_service.as_ref(),
                            self.projection_store.as_ref(),
                            topic_id,
                            &replica,
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(hydrated)
    }

    pub(crate) async fn maybe_restart_scope_replica_sync(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) {
        self.maybe_restart_replica_sync(topic_id, &topic_replica_id(topic_id))
            .await;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    self.maybe_restart_private_channel_subscription(
                        topic_id,
                        state.channel_id.as_str(),
                    )
                    .await;
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        self.maybe_restart_replica_sync(topic_id, &replica).await;
                    }
                }
            }
            TimelineScope::Channel { channel_id } => {
                if let Some(state) = self
                    .joined_private_channel_state(topic_id, channel_id.as_str())
                    .await
                {
                    self.maybe_restart_private_channel_subscription(topic_id, channel_id.as_str())
                        .await;
                    for replica in
                        private_channel_epoch_capabilities(&state)
                            .into_iter()
                            .map(|epoch| {
                                private_channel_replica_for_epoch(
                                    state.channel_id.as_str(),
                                    epoch.epoch_id.as_str(),
                                )
                            })
                    {
                        self.maybe_restart_replica_sync(topic_id, &replica).await;
                    }
                }
            }
        }
    }

    pub(crate) async fn maybe_restart_replica_sync(&self, topic_id: &str, replica: &ReplicaId) {
        let key = replica.as_str().to_string();
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self.docs_sync.restart_replica_sync(replica).await {
            warn!(
                topic = %topic_id,
                replica = %replica.as_str(),
                error = %error,
                "failed to restart replica sync"
            );
        }
    }

    pub(crate) async fn maybe_restart_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) {
        let key = format!("private-channel:{topic_id}:{channel_id}");
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self
            .restart_private_channel_subscription(topic_id, channel_id)
            .await
        {
            warn!(
                topic = %topic_id,
                channel_id = %channel_id,
                error = %error,
                "failed to restart private channel subscription"
            );
        }
    }

    pub(crate) async fn maybe_restart_topic_subscription(&self, topic_id: &str) {
        let key = format!("topic-subscription:{topic_id}");
        let now = Utc::now().timestamp();
        {
            let mut deadlines = self.replica_sync_restart_deadlines.lock().await;
            let next_due_at = deadlines.get(key.as_str()).copied().unwrap_or_default();
            if next_due_at > now {
                return;
            }
            deadlines.insert(key, now.saturating_add(REPLICA_SYNC_RESTART_RETRY_SECONDS));
        }
        if let Err(error) = self.restart_topic_subscription(topic_id).await {
            warn!(
                topic = %topic_id,
                error = %error,
                "failed to restart topic subscription"
            );
        }
    }

    pub(crate) async fn maybe_restart_scope_subscription(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) {
        self.maybe_restart_topic_subscription(topic_id).await;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    self.maybe_restart_private_channel_subscription(
                        topic_id,
                        state.channel_id.as_str(),
                    )
                    .await;
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.maybe_restart_private_channel_subscription(topic_id, channel_id.as_str())
                    .await;
            }
        }
    }

    pub(crate) async fn page_to_view(
        &self,
        page: Page<ObjectProjectionRow>,
    ) -> Result<TimelineView> {
        let local_author = self.current_author_pubkey();
        let mut author_pubkeys = BTreeSet::new();
        let mut targets_by_replica = BTreeMap::<String, Vec<EnvelopeId>>::new();
        for row in &page.items {
            author_pubkeys.insert(row.author_pubkey.clone());
            if let Some(repost_of) = row.repost_of.as_ref() {
                author_pubkeys.insert(repost_of.source_author_pubkey.as_str().to_string());
            }
            targets_by_replica
                .entry(row.source_replica_id.as_str().to_string())
                .or_default()
                .push(row.object_id.clone());
        }

        let author_pubkeys = author_pubkeys.into_iter().collect::<Vec<_>>();
        let profiles = self.store.get_profiles(&author_pubkeys).await?;
        let relationships = self
            .projection_store
            .list_author_relationships(local_author.as_str(), &author_pubkeys)
            .await?;
        let mut reactions_by_target = HashMap::<String, Vec<ReactionProjectionRow>>::new();
        for (replica_id, object_ids) in targets_by_replica {
            let grouped = self
                .projection_store
                .list_reaction_cache_for_targets(&ReplicaId::new(replica_id.clone()), &object_ids)
                .await?;
            for (object_id, rows) in grouped {
                reactions_by_target.insert(format!("{replica_id}:{object_id}"), rows);
            }
        }

        let mut items = Vec::with_capacity(page.items.len());
        for row in page.items {
            items.push(
                self.row_to_view_with_cache(row, &profiles, &relationships, &reactions_by_target)
                    .await?,
            );
        }
        Ok(TimelineView {
            items,
            next_cursor: page.next_cursor,
        })
    }

    pub(crate) async fn row_to_view_with_cache(
        &self,
        row: ObjectProjectionRow,
        profiles: &HashMap<String, Profile>,
        relationships: &HashMap<String, AuthorRelationshipProjectionRow>,
        reactions_by_target: &HashMap<String, Vec<ReactionProjectionRow>>,
    ) -> Result<PostView> {
        let profile = profiles.get(row.author_pubkey.as_str());
        let relationship = relationships.get(row.author_pubkey.as_str());
        let repost_commentary = normalize_repost_commentary(row.content.clone());
        let content_status = if row.object_kind == "repost" {
            BlobViewStatus::Available
        } else {
            blob_view_status_for_payload(self.blob_service.as_ref(), &row.payload_ref).await?
        };
        let attachments = self.attachment_views_for_projection_row(&row).await?;
        let repost_of = match row.repost_of.clone() {
            Some(snapshot) => Some(
                self.repost_snapshot_to_view_with_profiles(snapshot, profiles)
                    .await?,
            ),
            None => None,
        };
        let audience_label = self
            .audience_label_for_storage(row.topic_id.as_str(), row.channel_id.as_str())
            .await;
        let reaction_state = reaction_state_view_from_rows(
            &row.source_replica_id,
            &row.object_id,
            reactions_by_target
                .get(reaction_cache_key(&row.source_replica_id, &row.object_id).as_str())
                .cloned()
                .unwrap_or_default(),
            self.current_author_pubkey().as_str(),
        );

        Ok(PostView {
            object_id: row.object_id.0.clone(),
            envelope_id: row.source_envelope_id.0.clone(),
            author_pubkey: row.author_pubkey.clone(),
            author_name: profile.and_then(|profile| profile.name.clone()),
            author_display_name: profile.and_then(|profile| profile.display_name.clone()),
            following: relationship.is_some_and(|value| value.following),
            followed_by: relationship.is_some_and(|value| value.followed_by),
            mutual: relationship.is_some_and(|value| value.mutual),
            friend_of_friend: relationship.is_some_and(|value| value.friend_of_friend),
            content: row.content.unwrap_or_else(|| "[blob pending]".to_string()),
            content_status,
            attachments,
            created_at: row.created_at,
            reply_to: row.reply_to_object_id.clone().map(|id| id.0),
            root_id: row.root_object_id.clone().map(|id| id.0),
            object_kind: row.object_kind.clone(),
            published_topic_id: Some(row.topic_id.clone()),
            origin_topic_id: Some(row.topic_id.clone()),
            repost_of,
            repost_commentary: repost_commentary.clone(),
            is_threadable: row.object_kind != "repost" || repost_commentary.is_some(),
            channel_id: channel_id_for_view(row.channel_id.as_str()),
            audience_label,
            reaction_summary: reaction_state.reaction_summary,
            my_reactions: reaction_state.my_reactions,
        })
    }

    pub(crate) async fn attachment_views_for_projection_row(
        &self,
        row: &ObjectProjectionRow,
    ) -> Result<Vec<AttachmentView>> {
        if row.object_kind == "repost" {
            return Ok(Vec::new());
        }
        if !row.attachments.is_empty() || row.projection_version >= 2 {
            return attachment_views_from_refs(self.blob_service.as_ref(), &row.attachments).await;
        }

        let post_object = fetch_post_object_for_projection(
            self.docs_sync.as_ref(),
            &row.source_replica_id,
            row.source_key.as_str(),
        )
        .await?;
        if let Some(post_object) = post_object {
            return attachment_views(self.blob_service.as_ref(), &post_object).await;
        }
        Ok(Vec::new())
    }

    pub(crate) async fn bookmarked_post_view_from_row(
        &self,
        row: BookmarkedPostRow,
    ) -> Result<BookmarkedPostView> {
        let profile = self.store.get_profile(row.author_pubkey.as_str()).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                row.author_pubkey.as_str(),
            )
            .await?;
        let content_status = if row.object_kind == "repost" {
            BlobViewStatus::Available
        } else {
            blob_view_status_for_payload(self.blob_service.as_ref(), &row.payload_ref).await?
        };
        let attachments = if row.object_kind == "repost" {
            Vec::new()
        } else {
            attachment_views_from_refs(self.blob_service.as_ref(), &row.attachments).await?
        };
        let repost_commentary = normalize_repost_commentary(row.content.clone());
        let repost_of = match row.repost_of.clone() {
            Some(snapshot) => Some(self.repost_snapshot_to_view(snapshot).await?),
            None => None,
        };
        let audience_label = self
            .audience_label_for_storage(row.topic_id.as_str(), row.channel_id.as_str())
            .await;
        let reaction_state = self
            .reaction_state_for_target(&row.source_replica_id, &row.source_object_id)
            .await?;

        Ok(BookmarkedPostView {
            bookmarked_at: row.bookmarked_at,
            post: PostView {
                object_id: row.source_object_id.as_str().to_string(),
                envelope_id: row.source_envelope_id.as_str().to_string(),
                author_pubkey: row.author_pubkey.clone(),
                author_name: profile.as_ref().and_then(|profile| profile.name.clone()),
                author_display_name: profile
                    .as_ref()
                    .and_then(|profile| profile.display_name.clone()),
                following: relationship.as_ref().is_some_and(|value| value.following),
                followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
                mutual: relationship.as_ref().is_some_and(|value| value.mutual),
                friend_of_friend: relationship
                    .as_ref()
                    .is_some_and(|value| value.friend_of_friend),
                object_kind: row.object_kind.clone(),
                content: row.content.unwrap_or_else(|| "[blob pending]".to_string()),
                content_status,
                attachments,
                created_at: row.created_at,
                reply_to: row.reply_to_object_id.map(|id| id.0),
                root_id: row.root_object_id.map(|id| id.0),
                published_topic_id: Some(row.topic_id.clone()),
                origin_topic_id: Some(row.topic_id.clone()),
                repost_of,
                repost_commentary: repost_commentary.clone(),
                is_threadable: row.object_kind != "repost" || repost_commentary.is_some(),
                channel_id: channel_id_for_view(row.channel_id.as_str()),
                audience_label,
                reaction_summary: reaction_state.reaction_summary,
                my_reactions: reaction_state.my_reactions,
            },
        })
    }

    pub(crate) async fn profile_post_to_view(&self, profile_post: ProfilePost) -> Result<PostView> {
        let profile = self
            .store
            .get_profile(profile_post.author_pubkey.as_str())
            .await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                profile_post.author_pubkey.as_str(),
            )
            .await?;

        Ok(PostView {
            object_id: profile_post.object_id.0.clone(),
            envelope_id: profile_post.object_id.0.clone(),
            author_pubkey: profile_post.author_pubkey.as_str().to_string(),
            author_name: profile.as_ref().and_then(|value| value.name.clone()),
            author_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            following: relationship.as_ref().is_some_and(|value| value.following),
            followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
            mutual: relationship.as_ref().is_some_and(|value| value.mutual),
            friend_of_friend: relationship
                .as_ref()
                .is_some_and(|value| value.friend_of_friend),
            object_kind: profile_post.object_kind,
            content: profile_post.content,
            content_status: BlobViewStatus::Available,
            attachments: attachment_views_from_refs(
                self.blob_service.as_ref(),
                &profile_post.attachments,
            )
            .await?,
            created_at: profile_post.created_at,
            reply_to: profile_post.reply_to_object_id.map(|id| id.0),
            root_id: profile_post.root_id.map(|id| id.0),
            published_topic_id: Some(profile_post.published_topic_id.as_str().to_string()),
            origin_topic_id: Some(profile_post.published_topic_id.as_str().to_string()),
            repost_of: None,
            repost_commentary: None,
            is_threadable: true,
            channel_id: None,
            audience_label: "Public".into(),
            reaction_summary: Vec::new(),
            my_reactions: Vec::new(),
        })
    }

    pub(crate) async fn profile_repost_to_view(
        &self,
        profile_repost: ProfileRepost,
    ) -> Result<PostView> {
        let profile = self
            .store
            .get_profile(profile_repost.author_pubkey.as_str())
            .await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                profile_repost.author_pubkey.as_str(),
            )
            .await?;

        Ok(PostView {
            object_id: profile_repost.object_id.0.clone(),
            envelope_id: profile_repost.envelope_id.0.clone(),
            author_pubkey: profile_repost.author_pubkey.as_str().to_string(),
            author_name: profile.as_ref().and_then(|value| value.name.clone()),
            author_display_name: profile
                .as_ref()
                .and_then(|value| value.display_name.clone()),
            following: relationship.as_ref().is_some_and(|value| value.following),
            followed_by: relationship.as_ref().is_some_and(|value| value.followed_by),
            mutual: relationship.as_ref().is_some_and(|value| value.mutual),
            friend_of_friend: relationship
                .as_ref()
                .is_some_and(|value| value.friend_of_friend),
            object_kind: "repost".into(),
            content: profile_repost.commentary.clone().unwrap_or_default(),
            content_status: BlobViewStatus::Available,
            attachments: Vec::new(),
            created_at: profile_repost.created_at,
            reply_to: None,
            root_id: None,
            published_topic_id: Some(profile_repost.published_topic_id.as_str().to_string()),
            origin_topic_id: Some(profile_repost.published_topic_id.as_str().to_string()),
            repost_of: Some(
                self.repost_snapshot_to_view(profile_repost.repost_of)
                    .await?,
            ),
            repost_commentary: profile_repost.commentary.clone(),
            is_threadable: profile_repost.commentary.is_some(),
            channel_id: None,
            audience_label: "Public".into(),
            reaction_summary: Vec::new(),
            my_reactions: Vec::new(),
        })
    }

    pub(crate) async fn repost_snapshot_to_view(
        &self,
        snapshot: RepostSourceSnapshotV1,
    ) -> Result<RepostSourceView> {
        let profiles = self
            .store
            .get_profiles(&[snapshot.source_author_pubkey.as_str().to_string()])
            .await?;
        self.repost_snapshot_to_view_with_profiles(snapshot, &profiles)
            .await
    }

    pub(crate) async fn repost_snapshot_to_view_with_profiles(
        &self,
        snapshot: RepostSourceSnapshotV1,
        profiles: &HashMap<String, Profile>,
    ) -> Result<RepostSourceView> {
        let source_profile = profiles.get(snapshot.source_author_pubkey.as_str());
        Ok(RepostSourceView {
            source_object_id: snapshot.source_object_id.as_str().to_string(),
            source_topic_id: snapshot.source_topic_id.as_str().to_string(),
            source_author_pubkey: snapshot.source_author_pubkey.as_str().to_string(),
            source_author_name: source_profile.and_then(|value| value.name.clone()),
            source_author_display_name: source_profile.and_then(|value| value.display_name.clone()),
            source_object_kind: snapshot.source_object_kind,
            content: snapshot.content,
            attachments: attachment_views_from_refs(
                self.blob_service.as_ref(),
                &snapshot.attachments,
            )
            .await?,
            reply_to: snapshot.reply_to_object_id.map(|id| id.0),
            root_id: snapshot.root_id.map(|id| id.0),
        })
    }
}

pub(crate) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

pub(crate) fn profile_asset_view_from_ref(
    asset: Option<&kukuri_core::AssetRef>,
) -> Option<ProfileAssetView> {
    asset.map(|asset| ProfileAssetView {
        hash: asset.hash.as_str().to_string(),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        role: "profile_avatar".into(),
    })
}

pub(crate) fn normalize_repost_commentary(value: Option<String>) -> Option<String> {
    normalize_optional_text(value)
}

pub(crate) fn content_from_payload_ref(payload_ref: &PayloadRef) -> Option<String> {
    match payload_ref {
        PayloadRef::InlineText { text } => Some(text.clone()),
        PayloadRef::BlobText { .. } => None,
    }
}

pub(crate) async fn notification_candidate_from_object_event(
    projection_store: &dyn ProjectionStore,
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    local_author_pubkey: &str,
    event: &DocEvent,
) -> Result<Option<NotificationCandidate>> {
    if event.source_peer.is_none()
        || !event.key.starts_with("objects/")
        || !event.key.ends_with("/state")
    {
        return Ok(None);
    }
    let Some(record) = docs_sync
        .query_replica(&event.replica_id, DocQuery::Exact(event.key.clone()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
    if header.author.as_str() == local_author_pubkey {
        return Ok(None);
    }
    let content = notification_text_from_payload_ref(blob_service, &header.payload_ref).await;
    let repost_commentary = if header.object_kind == "repost" {
        normalize_repost_commentary(content.clone())
    } else {
        None
    };
    let reply_preview = if header.object_kind == "repost" {
        repost_commentary.clone().or(content.clone())
    } else {
        content.clone()
    };
    if let Some(reply_to_object_id) = header.reply_to.as_ref()
        && projection_store
            .get_object_projection(reply_to_object_id)
            .await?
            .as_ref()
            .is_some_and(|row| row.author_pubkey == local_author_pubkey)
    {
        return Ok(Some(NotificationCandidate {
            kind: NotificationKind::Reply,
            actor_pubkey: header.author.as_str().to_string(),
            source_envelope_id: Some(header.envelope_id.clone()),
            source_replica_id: Some(event.replica_id.clone()),
            topic_id: Some(header.topic_id.as_str().to_string()),
            channel_id: header
                .channel_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
            object_id: Some(header.object_id.clone()),
            dm_id: None,
            message_id: None,
            preview_text: notification_preview_text(reply_preview),
            created_at: header.created_at,
            received_at: Utc::now().timestamp_millis(),
        }));
    }
    if header.channel_id.is_none()
        && let Some(repost_of) = header.repost_of.as_ref()
        && repost_of.source_author_pubkey.as_str() == local_author_pubkey
    {
        let (kind, preview_source) = if repost_commentary.is_some() {
            (NotificationKind::QuoteRepost, repost_commentary)
        } else {
            (
                NotificationKind::Repost,
                normalize_optional_text(Some(repost_of.content.clone())),
            )
        };
        return Ok(Some(NotificationCandidate {
            kind,
            actor_pubkey: header.author.as_str().to_string(),
            source_envelope_id: Some(header.envelope_id.clone()),
            source_replica_id: Some(event.replica_id.clone()),
            topic_id: Some(header.topic_id.as_str().to_string()),
            channel_id: None,
            object_id: Some(header.object_id.clone()),
            dm_id: None,
            message_id: None,
            preview_text: notification_preview_text(preview_source),
            created_at: header.created_at,
            received_at: Utc::now().timestamp_millis(),
        }));
    }
    let mention_source = if header.object_kind == "repost" {
        repost_commentary
    } else {
        normalize_optional_text(content)
    };
    if mention_source
        .as_deref()
        .is_some_and(|text| text_contains_pubkey_mention(text, local_author_pubkey))
    {
        return Ok(Some(NotificationCandidate {
            kind: NotificationKind::Mention,
            actor_pubkey: header.author.as_str().to_string(),
            source_envelope_id: Some(header.envelope_id.clone()),
            source_replica_id: Some(event.replica_id.clone()),
            topic_id: Some(header.topic_id.as_str().to_string()),
            channel_id: header
                .channel_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
            object_id: Some(header.object_id.clone()),
            dm_id: None,
            message_id: None,
            preview_text: notification_preview_text(mention_source),
            created_at: header.created_at,
            received_at: Utc::now().timestamp_millis(),
        }));
    }
    Ok(None)
}

pub(crate) async fn notification_candidate_from_follow_event(
    _store: &dyn Store,
    docs_sync: &dyn DocsSync,
    local_author_pubkey: &str,
    author_pubkey: &str,
    event: &DocEvent,
) -> Result<Option<NotificationCandidate>> {
    if event.source_peer.is_none() || !event.key.starts_with("graph/follows/") {
        return Ok(None);
    }
    let Some(record) = docs_sync
        .query_replica(&event.replica_id, DocQuery::Exact(event.key.clone()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let doc: FollowEdgeDocV1 = serde_json::from_slice(&record.value)?;
    if doc.subject_pubkey.as_str() != author_pubkey {
        return Ok(None);
    }
    let Some(envelope) =
        fetch_author_envelope_by_id(docs_sync, &event.replica_id, &doc.envelope_id).await?
    else {
        return Ok(None);
    };
    let Some(edge) = parse_follow_edge(&envelope)? else {
        return Ok(None);
    };
    if edge.subject_pubkey.as_str() == local_author_pubkey
        || edge.target_pubkey.as_str() != local_author_pubkey
        || edge.status != FollowEdgeStatus::Active
    {
        return Ok(None);
    }
    Ok(Some(NotificationCandidate {
        kind: NotificationKind::Followed,
        actor_pubkey: edge.subject_pubkey.as_str().to_string(),
        source_envelope_id: Some(edge.envelope_id.clone()),
        source_replica_id: Some(event.replica_id.clone()),
        topic_id: None,
        channel_id: None,
        object_id: None,
        dm_id: None,
        message_id: None,
        preview_text: None,
        created_at: edge.updated_at,
        received_at: Utc::now().timestamp_millis(),
    }))
}

pub(crate) async fn notification_text_from_payload_ref(
    blob_service: &dyn BlobService,
    payload_ref: &PayloadRef,
) -> Option<String> {
    match payload_ref {
        PayloadRef::InlineText { text } => Some(text.clone()),
        PayloadRef::BlobText { hash, .. } => fetch_projection_blob_text(blob_service, hash).await,
    }
}

pub(crate) fn notification_preview_text(value: Option<String>) -> Option<String> {
    normalize_optional_text(value)
        .map(|text| text.chars().take(NOTIFICATION_PREVIEW_LIMIT).collect())
}

pub(crate) fn notification_kind_key(kind: &NotificationKind) -> &'static str {
    match kind {
        NotificationKind::Mention => "mention",
        NotificationKind::Reply => "reply",
        NotificationKind::Repost => "repost",
        NotificationKind::QuoteRepost => "quote_repost",
        NotificationKind::DirectMessage => "direct_message",
        NotificationKind::Followed => "followed",
    }
}

pub(crate) fn notification_doc_event_fingerprint_parts(key: &str, content_hash: &str) -> String {
    format!("{key}|{content_hash}")
}

pub(crate) fn notification_doc_event_fingerprint(event: &DocEvent) -> String {
    notification_doc_event_fingerprint_parts(&event.key, &event.content_hash)
}

pub(crate) fn document_notification_id(
    recipient_pubkey: &str,
    kind: &NotificationKind,
    source_envelope_id: &EnvelopeId,
) -> String {
    format!(
        "notification:{recipient_pubkey}:{}:{}",
        notification_kind_key(kind),
        source_envelope_id.as_str()
    )
}

pub(crate) fn direct_message_notification_id(
    recipient_pubkey: &str,
    kind: &NotificationKind,
    dm_id: &str,
    message_id: &str,
) -> String {
    format!(
        "notification:{recipient_pubkey}:{}:{dm_id}:{message_id}",
        notification_kind_key(kind)
    )
}

pub(crate) fn text_contains_pubkey_mention(text: &str, pubkey: &str) -> bool {
    let bytes = text.as_bytes();
    let pubkey_bytes = pubkey.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'@' {
            let start = index + 1;
            let end = start + 64;
            if end <= bytes.len() {
                let candidate = &bytes[start..end];
                let next_is_hex = bytes
                    .get(end)
                    .is_some_and(|value| char::from(*value).is_ascii_hexdigit());
                if !next_is_hex
                    && candidate.len() == 64
                    && candidate
                        .iter()
                        .all(|value| char::from(*value).is_ascii_hexdigit())
                    && candidate.len() == pubkey_bytes.len()
                    && candidate
                        .iter()
                        .zip(pubkey_bytes.iter())
                        .all(|(left, right)| {
                            char::from(*left).eq_ignore_ascii_case(&char::from(*right))
                        })
                {
                    return true;
                }
            }
        }
        index += 1;
    }
    false
}

pub(crate) fn normalize_author_pubkey(pubkey: &str) -> Result<String> {
    let trimmed = pubkey.trim();
    if trimmed.len() != 64 || !trimmed.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!("invalid author pubkey"));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn author_social_view_from_parts(
    author_pubkey: &str,
    profile: Option<&Profile>,
    relationship: Option<&AuthorRelationshipProjectionRow>,
    muted: bool,
) -> AuthorSocialView {
    AuthorSocialView {
        author_pubkey: author_pubkey.to_string(),
        name: profile.and_then(|profile| profile.name.clone()),
        display_name: profile.and_then(|profile| profile.display_name.clone()),
        about: profile.and_then(|profile| profile.about.clone()),
        picture: profile.and_then(|profile| profile.picture.clone()),
        picture_asset: profile_asset_view_from_ref(
            profile.and_then(|profile| profile.picture_asset.as_ref()),
        ),
        updated_at: profile.map(|profile| profile.updated_at),
        following: relationship.is_some_and(|relationship| relationship.following),
        followed_by: relationship.is_some_and(|relationship| relationship.followed_by),
        mutual: relationship.is_some_and(|relationship| relationship.mutual),
        friend_of_friend: relationship.is_some_and(|relationship| relationship.friend_of_friend),
        friend_of_friend_via_pubkeys: relationship
            .map(|relationship| relationship.friend_of_friend_via_pubkeys.clone())
            .unwrap_or_default(),
        muted,
    }
}

pub(crate) fn author_social_view_sort_key(
    left: &AuthorSocialView,
    right: &AuthorSocialView,
) -> std::cmp::Ordering {
    fn key(value: Option<&str>) -> (u8, String) {
        let normalized = value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        match normalized {
            Some(value) => (0, value),
            None => (1, String::new()),
        }
    }

    key(left.display_name.as_deref())
        .cmp(&key(right.display_name.as_deref()))
        .then_with(|| key(left.name.as_deref()).cmp(&key(right.name.as_deref())))
        .then_with(|| left.author_pubkey.cmp(&right.author_pubkey))
}

pub(crate) async fn current_mutual_direct_message_peers_with_services(
    store: &dyn Store,
    local_author_pubkey: &str,
) -> Result<BTreeSet<String>> {
    let following = store
        .list_follow_edges_by_subject(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .map(|edge| edge.target_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let followed_by = store
        .list_follow_edges_by_target(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .map(|edge| edge.subject_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();
    Ok(following.intersection(&followed_by).cloned().collect())
}

pub(crate) async fn stop_direct_message_subscription_with_services(
    direct_message_subscriptions: &Mutex<HashMap<String, JoinHandle<()>>>,
    hint_transport: &dyn HintTransport,
    keys: &KukuriKeys,
    peer_pubkey: &str,
) -> Result<()> {
    let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
    if let Some(handle) = direct_message_subscriptions
        .lock()
        .await
        .remove(peer_pubkey.as_str())
    {
        handle.abort();
    }
    let topic = derive_direct_message_topic(keys, &Pubkey::from(peer_pubkey.as_str()))?;
    hint_transport.unsubscribe_hints(&topic).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn schedule_direct_message_reconcile_with_services(
    store: Arc<dyn Store>,
    projection_store: Arc<dyn ProjectionStore>,
    blob_service: Arc<dyn BlobService>,
    hint_transport: Arc<dyn HintTransport>,
    transport: Arc<dyn Transport>,
    keys: Arc<KukuriKeys>,
    last_sync: Arc<Mutex<Option<i64>>>,
    direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    local_author_pubkey: String,
    author_pubkey: String,
) {
    tokio::spawn(async move {
        if let Err(error) = reconcile_direct_message_subscriptions_with_services(
            store.as_ref(),
            projection_store,
            blob_service,
            hint_transport,
            transport,
            keys,
            last_sync,
            direct_message_subscriptions,
            local_author_pubkey.as_str(),
        )
        .await
        {
            warn!(
                author_pubkey = %author_pubkey,
                error = %error,
                "failed to reconcile direct message subscriptions after author update"
            );
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn reconcile_direct_message_subscriptions_with_services(
    store: &dyn Store,
    projection_store: Arc<dyn ProjectionStore>,
    blob_service: Arc<dyn BlobService>,
    hint_transport: Arc<dyn HintTransport>,
    transport: Arc<dyn Transport>,
    keys: Arc<KukuriKeys>,
    last_sync: Arc<Mutex<Option<i64>>>,
    direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    local_author_pubkey: &str,
) -> Result<()> {
    let desired_peers =
        current_mutual_direct_message_peers_with_services(store, local_author_pubkey).await?;
    let current_entries = {
        let subscriptions = direct_message_subscriptions.lock().await;
        subscriptions
            .iter()
            .map(|(peer_pubkey, handle)| (peer_pubkey.clone(), handle.is_finished()))
            .collect::<Vec<_>>()
    };

    for (peer_pubkey, finished) in &current_entries {
        if *finished || !desired_peers.contains(peer_pubkey) {
            stop_direct_message_subscription_with_services(
                direct_message_subscriptions.as_ref(),
                hint_transport.as_ref(),
                keys.as_ref(),
                peer_pubkey.as_str(),
            )
            .await?;
        }
    }

    for peer_pubkey in desired_peers {
        AppService::spawn_direct_message_subscription_with_services(
            Arc::clone(&direct_message_subscriptions),
            Arc::clone(&projection_store),
            Arc::clone(&blob_service),
            Arc::clone(&hint_transport),
            Arc::clone(&transport),
            Arc::clone(&keys),
            Arc::clone(&last_sync),
            local_author_pubkey,
            peer_pubkey.as_str(),
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn rebuild_author_relationships_with_services(
    store: &dyn Store,
    projection_store: &dyn ProjectionStore,
    local_author_pubkey: &str,
) -> Result<()> {
    let following_edges = store
        .list_follow_edges_by_subject(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .collect::<Vec<_>>();
    let followed_by_edges = store
        .list_follow_edges_by_target(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .collect::<Vec<_>>();

    let following = following_edges
        .iter()
        .map(|edge| edge.target_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let followed_by = followed_by_edges
        .iter()
        .map(|edge| edge.subject_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();

    let mut friend_of_friend_via = BTreeMap::<String, BTreeSet<String>>::new();
    for via_author in &following {
        for edge in store
            .list_follow_edges_by_subject(via_author.as_str())
            .await?
        {
            if edge.status != FollowEdgeStatus::Active {
                continue;
            }
            let target = edge.target_pubkey.as_str();
            if target == local_author_pubkey || following.contains(target) {
                continue;
            }
            friend_of_friend_via
                .entry(target.to_string())
                .or_default()
                .insert(via_author.clone());
        }
    }

    let derived_at = Utc::now().timestamp_millis();
    let mut author_pubkeys = BTreeSet::new();
    author_pubkeys.extend(following.iter().cloned());
    author_pubkeys.extend(followed_by.iter().cloned());
    author_pubkeys.extend(friend_of_friend_via.keys().cloned());
    author_pubkeys.remove(local_author_pubkey);

    let rows = author_pubkeys
        .into_iter()
        .map(|author_pubkey| {
            let following_flag = following.contains(author_pubkey.as_str());
            let followed_by_flag = followed_by.contains(author_pubkey.as_str());
            let via_pubkeys = friend_of_friend_via
                .get(author_pubkey.as_str())
                .map(|values| values.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            AuthorRelationshipProjectionRow {
                local_author_pubkey: local_author_pubkey.to_string(),
                author_pubkey: author_pubkey.clone(),
                following: following_flag,
                followed_by: followed_by_flag,
                mutual: following_flag && followed_by_flag,
                friend_of_friend: !following_flag && !via_pubkeys.is_empty(),
                friend_of_friend_via_pubkeys: via_pubkeys,
                derived_at,
            }
        })
        .collect::<Vec<_>>();
    projection_store
        .rebuild_author_relationships(local_author_pubkey, rows)
        .await
}

pub(crate) async fn persist_profile_doc(
    docs_sync: &dyn DocsSync,
    profile: &Profile,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(profile.pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("profile", "latest"),
                value: serde_json::to_value(AuthorProfileDocV1 {
                    author_pubkey: profile.pubkey.clone(),
                    name: profile.name.clone(),
                    display_name: profile.display_name.clone(),
                    about: profile.about.clone(),
                    picture: profile.picture.clone(),
                    picture_asset: profile.picture_asset.clone(),
                    updated_at: profile.updated_at,
                    envelope_id: envelope.id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_profile_post_doc(
    docs_sync: &dyn DocsSync,
    profile_post: &ProfilePost,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(profile_post.author_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("profile/posts", profile_post.object_id.as_str()),
                value: serde_json::to_value(AuthorProfilePostDocV1 {
                    author_pubkey: profile_post.author_pubkey.clone(),
                    profile_topic_id: profile_post.profile_topic_id.clone(),
                    published_topic_id: profile_post.published_topic_id.clone(),
                    object_id: profile_post.object_id.clone(),
                    created_at: profile_post.created_at,
                    object_kind: profile_post.object_kind.clone(),
                    content: profile_post.content.clone(),
                    attachments: profile_post.attachments.clone(),
                    reply_to_object_id: profile_post.reply_to_object_id.clone(),
                    root_id: profile_post.root_id.clone(),
                    envelope_id: envelope.id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_profile_repost_doc(
    docs_sync: &dyn DocsSync,
    profile_repost: &ProfileRepost,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(profile_repost.author_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("profile/reposts", profile_repost.object_id.as_str()),
                value: serde_json::to_value(AuthorProfileRepostDocV1 {
                    author_pubkey: profile_repost.author_pubkey.clone(),
                    profile_topic_id: profile_repost.profile_topic_id.clone(),
                    published_topic_id: profile_repost.published_topic_id.clone(),
                    object_id: profile_repost.object_id.clone(),
                    created_at: profile_repost.created_at,
                    commentary: profile_repost.commentary.clone(),
                    repost_of: profile_repost.repost_of.clone(),
                    envelope_id: envelope.id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_follow_edge_doc(
    docs_sync: &dyn DocsSync,
    edge: &FollowEdge,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(edge.subject_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("graph/follows", edge.target_pubkey.as_str()),
                value: serde_json::to_value(FollowEdgeDocV1 {
                    subject_pubkey: edge.subject_pubkey.clone(),
                    target_pubkey: edge.target_pubkey.clone(),
                    status: edge.status.clone(),
                    updated_at: edge.updated_at,
                    envelope_id: edge.envelope_id.clone(),
                })?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_custom_reaction_asset_doc(
    docs_sync: &dyn DocsSync,
    asset: &CustomReactionAssetDocV1,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    let replica = author_replica_id(asset.author_pubkey.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("reactions/assets", &format!("{}/state", asset.asset_id)),
                value: serde_json::to_value(asset)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("reactions/assets", &format!("{}/envelope", asset.asset_id)),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_reaction_doc(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    reaction: &ReactionDocV1,
    envelope: &KukuriEnvelope,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "reactions",
                    &format!(
                        "{}/{}/state",
                        reaction.target_object_id.as_str(),
                        reaction.reaction_id.as_str()
                    ),
                ),
                value: serde_json::to_value(reaction)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "reactions",
                    &format!(
                        "{}/{}/envelope",
                        reaction.target_object_id.as_str(),
                        reaction.reaction_id.as_str()
                    ),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("envelopes", envelope.id.as_str()),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn hydrate_author_state_with_services(
    docs_sync: &dyn DocsSync,
    store: &dyn Store,
    projection_store: &dyn ProjectionStore,
    local_author_pubkey: &str,
    author_pubkey: &str,
) -> Result<usize> {
    let replica = author_replica_id(author_pubkey);
    let mut count = 0usize;
    if let Some(record) = docs_sync
        .query_replica(&replica, DocQuery::Exact(stable_key("profile", "latest")))
        .await?
        .into_iter()
        .next()
    {
        match serde_json::from_slice::<AuthorProfileDocV1>(record.value.as_slice()) {
            Ok(doc) if doc.author_pubkey.as_str() == author_pubkey => {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                {
                    store.put_envelope(envelope.clone()).await?;
                    if let Some(profile) = parse_profile(&envelope)? {
                        projection_store.upsert_profile_cache(profile).await?;
                    }
                    count += 1;
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring profile doc with mismatched author"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode author profile doc"
                );
            }
        }
    }

    for record in docs_sync
        .query_replica(&replica, DocQuery::Prefix("graph/follows/".into()))
        .await?
    {
        match serde_json::from_slice::<FollowEdgeDocV1>(record.value.as_slice()) {
            Ok(doc) if doc.subject_pubkey.as_str() == author_pubkey => {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                    && let Some(edge) = parse_follow_edge(&envelope)?
                    && edge.target_pubkey == doc.target_pubkey
                    && edge.status == doc.status
                {
                    store.put_envelope(envelope).await?;
                    count += 1;
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring follow doc with mismatched subject"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode follow edge doc"
                );
            }
        }
    }

    rebuild_author_relationships_with_services(store, projection_store, local_author_pubkey)
        .await?;
    Ok(count)
}

pub(crate) async fn fetch_author_envelope_by_id(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    envelope_id: &EnvelopeId,
) -> Result<Option<KukuriEnvelope>> {
    let Some(record) = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("envelopes", envelope_id.as_str())),
        )
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(record.value.as_slice())?;
    envelope.verify()?;
    Ok(Some(envelope))
}

pub(crate) async fn load_custom_reaction_assets_from_author_replica(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Result<Vec<CustomReactionAssetDocV1>> {
    let replica = author_replica_id(author_pubkey);
    let mut items = Vec::new();
    for record in docs_sync
        .query_replica(
            &replica,
            DocQuery::Prefix(stable_key("reactions/assets", "")),
        )
        .await?
    {
        if !record.key.ends_with("/state") {
            continue;
        }
        let doc: CustomReactionAssetDocV1 = serde_json::from_slice(record.value.as_slice())?;
        if doc.author_pubkey.as_str() == author_pubkey {
            items.push(doc);
        }
    }
    Ok(items)
}

pub(crate) async fn load_profile_posts_from_author_replica(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Result<Vec<ProfilePost>> {
    let author_pubkey = normalize_author_pubkey(author_pubkey)?;
    let replica = author_replica_id(author_pubkey.as_str());
    let expected_profile_topic_id = author_profile_topic_id(author_pubkey.as_str());
    let mut items = Vec::new();
    let mut seen_object_ids = BTreeSet::new();

    for record in docs_sync
        .query_replica(&replica, DocQuery::Prefix("profile/posts/".into()))
        .await?
    {
        match serde_json::from_slice::<AuthorProfilePostDocV1>(record.value.as_slice()) {
            Ok(doc)
                if doc.author_pubkey.as_str() == author_pubkey
                    && doc.profile_topic_id == expected_profile_topic_id =>
            {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                {
                    match parse_profile_post(&envelope) {
                        Ok(Some(profile_post))
                            if profile_post.author_pubkey == doc.author_pubkey
                                && profile_post.profile_topic_id == doc.profile_topic_id
                                && profile_post.published_topic_id == doc.published_topic_id
                                && profile_post.object_id == doc.object_id
                                && profile_post.created_at == doc.created_at
                                && profile_post.object_kind == doc.object_kind
                                && profile_post.content == doc.content
                                && profile_post.attachments == doc.attachments
                                && profile_post.reply_to_object_id == doc.reply_to_object_id
                                && profile_post.root_id == doc.root_id =>
                        {
                            if seen_object_ids.insert(profile_post.object_id.clone()) {
                                items.push(profile_post);
                            }
                        }
                        Ok(Some(_)) | Ok(None) => {}
                        Err(error) => {
                            warn!(
                                author_pubkey = %author_pubkey,
                                key = %record.key,
                                envelope_id = %doc.envelope_id.as_str(),
                                error = %error,
                                "ignoring invalid profile post envelope"
                            );
                        }
                    }
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring profile post doc with mismatched author or topic"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode profile post doc"
                );
            }
        }
    }

    Ok(items)
}

pub(crate) async fn load_profile_reposts_from_author_replica(
    docs_sync: &dyn DocsSync,
    author_pubkey: &str,
) -> Result<Vec<ProfileRepost>> {
    let author_pubkey = normalize_author_pubkey(author_pubkey)?;
    let replica = author_replica_id(author_pubkey.as_str());
    let expected_profile_topic_id = author_profile_topic_id(author_pubkey.as_str());
    let mut items = Vec::new();
    let mut seen_object_ids = BTreeSet::new();

    for record in docs_sync
        .query_replica(&replica, DocQuery::Prefix("profile/reposts/".into()))
        .await?
    {
        match serde_json::from_slice::<AuthorProfileRepostDocV1>(record.value.as_slice()) {
            Ok(doc)
                if doc.author_pubkey.as_str() == author_pubkey
                    && doc.profile_topic_id == expected_profile_topic_id =>
            {
                if let Some(envelope) =
                    fetch_author_envelope_by_id(docs_sync, &replica, &doc.envelope_id).await?
                {
                    match parse_profile_repost(&envelope) {
                        Ok(Some(profile_repost))
                            if profile_repost.author_pubkey == doc.author_pubkey
                                && profile_repost.profile_topic_id == doc.profile_topic_id
                                && profile_repost.published_topic_id == doc.published_topic_id
                                && profile_repost.object_id == doc.object_id
                                && profile_repost.created_at == doc.created_at
                                && profile_repost.commentary == doc.commentary
                                && profile_repost.repost_of == doc.repost_of =>
                        {
                            if seen_object_ids.insert(profile_repost.object_id.clone()) {
                                items.push(profile_repost);
                            }
                        }
                        Ok(Some(_)) | Ok(None) => {}
                        Err(error) => {
                            warn!(
                                author_pubkey = %author_pubkey,
                                key = %record.key,
                                envelope_id = %doc.envelope_id.as_str(),
                                error = %error,
                                "ignoring invalid profile repost envelope"
                            );
                        }
                    }
                }
            }
            Ok(_) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    "ignoring profile repost doc with mismatched author or topic"
                );
            }
            Err(error) => {
                warn!(
                    author_pubkey = %author_pubkey,
                    key = %record.key,
                    error = %error,
                    "failed to decode profile repost doc"
                );
            }
        }
    }

    Ok(items)
}

pub(crate) async fn snapshot_object_notification_baseline(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<NotificationDocEventBaseline> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("objects/".into()))
        .await?;
    Ok(NotificationDocEventBaseline::from_records(
        &records
            .into_iter()
            .filter(|record| record.key.ends_with("/state"))
            .collect::<Vec<_>>(),
    ))
}

pub(crate) async fn snapshot_follow_notification_baseline(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<NotificationDocEventBaseline> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("graph/follows/".into()))
        .await?;
    Ok(NotificationDocEventBaseline::from_records(&records))
}

pub(crate) fn merge_seed_peers(
    configured_seed_peers: Vec<SeedPeer>,
    bootstrap_seed_peers: Vec<SeedPeer>,
) -> Vec<SeedPeer> {
    let mut deduped = BTreeMap::new();
    for seed_peer in configured_seed_peers
        .into_iter()
        .chain(bootstrap_seed_peers.into_iter())
    {
        let key = match seed_peer.addr_hint.as_deref() {
            Some(addr_hint) => format!("{}@{}", seed_peer.endpoint_id, addr_hint),
            None => seed_peer.endpoint_id.clone(),
        };
        deduped.insert(key, seed_peer);
    }
    deduped.into_values().collect()
}

pub(crate) async fn persist_post_object(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    object: CanonicalPostHeader,
    envelope: KukuriEnvelope,
) -> Result<()> {
    let sort_key = timeline_sort_key(object.created_at, &object.object_id);
    let object_json = serde_json::to_value(&object)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("objects", &format!("{}/state", object.object_id.as_str())),
                value: object_json,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "objects",
                    &format!("{}/envelope", object.object_id.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "indexes/timeline",
                    &format!("{sort_key}/{}", object.object_id.as_str()),
                ),
                value: serde_json::json!({
                    "object_id": object.object_id,
                    "created_at": object.created_at,
                    "object_kind": object.object_kind,
                }),
            },
        )
        .await?;
    let root_id = object
        .root
        .clone()
        .unwrap_or_else(|| object.object_id.clone());
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "indexes/thread",
                    &format!(
                        "{}/{sort_key}/{}",
                        root_id.as_str(),
                        object.object_id.as_str()
                    ),
                ),
                value: serde_json::json!({
                    "object_id": object.object_id,
                    "root_id": root_id,
                    "reply_to": object.reply_to,
                }),
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_media_manifest(
    replica: &ReplicaId,
    envelope: &KukuriEnvelope,
    manifest: &KukuriMediaManifestV1,
    docs_sync: &dyn DocsSync,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "manifests/media",
                    &format!("{}/state", manifest.manifest_id),
                ),
                value: serde_json::to_value(manifest)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "manifests/media",
                    &format!("{}/envelope", manifest.manifest_id),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_live_session_state(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    state: &LiveSessionStateDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_game_room_state(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    state: &GameRoomStateDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

pub(crate) async fn persist_private_channel_metadata(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    metadata: &PrivateChannelMetadataDocV1,
) -> Result<()> {
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "metadata"),
                value: serde_json::to_value(metadata)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "topic"),
                value: serde_json::json!({ "topic_id": metadata.topic_id }),
            },
        )
        .await
}

pub(crate) async fn persist_private_channel_policy(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    policy: &PrivateChannelPolicyDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_policy_envelope(keys, policy)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key("channels", "policy/envelope"),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_private_channel_participant(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    participant: &PrivateChannelParticipantDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_participant_envelope(keys, participant)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "channels/participants",
                    &format!("{}/envelope", participant.participant_pubkey.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn persist_private_channel_rotation_grant(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    grant: &PrivateChannelEpochHandoffGrantDocV1,
    replica: &ReplicaId,
) -> Result<()> {
    let envelope = build_private_channel_epoch_handoff_grant_envelope(keys, grant)?;
    docs_sync.open_replica(replica).await?;
    docs_sync
        .apply_doc_op(
            replica,
            DocOp::SetJson {
                key: stable_key(
                    "channels/rotation-grants",
                    &format!("{}/envelope", grant.recipient_pubkey.as_str()),
                ),
                value: serde_json::to_value(envelope)?,
            },
        )
        .await
}

pub(crate) async fn fetch_private_channel_metadata_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Option<PrivateChannelMetadataDocV1>> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(stable_key("channels", "metadata")))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let mut metadata: PrivateChannelMetadataDocV1 = serde_json::from_slice(&record.value)?;
    if metadata.owner_pubkey.as_str().trim().is_empty() {
        metadata.owner_pubkey = metadata.creator_pubkey.clone();
    }
    Ok(Some(metadata))
}

pub(crate) async fn fetch_private_channel_policy_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Option<PrivateChannelPolicyDocV1>> {
    let Some(record) = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("channels", "policy/envelope")),
        )
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
    envelope.verify()?;
    parse_private_channel_policy(&envelope)
}

pub(crate) async fn fetch_private_channel_participants_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
) -> Result<Vec<PrivateChannelParticipantDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Prefix(stable_key("channels/participants", "")),
        )
        .await?;
    let mut items = Vec::new();
    for record in records {
        if !record.key.ends_with("/envelope") {
            continue;
        }
        let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
        envelope.verify()?;
        if let Some(participant) = parse_private_channel_participant(&envelope)? {
            items.push(participant);
        }
    }
    items.sort_by(|left, right| left.participant_pubkey.cmp(&right.participant_pubkey));
    Ok(items)
}

pub(crate) async fn fetch_private_channel_rotation_grant_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    recipient_pubkey: &str,
) -> Result<Option<PrivateChannelEpochHandoffGrantDocV1>> {
    let Some(record) = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key(
                "channels/rotation-grants",
                &format!("{recipient_pubkey}/envelope"),
            )),
        )
        .await?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let envelope: KukuriEnvelope = serde_json::from_slice(&record.value)?;
    envelope.verify()?;
    parse_private_channel_epoch_handoff_grant(&envelope)
}

pub(crate) async fn wait_for_private_channel_epoch_snapshot(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    timeout_label: &str,
) -> Result<(
    PrivateChannelMetadataDocV1,
    PrivateChannelPolicyDocV1,
    Vec<PrivateChannelParticipantDocV1>,
)> {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let metadata = fetch_private_channel_metadata_from_replica(docs_sync, replica).await?;
            let policy = fetch_private_channel_policy_from_replica(docs_sync, replica).await?;
            let participants =
                fetch_private_channel_participants_from_replica(docs_sync, replica).await?;
            let owner_participant_visible = policy.as_ref().is_some_and(|policy| {
                participants.iter().any(|participant| {
                    participant.participant_pubkey == policy.owner_pubkey
                        && participant.epoch_id == policy.epoch_id
                        && participant.is_owner
                })
            });
            if let (Some(metadata), Some(policy)) = (metadata, policy)
                && owner_participant_visible
            {
                return Ok::<_, anyhow::Error>((metadata, policy, participants));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("timed out waiting for {timeout_label}"))?
}

pub(crate) async fn private_channel_rotation_is_pending(
    docs_sync: &dyn DocsSync,
    keys: &KukuriKeys,
    state: &JoinedPrivateChannelState,
) -> Result<bool> {
    let replica = current_private_channel_replica_id(state);
    let Some(policy) = fetch_private_channel_policy_from_replica(docs_sync, &replica).await? else {
        return Ok(false);
    };
    if policy.sharing_state != ChannelSharingState::Frozen || policy.rotated_at.is_none() {
        return Ok(false);
    }
    let local_author = keys.public_key_hex();
    let Some(grant) = fetch_private_channel_rotation_grant_from_replica(
        docs_sync,
        &replica,
        local_author.as_str(),
    )
    .await?
    else {
        return Ok(false);
    };
    let payload = decrypt_private_channel_epoch_handoff_grant(keys, &grant)?;
    Ok(payload.new_epoch_id != state.current_epoch_id)
}

pub(crate) async fn store_manifest_blob<T: Serialize>(
    blob_service: &dyn BlobService,
    manifest: &T,
    mime: &str,
) -> Result<StoredBlob> {
    let payload = serde_json::to_vec(manifest)?;
    blob_service.put_blob(payload, mime).await
}

pub(crate) async fn fetch_manifest_blob<T: DeserializeOwned>(
    blob_service: &dyn BlobService,
    blob_ref: &ManifestBlobRef,
) -> Result<Option<T>> {
    let Some(bytes) = blob_service.fetch_blob(&blob_ref.hash).await? else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

pub(crate) fn projection_blob_fetch_timeout() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_secs(5)
    } else {
        tokio::time::Duration::from_secs(2)
    }
}

pub(crate) fn projection_blob_status_timeout() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_secs(1)
    } else {
        tokio::time::Duration::from_millis(250)
    }
}

pub(crate) fn session_projection_retry_attempts() -> usize {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        20
    } else {
        10
    }
}

pub(crate) fn session_projection_retry_delay() -> tokio::time::Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        tokio::time::Duration::from_millis(500)
    } else {
        tokio::time::Duration::from_millis(250)
    }
}

pub(crate) async fn fetch_projection_blob_text(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> Option<String> {
    match tokio::time::timeout(
        projection_blob_fetch_timeout(),
        blob_service.fetch_blob(hash),
    )
    .await
    {
        Ok(Ok(Some(bytes))) => Some(String::from_utf8_lossy(&bytes).to_string()),
        Ok(Ok(None)) | Ok(Err(_)) | Err(_) => None,
    }
}

pub(crate) async fn best_effort_blob_cache_status(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> BlobCacheStatus {
    match tokio::time::timeout(
        projection_blob_status_timeout(),
        blob_service.blob_status(hash),
    )
    .await
    {
        Ok(Ok(status)) => blob_status(status),
        Ok(Err(_)) | Err(_) => BlobCacheStatus::Missing,
    }
}

pub(crate) async fn best_effort_blob_view_status(
    blob_service: &dyn BlobService,
    hash: &kukuri_core::BlobHash,
) -> BlobViewStatus {
    match tokio::time::timeout(
        projection_blob_status_timeout(),
        blob_service.blob_status(hash),
    )
    .await
    {
        Ok(Ok(status)) => blob_view_status(status),
        Ok(Err(_)) | Err(_) => BlobViewStatus::Missing,
    }
}

pub(crate) async fn fetch_live_session_state_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    session_id: &str,
) -> Result<Option<LiveSessionStateDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("sessions/live", &format!("{session_id}/state"))),
        )
        .await?;
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&record.value)?))
}

pub(crate) async fn fetch_game_room_state_from_replica(
    docs_sync: &dyn DocsSync,
    replica: &ReplicaId,
    room_id: &str,
) -> Result<Option<GameRoomStateDocV1>> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Exact(stable_key("sessions/game", &format!("{room_id}/state"))),
        )
        .await?;
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&record.value)?))
}

pub(crate) fn live_projection_row_from_state(
    state: &LiveSessionStateDocV1,
    manifest: &LiveSessionManifestBlobV1,
    topic_id: &str,
    source_replica_id: &ReplicaId,
) -> LiveSessionProjectionRow {
    LiveSessionProjectionRow {
        session_id: state.session_id.clone(),
        topic_id: topic_id.to_string(),
        channel_id: channel_storage_id(state.channel_id.as_ref()),
        host_pubkey: state.owner_pubkey.as_str().to_string(),
        title: manifest.title.clone(),
        description: manifest.description.clone(),
        status: state.status.clone(),
        started_at: manifest.started_at,
        ended_at: manifest.ended_at,
        updated_at: state.updated_at,
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
        viewer_count: 0,
    }
}

pub(crate) fn game_projection_row_from_state(
    state: &GameRoomStateDocV1,
    manifest: &GameRoomManifestBlobV1,
    topic_id: &str,
    source_replica_id: &ReplicaId,
) -> GameRoomProjectionRow {
    GameRoomProjectionRow {
        room_id: state.room_id.clone(),
        topic_id: topic_id.to_string(),
        channel_id: channel_storage_id(state.channel_id.as_ref()),
        host_pubkey: state.owner_pubkey.as_str().to_string(),
        title: manifest.title.clone(),
        description: manifest.description.clone(),
        status: state.status.clone(),
        phase_label: manifest.phase_label.clone(),
        scores: manifest.scores.clone(),
        updated_at: state.updated_at,
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

pub(crate) fn projection_row_from_header(
    header: &CanonicalPostHeader,
    content: Option<String>,
    source_replica_id: &ReplicaId,
) -> ObjectProjectionRow {
    let source_blob_hash = match &header.payload_ref {
        PayloadRef::BlobText { hash, .. } => Some(hash.clone()),
        PayloadRef::InlineText { .. } => None,
    };
    ObjectProjectionRow {
        object_id: header.object_id.clone(),
        topic_id: header.topic_id.as_str().to_string(),
        channel_id: channel_storage_id(header.channel_id.as_ref()),
        author_pubkey: header.author.as_str().to_string(),
        created_at: header.created_at,
        object_kind: header.object_kind.clone(),
        root_object_id: header.root.clone(),
        reply_to_object_id: header.reply_to.clone(),
        payload_ref: header.payload_ref.clone(),
        content,
        attachments: header.attachments.clone(),
        repost_of: header.repost_of.clone(),
        source_replica_id: source_replica_id.clone(),
        source_key: stable_key("objects", &format!("{}/state", header.object_id.as_str())),
        source_envelope_id: header.envelope_id.clone(),
        source_blob_hash,
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 2,
    }
}

pub(crate) fn reaction_projection_row_from_doc(
    reaction: &ReactionDocV1,
    source_replica_id: &ReplicaId,
) -> ReactionProjectionRow {
    ReactionProjectionRow {
        source_replica_id: source_replica_id.clone(),
        target_object_id: reaction.target_object_id.clone(),
        reaction_id: reaction.reaction_id.clone(),
        author_pubkey: reaction.author_pubkey.as_str().to_string(),
        created_at: reaction.created_at,
        updated_at: reaction.updated_at,
        reaction_key_kind: reaction.reaction_key_kind.clone(),
        normalized_reaction_key: reaction.normalized_reaction_key.clone(),
        emoji: reaction.emoji.clone(),
        custom_asset_id: reaction.custom_asset_id.clone(),
        custom_asset_snapshot: reaction.custom_asset_snapshot.clone(),
        status: reaction.status.clone(),
        source_key: stable_key(
            "reactions",
            &format!(
                "{}/{}/state",
                reaction.target_object_id.as_str(),
                reaction.reaction_id.as_str()
            ),
        ),
        source_envelope_id: reaction.envelope_id.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

pub(crate) fn custom_reaction_asset_view_from_snapshot(
    snapshot: &CustomReactionAssetSnapshotV1,
) -> CustomReactionAssetView {
    CustomReactionAssetView {
        asset_id: snapshot.asset_id.clone(),
        owner_pubkey: snapshot.owner_pubkey.as_str().to_string(),
        blob_hash: snapshot.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(
            snapshot.search_key.as_str(),
            snapshot.asset_id.as_str(),
        ),
        mime: snapshot.mime.clone(),
        bytes: snapshot.bytes,
        width: snapshot.width,
        height: snapshot.height,
    }
}

pub(crate) fn custom_reaction_asset_view_from_doc(
    asset: &CustomReactionAssetDocV1,
) -> CustomReactionAssetView {
    CustomReactionAssetView {
        asset_id: asset.asset_id.clone(),
        owner_pubkey: asset.author_pubkey.as_str().to_string(),
        blob_hash: asset.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(asset.search_key.as_str(), asset.asset_id.as_str()),
        mime: asset.mime.clone(),
        bytes: asset.bytes,
        width: asset.width,
        height: asset.height,
    }
}

pub(crate) fn bookmarked_custom_reaction_view_from_row(
    row: BookmarkedCustomReactionRow,
) -> BookmarkedCustomReactionView {
    let asset_id = row.asset_id;
    BookmarkedCustomReactionView {
        asset_id: asset_id.clone(),
        owner_pubkey: row.owner_pubkey,
        blob_hash: row.blob_hash.as_str().to_string(),
        search_key: search_key_or_asset_id(row.search_key.as_str(), asset_id.as_str()),
        mime: row.mime,
        bytes: row.bytes,
        width: row.width,
        height: row.height,
    }
}

pub(crate) fn recent_reaction_view_from_projection(
    row: &ReactionProjectionRow,
) -> RecentReactionView {
    RecentReactionView {
        reaction_key_kind: reaction_key_kind_label(&row.reaction_key_kind).to_string(),
        normalized_reaction_key: row.normalized_reaction_key.clone(),
        emoji: row.emoji.clone(),
        custom_asset: row
            .custom_asset_snapshot
            .as_ref()
            .map(custom_reaction_asset_view_from_snapshot),
        updated_at: row.updated_at,
    }
}

pub(crate) fn reaction_key_kind_label(kind: &ReactionKeyKind) -> &'static str {
    match kind {
        ReactionKeyKind::Emoji => "emoji",
        ReactionKeyKind::CustomAsset => "custom_asset",
    }
}

pub(crate) fn reaction_key_view_from_projection(row: &ReactionProjectionRow) -> ReactionKeyView {
    ReactionKeyView {
        reaction_key_kind: reaction_key_kind_label(&row.reaction_key_kind).to_string(),
        normalized_reaction_key: row.normalized_reaction_key.clone(),
        emoji: row.emoji.clone(),
        custom_asset: row
            .custom_asset_snapshot
            .as_ref()
            .map(custom_reaction_asset_view_from_snapshot),
    }
}

pub(crate) fn reaction_cache_key(
    source_replica_id: &ReplicaId,
    target_object_id: &EnvelopeId,
) -> String {
    format!(
        "{}:{}",
        source_replica_id.as_str(),
        target_object_id.as_str()
    )
}

pub(crate) fn reaction_state_view_from_rows(
    source_replica_id: &ReplicaId,
    target_object_id: &EnvelopeId,
    rows: Vec<ReactionProjectionRow>,
    current_author: &str,
) -> ReactionStateView {
    let mut summary = BTreeMap::<String, ReactionSummaryView>::new();
    let mut my_reactions = Vec::new();
    for row in rows {
        let key_view = reaction_key_view_from_projection(&row);
        if row.status == ObjectStatus::Active {
            summary
                .entry(row.normalized_reaction_key.clone())
                .and_modify(|value| value.count += 1)
                .or_insert_with(|| ReactionSummaryView {
                    reaction_key_kind: key_view.reaction_key_kind.clone(),
                    normalized_reaction_key: key_view.normalized_reaction_key.clone(),
                    emoji: key_view.emoji.clone(),
                    custom_asset: key_view.custom_asset.clone(),
                    count: 1,
                });
            if row.author_pubkey == current_author {
                my_reactions.push(key_view);
            }
        }
    }
    ReactionStateView {
        target_object_id: target_object_id.as_str().to_string(),
        source_replica_id: source_replica_id.as_str().to_string(),
        reaction_summary: summary.into_values().collect(),
        my_reactions,
    }
}

pub(crate) fn search_key_or_asset_id(search_key: &str, asset_id: &str) -> String {
    let normalized = search_key.trim();
    if normalized.is_empty() {
        return asset_id.to_string();
    }
    normalized.to_string()
}

pub(crate) async fn hydrate_object_projection_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("objects/".into()))
        .await?;
    let mut hydrated = 0usize;
    let mut blob_statuses = Vec::new();
    let mut projections = Vec::new();
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
        let content = match &header.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => {
                let payload = fetch_projection_blob_text(blob_service, hash).await;
                blob_statuses.push((
                    hash.clone(),
                    match payload {
                        Some(_) => BlobCacheStatus::Available,
                        None => BlobCacheStatus::Missing,
                    },
                ));
                payload
            }
        };
        for attachment in &header.attachments {
            let status = best_effort_blob_cache_status(blob_service, &attachment.hash).await;
            blob_statuses.push((attachment.hash.clone(), status));
        }
        projections.push(projection_row_from_header(&header, content, replica));
        hydrated += 1;
    }
    projection_store.mark_blob_statuses(blob_statuses).await?;
    projection_store.put_object_projections(projections).await?;
    Ok(hydrated)
}

pub(crate) async fn hydrate_object_projection_from_record(
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
    let content = match &header.payload_ref {
        PayloadRef::InlineText { text } => Some(text.clone()),
        PayloadRef::BlobText { hash, .. } => {
            let payload = fetch_projection_blob_text(blob_service, hash).await;
            projection_store
                .mark_blob_status(
                    hash,
                    match payload {
                        Some(_) => BlobCacheStatus::Available,
                        None => BlobCacheStatus::Missing,
                    },
                )
                .await?;
            payload
        }
    };
    for attachment in &header.attachments {
        let status = best_effort_blob_cache_status(blob_service, &attachment.hash).await;
        projection_store
            .mark_blob_status(&attachment.hash, status)
            .await?;
    }
    projection_store
        .put_object_projection(projection_row_from_header(&header, content, replica))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_object_projection_from_key(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_object_projection_from_record(blob_service, projection_store, replica, record).await
}

pub(crate) async fn hydrate_reaction_cache_from_replica(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("reactions/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        let reaction: ReactionDocV1 = serde_json::from_slice(record.value.as_slice())?;
        projection_store
            .upsert_reaction_cache(reaction_projection_row_from_doc(&reaction, replica))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_reaction_cache_from_record(
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let reaction: ReactionDocV1 = serde_json::from_slice(record.value.as_slice())?;
    projection_store
        .upsert_reaction_cache(reaction_projection_row_from_doc(&reaction, replica))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_reaction_cache_from_key(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_reaction_cache_from_record(projection_store, replica, record).await
}

pub(crate) async fn hydrate_reaction_cache_for_target(
    docs_sync: &dyn DocsSync,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
    target_object_id: &str,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(
            replica,
            DocQuery::Prefix(stable_key("reactions", &format!("{target_object_id}/"))),
        )
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        hydrated +=
            hydrate_reaction_cache_from_record(projection_store, replica, record).await? as usize;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_topic_state_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
) -> Result<usize> {
    hydrate_subscription_state_with_services(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        &topic_replica_id(topic_id),
    )
    .await
}

pub(crate) async fn hydrate_subscription_state_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    let post_count =
        hydrate_object_projection_from_replica(docs_sync, blob_service, projection_store, replica)
            .await?;
    let reaction_count =
        hydrate_reaction_cache_from_replica(docs_sync, projection_store, replica).await?;
    let live_count = hydrate_live_sessions_from_replica(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
    )
    .await?;
    let game_count = hydrate_game_rooms_from_replica(
        docs_sync,
        blob_service,
        projection_store,
        topic_id,
        replica,
    )
    .await?;
    Ok(post_count + reaction_count + live_count + game_count)
}

pub(crate) async fn hydrate_live_sessions_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("sessions/live/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        let state: LiveSessionStateDocV1 = serde_json::from_slice(&record.value)?;
        projection_store
            .mark_blob_status(
                &state.current_manifest.hash,
                blob_status(
                    blob_service
                        .blob_status(&state.current_manifest.hash)
                        .await?,
                ),
            )
            .await?;
        let Some(manifest) =
            fetch_manifest_blob::<LiveSessionManifestBlobV1>(blob_service, &state.current_manifest)
                .await?
        else {
            continue;
        };
        projection_store
            .upsert_live_session_cache(live_projection_row_from_state(
                &state, &manifest, topic_id, replica,
            ))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_live_session_from_record(
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let state: LiveSessionStateDocV1 = serde_json::from_slice(&record.value)?;
    projection_store
        .mark_blob_status(
            &state.current_manifest.hash,
            blob_status(
                blob_service
                    .blob_status(&state.current_manifest.hash)
                    .await?,
            ),
        )
        .await?;
    let Some(manifest) =
        fetch_manifest_blob::<LiveSessionManifestBlobV1>(blob_service, &state.current_manifest)
            .await?
    else {
        return Ok(false);
    };
    projection_store
        .upsert_live_session_cache(live_projection_row_from_state(
            &state, &manifest, topic_id, replica,
        ))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_live_session_from_key(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_live_session_from_record(blob_service, projection_store, topic_id, replica, record)
        .await
}

pub(crate) async fn hydrate_live_session_from_key_with_retry(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<usize> {
    for attempt in 0..session_projection_retry_attempts() {
        if hydrate_live_session_from_key(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await?
        {
            return Ok(1);
        }
        if attempt + 1 < session_projection_retry_attempts() {
            tokio::time::sleep(session_projection_retry_delay()).await;
        }
    }
    Ok(0)
}

pub(crate) async fn hydrate_game_rooms_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("sessions/game/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        let state: GameRoomStateDocV1 = serde_json::from_slice(&record.value)?;
        projection_store
            .mark_blob_status(
                &state.current_manifest.hash,
                blob_status(
                    blob_service
                        .blob_status(&state.current_manifest.hash)
                        .await?,
                ),
            )
            .await?;
        let Some(manifest) =
            fetch_manifest_blob::<GameRoomManifestBlobV1>(blob_service, &state.current_manifest)
                .await?
        else {
            continue;
        };
        projection_store
            .upsert_game_room_cache(game_projection_row_from_state(
                &state, &manifest, topic_id, replica,
            ))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

pub(crate) async fn hydrate_game_room_from_record(
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    record: DocRecord,
) -> Result<bool> {
    let state: GameRoomStateDocV1 = serde_json::from_slice(&record.value)?;
    projection_store
        .mark_blob_status(
            &state.current_manifest.hash,
            blob_status(
                blob_service
                    .blob_status(&state.current_manifest.hash)
                    .await?,
            ),
        )
        .await?;
    let Some(manifest) =
        fetch_manifest_blob::<GameRoomManifestBlobV1>(blob_service, &state.current_manifest)
            .await?
    else {
        return Ok(false);
    };
    projection_store
        .upsert_game_room_cache(game_projection_row_from_state(
            &state, &manifest, topic_id, replica,
        ))
        .await?;
    Ok(true)
}

pub(crate) async fn hydrate_game_room_from_key(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<bool> {
    let Some(record) = docs_sync
        .query_replica(replica, DocQuery::Exact(key.to_string()))
        .await?
        .into_iter()
        .next()
    else {
        return Ok(false);
    };
    hydrate_game_room_from_record(blob_service, projection_store, topic_id, replica, record).await
}

pub(crate) async fn hydrate_game_room_from_key_with_retry(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<usize> {
    for attempt in 0..session_projection_retry_attempts() {
        if hydrate_game_room_from_key(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await?
        {
            return Ok(1);
        }
        if attempt + 1 < session_projection_retry_attempts() {
            tokio::time::sleep(session_projection_retry_delay()).await;
        }
    }
    Ok(0)
}

pub(crate) async fn hydrate_subscription_event_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    key: &str,
) -> Result<usize> {
    if key.starts_with("objects/") && key.ends_with("/state") {
        return Ok(hydrate_object_projection_from_key(
            docs_sync,
            blob_service,
            projection_store,
            replica,
            key,
        )
        .await? as usize);
    }
    if key.starts_with("reactions/") && key.ends_with("/state") {
        return Ok(
            hydrate_reaction_cache_from_key(docs_sync, projection_store, replica, key).await?
                as usize,
        );
    }
    if key.starts_with("sessions/live/") && key.ends_with("/state") {
        return hydrate_live_session_from_key_with_retry(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await;
    }
    if key.starts_with("sessions/game/") && key.ends_with("/state") {
        return hydrate_game_room_from_key_with_retry(
            docs_sync,
            blob_service,
            projection_store,
            topic_id,
            replica,
            key,
        )
        .await;
    }
    Ok(0)
}

pub(crate) async fn hydrate_subscription_hint_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
    hint: &GossipHint,
) -> Result<usize> {
    match hint {
        GossipHint::TopicObjectsChanged { objects, .. } => {
            let mut hydrated = 0usize;
            for object in objects {
                if object.object_kind == "reaction" {
                    hydrated += hydrate_reaction_cache_for_target(
                        docs_sync,
                        projection_store,
                        replica,
                        object.object_id.as_str(),
                    )
                    .await?;
                    continue;
                }
                hydrated += hydrate_object_projection_from_key(
                    docs_sync,
                    blob_service,
                    projection_store,
                    replica,
                    stable_key("objects", &format!("{}/state", object.object_id)).as_str(),
                )
                .await? as usize;
            }
            Ok(hydrated)
        }
        GossipHint::ThreadUpdated { object_ids, .. } => {
            let mut hydrated = 0usize;
            for object_id in object_ids {
                hydrated += hydrate_object_projection_from_key(
                    docs_sync,
                    blob_service,
                    projection_store,
                    replica,
                    stable_key("objects", &format!("{}/state", object_id.as_str())).as_str(),
                )
                .await? as usize;
            }
            Ok(hydrated)
        }
        GossipHint::SessionChanged {
            session_id,
            object_kind,
            ..
        } => match object_kind.as_str() {
            "live-session" => {
                hydrate_live_session_from_key_with_retry(
                    docs_sync,
                    blob_service,
                    projection_store,
                    topic_id,
                    replica,
                    stable_key("sessions/live", &format!("{session_id}/state")).as_str(),
                )
                .await
            }
            "game-session" => {
                hydrate_game_room_from_key_with_retry(
                    docs_sync,
                    blob_service,
                    projection_store,
                    topic_id,
                    replica,
                    stable_key("sessions/game", &format!("{session_id}/state")).as_str(),
                )
                .await
            }
            _ => Ok(0),
        },
        GossipHint::ProfileUpdated { .. }
        | GossipHint::Presence { .. }
        | GossipHint::Typing { .. }
        | GossipHint::LivePresence { .. }
        | GossipHint::DirectMessageFrame { .. }
        | GossipHint::DirectMessageAck { .. } => Ok(0),
    }
}

pub(crate) fn hint_targets_topic(hint: &GossipHint, topic: &str) -> bool {
    match hint {
        GossipHint::TopicObjectsChanged { topic_id, .. }
        | GossipHint::Presence { topic_id, .. }
        | GossipHint::Typing { topic_id, .. }
        | GossipHint::SessionChanged { topic_id, .. }
        | GossipHint::LivePresence { topic_id, .. }
        | GossipHint::DirectMessageFrame { topic_id, .. }
        | GossipHint::DirectMessageAck { topic_id, .. } => topic_id.as_str() == topic,
        GossipHint::ThreadUpdated { .. } | GossipHint::ProfileUpdated { .. } => true,
    }
}

pub(crate) fn projection_page_needs_hydration(page: &Page<ObjectProjectionRow>) -> bool {
    page.items.iter().any(|item| item.content.is_none())
}

pub(crate) fn profile_timeline_page(
    posts: Vec<ProfileTimelineItem>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<ProfileTimelineItem> {
    if limit == 0 {
        return Page {
            items: Vec::new(),
            next_cursor: cursor,
        };
    }

    let mut items = Vec::new();
    let mut next_cursor = None;
    for post in posts {
        let include = cursor.as_ref().is_none_or(|current| {
            post.created_at() < current.created_at
                || (post.created_at() == current.created_at
                    && post.object_id() < &current.object_id)
        });
        if !include {
            continue;
        }
        if items.len() >= limit {
            next_cursor = Some(TimelineCursor {
                created_at: post.created_at(),
                object_id: post.object_id().clone(),
            });
            break;
        }
        items.push(post);
    }

    Page { items, next_cursor }
}

pub(crate) async fn filtered_timeline_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channels: &BTreeSet<String>,
    muted_author_pubkeys: &BTreeSet<String>,
) -> Result<Page<ObjectProjectionRow>> {
    if limit == 0 {
        return Ok(Page {
            items: Vec::new(),
            next_cursor: cursor,
        });
    }
    let mut current_cursor = cursor;
    let mut items = Vec::new();
    let page_size = limit.max(20);
    loop {
        let page = ProjectionStore::list_topic_timeline_filtered(
            projection_store,
            topic_id,
            allowed_channels,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if !object_projection_row_is_muted(&row, muted_author_pubkeys) {
                items.push(row);
                if items.len() >= limit {
                    return Ok(Page { items, next_cursor });
                }
            }
        }
        if next_cursor.is_none() {
            return Ok(Page { items, next_cursor });
        }
        current_cursor = next_cursor;
    }
}

pub(crate) async fn filtered_thread_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    thread_root_object_id: &EnvelopeId,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channel: Option<&str>,
    muted_author_pubkeys: &BTreeSet<String>,
) -> Result<Page<ObjectProjectionRow>> {
    if limit == 0 {
        return Ok(Page {
            items: Vec::new(),
            next_cursor: cursor,
        });
    }
    let mut current_cursor = cursor;
    let mut items = Vec::new();
    let page_size = limit.max(20);
    loop {
        let page = ProjectionStore::list_thread_filtered(
            projection_store,
            topic_id,
            thread_root_object_id,
            allowed_channel,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if !object_projection_row_is_muted(&row, muted_author_pubkeys) {
                items.push(row);
                if items.len() >= limit {
                    return Ok(Page { items, next_cursor });
                }
            }
        }
        if next_cursor.is_none() {
            return Ok(Page { items, next_cursor });
        }
        current_cursor = next_cursor;
    }
}

pub(crate) fn filter_channel_rows<T>(
    rows: Vec<T>,
    allowed_channels: &BTreeSet<String>,
    channel_id: impl Fn(&T) -> &str,
) -> Vec<T> {
    rows.into_iter()
        .filter(|row| allowed_channels.contains(channel_id(row)))
        .collect()
}

pub(crate) fn object_projection_row_is_muted(
    row: &ObjectProjectionRow,
    muted_author_pubkeys: &BTreeSet<String>,
) -> bool {
    muted_author_pubkeys.contains(row.author_pubkey.as_str())
        || row.repost_of.as_ref().is_some_and(|snapshot| {
            muted_author_pubkeys.contains(snapshot.source_author_pubkey.as_str())
        })
}

pub(crate) fn bookmarked_post_row_is_muted(
    row: &BookmarkedPostRow,
    muted_author_pubkeys: &BTreeSet<String>,
) -> bool {
    muted_author_pubkeys.contains(row.author_pubkey.as_str())
        || row.repost_of.as_ref().is_some_and(|snapshot| {
            muted_author_pubkeys.contains(snapshot.source_author_pubkey.as_str())
        })
}

pub(crate) fn profile_timeline_item_is_muted(
    item: &ProfileTimelineItem,
    muted_author_pubkeys: &BTreeSet<String>,
) -> bool {
    match item {
        ProfileTimelineItem::Post(post) => {
            muted_author_pubkeys.contains(post.author_pubkey.as_str())
        }
        ProfileTimelineItem::Repost(repost) => {
            muted_author_pubkeys.contains(repost.author_pubkey.as_str())
                || muted_author_pubkeys.contains(repost.repost_of.source_author_pubkey.as_str())
        }
    }
}

pub(crate) async fn fetch_post_object_for_projection(
    docs_sync: &dyn DocsSync,
    replica_id: &ReplicaId,
    source_key: &str,
) -> Result<Option<CanonicalPostHeader>> {
    let Ok(records) = docs_sync
        .query_replica(replica_id, DocQuery::Exact(source_key.to_string()))
        .await
    else {
        return Ok(None);
    };
    let Some(record) = records.into_iter().next() else {
        return Ok(None);
    };
    let header = serde_json::from_slice(&record.value)?;
    Ok(Some(header))
}

pub(crate) fn legacy_epoch_id() -> &'static str {
    "legacy"
}

pub(crate) fn private_channel_is_epoch_aware(audience_kind: &ChannelAudienceKind) -> bool {
    let _ = audience_kind;
    true
}

pub(crate) fn initial_private_channel_epoch_id(
    audience_kind: &ChannelAudienceKind,
    now_ms: i64,
    owner_pubkey: &str,
) -> String {
    let _ = audience_kind;
    format!("epoch-{now_ms}-{}", short_id_suffix(owner_pubkey))
}

pub(crate) fn next_private_channel_epoch_id(owner_pubkey: &str) -> String {
    format!(
        "epoch-{}-{}",
        Utc::now().timestamp_millis(),
        short_id_suffix(owner_pubkey)
    )
}

pub(crate) fn private_channel_replica_for_epoch(channel_id: &str, epoch_id: &str) -> ReplicaId {
    if epoch_id == legacy_epoch_id() {
        return private_channel_replica_id(channel_id);
    }
    private_channel_epoch_replica_id(channel_id, epoch_id)
}

pub(crate) fn current_private_channel_replica_id(state: &JoinedPrivateChannelState) -> ReplicaId {
    private_channel_replica_for_epoch(state.channel_id.as_str(), state.current_epoch_id.as_str())
}

pub(crate) fn private_channel_epoch_capabilities(
    state: &JoinedPrivateChannelState,
) -> Vec<PrivateChannelEpochCapability> {
    let mut items = vec![PrivateChannelEpochCapability {
        epoch_id: state.current_epoch_id.clone(),
        namespace_secret_hex: state.current_epoch_secret_hex.clone(),
    }];
    for epoch in &state.archived_epochs {
        if items.iter().any(|item| item.epoch_id == epoch.epoch_id) {
            continue;
        }
        items.push(epoch.clone());
    }
    items
}

pub(crate) fn joined_private_channel_state_from_capability(
    capability: PrivateChannelCapability,
) -> Result<JoinedPrivateChannelState> {
    let current_epoch_id = if capability.current_epoch_id.trim().is_empty() {
        legacy_epoch_id().to_string()
    } else {
        capability.current_epoch_id
    };
    let current_epoch_secret_hex = if capability.current_epoch_secret_hex.trim().is_empty() {
        capability.namespace_secret_hex.clone()
    } else {
        capability.current_epoch_secret_hex
    };
    if current_epoch_secret_hex.trim().is_empty() {
        anyhow::bail!("private channel capability is missing current epoch secret");
    }
    let owner_pubkey = if capability.owner_pubkey.trim().is_empty() {
        capability.creator_pubkey.clone()
    } else {
        capability.owner_pubkey
    };
    Ok(JoinedPrivateChannelState {
        topic_id: capability.topic_id,
        channel_id: ChannelId::new(capability.channel_id),
        label: capability.label.trim().to_string(),
        creator_pubkey: capability.creator_pubkey,
        owner_pubkey,
        joined_via_pubkey: capability.joined_via_pubkey,
        audience_kind: capability.audience_kind,
        current_epoch_id,
        current_epoch_secret_hex,
        archived_epochs: capability.archived_epochs,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn merged_private_channel_state_from_epoch_join(
    existing: Option<JoinedPrivateChannelState>,
    topic_id: &str,
    channel_id: ChannelId,
    label: &str,
    creator_pubkey: &str,
    owner_pubkey: &str,
    joined_via_pubkey: Option<&str>,
    audience_kind: ChannelAudienceKind,
    epoch_id: &str,
    namespace_secret_hex: &str,
) -> JoinedPrivateChannelState {
    let mut archived_epochs = existing
        .as_ref()
        .map(|state| state.archived_epochs.clone())
        .unwrap_or_default();
    archived_epochs.retain(|epoch| epoch.epoch_id != epoch_id);
    if let Some(existing_state) = existing.as_ref()
        && existing_state.current_epoch_id != epoch_id
        && !archived_epochs
            .iter()
            .any(|epoch| epoch.epoch_id == existing_state.current_epoch_id)
    {
        archived_epochs.push(PrivateChannelEpochCapability {
            epoch_id: existing_state.current_epoch_id.clone(),
            namespace_secret_hex: existing_state.current_epoch_secret_hex.clone(),
        });
    }
    JoinedPrivateChannelState {
        topic_id: topic_id.to_string(),
        channel_id,
        label: label.to_string(),
        creator_pubkey: creator_pubkey.to_string(),
        owner_pubkey: owner_pubkey.to_string(),
        joined_via_pubkey: joined_via_pubkey.map(str::to_string),
        audience_kind,
        current_epoch_id: epoch_id.to_string(),
        current_epoch_secret_hex: namespace_secret_hex.to_string(),
        archived_epochs,
    }
}

pub(crate) fn archive_private_channel_epoch(
    state: &mut JoinedPrivateChannelState,
    epoch_id: &str,
    namespace_secret_hex: &str,
) {
    if state
        .archived_epochs
        .iter()
        .any(|epoch| epoch.epoch_id == epoch_id)
    {
        return;
    }
    state.archived_epochs.push(PrivateChannelEpochCapability {
        epoch_id: epoch_id.to_string(),
        namespace_secret_hex: namespace_secret_hex.to_string(),
    });
}

pub(crate) fn active_private_channel_participants(
    participants: &[PrivateChannelParticipantDocV1],
    epoch_id: &str,
) -> Vec<PrivateChannelParticipantDocV1> {
    participants
        .iter()
        .filter(|participant| participant.epoch_id == epoch_id)
        .cloned()
        .collect()
}

pub(crate) async fn register_private_channel_replica_secrets(
    docs_sync: &dyn DocsSync,
    state: &JoinedPrivateChannelState,
) -> Result<()> {
    for epoch in private_channel_epoch_capabilities(state) {
        let replica =
            private_channel_replica_for_epoch(state.channel_id.as_str(), epoch.epoch_id.as_str());
        docs_sync
            .register_private_replica_secret(&replica, epoch.namespace_secret_hex.as_str())
            .await?;
    }
    Ok(())
}

pub(crate) fn joined_private_channel_subscription_prefix(
    topic_id: &str,
    channel_id: &str,
) -> String {
    format!("{topic_id}::{channel_id}::")
}

pub(crate) fn joined_private_channel_subscription_key(
    topic_id: &str,
    channel_id: &str,
    replica: &ReplicaId,
) -> String {
    format!("{topic_id}::{channel_id}::{}", replica.as_str())
}

pub(crate) fn subscription_replicas_for_topic(
    topic_id: &str,
    joined_channels: Vec<JoinedPrivateChannelState>,
) -> Vec<ReplicaId> {
    let mut replicas = vec![topic_replica_id(topic_id)];
    replicas.extend(joined_channels.into_iter().flat_map(|state| {
        private_channel_epoch_capabilities(&state)
            .into_iter()
            .map(move |epoch| {
                private_channel_replica_for_epoch(
                    state.channel_id.as_str(),
                    epoch.epoch_id.as_str(),
                )
            })
    }));
    replicas
}

pub(crate) async fn blob_view_status_for_payload(
    blob_service: &dyn BlobService,
    payload_ref: &PayloadRef,
) -> Result<BlobViewStatus> {
    match payload_ref {
        PayloadRef::InlineText { .. } => Ok(BlobViewStatus::Available),
        PayloadRef::BlobText { hash, .. } => {
            Ok(best_effort_blob_view_status(blob_service, hash).await)
        }
    }
}

pub(crate) async fn attachment_views(
    blob_service: &dyn BlobService,
    header: &CanonicalPostHeader,
) -> Result<Vec<AttachmentView>> {
    let mut attachments = Vec::with_capacity(header.attachments.len());
    for attachment in &header.attachments {
        attachments.push(AttachmentView {
            hash: attachment.hash.as_str().to_string(),
            mime: attachment.mime.clone(),
            bytes: attachment.bytes,
            role: attachment_role_name(&attachment.role).to_string(),
            status: best_effort_blob_view_status(blob_service, &attachment.hash).await,
        });
    }
    Ok(attachments)
}

pub(crate) async fn attachment_views_from_refs(
    blob_service: &dyn BlobService,
    refs: &[kukuri_core::AssetRef],
) -> Result<Vec<AttachmentView>> {
    let mut attachments = Vec::with_capacity(refs.len());
    for attachment in refs {
        attachments.push(AttachmentView {
            hash: attachment.hash.as_str().to_string(),
            mime: attachment.mime.clone(),
            bytes: attachment.bytes,
            role: attachment_role_name(&attachment.role).to_string(),
            status: best_effort_blob_view_status(blob_service, &attachment.hash).await,
        });
    }
    Ok(attachments)
}

pub(crate) async fn direct_message_attachment_views(
    blob_service: &dyn BlobService,
    manifest: Option<&DirectMessageAttachmentManifestV1>,
) -> Result<Vec<AttachmentView>> {
    let Some(manifest) = manifest else {
        return Ok(Vec::new());
    };
    let mut attachments = Vec::new();
    attachments.push(AttachmentView {
        hash: manifest.original.hash.as_str().to_string(),
        mime: manifest.original.mime.clone(),
        bytes: manifest.original.bytes,
        role: match manifest.kind {
            DirectMessageAttachmentKind::Image => "image_original".into(),
            DirectMessageAttachmentKind::Video => "video_manifest".into(),
        },
        status: best_effort_blob_view_status(blob_service, &manifest.original.hash).await,
    });
    if let Some(poster) = manifest.poster.as_ref() {
        attachments.push(AttachmentView {
            hash: poster.hash.as_str().to_string(),
            mime: poster.mime.clone(),
            bytes: poster.bytes,
            role: "video_poster".into(),
            status: best_effort_blob_view_status(blob_service, &poster.hash).await,
        });
    }
    Ok(attachments)
}

pub(crate) fn direct_message_preview(row: &DirectMessageMessageRow) -> String {
    if let Some(text) = row
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return text.chars().take(80).collect();
    }
    match row
        .attachment_manifest
        .as_ref()
        .map(|manifest| &manifest.kind)
    {
        Some(DirectMessageAttachmentKind::Image) => "[Image]".into(),
        Some(DirectMessageAttachmentKind::Video) => "[Video]".into(),
        None => String::new(),
    }
}

pub(crate) async fn materialize_direct_message_manifest(
    blob_service: &dyn BlobService,
    keys: &KukuriKeys,
    sender_pubkey: &Pubkey,
    message_id: &str,
    manifest: Option<&DirectMessageAttachmentManifestV1>,
) -> Result<Option<DirectMessageAttachmentManifestV1>> {
    let Some(manifest) = manifest else {
        return Ok(None);
    };
    let original = materialize_direct_message_blob_ref(
        blob_service,
        keys,
        sender_pubkey,
        message_id,
        &manifest.original,
    )
    .await?;
    let poster = match manifest.poster.as_ref() {
        Some(poster) => Some(
            materialize_direct_message_blob_ref(
                blob_service,
                keys,
                sender_pubkey,
                message_id,
                poster,
            )
            .await?,
        ),
        None => None,
    };
    Ok(Some(DirectMessageAttachmentManifestV1 {
        attachment_id: manifest.attachment_id.clone(),
        kind: manifest.kind.clone(),
        original,
        poster,
    }))
}

pub(crate) async fn materialize_direct_message_blob_ref(
    blob_service: &dyn BlobService,
    keys: &KukuriKeys,
    sender_pubkey: &Pubkey,
    message_id: &str,
    encrypted_ref: &DirectMessageEncryptedBlobRefV1,
) -> Result<DirectMessageEncryptedBlobRefV1> {
    let Some(bytes) = blob_service.fetch_blob(&encrypted_ref.hash).await? else {
        anyhow::bail!("direct message attachment blob is missing");
    };
    let encrypted: DirectMessageEncryptedAttachmentV1 = serde_json::from_slice(bytes.as_slice())
        .context("failed to decode direct message attachment blob")?;
    let decrypted = decrypt_direct_message_attachment(keys, sender_pubkey, message_id, &encrypted)?;
    let local = blob_service
        .put_blob(decrypted, encrypted_ref.mime.as_str())
        .await?;
    Ok(DirectMessageEncryptedBlobRefV1 {
        blob_id: encrypted_ref.blob_id.clone(),
        hash: local.hash,
        mime: encrypted_ref.mime.clone(),
        bytes: encrypted_ref.bytes,
        nonce_hex: String::new(),
    })
}

pub(crate) async fn direct_message_topic_peer_count(
    transport: &dyn Transport,
    topic: &TopicId,
) -> Result<usize> {
    let snapshot = transport.peers().await?;
    let hint_topic = format!("hint/{}", topic.as_str());
    let topic_peer_count = snapshot
        .topic_diagnostics
        .iter()
        .find(|diagnostic| diagnostic.topic == hint_topic || diagnostic.topic == topic.as_str())
        .map(|diagnostic| diagnostic.peer_count)
        .unwrap_or(0);
    if topic_peer_count > 0 {
        return Ok(topic_peer_count);
    }
    if snapshot.connected && snapshot.peer_count > 0 {
        return Ok(snapshot.peer_count);
    }
    Ok(0)
}

pub(crate) fn blob_view_status(status: BlobStatus) -> BlobViewStatus {
    match status {
        BlobStatus::Missing => BlobViewStatus::Missing,
        BlobStatus::Available => BlobViewStatus::Available,
        BlobStatus::Pinned => BlobViewStatus::Pinned,
    }
}

pub(crate) fn blob_status(status: BlobStatus) -> BlobCacheStatus {
    match status {
        BlobStatus::Missing => BlobCacheStatus::Missing,
        BlobStatus::Available => BlobCacheStatus::Available,
        BlobStatus::Pinned => BlobCacheStatus::Pinned,
    }
}

pub(crate) fn attachment_role_name(role: &AssetRole) -> &'static str {
    match role {
        AssetRole::ImageOriginal => "image_original",
        AssetRole::ImagePreview => "image_preview",
        AssetRole::VideoPoster => "video_poster",
        AssetRole::VideoManifest => "video_manifest",
        AssetRole::ProfileAvatar => "profile_avatar",
        AssetRole::Attachment => "attachment",
    }
}

pub(crate) fn sanitize_game_participants(participants: Vec<String>) -> Result<Vec<String>> {
    let mut seen = BTreeSet::new();
    let normalized = participants
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect::<Vec<_>>();
    if normalized.len() < 2 {
        anyhow::bail!("game room requires at least two unique participants");
    }
    Ok(normalized)
}

pub(crate) fn validate_game_room_transition(
    current: &GameRoomStatus,
    next: &GameRoomStatus,
) -> Result<()> {
    match (current, next) {
        (GameRoomStatus::Ended, GameRoomStatus::Ended) => {
            anyhow::bail!("ended game room cannot be updated")
        }
        (GameRoomStatus::Ended, _) => anyhow::bail!("ended game room cannot be updated"),
        (GameRoomStatus::Waiting, GameRoomStatus::Waiting)
        | (GameRoomStatus::Waiting, GameRoomStatus::Running)
        | (GameRoomStatus::Waiting, GameRoomStatus::Ended)
        | (GameRoomStatus::Running, GameRoomStatus::Running)
        | (GameRoomStatus::Running, GameRoomStatus::Paused)
        | (GameRoomStatus::Running, GameRoomStatus::Ended)
        | (GameRoomStatus::Paused, GameRoomStatus::Paused)
        | (GameRoomStatus::Paused, GameRoomStatus::Running)
        | (GameRoomStatus::Paused, GameRoomStatus::Ended) => Ok(()),
        (GameRoomStatus::Waiting, GameRoomStatus::Paused) => {
            anyhow::bail!("game room cannot pause before it starts")
        }
        (GameRoomStatus::Running, GameRoomStatus::Waiting)
        | (GameRoomStatus::Paused, GameRoomStatus::Waiting) => {
            anyhow::bail!("game room cannot move back to waiting")
        }
    }
}

pub(crate) fn validate_game_room_scores(
    manifest: &GameRoomManifestBlobV1,
    scores: &[GameScoreView],
) -> Result<()> {
    if manifest.scores.len() != scores.len() {
        anyhow::bail!("score update must include all participants");
    }
    let expected = manifest
        .scores
        .iter()
        .map(|score| score.participant_id.clone())
        .collect::<BTreeSet<_>>();
    let provided = scores
        .iter()
        .map(|score| score.participant_id.clone())
        .collect::<BTreeSet<_>>();
    if expected != provided {
        anyhow::bail!("score update participants do not match the room roster");
    }
    let expected_labels = manifest
        .scores
        .iter()
        .map(|score| (score.participant_id.as_str(), score.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    for score in scores {
        if expected_labels.get(score.participant_id.as_str()) != Some(&score.label.as_str()) {
            anyhow::bail!("score update labels do not match the room roster");
        }
    }
    Ok(())
}

pub(crate) fn channel_storage_id(channel_id: Option<&ChannelId>) -> String {
    channel_id
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| PUBLIC_CHANNEL_ID.to_string())
}

pub(crate) fn channel_hint_topic_for(topic_id: &str, channel_id: Option<&ChannelId>) -> TopicId {
    channel_id
        .map(|value| private_channel_hint_topic(value.as_str()))
        .unwrap_or_else(|| TopicId::new(topic_id))
}

pub(crate) fn channel_id_from_storage(channel_id: &str) -> Option<ChannelId> {
    (channel_id != PUBLIC_CHANNEL_ID).then(|| ChannelId::new(channel_id.to_string()))
}

pub(crate) fn channel_id_for_view(channel_id: &str) -> Option<String> {
    channel_id_from_storage(channel_id).map(|value| value.as_str().to_string())
}

pub(crate) fn joined_private_channel_key(topic_id: &str, channel_id: &str) -> String {
    format!("{topic_id}::{channel_id}")
}

pub(crate) fn live_presence_task_key(topic_id: &str, channel_id: &str, session_id: &str) -> String {
    format!("{topic_id}::{channel_id}::{session_id}")
}

pub(crate) fn short_id_suffix(author_pubkey: &str) -> &str {
    author_pubkey.get(..8).unwrap_or(author_pubkey)
}

pub(crate) fn normalize_topic_name(topic: String) -> Option<String> {
    let normalized = topic
        .strip_prefix("hint/")
        .map_or(topic.clone(), ToOwned::to_owned);
    if normalized.starts_with("private/") || normalized.starts_with("kukuri:dm:") {
        None
    } else {
        Some(normalized)
    }
}

pub(crate) fn normalize_topics(topics: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for topic in topics {
        let Some(topic) = normalize_topic_name(topic) else {
            continue;
        };
        if seen.insert(topic.clone()) {
            normalized.push(topic);
        }
    }
    normalized
}

pub(crate) fn normalize_topic_diagnostics(
    diagnostics: Vec<TopicPeerSnapshot>,
) -> Vec<TopicPeerSnapshot> {
    let mut merged = BTreeMap::<String, TopicPeerSnapshot>::new();
    for diagnostic in diagnostics {
        let Some(topic) = normalize_topic_name(diagnostic.topic) else {
            continue;
        };
        let entry = merged
            .entry(topic.clone())
            .or_insert_with(|| TopicPeerSnapshot {
                topic: topic.clone(),
                joined: false,
                peer_count: 0,
                connected_peers: Vec::new(),
                configured_peer_ids: Vec::new(),
                missing_peer_ids: Vec::new(),
                last_received_at: None,
                status_detail: diagnostic.status_detail.clone(),
                last_error: diagnostic.last_error.clone(),
            });
        entry.joined |= diagnostic.joined;
        entry.peer_count = entry.peer_count.max(diagnostic.peer_count);
        for peer in diagnostic.connected_peers {
            if !entry.connected_peers.contains(&peer) {
                entry.connected_peers.push(peer);
            }
        }
        for peer in diagnostic.configured_peer_ids {
            if !entry.configured_peer_ids.contains(&peer) {
                entry.configured_peer_ids.push(peer);
            }
        }
        for peer in diagnostic.missing_peer_ids {
            if !entry.missing_peer_ids.contains(&peer) {
                entry.missing_peer_ids.push(peer);
            }
        }
        entry.last_received_at = match (entry.last_received_at, diagnostic.last_received_at) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (None, value) | (value, None) => value,
        };
        if entry.status_detail.starts_with("No peers configured")
            || entry.status_detail.starts_with("Waiting")
        {
            entry.status_detail = diagnostic.status_detail;
        }
        if entry.last_error.is_none() {
            entry.last_error = diagnostic.last_error;
        }
    }
    merged.into_values().collect()
}

pub(crate) fn merge_peer_ids(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    left.into_iter()
        .chain(right)
        .filter(|peer| !peer.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn effective_sync_status_detail(
    base: &str,
    gossip_peer_count: usize,
    assist_peer_count: usize,
    subscribed_topic_count: usize,
) -> String {
    if gossip_peer_count > 0 || assist_peer_count == 0 {
        return base.to_string();
    }
    if subscribed_topic_count > 0 {
        format!("relay-assisted sync available via {assist_peer_count} peer(s)")
    } else {
        format!("relay-assisted connectivity available via {assist_peer_count} peer(s)")
    }
}

pub(crate) fn effective_topic_status_detail(
    base: &str,
    gossip_peer_count: usize,
    assist_peer_count: usize,
) -> String {
    if gossip_peer_count > 0 || assist_peer_count == 0 {
        return base.to_string();
    }
    format!("relay-assisted sync available via {assist_peer_count} peer(s)")
}

impl Drop for AppService {
    fn drop(&mut self) {
        if let Ok(mut subscriptions) = self.subscriptions.try_lock() {
            for (_, handle) in subscriptions.drain() {
                handle.abort();
            }
        }
        if let Ok(mut subscriptions) = self.private_channel_subscriptions.try_lock() {
            for (_, handle) in subscriptions.drain() {
                handle.abort();
            }
        }
        if let Ok(mut subscriptions) = self.author_subscriptions.try_lock() {
            for (_, handle) in subscriptions.drain() {
                handle.abort();
            }
        }
        if let Ok(mut tasks) = self.live_presence_tasks.try_lock() {
            for (_, handle) in tasks.drain() {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
