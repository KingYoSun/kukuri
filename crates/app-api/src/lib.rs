use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use futures_util::StreamExt;
use kukuri_blob_service::{BlobService, BlobStatus, MemoryBlobService, StoredBlob};
use kukuri_core::{
    AssetRole, AuthorProfileDocV1, CanonicalPostHeader, ChannelId, ChannelRef,
    CreatePrivateChannelInput, EnvelopeId, FollowEdge, FollowEdgeDocV1, FollowEdgeStatus,
    GAME_MANIFEST_MIME, GameParticipant, GameRoomManifestBlobV1, GameRoomStateDocV1,
    GameRoomStatus, GameScoreEntry, GossipHint, HintObjectRef, KukuriEnvelope, KukuriKeys,
    KukuriMediaManifestV1, KukuriProfileEnvelopeContentV1, LIVE_MANIFEST_MIME,
    LiveSessionManifestBlobV1, LiveSessionStateDocV1, LiveSessionStatus, ManifestBlobRef,
    MediaManifestItem, ObjectVisibility, PayloadRef, PrivateChannelInvitePreview,
    PrivateChannelMetadataDocV1, Profile, Pubkey, ReplicaId, TimelineScope, TopicId,
    build_follow_edge_envelope, build_game_session_envelope, build_live_session_envelope,
    build_media_manifest_envelope, build_post_envelope_with_payload_in_channel,
    build_private_channel_invite_token, build_profile_envelope, generate_keys, parse_follow_edge,
    parse_private_channel_invite_token, parse_profile, timeline_sort_key,
};
use kukuri_docs_sync::{
    DocOp, DocQuery, DocsSync, MemoryDocsSync, author_replica_id, private_channel_hint_topic,
    private_channel_replica_id, stable_key, topic_replica_id,
};
use kukuri_store::{
    AuthorRelationshipProjectionRow, BlobCacheStatus, GameRoomProjectionRow,
    LiveSessionProjectionRow, ObjectProjectionRow, Page, ProjectionStore, Store, TimelineCursor,
};
use kukuri_transport::{
    ConnectMode, DiscoveryMode, DiscoverySnapshot, HintTransport, PeerSnapshot, SeedPeer,
    TopicPeerSnapshot, Transport,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};

const REPLICA_SYNC_RESTART_RETRY_SECONDS: i64 = 5;
const PUBLIC_CHANNEL_ID: &str = "public";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostView {
    pub object_id: String,
    pub envelope_id: String,
    pub author_pubkey: String,
    pub author_name: Option<String>,
    pub author_display_name: Option<String>,
    pub following: bool,
    pub followed_by: bool,
    pub mutual: bool,
    pub friend_of_friend: bool,
    pub content: String,
    pub content_status: BlobViewStatus,
    pub attachments: Vec<AttachmentView>,
    pub created_at: i64,
    pub reply_to: Option<String>,
    pub root_id: Option<String>,
    pub object_kind: String,
    pub channel_id: Option<String>,
    pub audience_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobViewStatus {
    Missing,
    Available,
    Pinned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentView {
    pub hash: String,
    pub mime: String,
    pub bytes: u64,
    pub role: String,
    pub status: BlobViewStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobMediaPayload {
    pub bytes_base64: String,
    pub mime: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileInput {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorSocialView {
    pub author_pubkey: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub picture: Option<String>,
    pub updated_at: Option<i64>,
    pub following: bool,
    pub followed_by: bool,
    pub mutual: bool,
    pub friend_of_friend: bool,
    pub friend_of_friend_via_pubkeys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingAttachment {
    pub mime: String,
    pub bytes: Vec<u8>,
    pub role: AssetRole,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionView {
    pub session_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: LiveSessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub viewer_count: usize,
    pub joined_by_me: bool,
    pub channel_id: Option<String>,
    pub audience_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomView {
    pub room_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreView>,
    pub updated_at: i64,
    pub channel_id: Option<String>,
    pub audience_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameScoreView {
    pub participant_id: String,
    pub label: String,
    pub score: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateLiveSessionInput {
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateGameRoomInput {
    pub title: String,
    pub description: String,
    pub participants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateGameRoomInput {
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineView {
    pub items: Vec<PostView>,
    pub next_cursor: Option<TimelineCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinedPrivateChannelView {
    pub topic_id: String,
    pub channel_id: String,
    pub label: String,
    pub creator_pubkey: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateChannelCapability {
    pub topic_id: String,
    pub channel_id: String,
    pub label: String,
    pub creator_pubkey: String,
    pub namespace_secret_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub connected: bool,
    pub last_sync_ts: Option<i64>,
    pub peer_count: usize,
    pub pending_events: usize,
    pub status_detail: String,
    pub last_error: Option<String>,
    pub configured_peers: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub topic_diagnostics: Vec<TopicSyncStatus>,
    pub local_author_pubkey: String,
    pub discovery: DiscoveryStatus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryStatus {
    pub mode: DiscoveryMode,
    pub connect_mode: ConnectMode,
    pub env_locked: bool,
    pub configured_seed_peer_ids: Vec<String>,
    pub bootstrap_seed_peer_ids: Vec<String>,
    pub manual_ticket_peer_ids: Vec<String>,
    pub connected_peer_ids: Vec<String>,
    pub assist_peer_ids: Vec<String>,
    pub local_endpoint_id: String,
    pub last_discovery_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSyncStatus {
    pub topic: String,
    pub joined: bool,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub assist_peer_ids: Vec<String>,
    pub configured_peer_ids: Vec<String>,
    pub missing_peer_ids: Vec<String>,
    pub last_received_at: Option<i64>,
    pub status_detail: String,
    pub last_error: Option<String>,
}

pub struct AppService {
    store: Arc<dyn Store>,
    projection_store: Arc<dyn ProjectionStore>,
    transport: Arc<dyn Transport>,
    hint_transport: Arc<dyn HintTransport>,
    docs_sync: Arc<dyn DocsSync>,
    blob_service: Arc<dyn BlobService>,
    keys: Arc<KukuriKeys>,
    subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    private_channel_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    author_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    joined_private_channels: Arc<Mutex<HashMap<String, JoinedPrivateChannelState>>>,
    live_presence_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    last_sync_ts: Arc<Mutex<Option<i64>>>,
    replica_sync_restart_deadlines: Arc<Mutex<HashMap<String, i64>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct JoinedPrivateChannelState {
    topic_id: String,
    channel_id: ChannelId,
    label: String,
    creator_pubkey: String,
    namespace_secret_hex: String,
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
            private_channel_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            author_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            joined_private_channels: Arc::new(Mutex::new(HashMap::new())),
            live_presence_tasks: Arc::new(Mutex::new(HashMap::new())),
            last_sync_ts: Arc::new(Mutex::new(None)),
            replica_sync_restart_deadlines: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn warm_social_graph(&self) -> Result<()> {
        let local_author = self.current_author_pubkey();
        self.ensure_author_subscription(local_author.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        for edge in self
            .store
            .list_follow_edges_by_subject(local_author.as_str())
            .await?
        {
            if edge.status == FollowEdgeStatus::Active {
                self.ensure_author_subscription(edge.target_pubkey.as_str())
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_my_profile(&self) -> Result<Profile> {
        let local_author = self.current_author_pubkey();
        self.ensure_author_subscription(local_author.as_str())
            .await?;
        Ok(self
            .store
            .get_profile(local_author.as_str())
            .await?
            .unwrap_or(Profile {
                pubkey: Pubkey::from(local_author),
                ..Profile::default()
            }))
    }

    pub async fn set_my_profile(&self, input: ProfileInput) -> Result<Profile> {
        let author_pubkey = Pubkey::from(self.current_author_pubkey());
        let envelope = build_profile_envelope(
            self.keys.as_ref(),
            &KukuriProfileEnvelopeContentV1 {
                author_pubkey: author_pubkey.clone(),
                name: normalize_optional_text(input.name),
                display_name: normalize_optional_text(input.display_name),
                about: normalize_optional_text(input.about),
                picture: normalize_optional_text(input.picture),
            },
        )?;
        let profile = parse_profile(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse profile envelope"))?;
        self.store.put_envelope(envelope.clone()).await?;
        self.projection_store
            .upsert_profile_cache(profile.clone())
            .await?;
        persist_profile_doc(self.docs_sync.as_ref(), &profile, &envelope).await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(profile)
    }

    pub async fn follow_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let target_pubkey = Pubkey::from(normalize_author_pubkey(pubkey)?);
        let envelope = build_follow_edge_envelope(
            self.keys.as_ref(),
            &target_pubkey,
            FollowEdgeStatus::Active,
        )?;
        let edge = parse_follow_edge(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse follow edge"))?;
        self.store.put_envelope(envelope.clone()).await?;
        persist_follow_edge_doc(self.docs_sync.as_ref(), &edge, &envelope).await?;
        self.ensure_author_subscription(target_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        self.build_author_social_view(target_pubkey.as_str()).await
    }

    pub async fn unfollow_author(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let target_pubkey = Pubkey::from(normalize_author_pubkey(pubkey)?);
        let envelope = build_follow_edge_envelope(
            self.keys.as_ref(),
            &target_pubkey,
            FollowEdgeStatus::Revoked,
        )?;
        let edge = parse_follow_edge(&envelope)?
            .ok_or_else(|| anyhow::anyhow!("failed to parse follow edge"))?;
        self.store.put_envelope(envelope.clone()).await?;
        persist_follow_edge_doc(self.docs_sync.as_ref(), &edge, &envelope).await?;
        self.ensure_author_subscription(target_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        self.build_author_social_view(target_pubkey.as_str()).await
    }

    pub async fn get_author_social_view(&self, pubkey: &str) -> Result<AuthorSocialView> {
        let author_pubkey = normalize_author_pubkey(pubkey)?;
        self.ensure_author_subscription(author_pubkey.as_str())
            .await?;
        self.rebuild_author_relationships().await?;
        self.build_author_social_view(author_pubkey.as_str()).await
    }

    pub async fn create_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<String> {
        self.create_post_in_channel(topic_id, ChannelRef::Public, content, reply_to)
            .await
    }

    pub async fn create_post_with_attachments(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        self.create_post_with_attachments_in_channel(
            topic_id,
            ChannelRef::Public,
            content,
            reply_to,
            attachments,
        )
        .await
    }

    pub async fn create_post_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<String> {
        self.create_post_with_attachments_in_channel(
            topic_id,
            channel_ref,
            content,
            reply_to,
            Vec::new(),
        )
        .await
    }

    pub async fn create_post_with_attachments_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        content: &str,
        reply_to: Option<&str>,
        attachments: Vec<PendingAttachment>,
    ) -> Result<String> {
        self.ensure_topic_subscription(topic_id).await?;
        let topic = TopicId::new(topic_id);
        let parent = if let Some(reply_to) = reply_to {
            self.resolve_parent_object(&EnvelopeId::from(reply_to))
                .await?
        } else {
            None
        };
        let effective_channel_id = if let Some(parent) = parent.as_ref() {
            let content = parent
                .post_content()?
                .ok_or_else(|| anyhow::anyhow!("reply target is not a post object"))?;
            if content.topic_id.as_str() != topic_id {
                anyhow::bail!("reply target topic does not match");
            }
            if let Some(channel_id) = content.channel_id.clone() {
                self.ensure_private_channel_access(topic_id, &channel_id)
                    .await?;
                Some(channel_id)
            } else {
                None
            }
        } else {
            match channel_ref {
                ChannelRef::Public => None,
                ChannelRef::PrivateChannel { channel_id } => {
                    self.ensure_private_channel_access(topic_id, &channel_id)
                        .await?;
                    self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
                        .await?;
                    Some(channel_id)
                }
            }
        };
        let now = Utc::now().timestamp_millis();
        let stored_blob = self
            .blob_service
            .put_blob(content.as_bytes().to_vec(), "text/plain")
            .await?;
        let stored_attachments = futures_util::future::try_join_all(attachments.into_iter().map(
            |attachment| async move {
                let stored = self
                    .blob_service
                    .put_blob(attachment.bytes, attachment.mime.as_str())
                    .await?;
                Ok::<_, anyhow::Error>((attachment.role, stored))
            },
        ))
        .await?;
        let manifest_ids = if stored_attachments.is_empty() {
            Vec::new()
        } else {
            let manifest_id = format!(
                "media-{}-{}",
                now,
                short_id_suffix(self.current_author_pubkey().as_str())
            );
            let manifest = KukuriMediaManifestV1 {
                manifest_id: manifest_id.clone(),
                owner_pubkey: Pubkey::from(self.current_author_pubkey()),
                created_at: now,
                items: stored_attachments
                    .iter()
                    .map(|(role, stored)| MediaManifestItem {
                        blob_hash: stored.hash.clone(),
                        mime: stored.mime.clone(),
                        size: stored.bytes,
                        width: None,
                        height: None,
                        duration_ms: None,
                        codec: None,
                        thumbnail_blob_hash: match role {
                            AssetRole::VideoManifest => None,
                            _ => None,
                        },
                    })
                    .collect(),
            };
            let envelope = build_media_manifest_envelope(self.keys.as_ref(), &topic, &manifest)?;
            persist_media_manifest(
                &topic,
                effective_channel_id.as_ref(),
                &envelope,
                &manifest,
                self.docs_sync.as_ref(),
            )
            .await?;
            vec![manifest_id]
        };
        let envelope = build_post_envelope_with_payload_in_channel(
            self.keys.as_ref(),
            &topic,
            PayloadRef::BlobText {
                hash: stored_blob.hash.clone(),
                mime: stored_blob.mime.clone(),
                bytes: stored_blob.bytes,
            },
            stored_attachments
                .iter()
                .map(|(role, stored)| kukuri_core::AssetRef {
                    hash: stored.hash.clone(),
                    mime: stored.mime.clone(),
                    bytes: stored.bytes,
                    role: role.clone(),
                })
                .collect(),
            manifest_ids,
            parent.as_ref(),
            if effective_channel_id.is_some() {
                ObjectVisibility::Private
            } else {
                ObjectVisibility::Public
            },
            effective_channel_id.as_ref(),
        )?;
        self.ingest_event(
            envelope.clone(),
            Some(stored_blob.clone()),
            stored_attachments,
        )
        .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, effective_channel_id.as_ref()),
                GossipHint::TopicObjectsChanged {
                    topic_id: topic.clone(),
                    objects: vec![HintObjectRef {
                        object_id: envelope.id.0.clone(),
                        object_kind: envelope.kind.clone(),
                    }],
                },
            )
            .await?;
        Ok(envelope.id.0)
    }

    pub async fn list_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.list_timeline_scoped(topic_id, TimelineScope::Public, cursor, limit)
            .await
    }

    pub async fn list_timeline_scoped(
        &self,
        topic_id: &str,
        scope: TimelineScope,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.ensure_scope_subscriptions(topic_id, &scope).await?;
        let mut page = filtered_timeline_page(
            self.projection_store.as_ref(),
            topic_id,
            cursor.clone(),
            limit,
            &self.allowed_channel_ids_for_scope(topic_id, &scope).await?,
        )
        .await?;
        if page.items.is_empty() || projection_page_needs_hydration(&page) {
            self.maybe_restart_scope_replica_sync(topic_id, &scope)
                .await;
            if self.hydrate_scope_projection(topic_id, &scope).await? > 0 {
                *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
            }
            page = filtered_timeline_page(
                self.projection_store.as_ref(),
                topic_id,
                cursor,
                limit,
                &self.allowed_channel_ids_for_scope(topic_id, &scope).await?,
            )
            .await?;
        }
        self.ensure_author_subscriptions_for_rows(&page.items)
            .await?;
        let view = self.page_to_view(page).await?;
        let mut last_sync = self.last_sync_ts.lock().await;
        if !view.items.is_empty() && last_sync.is_none() {
            *last_sync = Some(Utc::now().timestamp_millis());
        }
        Ok(view)
    }

    pub async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<TimelineView> {
        self.ensure_scope_subscriptions(topic_id, &TimelineScope::AllJoined)
            .await?;
        let thread_root = EnvelopeId::from(thread_id);
        let mut page = filtered_thread_page(
            self.projection_store.as_ref(),
            topic_id,
            &thread_root,
            cursor.clone(),
            limit,
            None,
        )
        .await?;
        if page.items.is_empty() || projection_page_needs_hydration(&page) {
            self.maybe_restart_scope_replica_sync(topic_id, &TimelineScope::AllJoined)
                .await;
            if self
                .hydrate_scope_projection(topic_id, &TimelineScope::AllJoined)
                .await?
                > 0
            {
                *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
            }
            let root_channel = self
                .projection_store
                .get_object_projection(&thread_root)
                .await?
                .map(|row| row.channel_id);
            page = filtered_thread_page(
                self.projection_store.as_ref(),
                topic_id,
                &thread_root,
                cursor,
                limit,
                root_channel.as_deref(),
            )
            .await?;
        }
        self.ensure_author_subscriptions_for_rows(&page.items)
            .await?;
        let view = self.page_to_view(page).await?;
        let mut last_sync = self.last_sync_ts.lock().await;
        if !view.items.is_empty() && last_sync.is_none() {
            *last_sync = Some(Utc::now().timestamp_millis());
        }
        Ok(view)
    }

    pub async fn list_live_sessions(&self, topic_id: &str) -> Result<Vec<LiveSessionView>> {
        self.list_live_sessions_scoped(topic_id, TimelineScope::Public)
            .await
    }

    pub async fn list_live_sessions_scoped(
        &self,
        topic_id: &str,
        scope: TimelineScope,
    ) -> Result<Vec<LiveSessionView>> {
        self.ensure_scope_subscriptions(topic_id, &scope).await?;
        self.projection_store
            .clear_expired_live_presence(Utc::now().timestamp_millis())
            .await?;
        let allowed = self.allowed_channel_ids_for_scope(topic_id, &scope).await?;
        let mut rows = filter_channel_rows(
            self.projection_store
                .list_topic_live_sessions(topic_id)
                .await?,
            &allowed,
            |row| row.channel_id.as_str(),
        );
        if rows.is_empty() {
            self.hydrate_scope_projection(topic_id, &scope).await?;
            self.projection_store
                .clear_expired_live_presence(Utc::now().timestamp_millis())
                .await?;
            rows = filter_channel_rows(
                self.projection_store
                    .list_topic_live_sessions(topic_id)
                    .await?,
                &allowed,
                |row| row.channel_id.as_str(),
            );
        }
        self.cleanup_ended_live_presence_tasks(&rows).await;
        let joined_sessions = self.live_presence_tasks.lock().await;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(LiveSessionView {
                session_id: row.session_id.clone(),
                host_pubkey: row.host_pubkey,
                title: row.title,
                description: row.description,
                status: row.status,
                started_at: row.started_at,
                ended_at: row.ended_at,
                viewer_count: row.viewer_count,
                joined_by_me: joined_sessions.contains_key(
                    live_presence_task_key(
                        topic_id,
                        row.channel_id.as_str(),
                        row.session_id.as_str(),
                    )
                    .as_str(),
                ),
                channel_id: channel_id_for_view(row.channel_id.as_str()),
                audience_label: self
                    .audience_label_for_storage(topic_id, row.channel_id.as_str())
                    .await,
            });
        }
        Ok(items)
    }

    pub async fn create_live_session(
        &self,
        topic_id: &str,
        input: CreateLiveSessionInput,
    ) -> Result<String> {
        self.create_live_session_in_channel(topic_id, ChannelRef::Public, input)
            .await
    }

    pub async fn create_live_session_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        input: CreateLiveSessionInput,
    ) -> Result<String> {
        self.ensure_topic_subscription(topic_id).await?;
        let now = Utc::now().timestamp_millis();
        let title = input.title.trim();
        if title.is_empty() {
            anyhow::bail!("live session title is required");
        }
        let channel_id = match channel_ref {
            ChannelRef::Public => None,
            ChannelRef::PrivateChannel { channel_id } => {
                self.ensure_private_channel_access(topic_id, &channel_id)
                    .await?;
                self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
                    .await?;
                Some(channel_id)
            }
        };
        let session_id = format!(
            "live-{}-{}",
            now,
            short_id_suffix(self.current_author_pubkey().as_str())
        );
        let topic = TopicId::new(topic_id);
        let manifest = LiveSessionManifestBlobV1 {
            session_id: session_id.clone(),
            topic_id: topic.clone(),
            channel_id: channel_id.clone(),
            owner_pubkey: Pubkey::from(self.current_author_pubkey()),
            title: title.to_string(),
            description: input.description.trim().to_string(),
            status: LiveSessionStatus::Live,
            started_at: now,
            ended_at: None,
        };
        let envelope = build_live_session_envelope(
            self.keys.as_ref(),
            &topic,
            session_id.as_str(),
            &serde_json::json!({
                "session_id": session_id,
                "topic_id": topic,
                "channel_id": channel_id.as_ref().map(|value| value.as_str()),
                "status": "live",
                "title": manifest.title,
                "description": manifest.description,
            }),
        )?;
        let state = self
            .persist_live_session_manifest(topic_id, manifest.clone(), now, envelope.id.clone())
            .await?;
        self.projection_store
            .upsert_live_session_cache(live_projection_row_from_state(&state, &manifest, topic_id))
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, channel_id.as_ref()),
                GossipHint::SessionChanged {
                    topic_id: topic.clone(),
                    session_id: session_id.clone(),
                    object_kind: "live-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(now);
        Ok(session_id)
    }

    pub async fn end_live_session(&self, topic_id: &str, session_id: &str) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let (state, mut manifest) = self
            .fetch_live_session_state_and_manifest(topic_id, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("live session not found"))?;
        let owner = self.current_author_pubkey();
        if state.owner_pubkey.as_str() != owner {
            anyhow::bail!("only the live session owner can end the session");
        }
        let channel_key = channel_storage_id(state.channel_id.as_ref());
        let hint_topic = channel_hint_topic_for(topic_id, state.channel_id.as_ref());
        if manifest.status == LiveSessionStatus::Ended {
            self.stop_live_presence_task(topic_id, channel_key.as_str(), session_id)
                .await;
            return Ok(());
        }
        let now = Utc::now().timestamp_millis();
        manifest.status = LiveSessionStatus::Ended;
        manifest.ended_at = Some(now);
        let envelope = build_live_session_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            session_id,
            &serde_json::json!({
                "session_id": session_id,
                "topic_id": topic_id,
                "channel_id": state.channel_id.as_ref().map(|value| value.as_str()),
                "status": "ended",
            }),
        )?;
        let state = self
            .persist_live_session_manifest(
                topic_id,
                manifest.clone(),
                state.created_at,
                envelope.id.clone(),
            )
            .await?;
        self.projection_store
            .upsert_live_session_cache(live_projection_row_from_state(&state, &manifest, topic_id))
            .await?;
        self.stop_live_presence_task(topic_id, channel_key.as_str(), session_id)
            .await;
        self.hint_transport
            .publish_hint(
                &hint_topic,
                GossipHint::SessionChanged {
                    topic_id: TopicId::new(topic_id),
                    session_id: session_id.to_string(),
                    object_kind: "live-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(now);
        Ok(())
    }

    pub async fn join_live_session(&self, topic_id: &str, session_id: &str) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let Some((state, manifest)) = self
            .fetch_live_session_state_and_manifest(topic_id, session_id)
            .await?
        else {
            anyhow::bail!("live session not found");
        };
        if manifest.status == LiveSessionStatus::Ended {
            anyhow::bail!("cannot join an ended live session");
        }
        let channel_key = channel_storage_id(state.channel_id.as_ref());
        let task_key = live_presence_task_key(topic_id, channel_key.as_str(), session_id);
        if self
            .live_presence_tasks
            .lock()
            .await
            .contains_key(task_key.as_str())
        {
            return Ok(());
        }
        self.apply_live_presence(topic_id, state.channel_id.as_ref(), session_id, 30_000)
            .await?;
        let hint_transport = Arc::clone(&self.hint_transport);
        let projection_store = Arc::clone(&self.projection_store);
        let hint_topic = channel_hint_topic_for(topic_id, state.channel_id.as_ref());
        let topic_key = topic_id.to_string();
        let channel_key_for_task = channel_key.clone();
        let session_key = session_id.to_string();
        let author = Pubkey::from(self.current_author_pubkey());
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let now = Utc::now().timestamp_millis();
                let _ = projection_store
                    .upsert_live_presence(
                        topic_key.as_str(),
                        channel_key_for_task.as_str(),
                        session_key.as_str(),
                        author.as_str(),
                        now + 30_000,
                        now,
                    )
                    .await;
                let _ = hint_transport
                    .publish_hint(
                        &hint_topic,
                        GossipHint::LivePresence {
                            topic_id: TopicId::new(topic_key.clone()),
                            session_id: session_key.clone(),
                            author: author.clone(),
                            ttl_ms: 30_000,
                        },
                    )
                    .await;
            }
        });
        self.live_presence_tasks
            .lock()
            .await
            .insert(task_key, handle);
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    pub async fn leave_live_session(&self, topic_id: &str, session_id: &str) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let (state, _) = self
            .fetch_live_session_state_and_manifest(topic_id, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("live session not found"))?;
        let channel_key = channel_storage_id(state.channel_id.as_ref());
        self.stop_live_presence_task(topic_id, channel_key.as_str(), session_id)
            .await;
        self.apply_live_presence(topic_id, state.channel_id.as_ref(), session_id, 0)
            .await?;
        *self.last_sync_ts.lock().await = Some(Utc::now().timestamp_millis());
        Ok(())
    }

    pub async fn list_game_rooms(&self, topic_id: &str) -> Result<Vec<GameRoomView>> {
        self.list_game_rooms_scoped(topic_id, TimelineScope::Public)
            .await
    }

    pub async fn list_game_rooms_scoped(
        &self,
        topic_id: &str,
        scope: TimelineScope,
    ) -> Result<Vec<GameRoomView>> {
        self.ensure_scope_subscriptions(topic_id, &scope).await?;
        let allowed = self.allowed_channel_ids_for_scope(topic_id, &scope).await?;
        let mut rows = filter_channel_rows(
            self.projection_store
                .list_topic_game_rooms(topic_id)
                .await?,
            &allowed,
            |row| row.channel_id.as_str(),
        );
        if rows.is_empty() {
            self.hydrate_scope_projection(topic_id, &scope).await?;
            rows = filter_channel_rows(
                self.projection_store
                    .list_topic_game_rooms(topic_id)
                    .await?,
                &allowed,
                |row| row.channel_id.as_str(),
            );
        }
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(GameRoomView {
                room_id: row.room_id,
                host_pubkey: row.host_pubkey,
                title: row.title,
                description: row.description,
                status: row.status,
                phase_label: row.phase_label,
                scores: row
                    .scores
                    .into_iter()
                    .map(|score| GameScoreView {
                        participant_id: score.participant_id,
                        label: score.label,
                        score: score.score,
                    })
                    .collect(),
                updated_at: row.updated_at,
                channel_id: channel_id_for_view(row.channel_id.as_str()),
                audience_label: self
                    .audience_label_for_storage(topic_id, row.channel_id.as_str())
                    .await,
            });
        }
        Ok(items)
    }

    pub async fn create_game_room(
        &self,
        topic_id: &str,
        input: CreateGameRoomInput,
    ) -> Result<String> {
        self.create_game_room_in_channel(topic_id, ChannelRef::Public, input)
            .await
    }

    pub async fn create_game_room_in_channel(
        &self,
        topic_id: &str,
        channel_ref: ChannelRef,
        input: CreateGameRoomInput,
    ) -> Result<String> {
        self.ensure_topic_subscription(topic_id).await?;
        let channel_id = match channel_ref {
            ChannelRef::Public => None,
            ChannelRef::PrivateChannel { channel_id } => {
                self.ensure_private_channel_access(topic_id, &channel_id)
                    .await?;
                self.ensure_private_channel_subscription(topic_id, channel_id.as_str())
                    .await?;
                Some(channel_id)
            }
        };
        let participants = sanitize_game_participants(input.participants)?;
        let now = Utc::now().timestamp_millis();
        let title = input.title.trim();
        if title.is_empty() {
            anyhow::bail!("game room title is required");
        }
        let room_id = format!(
            "game-{}-{}",
            now,
            short_id_suffix(self.current_author_pubkey().as_str())
        );
        let manifest = GameRoomManifestBlobV1 {
            room_id: room_id.clone(),
            topic_id: TopicId::new(topic_id),
            channel_id: channel_id.clone(),
            owner_pubkey: Pubkey::from(self.current_author_pubkey()),
            title: title.to_string(),
            description: input.description.trim().to_string(),
            status: GameRoomStatus::Waiting,
            phase_label: None,
            participants: participants
                .iter()
                .enumerate()
                .map(|(index, label)| GameParticipant {
                    participant_id: format!("participant-{}", index + 1),
                    label: label.clone(),
                })
                .collect(),
            scores: participants
                .iter()
                .enumerate()
                .map(|(index, label)| GameScoreEntry {
                    participant_id: format!("participant-{}", index + 1),
                    label: label.clone(),
                    score: 0,
                })
                .collect(),
            updated_at: now,
        };
        let envelope = build_game_session_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            room_id.as_str(),
            &serde_json::json!({
                "room_id": room_id,
                "topic_id": topic_id,
                "channel_id": channel_id.as_ref().map(|value| value.as_str()),
                "status": "waiting",
            }),
        )?;
        let state = self
            .persist_game_room_manifest(topic_id, manifest.clone(), now, envelope.id.clone())
            .await?;
        self.projection_store
            .upsert_game_room_cache(game_projection_row_from_state(&state, &manifest, topic_id))
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, channel_id.as_ref()),
                GossipHint::SessionChanged {
                    topic_id: TopicId::new(topic_id),
                    session_id: room_id.clone(),
                    object_kind: "game-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(now);
        Ok(room_id)
    }

    pub async fn update_game_room(
        &self,
        topic_id: &str,
        room_id: &str,
        input: UpdateGameRoomInput,
    ) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        let (state, mut manifest) = self
            .fetch_game_room_state_and_manifest(topic_id, room_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("game room not found"))?;
        let owner = self.current_author_pubkey();
        if state.owner_pubkey.as_str() != owner {
            anyhow::bail!("only the game room owner can update the room");
        }
        validate_game_room_transition(&manifest.status, &input.status)?;
        validate_game_room_scores(&manifest, &input.scores)?;
        manifest.status = input.status;
        manifest.phase_label = input
            .phase_label
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        manifest.scores = input
            .scores
            .into_iter()
            .map(|score| GameScoreEntry {
                participant_id: score.participant_id,
                label: score.label,
                score: score.score,
            })
            .collect();
        manifest.updated_at = Utc::now().timestamp_millis();
        let envelope = build_game_session_envelope(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            room_id,
            &serde_json::json!({
                "room_id": room_id,
                "topic_id": topic_id,
                "channel_id": state.channel_id.as_ref().map(|value| value.as_str()),
                "status": format!("{:?}", manifest.status).to_lowercase(),
                "phase_label": manifest.phase_label,
            }),
        )?;
        let state = self
            .persist_game_room_manifest(
                topic_id,
                manifest.clone(),
                state.created_at,
                envelope.id.clone(),
            )
            .await?;
        self.projection_store
            .upsert_game_room_cache(game_projection_row_from_state(&state, &manifest, topic_id))
            .await?;
        self.hint_transport
            .publish_hint(
                &channel_hint_topic_for(topic_id, state.channel_id.as_ref()),
                GossipHint::SessionChanged {
                    topic_id: TopicId::new(topic_id),
                    session_id: room_id.to_string(),
                    object_kind: "game-session".into(),
                },
            )
            .await?;
        *self.last_sync_ts.lock().await = Some(manifest.updated_at);
        Ok(())
    }

    pub async fn create_private_channel(
        &self,
        input: CreatePrivateChannelInput,
    ) -> Result<JoinedPrivateChannelView> {
        self.ensure_topic_subscription(input.topic_id.as_str())
            .await?;
        let label = input.label.trim();
        if label.is_empty() {
            anyhow::bail!("private channel label is required");
        }
        let now = Utc::now().timestamp_millis();
        let channel_id = ChannelId::new(format!(
            "channel-{}-{}",
            now,
            short_id_suffix(self.current_author_pubkey().as_str())
        ));
        let namespace_secret_hex = generate_keys().export_secret_hex();
        let state = JoinedPrivateChannelState {
            topic_id: input.topic_id.as_str().to_string(),
            channel_id: channel_id.clone(),
            label: label.to_string(),
            creator_pubkey: self.current_author_pubkey(),
            namespace_secret_hex: namespace_secret_hex.clone(),
        };
        self.register_joined_private_channel(state.clone()).await?;
        let metadata = PrivateChannelMetadataDocV1 {
            channel_id: channel_id.clone(),
            topic_id: input.topic_id.clone(),
            label: label.to_string(),
            creator_pubkey: Pubkey::from(state.creator_pubkey.clone()),
            created_at: now,
        };
        persist_private_channel_metadata(
            self.docs_sync.as_ref(),
            input.topic_id.as_str(),
            &channel_id,
            &metadata,
        )
        .await?;
        Ok(joined_private_channel_view_from_state(&state))
    }

    pub async fn export_private_channel_invite(
        &self,
        topic_id: &str,
        channel_id: &str,
        expires_at: Option<i64>,
    ) -> Result<String> {
        let state = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("private channel is not joined"))?;
        build_private_channel_invite_token(
            self.keys.as_ref(),
            &TopicId::new(topic_id),
            &state.channel_id,
            state.label.as_str(),
            state.namespace_secret_hex.as_str(),
            expires_at,
        )
    }

    pub async fn import_private_channel_invite(
        &self,
        token: &str,
    ) -> Result<PrivateChannelInvitePreview> {
        let preview = parse_private_channel_invite_token(token)?;
        if let Some(expires_at) = preview.expires_at
            && expires_at < Utc::now().timestamp_millis()
        {
            anyhow::bail!("private channel invite is expired");
        }
        let state = JoinedPrivateChannelState {
            topic_id: preview.topic_id.as_str().to_string(),
            channel_id: preview.channel_id.clone(),
            label: preview.channel_label.clone(),
            creator_pubkey: preview.inviter_pubkey.as_str().to_string(),
            namespace_secret_hex: preview.namespace_secret_hex.clone(),
        };
        self.ensure_topic_subscription(preview.topic_id.as_str())
            .await?;
        self.register_joined_private_channel(state).await?;
        Ok(preview)
    }

    pub async fn restore_private_channel_capability(
        &self,
        topic_id: &str,
        channel_id: &str,
        label: &str,
        creator_pubkey: &str,
        namespace_secret_hex: &str,
    ) -> Result<()> {
        self.ensure_topic_subscription(topic_id).await?;
        self.register_joined_private_channel(JoinedPrivateChannelState {
            topic_id: topic_id.to_string(),
            channel_id: ChannelId::new(channel_id.to_string()),
            label: label.trim().to_string(),
            creator_pubkey: creator_pubkey.to_string(),
            namespace_secret_hex: namespace_secret_hex.to_string(),
        })
        .await
    }

    pub async fn list_joined_private_channels(
        &self,
        topic_id: &str,
    ) -> Result<Vec<JoinedPrivateChannelView>> {
        self.ensure_topic_subscription(topic_id).await?;
        Ok(self
            .joined_private_channel_states_for_topic(topic_id)
            .await
            .into_iter()
            .map(|state| joined_private_channel_view_from_state(&state))
            .collect())
    }

    pub async fn get_private_channel_capability(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<Option<PrivateChannelCapability>> {
        Ok(self
            .joined_private_channel_state(topic_id, channel_id)
            .await
            .map(private_channel_capability_from_state))
    }

    pub async fn list_private_channel_capabilities(&self) -> Result<Vec<PrivateChannelCapability>> {
        Ok(self
            .joined_private_channels
            .lock()
            .await
            .values()
            .cloned()
            .map(private_channel_capability_from_state)
            .collect())
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let PeerSnapshot {
            connected,
            peer_count,
            connected_peers,
            configured_peers,
            subscribed_topics,
            pending_events,
            status_detail,
            last_error,
            topic_diagnostics,
        } = self.transport.peers().await?;
        let subscribed_topics = normalize_topics(subscribed_topics);
        let topic_diagnostics = normalize_topic_diagnostics(topic_diagnostics);
        let assist_peer_ids = self.assisted_peer_ids().await?;
        let effective_connected_peer_ids =
            merge_peer_ids(connected_peers.clone(), assist_peer_ids.clone());
        let discovery = self.get_discovery_status().await?;

        Ok(SyncStatus {
            connected: connected || !assist_peer_ids.is_empty(),
            last_sync_ts: *self.last_sync_ts.lock().await,
            peer_count: peer_count.max(effective_connected_peer_ids.len()),
            pending_events,
            status_detail: effective_sync_status_detail(
                status_detail.as_str(),
                connected_peers.len(),
                assist_peer_ids.len(),
                subscribed_topics.len(),
            ),
            last_error,
            configured_peers,
            subscribed_topics,
            topic_diagnostics: topic_diagnostics
                .into_iter()
                .map(|diagnostic| {
                    let gossip_peer_count = diagnostic.connected_peers.len();
                    TopicSyncStatus {
                        topic: diagnostic.topic,
                        joined: diagnostic.joined || !assist_peer_ids.is_empty(),
                        peer_count: diagnostic.peer_count.max(
                            merge_peer_ids(
                                diagnostic.connected_peers.clone(),
                                assist_peer_ids.clone(),
                            )
                            .len(),
                        ),
                        connected_peers: diagnostic.connected_peers,
                        assist_peer_ids: assist_peer_ids.clone(),
                        configured_peer_ids: diagnostic.configured_peer_ids,
                        missing_peer_ids: diagnostic.missing_peer_ids,
                        last_received_at: diagnostic.last_received_at,
                        status_detail: effective_topic_status_detail(
                            diagnostic.status_detail.as_str(),
                            gossip_peer_count,
                            assist_peer_ids.len(),
                        ),
                        last_error: diagnostic.last_error,
                    }
                })
                .collect(),
            local_author_pubkey: self.current_author_pubkey(),
            discovery,
        })
    }

    pub async fn get_discovery_status(&self) -> Result<DiscoveryStatus> {
        let DiscoverySnapshot {
            mode,
            connect_mode,
            env_locked,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            local_endpoint_id,
            last_discovery_error,
        } = self.transport.discovery().await?;
        let assist_peer_ids = self.assisted_peer_ids().await?;
        Ok(DiscoveryStatus {
            mode,
            connect_mode,
            env_locked,
            configured_seed_peer_ids,
            bootstrap_seed_peer_ids,
            manual_ticket_peer_ids,
            connected_peer_ids,
            assist_peer_ids,
            local_endpoint_id,
            last_discovery_error,
        })
    }

    async fn assisted_peer_ids(&self) -> Result<Vec<String>> {
        Ok(merge_peer_ids(
            self.docs_sync.assist_peer_ids().await?,
            self.blob_service.assist_peer_ids().await?,
        ))
    }

    pub async fn import_peer_ticket(&self, ticket: &str) -> Result<()> {
        self.transport.import_ticket(ticket).await?;
        self.docs_sync.import_peer_ticket(ticket).await?;
        self.blob_service.import_peer_ticket(ticket).await?;
        let existing_topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in existing_topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }
        let existing_private_topics = self
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
        for (topic_id, channel_id) in existing_private_topics {
            self.restart_private_channel_subscription(topic_id.as_str(), channel_id.as_str())
                .await?;
        }
        let existing_authors = self
            .author_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for author in existing_authors {
            self.restart_author_subscription(author.as_str()).await?;
        }
        Ok(())
    }

    pub async fn set_discovery_seeds(
        &self,
        mode: DiscoveryMode,
        env_locked: bool,
        configured_seed_peers: Vec<SeedPeer>,
        bootstrap_seed_peers: Vec<SeedPeer>,
    ) -> Result<()> {
        let effective_seed_peers =
            merge_seed_peers(configured_seed_peers.clone(), bootstrap_seed_peers.clone());
        self.transport
            .configure_discovery(
                mode,
                env_locked,
                configured_seed_peers,
                bootstrap_seed_peers,
            )
            .await?;
        self.docs_sync
            .set_seed_peers(effective_seed_peers.clone())
            .await?;
        self.blob_service
            .set_seed_peers(effective_seed_peers)
            .await?;
        let existing_topics = self
            .subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for topic in existing_topics {
            self.restart_topic_subscription(topic.as_str()).await?;
        }
        let existing_private_topics = self
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
        for (topic_id, channel_id) in existing_private_topics {
            self.restart_private_channel_subscription(topic_id.as_str(), channel_id.as_str())
                .await?;
        }
        let existing_authors = self
            .author_subscriptions
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for author in existing_authors {
            self.restart_author_subscription(author.as_str()).await?;
        }
        Ok(())
    }

    pub async fn unsubscribe_topic(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        let private_keys = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(&format!("{topic_id}::")))
            .cloned()
            .collect::<Vec<_>>();
        for key in private_keys {
            if let Some(handle) = self
                .private_channel_subscriptions
                .lock()
                .await
                .remove(key.as_str())
            {
                handle.abort();
            }
            if let Some((_, channel_id)) = key.split_once("::") {
                self.hint_transport
                    .unsubscribe_hints(&private_channel_hint_topic(channel_id))
                    .await?;
            }
        }
        let keys_to_remove = self
            .live_presence_tasks
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(&format!("{topic_id}::")))
            .cloned()
            .collect::<Vec<_>>();
        for key in keys_to_remove {
            let mut parts = key.splitn(3, "::");
            let _ = parts.next();
            let channel_id = parts.next().unwrap_or(PUBLIC_CHANNEL_ID).to_string();
            let session_id = parts.next().unwrap_or_default().to_string();
            self.stop_live_presence_task(topic_id, channel_id.as_str(), session_id.as_str())
                .await;
        }
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await
    }

    pub async fn peer_ticket(&self) -> Result<Option<String>> {
        self.transport.export_ticket().await
    }

    pub async fn blob_media_payload(
        &self,
        hash: &str,
        mime: &str,
    ) -> Result<Option<BlobMediaPayload>> {
        info!(hash = %hash, mime = %mime, "blob media payload fetch requested");
        let bytes = match self
            .blob_service
            .fetch_blob(&kukuri_core::BlobHash::new(hash.to_string()))
            .await
        {
            Ok(Some(bytes)) => {
                info!(
                    hash = %hash,
                    mime = %mime,
                    byte_len = bytes.len(),
                    "blob media payload fetch hit"
                );
                bytes
            }
            Ok(None) => {
                warn!(hash = %hash, mime = %mime, "blob media payload fetch miss");
                return Ok(None);
            }
            Err(error) => {
                warn!(
                    hash = %hash,
                    mime = %mime,
                    error = %error,
                    "blob media payload fetch failed"
                );
                return Err(error);
            }
        };
        Ok(Some(BlobMediaPayload {
            bytes_base64: BASE64_STANDARD.encode(bytes),
            mime: mime.to_string(),
        }))
    }

    pub async fn blob_preview_data_url(&self, hash: &str, mime: &str) -> Result<Option<String>> {
        let Some(payload) = self.blob_media_payload(hash, mime).await? else {
            return Ok(None);
        };
        Ok(Some(format!(
            "data:{};base64,{}",
            payload.mime, payload.bytes_base64
        )))
    }

    pub async fn shutdown(&self) {
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

    fn current_author_pubkey(&self) -> String {
        self.keys.public_key_hex()
    }

    async fn audience_label_for_storage(&self, topic_id: &str, channel_id: &str) -> String {
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

    async fn joined_private_channel_states_for_topic(
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

    async fn joined_private_channel_state(
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

    async fn ensure_private_channel_access(
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

    async fn register_joined_private_channel(
        &self,
        state: JoinedPrivateChannelState,
    ) -> Result<()> {
        let replica = private_channel_replica_id(state.channel_id.as_str());
        self.docs_sync
            .register_private_replica_secret(&replica, state.namespace_secret_hex.as_str())
            .await?;
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

    async fn ensure_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let key = joined_private_channel_key(topic_id, channel_id);
        if self
            .private_channel_subscriptions
            .lock()
            .await
            .contains_key(key.as_str())
        {
            return Ok(());
        }
        let Some(state) = self
            .joined_private_channel_state(topic_id, channel_id)
            .await
        else {
            anyhow::bail!("private channel is not joined");
        };
        self.spawn_private_channel_subscription(state).await
    }

    async fn ensure_joined_private_channel_subscriptions(&self, topic_id: &str) -> Result<()> {
        for state in self.joined_private_channel_states_for_topic(topic_id).await {
            self.ensure_private_channel_subscription(topic_id, state.channel_id.as_str())
                .await?;
        }
        Ok(())
    }

    async fn restart_private_channel_subscription(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let key = joined_private_channel_key(topic_id, channel_id);
        if let Some(handle) = self
            .private_channel_subscriptions
            .lock()
            .await
            .remove(key.as_str())
        {
            handle.abort();
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

    async fn spawn_private_channel_subscription(
        &self,
        state: JoinedPrivateChannelState,
    ) -> Result<()> {
        let docs_sync = Arc::clone(&self.docs_sync);
        let replica = private_channel_replica_id(state.channel_id.as_str());
        docs_sync
            .register_private_replica_secret(&replica, state.namespace_secret_hex.as_str())
            .await?;
        self.spawn_subscription_task(
            state.topic_id.as_str(),
            Some(state.channel_id.clone()),
            replica,
            private_channel_hint_topic(state.channel_id.as_str()),
            Some(joined_private_channel_key(
                state.topic_id.as_str(),
                state.channel_id.as_str(),
            )),
        )
        .await
    }

    async fn spawn_subscription_task(
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
        docs_sync.open_replica(&replica).await?;
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
                            if let Ok(count) = hydrate_subscription_state_with_services(
                                docs_sync.as_ref(),
                                blob_service.as_ref(),
                                projection_store.as_ref(),
                                topic.as_str(),
                                &replica_for_task,
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
                                    if let Ok(count) = hydrate_subscription_state_with_services(
                                        docs_sync.as_ref(),
                                        blob_service.as_ref(),
                                        projection_store.as_ref(),
                                        topic.as_str(),
                                        &replica_for_task,
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

    async fn stop_live_presence_task(&self, topic_id: &str, channel_id: &str, session_id: &str) {
        let key = live_presence_task_key(topic_id, channel_id, session_id);
        let handle = self.live_presence_tasks.lock().await.remove(key.as_str());
        if let Some(handle) = handle {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
    }

    async fn cleanup_ended_live_presence_tasks(&self, rows: &[LiveSessionProjectionRow]) {
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

    async fn apply_live_presence(
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

    async fn persist_live_session_manifest(
        &self,
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
        persist_live_session_state(self.docs_sync.as_ref(), &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    async fn persist_game_room_manifest(
        &self,
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
        persist_game_room_state(self.docs_sync.as_ref(), &state).await?;
        self.projection_store
            .mark_blob_status(&stored.hash, BlobCacheStatus::Available)
            .await?;
        Ok(state)
    }

    async fn fetch_live_session_state_and_manifest(
        &self,
        topic_id: &str,
        session_id: &str,
    ) -> Result<Option<(LiveSessionStateDocV1, LiveSessionManifestBlobV1)>> {
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
            return Ok(Some((state, manifest)));
        }
        Ok(None)
    }

    async fn fetch_game_room_state_and_manifest(
        &self,
        topic_id: &str,
        room_id: &str,
    ) -> Result<Option<(GameRoomStateDocV1, GameRoomManifestBlobV1)>> {
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
            return Ok(Some((state, manifest)));
        }
        Ok(None)
    }

    async fn build_author_social_view(&self, author_pubkey: &str) -> Result<AuthorSocialView> {
        let profile = self.store.get_profile(author_pubkey).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(self.current_author_pubkey().as_str(), author_pubkey)
            .await?;
        Ok(author_social_view_from_parts(
            author_pubkey,
            profile.as_ref(),
            relationship.as_ref(),
        ))
    }

    async fn rebuild_author_relationships(&self) -> Result<()> {
        rebuild_author_relationships_with_services(
            self.store.as_ref(),
            self.projection_store.as_ref(),
            self.current_author_pubkey().as_str(),
        )
        .await
    }

    async fn ensure_author_subscriptions_for_rows(
        &self,
        rows: &[ObjectProjectionRow],
    ) -> Result<()> {
        for author_pubkey in rows.iter().map(|row| row.author_pubkey.as_str()) {
            self.ensure_author_subscription(author_pubkey).await?;
        }
        Ok(())
    }

    async fn ensure_author_subscription(&self, author_pubkey: &str) -> Result<()> {
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

    async fn restart_author_subscription(&self, author_pubkey: &str) -> Result<()> {
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

    async fn spawn_author_subscription(&self, author_pubkey: &str) -> Result<()> {
        let store = Arc::clone(&self.store);
        let projection_store = Arc::clone(&self.projection_store);
        let docs_sync = Arc::clone(&self.docs_sync);
        let last_sync = Arc::clone(&self.last_sync_ts);
        let author_key = normalize_author_pubkey(author_pubkey)?;
        let local_author_pubkey = self.current_author_pubkey();
        let replica = author_replica_id(author_key.as_str());
        docs_sync.open_replica(&replica).await?;
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

    async fn ensure_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if self.subscriptions.lock().await.contains_key(topic_id) {
            return Ok(());
        }

        self.spawn_topic_subscription(topic_id).await
    }

    async fn restart_topic_subscription(&self, topic_id: &str) -> Result<()> {
        if let Some(handle) = self.subscriptions.lock().await.remove(topic_id) {
            handle.abort();
        }
        self.hint_transport
            .unsubscribe_hints(&TopicId::new(topic_id))
            .await?;
        self.spawn_topic_subscription(topic_id).await
    }

    async fn spawn_topic_subscription(&self, topic_id: &str) -> Result<()> {
        self.spawn_subscription_task(
            topic_id,
            None,
            topic_replica_id(topic_id),
            TopicId::new(topic_id),
            None,
        )
        .await
    }

    async fn ingest_event(
        &self,
        envelope: KukuriEnvelope,
        _stored_blob: Option<StoredBlob>,
        attachments: Vec<(AssetRole, StoredBlob)>,
    ) -> Result<()> {
        self.store.put_envelope(envelope.clone()).await?;
        let mut object = envelope
            .to_post_object()?
            .ok_or_else(|| anyhow::anyhow!("expected post/comment envelope"))?;
        object.attachments = attachments
            .iter()
            .map(|(role, stored)| kukuri_core::AssetRef {
                hash: stored.hash.clone(),
                mime: stored.mime.clone(),
                bytes: stored.bytes,
                role: role.clone(),
            })
            .collect();
        let content = match &object.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => self
                .blob_service
                .fetch_blob(hash)
                .await?
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string()),
        };
        persist_post_object(self.docs_sync.as_ref(), object.clone(), envelope.clone()).await?;
        ProjectionStore::put_object_projection(
            self.projection_store.as_ref(),
            projection_row_from_header(&object, content),
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

    async fn resolve_parent_object(
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

        let object_kind = if projection.reply_to_object_id.is_some() {
            "comment"
        } else {
            "post"
        };
        let mut tags = vec![
            vec!["topic".into(), projection.topic_id.clone()],
            vec!["object".into(), object_kind.into()],
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
            })?,
            sig: String::new(),
        }))
    }

    async fn ensure_scope_subscriptions(
        &self,
        topic_id: &str,
        scope: &TimelineScope,
    ) -> Result<()> {
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

    async fn allowed_channel_ids_for_scope(
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

    async fn hydrate_scope_projection(
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
                    hydrated += hydrate_subscription_state_with_services(
                        self.docs_sync.as_ref(),
                        self.blob_service.as_ref(),
                        self.projection_store.as_ref(),
                        topic_id,
                        &private_channel_replica_id(state.channel_id.as_str()),
                    )
                    .await?;
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.ensure_private_channel_access(topic_id, channel_id)
                    .await?;
                hydrated += hydrate_subscription_state_with_services(
                    self.docs_sync.as_ref(),
                    self.blob_service.as_ref(),
                    self.projection_store.as_ref(),
                    topic_id,
                    &private_channel_replica_id(channel_id.as_str()),
                )
                .await?;
            }
        }
        Ok(hydrated)
    }

    async fn maybe_restart_scope_replica_sync(&self, topic_id: &str, scope: &TimelineScope) {
        self.maybe_restart_replica_sync(topic_id, &topic_replica_id(topic_id))
            .await;
        match scope {
            TimelineScope::Public => {}
            TimelineScope::AllJoined => {
                for state in self.joined_private_channel_states_for_topic(topic_id).await {
                    self.maybe_restart_replica_sync(
                        topic_id,
                        &private_channel_replica_id(state.channel_id.as_str()),
                    )
                    .await;
                }
            }
            TimelineScope::Channel { channel_id } => {
                self.maybe_restart_replica_sync(
                    topic_id,
                    &private_channel_replica_id(channel_id.as_str()),
                )
                .await;
            }
        }
    }

    async fn maybe_restart_replica_sync(&self, topic_id: &str, replica: &ReplicaId) {
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
        if let Err(error) = self.docs_sync.restart_replica_sync(&replica).await {
            warn!(
                topic = %topic_id,
                replica = %replica.as_str(),
                error = %error,
                "failed to restart replica sync"
            );
        }
    }

    async fn page_to_view(&self, page: Page<ObjectProjectionRow>) -> Result<TimelineView> {
        let mut items = Vec::with_capacity(page.items.len());
        for row in page.items {
            items.push(self.row_to_view(row).await?);
        }
        Ok(TimelineView {
            items,
            next_cursor: page.next_cursor,
        })
    }

    async fn row_to_view(&self, row: ObjectProjectionRow) -> Result<PostView> {
        let post_object = fetch_post_object_for_projection(
            self.docs_sync.as_ref(),
            &row.source_replica_id,
            row.source_key.as_str(),
        )
        .await?;
        let profile = self.store.get_profile(row.author_pubkey.as_str()).await?;
        let relationship = self
            .projection_store
            .get_author_relationship(
                self.current_author_pubkey().as_str(),
                row.author_pubkey.as_str(),
            )
            .await?;
        let content_status =
            blob_view_status_for_payload(self.blob_service.as_ref(), &row.payload_ref).await?;
        let attachments = if let Some(post_object) = post_object {
            attachment_views(self.blob_service.as_ref(), &post_object).await?
        } else {
            Vec::new()
        };
        let audience_label = self
            .audience_label_for_storage(row.topic_id.as_str(), row.channel_id.as_str())
            .await;

        Ok(PostView {
            object_id: row.object_id.0.clone(),
            envelope_id: row.source_envelope_id.0.clone(),
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
            content: row.content.unwrap_or_else(|| "[blob pending]".to_string()),
            content_status,
            attachments,
            created_at: row.created_at,
            reply_to: row.reply_to_object_id.clone().map(|id| id.0),
            root_id: row.root_object_id.clone().map(|id| id.0),
            object_kind: if row.reply_to_object_id.is_some() {
                "comment".into()
            } else {
                "post".into()
            },
            channel_id: channel_id_for_view(row.channel_id.as_str()),
            audience_label,
        })
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn normalize_author_pubkey(pubkey: &str) -> Result<String> {
    let trimmed = pubkey.trim();
    if trimmed.len() != 64 || !trimmed.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!("invalid author pubkey"));
    }
    Ok(trimmed.to_string())
}

fn author_social_view_from_parts(
    author_pubkey: &str,
    profile: Option<&Profile>,
    relationship: Option<&AuthorRelationshipProjectionRow>,
) -> AuthorSocialView {
    AuthorSocialView {
        author_pubkey: author_pubkey.to_string(),
        name: profile.and_then(|profile| profile.name.clone()),
        display_name: profile.and_then(|profile| profile.display_name.clone()),
        about: profile.and_then(|profile| profile.about.clone()),
        picture: profile.and_then(|profile| profile.picture.clone()),
        updated_at: profile.map(|profile| profile.updated_at),
        following: relationship.is_some_and(|relationship| relationship.following),
        followed_by: relationship.is_some_and(|relationship| relationship.followed_by),
        mutual: relationship.is_some_and(|relationship| relationship.mutual),
        friend_of_friend: relationship.is_some_and(|relationship| relationship.friend_of_friend),
        friend_of_friend_via_pubkeys: relationship
            .map(|relationship| relationship.friend_of_friend_via_pubkeys.clone())
            .unwrap_or_default(),
    }
}

async fn rebuild_author_relationships_with_services(
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

async fn persist_profile_doc(
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

async fn persist_follow_edge_doc(
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

async fn hydrate_author_state_with_services(
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
                {
                    if let Some(edge) = parse_follow_edge(&envelope)? {
                        if edge.target_pubkey == doc.target_pubkey && edge.status == doc.status {
                            store.put_envelope(envelope).await?;
                            count += 1;
                        }
                    }
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

async fn fetch_author_envelope_by_id(
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

fn merge_seed_peers(
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

async fn persist_post_object(
    docs_sync: &dyn DocsSync,
    object: CanonicalPostHeader,
    envelope: KukuriEnvelope,
) -> Result<()> {
    let topic_replica = channel_replica_for(object.topic_id.as_str(), object.channel_id.as_ref());
    let sort_key = timeline_sort_key(object.created_at, &object.object_id);
    let object_json = serde_json::to_value(&object)?;
    docs_sync.open_replica(&topic_replica).await?;
    docs_sync
        .apply_doc_op(
            &topic_replica,
            DocOp::SetJson {
                key: stable_key("objects", &format!("{}/state", object.object_id.as_str())),
                value: object_json,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &topic_replica,
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
            &topic_replica,
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
            &topic_replica,
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

async fn persist_media_manifest(
    topic: &TopicId,
    channel_id: Option<&ChannelId>,
    envelope: &KukuriEnvelope,
    manifest: &KukuriMediaManifestV1,
    docs_sync: &dyn DocsSync,
) -> Result<()> {
    let replica = channel_replica_for(topic.as_str(), channel_id);
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
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
            &replica,
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

async fn persist_live_session_state(
    docs_sync: &dyn DocsSync,
    state: &LiveSessionStateDocV1,
) -> Result<()> {
    let replica = channel_replica_for(state.topic_id.as_str(), state.channel_id.as_ref());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

async fn persist_game_room_state(
    docs_sync: &dyn DocsSync,
    state: &GameRoomStateDocV1,
) -> Result<()> {
    let replica = channel_replica_for(state.topic_id.as_str(), state.channel_id.as_ref());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
                value: serde_json::to_value(state)?,
            },
        )
        .await?;
    Ok(())
}

async fn persist_private_channel_metadata(
    docs_sync: &dyn DocsSync,
    _topic_id: &str,
    channel_id: &ChannelId,
    metadata: &PrivateChannelMetadataDocV1,
) -> Result<()> {
    let replica = private_channel_replica_id(channel_id.as_str());
    docs_sync.open_replica(&replica).await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("channels", "metadata"),
                value: serde_json::to_value(metadata)?,
            },
        )
        .await?;
    docs_sync
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("channels", "topic"),
                value: serde_json::json!({ "topic_id": metadata.topic_id }),
            },
        )
        .await
}

async fn store_manifest_blob<T: Serialize>(
    blob_service: &dyn BlobService,
    manifest: &T,
    mime: &str,
) -> Result<StoredBlob> {
    let payload = serde_json::to_vec(manifest)?;
    blob_service.put_blob(payload, mime).await
}

async fn fetch_manifest_blob<T: DeserializeOwned>(
    blob_service: &dyn BlobService,
    blob_ref: &ManifestBlobRef,
) -> Result<Option<T>> {
    let Some(bytes) = blob_service.fetch_blob(&blob_ref.hash).await? else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&bytes)?))
}

async fn fetch_live_session_state_from_replica(
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

async fn fetch_game_room_state_from_replica(
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

fn live_projection_row_from_state(
    state: &LiveSessionStateDocV1,
    manifest: &LiveSessionManifestBlobV1,
    topic_id: &str,
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
        source_replica_id: channel_replica_for(topic_id, state.channel_id.as_ref()),
        source_key: stable_key("sessions/live", &format!("{}/state", state.session_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
        viewer_count: 0,
    }
}

fn game_projection_row_from_state(
    state: &GameRoomStateDocV1,
    manifest: &GameRoomManifestBlobV1,
    topic_id: &str,
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
        source_replica_id: channel_replica_for(topic_id, state.channel_id.as_ref()),
        source_key: stable_key("sessions/game", &format!("{}/state", state.room_id)),
        manifest_blob_hash: state.current_manifest.hash.clone(),
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

fn projection_row_from_header(
    header: &CanonicalPostHeader,
    content: Option<String>,
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
        root_object_id: header.root.clone(),
        reply_to_object_id: header.reply_to.clone(),
        payload_ref: header.payload_ref.clone(),
        content,
        source_replica_id: channel_replica_for(
            header.topic_id.as_str(),
            header.channel_id.as_ref(),
        ),
        source_key: stable_key("objects", &format!("{}/state", header.object_id.as_str())),
        source_envelope_id: header.envelope_id.clone(),
        source_blob_hash,
        derived_at: Utc::now().timestamp_millis(),
        projection_version: 1,
    }
}

async fn hydrate_object_projection_from_replica(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    replica: &ReplicaId,
) -> Result<usize> {
    let records = docs_sync
        .query_replica(replica, DocQuery::Prefix("objects/".into()))
        .await?;
    let mut hydrated = 0usize;
    for record in records {
        if !record.key.ends_with("/state") {
            continue;
        }
        let header: CanonicalPostHeader = serde_json::from_slice(&record.value)?;
        let content = match &header.payload_ref {
            PayloadRef::InlineText { text } => Some(text.clone()),
            PayloadRef::BlobText { hash, .. } => {
                let payload = blob_service
                    .fetch_blob(hash)
                    .await?
                    .map(|bytes| String::from_utf8_lossy(&bytes).to_string());
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
            let status = match blob_service.blob_status(&attachment.hash).await? {
                BlobStatus::Missing => BlobCacheStatus::Missing,
                BlobStatus::Available => BlobCacheStatus::Available,
                BlobStatus::Pinned => BlobCacheStatus::Pinned,
            };
            projection_store
                .mark_blob_status(&attachment.hash, status)
                .await?;
        }
        projection_store
            .put_object_projection(projection_row_from_header(&header, content))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

async fn hydrate_topic_state_with_services(
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

async fn hydrate_subscription_state_with_services(
    docs_sync: &dyn DocsSync,
    blob_service: &dyn BlobService,
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    replica: &ReplicaId,
) -> Result<usize> {
    let post_count =
        hydrate_object_projection_from_replica(docs_sync, blob_service, projection_store, replica)
            .await?;
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
    Ok(post_count + live_count + game_count)
}

async fn hydrate_live_sessions_from_replica(
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
            .upsert_live_session_cache(live_projection_row_from_state(&state, &manifest, topic_id))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

async fn hydrate_game_rooms_from_replica(
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
            .upsert_game_room_cache(game_projection_row_from_state(&state, &manifest, topic_id))
            .await?;
        hydrated += 1;
    }
    Ok(hydrated)
}

fn hint_targets_topic(hint: &GossipHint, topic: &str) -> bool {
    match hint {
        GossipHint::TopicObjectsChanged { topic_id, .. }
        | GossipHint::Presence { topic_id, .. }
        | GossipHint::Typing { topic_id, .. }
        | GossipHint::SessionChanged { topic_id, .. }
        | GossipHint::LivePresence { topic_id, .. } => topic_id.as_str() == topic,
        GossipHint::ThreadUpdated { .. } | GossipHint::ProfileUpdated { .. } => true,
    }
}

fn projection_page_needs_hydration(page: &Page<ObjectProjectionRow>) -> bool {
    page.items.iter().any(|item| item.content.is_none())
}

async fn filtered_timeline_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channels: &BTreeSet<String>,
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
        let page = ProjectionStore::list_topic_timeline(
            projection_store,
            topic_id,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if allowed_channels.contains(row.channel_id.as_str()) {
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

async fn filtered_thread_page(
    projection_store: &dyn ProjectionStore,
    topic_id: &str,
    thread_root_object_id: &EnvelopeId,
    cursor: Option<TimelineCursor>,
    limit: usize,
    allowed_channel: Option<&str>,
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
        let page = ProjectionStore::list_thread(
            projection_store,
            topic_id,
            thread_root_object_id,
            current_cursor.clone(),
            page_size,
        )
        .await?;
        let next_cursor = page.next_cursor.clone();
        for row in page.items {
            if allowed_channel.is_none_or(|channel_id| row.channel_id == channel_id) {
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

fn filter_channel_rows<T>(
    rows: Vec<T>,
    allowed_channels: &BTreeSet<String>,
    channel_id: impl Fn(&T) -> &str,
) -> Vec<T> {
    rows.into_iter()
        .filter(|row| allowed_channels.contains(channel_id(row)))
        .collect()
}

async fn fetch_post_object_for_projection(
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

fn joined_private_channel_view_from_state(
    state: &JoinedPrivateChannelState,
) -> JoinedPrivateChannelView {
    JoinedPrivateChannelView {
        topic_id: state.topic_id.clone(),
        channel_id: state.channel_id.as_str().to_string(),
        label: state.label.clone(),
        creator_pubkey: state.creator_pubkey.clone(),
    }
}

fn private_channel_capability_from_state(
    state: JoinedPrivateChannelState,
) -> PrivateChannelCapability {
    PrivateChannelCapability {
        topic_id: state.topic_id,
        channel_id: state.channel_id.as_str().to_string(),
        label: state.label,
        creator_pubkey: state.creator_pubkey,
        namespace_secret_hex: state.namespace_secret_hex,
    }
}

fn subscription_replicas_for_topic(
    topic_id: &str,
    joined_channels: Vec<JoinedPrivateChannelState>,
) -> Vec<ReplicaId> {
    let mut replicas = vec![topic_replica_id(topic_id)];
    replicas.extend(
        joined_channels
            .into_iter()
            .map(|state| private_channel_replica_id(state.channel_id.as_str())),
    );
    replicas
}

async fn blob_view_status_for_payload(
    blob_service: &dyn BlobService,
    payload_ref: &PayloadRef,
) -> Result<BlobViewStatus> {
    match payload_ref {
        PayloadRef::InlineText { .. } => Ok(BlobViewStatus::Available),
        PayloadRef::BlobText { hash, .. } => {
            let status = blob_service.blob_status(hash).await?;
            Ok(blob_view_status(status))
        }
    }
}

async fn attachment_views(
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
            status: blob_view_status(blob_service.blob_status(&attachment.hash).await?),
        });
    }
    Ok(attachments)
}

fn blob_view_status(status: BlobStatus) -> BlobViewStatus {
    match status {
        BlobStatus::Missing => BlobViewStatus::Missing,
        BlobStatus::Available => BlobViewStatus::Available,
        BlobStatus::Pinned => BlobViewStatus::Pinned,
    }
}

fn blob_status(status: BlobStatus) -> BlobCacheStatus {
    match status {
        BlobStatus::Missing => BlobCacheStatus::Missing,
        BlobStatus::Available => BlobCacheStatus::Available,
        BlobStatus::Pinned => BlobCacheStatus::Pinned,
    }
}

fn attachment_role_name(role: &AssetRole) -> &'static str {
    match role {
        AssetRole::ImageOriginal => "image_original",
        AssetRole::ImagePreview => "image_preview",
        AssetRole::VideoPoster => "video_poster",
        AssetRole::VideoManifest => "video_manifest",
        AssetRole::Attachment => "attachment",
    }
}

fn sanitize_game_participants(participants: Vec<String>) -> Result<Vec<String>> {
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

fn validate_game_room_transition(current: &GameRoomStatus, next: &GameRoomStatus) -> Result<()> {
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

fn validate_game_room_scores(
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

fn channel_storage_id(channel_id: Option<&ChannelId>) -> String {
    channel_id
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| PUBLIC_CHANNEL_ID.to_string())
}

fn channel_replica_for(topic_id: &str, channel_id: Option<&ChannelId>) -> ReplicaId {
    channel_id
        .map(|value| private_channel_replica_id(value.as_str()))
        .unwrap_or_else(|| topic_replica_id(topic_id))
}

fn channel_hint_topic_for(topic_id: &str, channel_id: Option<&ChannelId>) -> TopicId {
    channel_id
        .map(|value| private_channel_hint_topic(value.as_str()))
        .unwrap_or_else(|| TopicId::new(topic_id))
}

fn channel_id_from_storage(channel_id: &str) -> Option<ChannelId> {
    (channel_id != PUBLIC_CHANNEL_ID).then(|| ChannelId::new(channel_id.to_string()))
}

fn channel_id_for_view(channel_id: &str) -> Option<String> {
    channel_id_from_storage(channel_id).map(|value| value.as_str().to_string())
}

fn joined_private_channel_key(topic_id: &str, channel_id: &str) -> String {
    format!("{topic_id}::{channel_id}")
}

fn live_presence_task_key(topic_id: &str, channel_id: &str, session_id: &str) -> String {
    format!("{topic_id}::{channel_id}::{session_id}")
}

fn short_id_suffix(author_pubkey: &str) -> &str {
    author_pubkey.get(..8).unwrap_or(author_pubkey)
}

fn normalize_topic_name(topic: String) -> Option<String> {
    let normalized = topic
        .strip_prefix("hint/")
        .map_or(topic.clone(), ToOwned::to_owned);
    if normalized.starts_with("private/") {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_topics(topics: Vec<String>) -> Vec<String> {
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

fn normalize_topic_diagnostics(diagnostics: Vec<TopicPeerSnapshot>) -> Vec<TopicPeerSnapshot> {
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

fn merge_peer_ids(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    left.into_iter()
        .chain(right)
        .filter(|peer| !peer.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn effective_sync_status_detail(
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

fn effective_topic_status_detail(
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
mod tests {
    use super::*;
    use async_trait::async_trait;
    use iroh::address_lookup::EndpointInfo;
    use kukuri_blob_service::IrohBlobService;
    use kukuri_core::build_post_envelope_with_payload;
    use kukuri_docs_sync::IrohDocsNode;
    use kukuri_docs_sync::IrohDocsSync;
    use kukuri_store::MemoryStore;
    use kukuri_transport::{
        DhtDiscoveryOptions, DiscoveryMode, FakeNetwork, FakeTransport, HintEnvelope, HintStream,
        IrohGossipTransport, SeedPeer,
    };
    use pkarr::errors::{ConcurrencyError, PublishError};
    use pkarr::{Client as PkarrClient, SignedPacket, Timestamp, mainline::Testnet};
    use tempfile::tempdir;
    use tokio::sync::{Mutex as TokioMutex, broadcast};
    use tokio::time::{Duration, sleep, timeout};
    use tokio_stream::wrappers::BroadcastStream;

    #[derive(Clone)]
    struct StaticTransport {
        peers: Arc<TokioMutex<PeerSnapshot>>,
        hints: Arc<TokioMutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
        local_ticket: String,
    }

    impl StaticTransport {
        fn new(peers: PeerSnapshot) -> Self {
            Self {
                peers: Arc::new(TokioMutex::new(peers)),
                hints: Arc::new(TokioMutex::new(HashMap::new())),
                local_ticket: "static-peer".into(),
            }
        }

        async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
            let mut guard = self.hints.lock().await;
            guard
                .entry(topic.as_str().to_string())
                .or_insert_with(|| broadcast::channel(64).0)
                .clone()
        }
    }

    #[derive(Clone, Default)]
    struct AssistedDocsSync {
        peer_ids: Vec<String>,
    }

    impl AssistedDocsSync {
        fn new(peer_ids: Vec<&str>) -> Self {
            Self {
                peer_ids: peer_ids.into_iter().map(str::to_string).collect(),
            }
        }
    }

    #[async_trait]
    impl DocsSync for AssistedDocsSync {
        async fn open_replica(&self, _replica_id: &ReplicaId) -> Result<()> {
            Ok(())
        }

        async fn apply_doc_op(&self, _replica_id: &ReplicaId, _op: DocOp) -> Result<()> {
            Ok(())
        }

        async fn query_replica(
            &self,
            _replica_id: &ReplicaId,
            _query: DocQuery,
        ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
            Ok(Vec::new())
        }

        async fn subscribe_replica(
            &self,
            _replica_id: &ReplicaId,
        ) -> Result<kukuri_docs_sync::DocEventStream> {
            let (sender, _) = broadcast::channel::<kukuri_docs_sync::DocEvent>(1);
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok().map(Ok) });
            Ok(Box::pin(stream))
        }

        async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }

        async fn assist_peer_ids(&self) -> Result<Vec<String>> {
            Ok(self.peer_ids.clone())
        }
    }

    #[derive(Clone, Default)]
    struct TrackingDocsSync {
        restarted_replicas: Arc<TokioMutex<Vec<String>>>,
    }

    #[async_trait]
    impl DocsSync for TrackingDocsSync {
        async fn open_replica(&self, _replica_id: &ReplicaId) -> Result<()> {
            Ok(())
        }

        async fn apply_doc_op(&self, _replica_id: &ReplicaId, _op: DocOp) -> Result<()> {
            Ok(())
        }

        async fn query_replica(
            &self,
            _replica_id: &ReplicaId,
            _query: DocQuery,
        ) -> Result<Vec<kukuri_docs_sync::DocRecord>> {
            Ok(Vec::new())
        }

        async fn subscribe_replica(
            &self,
            _replica_id: &ReplicaId,
        ) -> Result<kukuri_docs_sync::DocEventStream> {
            let (sender, _) = broadcast::channel::<kukuri_docs_sync::DocEvent>(1);
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok().map(Ok) });
            Ok(Box::pin(stream))
        }

        async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }

        async fn restart_replica_sync(&self, replica_id: &ReplicaId) -> Result<()> {
            self.restarted_replicas
                .lock()
                .await
                .push(replica_id.as_str().to_string());
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct AssistedBlobService {
        peer_ids: Vec<String>,
    }

    impl AssistedBlobService {
        fn new(peer_ids: Vec<&str>) -> Self {
            Self {
                peer_ids: peer_ids.into_iter().map(str::to_string).collect(),
            }
        }
    }

    #[async_trait]
    impl BlobService for AssistedBlobService {
        async fn put_blob(&self, _data: Vec<u8>, mime: &str) -> Result<StoredBlob> {
            Ok(StoredBlob {
                hash: kukuri_core::BlobHash::new("test-hash"),
                mime: mime.to_string(),
                bytes: 0,
            })
        }

        async fn fetch_blob(&self, _hash: &kukuri_core::BlobHash) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }

        async fn pin_blob(&self, _hash: &kukuri_core::BlobHash) -> Result<()> {
            Ok(())
        }

        async fn blob_status(&self, _hash: &kukuri_core::BlobHash) -> Result<BlobStatus> {
            Ok(BlobStatus::Missing)
        }

        async fn import_peer_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }

        async fn assist_peer_ids(&self) -> Result<Vec<String>> {
            Ok(self.peer_ids.clone())
        }
    }

    async fn persist_test_post(
        docs_sync: &dyn DocsSync,
        projection_store: Option<&dyn ProjectionStore>,
        keys: &KukuriKeys,
        topic: &TopicId,
        payload_ref: PayloadRef,
        attachments: Vec<kukuri_core::AssetRef>,
        reply_to: Option<&KukuriEnvelope>,
    ) -> KukuriEnvelope {
        let envelope = build_post_envelope_with_payload(
            keys,
            topic,
            payload_ref,
            attachments,
            Vec::new(),
            reply_to,
            ObjectVisibility::Public,
        )
        .expect("event");
        let object = envelope
            .to_post_object()
            .expect("post object")
            .expect("post object");
        persist_post_object(docs_sync, object.clone(), envelope.clone())
            .await
            .expect("persist post object");
        if let Some(projection_store) = projection_store {
            ProjectionStore::put_object_projection(
                projection_store,
                projection_row_from_header(&object, None),
            )
            .await
            .expect("put placeholder projection");
        }
        envelope
    }

    #[async_trait]
    impl Transport for StaticTransport {
        async fn peers(&self) -> Result<PeerSnapshot> {
            Ok(self.peers.lock().await.clone())
        }

        async fn export_ticket(&self) -> Result<Option<String>> {
            Ok(Some(self.local_ticket.clone()))
        }

        async fn import_ticket(&self, _ticket: &str) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl HintTransport for StaticTransport {
        async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
            let sender = self.hint_sender(topic).await;
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok() });
            Ok(Box::pin(stream))
        }

        async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
            let sender = self.hint_sender(topic).await;
            let _ = sender.send(HintEnvelope {
                hint,
                received_at: Utc::now().timestamp_millis(),
                source_peer: "static".into(),
            });
            Ok(())
        }
    }

    struct TestIrohStack {
        _node: Arc<IrohDocsNode>,
        transport: Arc<IrohGossipTransport>,
        docs_sync: Arc<IrohDocsSync>,
        blob_service: Arc<IrohBlobService>,
    }

    impl TestIrohStack {
        async fn new(root: &std::path::Path) -> Self {
            Self::new_with_discovery(root, DhtDiscoveryOptions::disabled()).await
        }

        async fn new_with_dht(root: &std::path::Path, testnet: &Testnet) -> Self {
            let stack = Self::new_with_discovery(
                root,
                DhtDiscoveryOptions::with_client(dht_test_client(testnet)),
            )
            .await;
            publish_endpoint_to_testnet(stack._node.endpoint(), testnet).await;
            stack
        }

        async fn new_with_discovery(
            root: &std::path::Path,
            dht_options: DhtDiscoveryOptions,
        ) -> Self {
            let node = IrohDocsNode::persistent_with_discovery_config(
                root,
                kukuri_transport::TransportNetworkConfig::loopback(),
                dht_options,
                kukuri_transport::TransportRelayConfig::default(),
            )
            .await
            .expect("iroh docs node");
            let transport = Arc::new(
                IrohGossipTransport::from_shared_parts(
                    node.endpoint().clone(),
                    node.gossip().clone(),
                    node.discovery(),
                    kukuri_transport::TransportNetworkConfig::loopback(),
                    kukuri_transport::TransportRelayConfig::default(),
                )
                .expect("transport"),
            );
            let docs_sync = Arc::new(IrohDocsSync::new(node.clone()));
            let blob_service = Arc::new(IrohBlobService::new(node.clone()));
            Self {
                _node: node,
                transport,
                docs_sync,
                blob_service,
            }
        }
    }

    fn dht_test_client(testnet: &Testnet) -> PkarrClient {
        let mut builder = PkarrClient::builder();
        builder.no_default_network().bootstrap(&testnet.bootstrap);
        builder.build().expect("pkarr client")
    }

    fn build_endpoint_signed_packet_with_timestamp(
        endpoint_info: &EndpointInfo,
        secret_key: &iroh::SecretKey,
        ttl: u32,
        timestamp: Timestamp,
    ) -> SignedPacket {
        use pkarr::dns::{self, rdata};

        let keypair = pkarr::Keypair::from_secret_key(&secret_key.to_bytes());
        let mut builder = SignedPacket::builder().timestamp(timestamp);
        let name = dns::Name::new("_iroh").expect("iroh txt name");
        for entry in endpoint_info.to_txt_strings() {
            let mut txt = rdata::TXT::new();
            txt.add_string(&entry)
                .expect("valid endpoint info txt entry");
            builder = builder.txt(name.clone(), txt.into_owned(), ttl);
        }
        builder.sign(&keypair).expect("sign endpoint info packet")
    }

    async fn publish_endpoint_to_testnet(endpoint: &iroh::Endpoint, testnet: &Testnet) {
        let client = dht_test_client(testnet);
        let public_key =
            pkarr::PublicKey::try_from(endpoint.id().as_bytes()).expect("pkarr public key");
        let expected_info = EndpointInfo::from(endpoint.addr());
        for _ in 0..20 {
            let previous_timestamp = client
                .resolve_most_recent(&public_key)
                .await
                .map(|packet| packet.timestamp());
            let now = Timestamp::now();
            let timestamp = match previous_timestamp {
                Some(previous) if previous >= now => previous + 1,
                _ => now,
            };
            let signed_packet = build_endpoint_signed_packet_with_timestamp(
                &expected_info,
                endpoint.secret_key(),
                1,
                timestamp,
            );
            match client.publish(&signed_packet, previous_timestamp).await {
                Ok(()) => break,
                Err(PublishError::Concurrency(
                    ConcurrencyError::ConflictRisk
                    | ConcurrencyError::NotMostRecent
                    | ConcurrencyError::CasFailed,
                )) => sleep(Duration::from_millis(50)).await,
                Err(error) => panic!("publish endpoint info: {error}"),
            }
        }
        timeout(Duration::from_secs(5), async {
            loop {
                if client
                    .resolve_most_recent(&public_key)
                    .await
                    .as_ref()
                    .and_then(|packet| EndpointInfo::from_pkarr_signed_packet(packet).ok())
                    .is_some_and(|packet_info| {
                        packet_info.to_txt_strings() == expected_info.to_txt_strings()
                    })
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("resolve published endpoint info");
    }

    async fn configure_seeded_dht(app: &AppService, remote_endpoint_id: String) {
        app.set_discovery_seeds(
            DiscoveryMode::SeededDht,
            false,
            vec![SeedPeer {
                endpoint_id: remote_endpoint_id,
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("configure seeded dht");
    }

    fn app_with_iroh_services(store: Arc<MemoryStore>, stack: &TestIrohStack) -> AppService {
        AppService::new_with_services(
            store.clone(),
            store,
            stack.transport.clone(),
            stack.transport.clone(),
            stack.docs_sync.clone(),
            stack.blob_service.clone(),
            generate_keys(),
        )
    }

    fn pending_image_attachment(mime: &str, bytes: &[u8]) -> PendingAttachment {
        PendingAttachment {
            mime: mime.to_string(),
            bytes: bytes.to_vec(),
            role: AssetRole::ImageOriginal,
        }
    }

    fn pending_video_attachment(role: AssetRole, mime: &str, bytes: &[u8]) -> PendingAttachment {
        PendingAttachment {
            mime: mime.to_string(),
            bytes: bytes.to_vec(),
            role,
        }
    }

    #[derive(Clone)]
    struct NoopHintTransport;

    #[async_trait]
    impl HintTransport for NoopHintTransport {
        async fn subscribe_hints(&self, _topic: &TopicId) -> Result<HintStream> {
            Ok(Box::pin(futures_util::stream::empty()))
        }

        async fn unsubscribe_hints(&self, _topic: &TopicId) -> Result<()> {
            Ok(())
        }

        async fn publish_hint(&self, _topic: &TopicId, _hint: GossipHint) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct TrackingHintTransport {
        hints: Arc<TokioMutex<HashMap<String, broadcast::Sender<HintEnvelope>>>>,
        unsubscribed_topics: Arc<TokioMutex<Vec<String>>>,
    }

    impl TrackingHintTransport {
        async fn hint_sender(&self, topic: &TopicId) -> broadcast::Sender<HintEnvelope> {
            let mut guard = self.hints.lock().await;
            guard
                .entry(topic.as_str().to_string())
                .or_insert_with(|| broadcast::channel(64).0)
                .clone()
        }
    }

    #[async_trait]
    impl HintTransport for TrackingHintTransport {
        async fn subscribe_hints(&self, topic: &TopicId) -> Result<HintStream> {
            let sender = self.hint_sender(topic).await;
            let stream = BroadcastStream::new(sender.subscribe())
                .filter_map(|item| async move { item.ok() });
            Ok(Box::pin(stream))
        }

        async fn unsubscribe_hints(&self, topic: &TopicId) -> Result<()> {
            self.unsubscribed_topics
                .lock()
                .await
                .push(topic.as_str().to_string());
            Ok(())
        }

        async fn publish_hint(&self, topic: &TopicId, hint: GossipHint) -> Result<()> {
            let sender = self.hint_sender(topic).await;
            let _ = sender.send(HintEnvelope {
                hint,
                received_at: Utc::now().timestamp_millis(),
                source_peer: "tracking".into(),
            });
            Ok(())
        }
    }

    async fn assert_docs_sync_recovers_post_without_hints(topic: &str, content: &str) {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a,
            stack_a.transport.clone(),
            Arc::new(NoopHintTransport),
            stack_a.docs_sync.clone(),
            stack_a.blob_service.clone(),
            generate_keys(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b,
            stack_b.transport.clone(),
            Arc::new(NoopHintTransport),
            stack_b.docs_sync.clone(),
            stack_b.blob_service.clone(),
            generate_keys(),
        );

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let object_id = app_a
            .create_post(topic, content, None)
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("missing gossip timeout");

        assert_eq!(received.content, content);
    }

    #[tokio::test]
    async fn create_post_and_list_timeline() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post("kukuri:topic:api", "hello app", None)
            .await
            .expect("create post");
        let timeline = app
            .list_timeline("kukuri:topic:api", None, 10)
            .await
            .expect("timeline");

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].object_id, object_id);
        assert_eq!(timeline.items[0].content, "hello app");
    }

    #[tokio::test]
    async fn create_post_with_image_attachment_surfaces_attachment_metadata() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post_with_attachments(
                "kukuri:topic:image-write",
                "caption",
                None,
                vec![PendingAttachment {
                    mime: "image/png".into(),
                    bytes: b"fake-image".to_vec(),
                    role: AssetRole::ImageOriginal,
                }],
            )
            .await
            .expect("create image post");
        let timeline = app
            .list_timeline("kukuri:topic:image-write", None, 10)
            .await
            .expect("timeline");

        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("image post");
        assert_eq!(post.content, "caption");
        assert_eq!(post.attachments.len(), 1);
        assert_eq!(post.attachments[0].mime, "image/png");
        assert_eq!(post.attachments[0].role, "image_original");
        assert_eq!(post.attachments[0].status, BlobViewStatus::Available);
    }

    #[tokio::test]
    async fn create_post_with_image_only_succeeds() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post_with_attachments(
                "kukuri:topic:image-only",
                "",
                None,
                vec![PendingAttachment {
                    mime: "image/jpeg".into(),
                    bytes: b"fake-jpeg".to_vec(),
                    role: AssetRole::ImageOriginal,
                }],
            )
            .await
            .expect("create image-only post");
        let timeline = app
            .list_timeline("kukuri:topic:image-only", None, 10)
            .await
            .expect("timeline");

        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("image-only post");
        assert_eq!(post.attachments.len(), 1);
        assert_eq!(post.attachments[0].mime, "image/jpeg");
    }

    #[tokio::test]
    async fn create_post_with_video_attachments_surfaces_video_metadata() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let object_id = app
            .create_post_with_attachments(
                "kukuri:topic:video-write",
                "video caption",
                None,
                vec![
                    pending_video_attachment(
                        AssetRole::VideoManifest,
                        "video/mp4",
                        b"fake-video-manifest",
                    ),
                    pending_video_attachment(
                        AssetRole::VideoPoster,
                        "image/jpeg",
                        b"fake-video-poster",
                    ),
                ],
            )
            .await
            .expect("create video post");
        let timeline = app
            .list_timeline("kukuri:topic:video-write", None, 10)
            .await
            .expect("timeline");

        let post = timeline
            .items
            .iter()
            .find(|post| post.object_id == object_id)
            .expect("video post");
        assert_eq!(post.attachments.len(), 2);
        assert!(
            post.attachments
                .iter()
                .any(|attachment| attachment.role == "video_manifest")
        );
        assert!(
            post.attachments
                .iter()
                .any(|attachment| attachment.role == "video_poster")
        );
    }

    #[tokio::test]
    async fn tracking_multiple_topics_updates_sync_status() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let _ = app
            .list_timeline("kukuri:topic:one", None, 10)
            .await
            .expect("timeline one");
        let _ = app
            .list_timeline("kukuri:topic:two", None, 10)
            .await
            .expect("timeline two");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:one")
        );
        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:two")
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .any(|topic| topic.topic == "kukuri:topic:one")
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .any(|topic| topic.topic == "kukuri:topic:two")
        );
        assert_eq!(status.status_detail, "No peers configured");
        assert!(
            status
                .topic_diagnostics
                .iter()
                .all(|topic| !topic.status_detail.is_empty())
        );
        assert!(
            status
                .topic_diagnostics
                .iter()
                .all(|topic| topic.last_error.is_none())
        );
    }

    #[tokio::test]
    async fn discovery_status_separates_bootstrap_seed_peers_from_manual_tickets() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        transport
            .configure_discovery(
                DiscoveryMode::StaticPeer,
                false,
                vec![SeedPeer {
                    endpoint_id: "configured-peer".into(),
                    addr_hint: None,
                }],
                vec![SeedPeer {
                    endpoint_id: "bootstrap-peer".into(),
                    addr_hint: None,
                }],
            )
            .await
            .expect("configure discovery");
        transport
            .import_ticket("manual-ticket-peer")
            .await
            .expect("import ticket");
        let app = AppService::new(store, transport);

        let discovery = app.get_discovery_status().await.expect("discovery status");

        assert_eq!(
            discovery.configured_seed_peer_ids,
            vec!["configured-peer".to_string()]
        );
        assert_eq!(
            discovery.bootstrap_seed_peer_ids,
            vec!["bootstrap-peer".to_string()]
        );
        assert_eq!(
            discovery.manual_ticket_peer_ids,
            vec!["manual-ticket-peer".to_string()]
        );
        assert!(discovery.assist_peer_ids.is_empty());
    }

    #[tokio::test]
    async fn relay_assisted_peers_contribute_to_sync_status_and_topic_counts() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot {
            connected: false,
            peer_count: 0,
            connected_peers: Vec::new(),
            configured_peers: vec!["peer-a".into(), "peer-b".into()],
            subscribed_topics: vec!["kukuri:topic:relay-assisted".into()],
            pending_events: 0,
            status_detail: "No peers configured".into(),
            last_error: None,
            topic_diagnostics: vec![TopicPeerSnapshot {
                topic: "kukuri:topic:relay-assisted".into(),
                joined: false,
                peer_count: 0,
                connected_peers: Vec::new(),
                configured_peer_ids: vec!["peer-a".into(), "peer-b".into()],
                missing_peer_ids: vec!["peer-a".into(), "peer-b".into()],
                last_received_at: None,
                status_detail: "No peers configured".into(),
                last_error: None,
            }],
        }));
        let docs_sync = Arc::new(AssistedDocsSync::new(vec!["peer-a", "peer-b"]));
        let blob_service = Arc::new(AssistedBlobService::new(vec!["peer-b", "peer-c"]));
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            generate_keys(),
        );

        let status = app.get_sync_status().await.expect("sync status");

        assert!(status.connected);
        assert_eq!(status.peer_count, 3);
        assert_eq!(
            status.status_detail,
            "relay-assisted sync available via 3 peer(s)"
        );
        assert_eq!(
            status.discovery.assist_peer_ids,
            vec![
                "peer-a".to_string(),
                "peer-b".to_string(),
                "peer-c".to_string()
            ]
        );
        assert_eq!(status.topic_diagnostics.len(), 1);
        assert!(status.topic_diagnostics[0].joined);
        assert_eq!(status.topic_diagnostics[0].peer_count, 3);
        assert_eq!(
            status.topic_diagnostics[0].assist_peer_ids,
            vec![
                "peer-a".to_string(),
                "peer-b".to_string(),
                "peer-c".to_string()
            ]
        );
        assert_eq!(
            status.topic_diagnostics[0].status_detail,
            "relay-assisted sync available via 3 peer(s)"
        );
    }

    #[tokio::test]
    async fn list_timeline_restarts_topic_replica_sync_with_cooldown_when_projection_is_empty() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(TrackingDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync.clone(),
            blob_service,
            generate_keys(),
        );

        let timeline = app
            .list_timeline("kukuri:topic:replica-restart", None, 20)
            .await
            .expect("timeline");
        assert!(timeline.items.is_empty());

        let second_timeline = app
            .list_timeline("kukuri:topic:replica-restart", None, 20)
            .await
            .expect("second timeline");
        assert!(second_timeline.items.is_empty());

        let restarted = docs_sync.restarted_replicas.lock().await.clone();
        assert_eq!(
            restarted,
            vec![
                topic_replica_id("kukuri:topic:replica-restart")
                    .as_str()
                    .to_string()
            ]
        );
    }

    #[tokio::test]
    async fn set_discovery_seeds_restarts_topic_hint_subscription() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let hint_transport = Arc::new(TrackingHintTransport::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            hint_transport.clone(),
            Arc::new(MemoryDocsSync::default()),
            Arc::new(MemoryBlobService::default()),
            generate_keys(),
        );
        let topic = "kukuri:topic:hint-restart";

        let _ = app
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe timeline");

        app.set_discovery_seeds(
            DiscoveryMode::StaticPeer,
            false,
            vec![SeedPeer {
                endpoint_id: "peer-a".into(),
                addr_hint: None,
            }],
            Vec::new(),
        )
        .await
        .expect("set discovery seeds");

        assert_eq!(
            hint_transport.unsubscribed_topics.lock().await.clone(),
            vec![topic.to_string()]
        );
    }

    #[tokio::test]
    async fn list_timeline_rehydrates_placeholder_from_blob_store() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:hydrate");
        let stored_blob = blob_service
            .put_blob(b"hello after blob fetch".to_vec(), "text/plain")
            .await
            .expect("put blob");
        persist_test_post(
            docs_sync.as_ref(),
            Some(store.as_ref()),
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: stored_blob.hash.clone(),
                mime: stored_blob.mime.clone(),
                bytes: stored_blob.bytes,
            },
            Vec::new(),
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            keys,
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].content, "hello after blob fetch");
    }

    #[tokio::test]
    async fn on_demand_hydration_updates_last_sync_ts() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:on-demand-sync-ts");
        let stored_blob = blob_service
            .put_blob(b"hydrate updates sync ts".to_vec(), "text/plain")
            .await
            .expect("put blob");
        persist_test_post(
            docs_sync.as_ref(),
            None,
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: stored_blob.hash,
                mime: stored_blob.mime,
                bytes: stored_blob.bytes,
            },
            Vec::new(),
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            keys,
        );

        assert!(
            app.get_sync_status()
                .await
                .expect("status")
                .last_sync_ts
                .is_none()
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert_eq!(timeline.items.len(), 1);

        assert!(
            app.get_sync_status()
                .await
                .expect("status")
                .last_sync_ts
                .is_some()
        );
    }

    #[tokio::test]
    async fn sync_status_normalizes_hint_topic_names() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot {
            connected: true,
            peer_count: 1,
            connected_peers: vec!["peer-a".into()],
            configured_peers: vec!["peer-a".into()],
            subscribed_topics: vec!["hint/kukuri:topic:demo".into()],
            pending_events: 0,
            status_detail: "Connected".into(),
            last_error: None,
            topic_diagnostics: vec![TopicPeerSnapshot {
                topic: "hint/kukuri:topic:demo".into(),
                joined: true,
                peer_count: 1,
                connected_peers: vec!["peer-a".into()],
                configured_peer_ids: vec!["peer-a".into()],
                missing_peer_ids: Vec::new(),
                last_received_at: Some(1),
                status_detail: "Connected".into(),
                last_error: None,
            }],
        }));
        let app = AppService::new(store, transport);

        let status = app.get_sync_status().await.expect("sync status");

        assert_eq!(status.subscribed_topics, vec!["kukuri:topic:demo"]);
        assert_eq!(status.topic_diagnostics.len(), 1);
        assert_eq!(status.topic_diagnostics[0].topic, "kukuri:topic:demo");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_ticket_updates_sync_status_error_reason() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(
            IrohGossipTransport::bind_local()
                .await
                .expect("transport should bind"),
        );
        let app = AppService::new(store, transport);

        let error = app
            .import_peer_ticket("not-a-ticket")
            .await
            .expect_err("invalid ticket should fail");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(error.to_string().contains("failed to import peer ticket"));
        assert!(
            status
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("failed to import peer ticket"))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn missing_gossip_but_docs_sync_recovers_post() {
        assert_docs_sync_recovers_post_without_hints("kukuri:topic:missing-gossip", "docs recover")
            .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn gossip_loss_does_not_lose_durable_post() {
        assert_docs_sync_recovers_post_without_hints(
            "kukuri:topic:gossip-loss",
            "durable docs payload",
        )
        .await;
    }

    #[tokio::test]
    async fn thread_open_triggers_lazy_blob_fetch() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:thread-lazy");
        let root_blob = blob_service
            .put_blob(b"root body".to_vec(), "text/plain")
            .await
            .expect("put root blob");
        let root = persist_test_post(
            docs_sync.as_ref(),
            Some(store.as_ref()),
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: root_blob.hash,
                mime: root_blob.mime,
                bytes: root_blob.bytes,
            },
            Vec::new(),
            None,
        )
        .await;
        let reply_blob = blob_service
            .put_blob(b"reply body".to_vec(), "text/plain")
            .await
            .expect("put reply blob");
        let _reply = persist_test_post(
            docs_sync.as_ref(),
            Some(store.as_ref()),
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: reply_blob.hash,
                mime: reply_blob.mime,
                bytes: reply_blob.bytes,
            },
            Vec::new(),
            Some(&root),
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            docs_sync,
            blob_service,
            generate_keys(),
        );

        let thread = app
            .list_thread(topic.as_str(), root.id.as_str(), None, 20)
            .await
            .expect("thread");

        assert_eq!(thread.items.len(), 2);
        assert!(thread.items.iter().any(|post| post.content == "root body"));
        assert!(thread.items.iter().any(|post| post.content == "reply body"));
    }

    #[tokio::test]
    async fn image_post_visible_before_full_blob_download() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:image");
        let image_bytes = b"fake image bytes".to_vec();
        let image_hash = kukuri_core::blob_hash(&image_bytes);
        persist_test_post(
            docs_sync.as_ref(),
            None,
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: kukuri_core::BlobHash::new("f".repeat(64)),
                mime: "text/plain".into(),
                bytes: 0,
            },
            vec![kukuri_core::AssetRef {
                hash: image_hash.clone(),
                mime: "image/png".into(),
                bytes: image_bytes.len() as u64,
                role: kukuri_core::AssetRole::ImageOriginal,
            }],
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport.clone(),
            transport,
            docs_sync,
            blob_service.clone(),
            generate_keys(),
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].content, "[blob pending]");
        assert_eq!(timeline.items[0].content_status, BlobViewStatus::Missing);
        assert_eq!(timeline.items[0].attachments.len(), 1);
        assert_eq!(
            timeline.items[0].attachments[0].status,
            BlobViewStatus::Missing
        );
        assert_eq!(timeline.items[0].attachments[0].role, "image_original");

        blob_service
            .put_blob(image_bytes, "image/png")
            .await
            .expect("put image blob");

        let refreshed = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline after image fetch");
        assert_eq!(refreshed.items.len(), 1);
        assert_eq!(
            refreshed.items[0].attachments[0].status,
            BlobViewStatus::Available
        );
        assert_eq!(refreshed.items[0].attachments[0].mime, "image/png");
    }

    #[tokio::test]
    async fn video_post_visible_before_full_blob_download() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
        let docs_sync = Arc::new(MemoryDocsSync::default());
        let blob_service = Arc::new(MemoryBlobService::default());
        let keys = generate_keys();
        let topic = TopicId::new("kukuri:topic:video");
        let poster_hash = kukuri_core::blob_hash(b"poster-bytes");
        persist_test_post(
            docs_sync.as_ref(),
            None,
            &keys,
            &topic,
            PayloadRef::BlobText {
                hash: kukuri_core::BlobHash::new("f".repeat(64)),
                mime: "text/plain".into(),
                bytes: 13,
            },
            vec![
                kukuri_core::AssetRef {
                    hash: kukuri_core::blob_hash(b"video-bytes"),
                    mime: "video/mp4".into(),
                    bytes: 8192,
                    role: kukuri_core::AssetRole::VideoManifest,
                },
                kukuri_core::AssetRef {
                    hash: poster_hash.clone(),
                    mime: "image/jpeg".into(),
                    bytes: 1024,
                    role: kukuri_core::AssetRole::VideoPoster,
                },
            ],
            None,
        )
        .await;

        let app = AppService::new_with_services(
            store.clone(),
            store.clone(),
            transport.clone(),
            transport,
            docs_sync,
            blob_service.clone(),
            generate_keys(),
        );

        let timeline = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        let post = &timeline.items[0];
        assert!(
            post.attachments
                .iter()
                .any(|attachment| attachment.role == "video_manifest")
        );
        assert!(
            post.attachments
                .iter()
                .find(|attachment| attachment.role == "video_poster")
                .is_some_and(|attachment| attachment.status == BlobViewStatus::Missing)
        );

        blob_service
            .put_blob(b"poster-bytes".to_vec(), "image/jpeg")
            .await
            .expect("put poster blob");
        let refreshed = app
            .list_timeline(topic.as_str(), None, 20)
            .await
            .expect("timeline");
        assert!(
            refreshed.items[0]
                .attachments
                .iter()
                .find(|attachment| attachment.role == "video_poster")
                .is_some_and(|attachment| attachment.status == BlobViewStatus::Available)
        );
    }

    #[tokio::test]
    async fn new_writes_use_blob_text_payload_refs() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store.clone(), transport);
        let topic = "kukuri:topic:blobtext";

        let object_id = app
            .create_post(topic, "blob text only", None)
            .await
            .expect("create post");
        let projection =
            ProjectionStore::get_object_projection(store.as_ref(), &EnvelopeId::from(object_id))
                .await
                .expect("projection")
                .expect("projection row");

        assert!(matches!(
            projection.payload_ref,
            PayloadRef::BlobText { .. }
        ));
        assert!(!matches!(
            projection.payload_ref,
            PayloadRef::InlineText { .. }
        ));
    }

    #[tokio::test]
    async fn blob_media_payload_roundtrip() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let blob_service = Arc::new(MemoryBlobService::default());
        let app = AppService::new_with_services(
            store.clone(),
            store,
            transport.clone(),
            transport,
            Arc::new(MemoryDocsSync::default()),
            blob_service.clone(),
            generate_keys(),
        );

        let stored = blob_service
            .put_blob(b"fake-image".to_vec(), "image/png")
            .await
            .expect("put image");
        let payload = app
            .blob_media_payload(stored.hash.as_str(), "image/png")
            .await
            .expect("media payload")
            .expect("media payload present");

        assert_eq!(payload.bytes_base64, "ZmFrZS1pbWFnZQ==");
        assert_eq!(payload.mime, "image/png");
        assert!(
            app.blob_media_payload(&"f".repeat(64), "image/png")
                .await
                .expect("missing payload")
                .is_none()
        );
    }

    #[tokio::test]
    async fn unsubscribe_topic_removes_subscription_from_sync_status() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
        let app = AppService::new(store, transport);

        let _ = app
            .list_timeline("kukuri:topic:one", None, 10)
            .await
            .expect("timeline one");
        let _ = app
            .list_timeline("kukuri:topic:two", None, 10)
            .await
            .expect("timeline two");
        app.unsubscribe_topic("kukuri:topic:two")
            .await
            .expect("unsubscribe topic");
        let status = app.get_sync_status().await.expect("sync status");

        assert!(
            status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:one")
        );
        assert!(
            !status
                .subscribed_topics
                .iter()
                .any(|topic| topic == "kukuri:topic:two")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_post_between_apps() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("post-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("post-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b.clone(), &stack_b);

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let topic = "kukuri:topic:app-api-iroh";
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("app b should subscribe to topic");

        let object_id = app_a
            .create_post(topic, "hello over iroh transport", None)
            .await
            .expect("app a should create post");

        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("timeline sync timeout");

        assert_eq!(received.content, "hello over iroh transport");
        let status_b = app_b.get_sync_status().await.expect("sync status b");
        assert!(status_b.last_sync_ts.is_some());
        assert!(
            status_b
                .subscribed_topics
                .iter()
                .any(|value| value == topic)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_image_post_between_apps() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("image-post-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("image-post-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b.clone(), &stack_b);

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let topic = "kukuri:topic:image-sync";
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("app b should subscribe to topic");

        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "caption over iroh",
                None,
                vec![pending_image_attachment("image/png", b"fake-image-sync")],
            )
            .await
            .expect("create image post");

        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("image sync timeout");

        assert_eq!(received.content, "caption over iroh");
        assert_eq!(received.attachments.len(), 1);
        assert_eq!(received.attachments[0].mime, "image/png");
        assert_eq!(received.attachments[0].status, BlobViewStatus::Available);
        assert!(
            app_b
                .blob_preview_data_url(received.attachments[0].hash.as_str(), "image/png")
                .await
                .expect("preview data url")
                .is_some()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_video_manifest_payload_available_after_sync() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("video-post-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("video-post-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let topic = "kukuri:topic:video-sync";
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");

        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "video caption",
                None,
                vec![
                    pending_video_attachment(AssetRole::VideoManifest, "video/mp4", b"video-sync"),
                    pending_video_attachment(AssetRole::VideoPoster, "image/jpeg", b"poster-sync"),
                ],
            )
            .await
            .expect("create video post");

        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("video sync timeout");

        assert!(
            received
                .attachments
                .iter()
                .any(|attachment| attachment.role == "video_manifest")
        );
        let poster = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("video poster");
        assert_eq!(poster.status, BlobViewStatus::Available);
        let poster_payload = app_b
            .blob_media_payload(poster.hash.as_str(), "image/jpeg")
            .await
            .expect("poster media payload")
            .expect("poster payload present");
        assert_eq!(poster_payload.mime, "image/jpeg");
        let manifest = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("video manifest");
        let manifest_payload = app_b
            .blob_media_payload(manifest.hash.as_str(), "video/mp4")
            .await
            .expect("video media payload")
            .expect("manifest payload present");
        assert_eq!(manifest_payload.mime, "video/mp4");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn import_peer_ticket_rebuilds_existing_topic_subscription() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("rebind-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("rebind-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:rebind-after-import";

        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a before import");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b before import");

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        timeout(Duration::from_secs(10), async {
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
                let ready_a = status_a.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && topic_status.peer_count > 0
                });
                let ready_b = status_b.topic_diagnostics.iter().any(|topic_status| {
                    topic_status.topic == topic
                        && topic_status.joined
                        && topic_status.peer_count > 0
                });
                if ready_a && ready_b {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("subscription rebuild timeout");

        let object_id = app_a
            .create_post(topic, "hello after import", None)
            .await
            .expect("create post");
        let received = timeout(Duration::from_secs(30), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline should load");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("timeline sync timeout");

        assert_eq!(received.content, "hello after import");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seeded_dht_syncs_post_between_apps_without_ticket_import() {
        let dir = tempdir().expect("tempdir");
        let testnet = Testnet::new(5).expect("testnet");
        let stack_a = TestIrohStack::new_with_dht(&dir.path().join("seeded-dht-a"), &testnet).await;
        let stack_b = TestIrohStack::new_with_dht(&dir.path().join("seeded-dht-b"), &testnet).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let endpoint_a = app_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = app_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;

        configure_seeded_dht(&app_a, endpoint_b.clone()).await;
        configure_seeded_dht(&app_b, endpoint_a.clone()).await;
        let topic = "kukuri:topic:seeded-dht-app";
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a timeline");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        timeout(Duration::from_secs(90), async {
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
                let ready_a = status_a
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                let ready_b = status_b
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                if ready_a && ready_b {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded dht ready timeout");

        let object_id = app_a
            .create_post(topic, "seeded dht app sync", None)
            .await
            .expect("create post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded dht sync timeout");

        assert_eq!(received.content, "seeded dht app sync");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seeded_dht_rebuilds_existing_topic_subscription_after_seed_update() {
        let dir = tempdir().expect("tempdir");
        let testnet = Testnet::new(5).expect("testnet");
        let stack_a =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-rebind-a"), &testnet).await;
        let stack_b =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-rebind-b"), &testnet).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:seeded-rebind";

        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a before seed update");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b before seed update");

        let endpoint_a = app_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = app_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;
        configure_seeded_dht(&app_a, endpoint_b.clone()).await;
        configure_seeded_dht(&app_b, endpoint_a.clone()).await;

        timeout(Duration::from_secs(20), async {
            let mut stable_ready_polls = 0usize;
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
                let ready_a = status_a
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                let ready_b = status_b
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                if ready_a && ready_b {
                    stable_ready_polls += 1;
                    if stable_ready_polls >= 3 {
                        return;
                    }
                } else {
                    stable_ready_polls = 0;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("seeded dht topic rebind timeout");

        let object_id = app_a
            .create_post(topic, "seeded dht rebind", None)
            .await
            .expect("create post");

        timeout(Duration::from_secs(90), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if timeline
                    .items
                    .iter()
                    .any(|post| post.object_id == object_id)
                {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded dht propagation timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seeded_dht_backfills_docs_and_blobs_with_id_only_seed() {
        let dir = tempdir().expect("tempdir");
        let testnet = Testnet::new(5).expect("testnet");
        let stack_a =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-image-a"), &testnet).await;
        let stack_b =
            TestIrohStack::new_with_dht(&dir.path().join("seeded-image-b"), &testnet).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let endpoint_a = app_a
            .get_sync_status()
            .await
            .expect("status a")
            .discovery
            .local_endpoint_id;
        let endpoint_b = app_b
            .get_sync_status()
            .await
            .expect("status b")
            .discovery
            .local_endpoint_id;
        configure_seeded_dht(&app_a, endpoint_b.clone()).await;
        configure_seeded_dht(&app_b, endpoint_a.clone()).await;
        let topic = "kukuri:topic:seeded-image";
        let _ = app_a
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe a timeline");
        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        timeout(Duration::from_secs(20), async {
            loop {
                let status_a = app_a.get_sync_status().await.expect("status a");
                let status_b = app_b.get_sync_status().await.expect("status b");
                let ready_a = status_a
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                let ready_b = status_b
                    .topic_diagnostics
                    .iter()
                    .any(|topic_status| topic_status.topic == topic && topic_status.peer_count > 0);
                if ready_a && ready_b {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded dht image ready timeout");

        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "seeded image",
                None,
                vec![pending_image_attachment("image/png", b"seeded-image-bytes")],
            )
            .await
            .expect("create image post");

        let received = timeout(Duration::from_secs(20), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("seeded dht image backfill timeout");

        assert_eq!(received.attachments.len(), 1);
        assert_eq!(received.attachments[0].status, BlobViewStatus::Available);
        assert!(
            app_b
                .blob_preview_data_url(received.attachments[0].hash.as_str(), "image/png")
                .await
                .expect("preview")
                .is_some()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_image_post_from_docs() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("late-image-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("late-image-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);

        let topic = "kukuri:topic:late-image";
        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "late image caption",
                None,
                vec![pending_image_attachment("image/png", b"late-image-bytes")],
            )
            .await
            .expect("create image post before join");
        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(60), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    let post = post.clone();
                    if post.attachments.len() == 1 {
                        let preview = app_b
                            .blob_preview_data_url(post.attachments[0].hash.as_str(), "image/png")
                            .await
                            .expect("preview data url");
                        if preview.is_some() {
                            let refreshed_timeline = app_b
                                .list_timeline(topic, None, 20)
                                .await
                                .expect("timeline b refreshed");
                            if let Some(refreshed_post) = refreshed_timeline
                                .items
                                .iter()
                                .find(|candidate| candidate.object_id == object_id)
                                .cloned()
                                && refreshed_post.attachments.len() == 1
                                && refreshed_post.attachments[0].status == BlobViewStatus::Available
                            {
                                return refreshed_post;
                            }
                        }
                    }
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late image join timeout");

        assert_eq!(received.attachments.len(), 1);
        assert_eq!(received.attachments[0].status, BlobViewStatus::Available);
        assert!(
            app_b
                .blob_preview_data_url(received.attachments[0].hash.as_str(), "image/png")
                .await
                .expect("preview data url")
                .is_some()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_video_media_payload() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("late-video-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("late-video-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);

        let topic = "kukuri:topic:late-video";
        let object_id = app_a
            .create_post_with_attachments(
                topic,
                "late video caption",
                None,
                vec![
                    pending_video_attachment(AssetRole::VideoManifest, "video/mp4", b"late-video"),
                    pending_video_attachment(AssetRole::VideoPoster, "image/jpeg", b"late-poster"),
                ],
            )
            .await
            .expect("create video post before join");
        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");

        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if let Some(post) = timeline
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("late video join timeout");

        let poster = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_poster")
            .expect("video poster");
        assert_eq!(poster.status, BlobViewStatus::Available);
        let poster_payload = app_b
            .blob_media_payload(poster.hash.as_str(), "image/jpeg")
            .await
            .expect("poster media payload")
            .expect("poster payload present");
        assert_eq!(poster_payload.mime, "image/jpeg");
        let manifest = received
            .attachments
            .iter()
            .find(|attachment| attachment.role == "video_manifest")
            .expect("video manifest");
        let manifest_payload = app_b
            .blob_media_payload(manifest.hash.as_str(), "video/mp4")
            .await
            .expect("video media payload")
            .expect("manifest payload present");
        assert_eq!(manifest_payload.mime, "video/mp4");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_reply_into_thread() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("reply-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("reply-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:reply-thread";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        let root_id = app_a
            .create_post(topic, "root over iroh", None)
            .await
            .expect("create root");

        timeout(Duration::from_secs(10), async {
            loop {
                let timeline = app_b
                    .list_timeline(topic, None, 20)
                    .await
                    .expect("timeline b");
                if timeline.items.iter().any(|post| post.object_id == root_id) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("root propagation timeout");

        let reply_id = app_b
            .create_post(topic, "reply over iroh", Some(root_id.as_str()))
            .await
            .expect("create reply");
        let thread = timeout(Duration::from_secs(10), async {
            loop {
                let thread = app_a
                    .list_thread(topic, root_id.as_str(), None, 20)
                    .await
                    .expect("thread a");
                if thread.items.iter().any(|post| post.object_id == reply_id) {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("reply propagation timeout");

        assert_eq!(thread.items.len(), 2);
        let reply = thread
            .items
            .iter()
            .find(|post| post.object_id == reply_id)
            .expect("reply in thread");
        assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
        assert_eq!(reply.root_id.as_deref(), Some(root_id.as_str()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn image_reply_thread_syncs() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("image-thread-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("image-thread-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b.clone(), &stack_b);
        let topic = "kukuri:topic:image-thread";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let _ = app_b
            .list_timeline(topic, None, 20)
            .await
            .expect("subscribe b timeline");
        let root_id = app_a
            .create_post_with_attachments(
                topic,
                "root image",
                None,
                vec![pending_image_attachment("image/png", b"root-image")],
            )
            .await
            .expect("create root image");

        timeout(Duration::from_secs(10), async {
            loop {
                let projection = ProjectionStore::get_object_projection(
                    store_b.as_ref(),
                    &EnvelopeId::from(root_id.clone()),
                )
                .await
                .expect("root projection")
                .is_some();
                if projection {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("root image projection timeout");

        let reply_id = app_b
            .create_post_with_attachments(
                topic,
                "reply image",
                Some(root_id.as_str()),
                vec![pending_image_attachment("image/jpeg", b"reply-image")],
            )
            .await
            .expect("create reply image");
        let thread = timeout(Duration::from_secs(10), async {
            loop {
                let thread = app_a
                    .list_thread(topic, root_id.as_str(), None, 20)
                    .await
                    .expect("thread a");
                if thread.items.iter().any(|post| post.object_id == reply_id) {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("image reply propagation timeout");

        let root = thread
            .items
            .iter()
            .find(|post| post.object_id == root_id)
            .expect("root in thread");
        let reply = thread
            .items
            .iter()
            .find(|post| post.object_id == reply_id)
            .expect("reply in thread");
        assert_eq!(root.attachments[0].mime, "image/png");
        assert_eq!(reply.attachments[0].mime, "image/jpeg");
        assert_eq!(reply.reply_to.as_deref(), Some(root_id.as_str()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn iroh_transport_syncs_multiple_topics_bidirectionally() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("multi-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("multi-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic_one = "kukuri:topic:one";
        let topic_two = "kukuri:topic:two";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a
            .import_peer_ticket(&ticket_b)
            .await
            .expect("import b into a");
        app_b
            .import_peer_ticket(&ticket_a)
            .await
            .expect("import a into b");

        let _ = app_a
            .list_timeline(topic_one, None, 20)
            .await
            .expect("subscribe a topic one");
        let _ = app_a
            .list_timeline(topic_two, None, 20)
            .await
            .expect("subscribe a topic two");
        let _ = app_b
            .list_timeline(topic_one, None, 20)
            .await
            .expect("subscribe b topic one");
        let _ = app_b
            .list_timeline(topic_two, None, 20)
            .await
            .expect("subscribe b topic two");

        let id_one = app_a
            .create_post(topic_one, "topic one from a", None)
            .await
            .expect("post one");
        let id_two = app_b
            .create_post(topic_two, "topic two from b", None)
            .await
            .expect("post two");

        timeout(Duration::from_secs(10), async {
            loop {
                let timeline_b = app_b
                    .list_timeline(topic_one, None, 20)
                    .await
                    .expect("timeline b");
                let timeline_a = app_a
                    .list_timeline(topic_two, None, 20)
                    .await
                    .expect("timeline a");
                let has_one = timeline_b.items.iter().any(|post| post.object_id == id_one);
                let has_two = timeline_a.items.iter().any(|post| post.object_id == id_two);
                if has_one && has_two {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("multi topic propagation timeout");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_backfills_live_session_manifest() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("live-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("live-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:live-late";

        let session_id = app_a
            .create_live_session(
                topic,
                CreateLiveSessionInput {
                    title: "late live".into(),
                    description: "watch along".into(),
                },
            )
            .await
            .expect("create live session");

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let sessions = app_b
                    .list_live_sessions(topic)
                    .await
                    .expect("list live sessions");
                if let Some(session) = sessions
                    .into_iter()
                    .find(|session| session.session_id == session_id)
                {
                    return session;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("live session backfill timeout");

        assert_eq!(received.title, "late live");
        assert_eq!(received.status, LiveSessionStatus::Live);
    }

    #[tokio::test]
    async fn live_presence_expires_without_heartbeat() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
        let app = AppService::new(store, transport.clone());
        let topic = "kukuri:topic:presence-expiry";
        let session_id = app
            .create_live_session(
                topic,
                CreateLiveSessionInput {
                    title: "presence".into(),
                    description: "ttl".into(),
                },
            )
            .await
            .expect("create live session");

        transport
            .publish_hint(
                &TopicId::new(topic),
                GossipHint::LivePresence {
                    topic_id: TopicId::new(topic),
                    session_id: session_id.clone(),
                    author: Pubkey::from("a".repeat(64)),
                    ttl_ms: 100,
                },
            )
            .await
            .expect("publish live presence");

        timeout(Duration::from_secs(2), async {
            loop {
                let sessions = app
                    .list_live_sessions(topic)
                    .await
                    .expect("list live sessions");
                if sessions
                    .iter()
                    .any(|session| session.session_id == session_id && session.viewer_count == 1)
                {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("viewer count update timeout");

        sleep(Duration::from_millis(150)).await;
        let sessions = app
            .list_live_sessions(topic)
            .await
            .expect("list after expiry");
        let session = sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .expect("session present");
        assert_eq!(session.viewer_count, 0);
    }

    #[tokio::test]
    async fn ended_live_session_rejects_new_viewers() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
        let app = AppService::new(store, transport);
        let topic = "kukuri:topic:ended-live";
        let session_id = app
            .create_live_session(
                topic,
                CreateLiveSessionInput {
                    title: "ended".into(),
                    description: "session".into(),
                },
            )
            .await
            .expect("create live session");
        app.end_live_session(topic, session_id.as_str())
            .await
            .expect("end live session");

        let error = app
            .join_live_session(topic, session_id.as_str())
            .await
            .expect_err("join should fail");
        assert!(error.to_string().contains("ended live session"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn game_room_score_update_replicates() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("game-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("game-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:game-sync";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let room_id = app_a
            .create_game_room(
                topic,
                CreateGameRoomInput {
                    title: "sync room".into(),
                    description: "set".into(),
                    participants: vec!["Alice".into(), "Bob".into()],
                },
            )
            .await
            .expect("create game room");
        app_a
            .update_game_room(
                topic,
                room_id.as_str(),
                UpdateGameRoomInput {
                    status: GameRoomStatus::Running,
                    phase_label: Some("Round 2".into()),
                    scores: vec![
                        GameScoreView {
                            participant_id: "participant-1".into(),
                            label: "Alice".into(),
                            score: 2,
                        },
                        GameScoreView {
                            participant_id: "participant-2".into(),
                            label: "Bob".into(),
                            score: 1,
                        },
                    ],
                },
            )
            .await
            .expect("update game room");

        let received = timeout(Duration::from_secs(60), async {
            loop {
                let rooms = app_b.list_game_rooms(topic).await.expect("list game rooms");
                if let Some(room) = rooms.into_iter().find(|room| room.room_id == room_id) {
                    let alice_score = room
                        .scores
                        .iter()
                        .find(|score| score.label == "Alice")
                        .map(|score| score.score);
                    if room.status == GameRoomStatus::Running
                        && room.phase_label.as_deref() == Some("Round 2")
                        && alice_score == Some(2)
                    {
                        return room;
                    }
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("game room replication timeout");

        assert_eq!(received.status, GameRoomStatus::Running);
        assert_eq!(received.phase_label.as_deref(), Some("Round 2"));
        assert_eq!(
            received
                .scores
                .iter()
                .find(|score| score.label == "Alice")
                .map(|score| score.score),
            Some(2)
        );
    }

    #[tokio::test]
    async fn finished_game_room_rejects_updates() {
        let store = Arc::new(MemoryStore::default());
        let transport = Arc::new(FakeTransport::new("self", FakeNetwork::default()));
        let app = AppService::new(store, transport);
        let topic = "kukuri:topic:game-finished";
        let room_id = app
            .create_game_room(
                topic,
                CreateGameRoomInput {
                    title: "finished room".into(),
                    description: "set".into(),
                    participants: vec!["Alice".into(), "Bob".into()],
                },
            )
            .await
            .expect("create game room");

        app.update_game_room(
            topic,
            room_id.as_str(),
            UpdateGameRoomInput {
                status: GameRoomStatus::Ended,
                phase_label: Some("Final".into()),
                scores: vec![
                    GameScoreView {
                        participant_id: "participant-1".into(),
                        label: "Alice".into(),
                        score: 2,
                    },
                    GameScoreView {
                        participant_id: "participant-2".into(),
                        label: "Bob".into(),
                        score: 0,
                    },
                ],
            },
        )
        .await
        .expect("finish room");

        let error = app
            .update_game_room(
                topic,
                room_id.as_str(),
                UpdateGameRoomInput {
                    status: GameRoomStatus::Ended,
                    phase_label: Some("After".into()),
                    scores: vec![
                        GameScoreView {
                            participant_id: "participant-1".into(),
                            label: "Alice".into(),
                            score: 3,
                        },
                        GameScoreView {
                            participant_id: "participant-2".into(),
                            label: "Bob".into(),
                            score: 1,
                        },
                    ],
                },
            )
            .await
            .expect_err("ended room update should fail");
        assert!(error.to_string().contains("ended game room"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn private_channel_invite_scopes_posts_and_replies() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("private-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("private-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let app_a = app_with_iroh_services(store_a, &stack_a);
        let app_b = app_with_iroh_services(store_b, &stack_b);
        let topic = "kukuri:topic:private-channel";

        let ticket_a = app_a
            .peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = app_b
            .peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let channel = app_a
            .create_private_channel(CreatePrivateChannelInput {
                topic_id: TopicId::new(topic),
                label: "core".into(),
            })
            .await
            .expect("create private channel");
        let invite = app_a
            .export_private_channel_invite(topic, channel.channel_id.as_str(), None)
            .await
            .expect("export invite");
        let preview = app_b
            .import_private_channel_invite(invite.as_str())
            .await
            .expect("import invite");
        assert_eq!(preview.channel_id.as_str(), channel.channel_id);

        let private_channel_id = ChannelId::new(channel.channel_id.clone());
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: private_channel_id.clone(),
        };
        let private_scope = TimelineScope::Channel {
            channel_id: private_channel_id.clone(),
        };

        let object_id = app_a
            .create_post_in_channel(topic, private_ref.clone(), "private hello", None)
            .await
            .expect("create private post");

        let received = timeout(Duration::from_secs(10), async {
            loop {
                let public = app_b
                    .list_timeline_scoped(topic, TimelineScope::Public, None, 20)
                    .await
                    .expect("public timeline");
                assert!(
                    public.items.iter().all(|post| post.object_id != object_id),
                    "private post leaked into public scope"
                );
                let private = app_b
                    .list_timeline_scoped(topic, private_scope.clone(), None, 20)
                    .await
                    .expect("private timeline");
                if let Some(post) = private
                    .items
                    .iter()
                    .find(|post| post.object_id == object_id)
                {
                    return post.clone();
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("private timeline timeout");
        assert_eq!(
            received.channel_id.as_deref(),
            Some(channel.channel_id.as_str())
        );

        let reply_id = app_b
            .create_post_in_channel(
                topic,
                ChannelRef::Public,
                "private reply",
                Some(object_id.as_str()),
            )
            .await
            .expect("reply in private channel");

        let thread = timeout(Duration::from_secs(10), async {
            loop {
                let thread = app_a
                    .list_thread(topic, object_id.as_str(), None, 20)
                    .await
                    .expect("thread");
                if thread.items.iter().any(|post| post.object_id == reply_id) {
                    return thread;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("private thread timeout");
        let reply = thread
            .items
            .iter()
            .find(|post| post.object_id == reply_id)
            .expect("reply");
        assert_eq!(
            reply.channel_id.as_deref(),
            Some(channel.channel_id.as_str())
        );
        assert_eq!(reply.reply_to.as_deref(), Some(object_id.as_str()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn social_graph_derives_friend_of_friend_and_clears_after_unfollow() {
        let dir = tempdir().expect("tempdir");
        let stack_a = TestIrohStack::new(&dir.path().join("author-a")).await;
        let stack_b = TestIrohStack::new(&dir.path().join("author-b")).await;
        let store_a = Arc::new(MemoryStore::default());
        let store_b = Arc::new(MemoryStore::default());
        let keys_a = generate_keys();
        let keys_b = generate_keys();
        let keys_c = generate_keys();
        let app_a = AppService::new_with_services(
            store_a.clone(),
            store_a.clone(),
            stack_a.transport.clone(),
            stack_a.transport.clone(),
            stack_a.docs_sync.clone(),
            stack_a.blob_service.clone(),
            keys_a.clone(),
        );
        let app_b = AppService::new_with_services(
            store_b.clone(),
            store_b.clone(),
            stack_b.transport.clone(),
            stack_b.transport.clone(),
            stack_b.docs_sync.clone(),
            stack_b.blob_service.clone(),
            keys_b.clone(),
        );
        app_a
            .warm_social_graph()
            .await
            .expect("warm social graph a");
        app_b
            .warm_social_graph()
            .await
            .expect("warm social graph b");

        let ticket_a = stack_a
            .transport
            .export_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = stack_b
            .transport
            .export_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        app_a.import_peer_ticket(&ticket_b).await.expect("import b");
        app_b.import_peer_ticket(&ticket_a).await.expect("import a");

        let b_pubkey = keys_b.public_key_hex();
        let c_pubkey = keys_c.public_key_hex();
        app_a
            .follow_author(b_pubkey.as_str())
            .await
            .expect("a follows b");
        app_b
            .follow_author(c_pubkey.as_str())
            .await
            .expect("b follows c");

        timeout(Duration::from_secs(10), async {
            loop {
                let social_view = app_a
                    .get_author_social_view(c_pubkey.as_str())
                    .await
                    .expect("load c social view");
                if social_view.friend_of_friend {
                    assert_eq!(
                        social_view.friend_of_friend_via_pubkeys,
                        vec![b_pubkey.clone()]
                    );
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("derive friend of friend");

        let b_view = app_a
            .get_author_social_view(b_pubkey.as_str())
            .await
            .expect("load b social view");
        assert!(b_view.following);
        assert!(!b_view.friend_of_friend);

        app_a
            .unfollow_author(b_pubkey.as_str())
            .await
            .expect("a unfollows b");

        let c_view = app_a
            .get_author_social_view(c_pubkey.as_str())
            .await
            .expect("load c social view after unfollow");
        assert!(!c_view.friend_of_friend);
        assert!(c_view.friend_of_friend_via_pubkeys.is_empty());
    }
}
