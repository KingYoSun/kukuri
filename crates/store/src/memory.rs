use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use kukuri_core::{
    BlobHash, EnvelopeId, FollowEdge, KukuriEnvelope, LiveSessionStatus, Profile, ReplicaId,
    parse_follow_edge, parse_profile,
};
use tokio::sync::RwLock;

use crate::models::{
    AuthorRelationshipProjectionRow, BlobCacheStatus, BookmarkedCustomReactionRow,
    BookmarkedPostRow, DirectMessageConversationRow, DirectMessageMessageRow,
    DirectMessageOutboxRow, DirectMessageTombstoneRow, GameRoomProjectionRow,
    LiveSessionProjectionRow, MutedAuthorRow, NotificationRow, ObjectProjectionRow, Page,
    ReactionProjectionRow, TimelineCursor,
};
use crate::pagination::{
    apply_asc_cursor, apply_asc_projection_cursor, apply_desc_cursor,
    apply_desc_direct_message_cursor, apply_desc_projection_cursor,
};
use crate::traits::{ProjectionStore, Store};

type LivePresenceKey = (String, String, String);
type LivePresenceValue = (String, String, i64, i64);
type MemoryReactionProjectionRows = HashMap<(String, String, String), ReactionProjectionRow>;
type MemoryDirectMessageRows = HashMap<(String, String), DirectMessageMessageRow>;
type MemoryDirectMessageOutboxRows = HashMap<(String, String), DirectMessageOutboxRow>;
type MemoryDirectMessageTombstones = HashMap<(String, String), DirectMessageTombstoneRow>;
type MemoryNotificationRows = HashMap<String, NotificationRow>;

#[derive(Clone, Default)]
pub struct MemoryStore {
    envelopes: Arc<RwLock<HashMap<EnvelopeId, KukuriEnvelope>>>,
    topic_objects: Arc<RwLock<HashMap<String, Vec<EnvelopeId>>>>,
    object_threads: Arc<RwLock<HashMap<String, BTreeMap<String, EnvelopeId>>>>,
    profiles: Arc<RwLock<HashMap<String, Profile>>>,
    follow_edges: Arc<RwLock<HashMap<(String, String), FollowEdge>>>,
    object_projection_rows: Arc<RwLock<HashMap<EnvelopeId, ObjectProjectionRow>>>,
    live_session_rows: Arc<RwLock<HashMap<String, LiveSessionProjectionRow>>>,
    game_room_rows: Arc<RwLock<HashMap<String, GameRoomProjectionRow>>>,
    author_relationship_rows:
        Arc<RwLock<HashMap<(String, String), AuthorRelationshipProjectionRow>>>,
    muted_authors: Arc<RwLock<HashMap<String, MutedAuthorRow>>>,
    live_presence: Arc<RwLock<HashMap<LivePresenceKey, LivePresenceValue>>>,
    blob_statuses: Arc<RwLock<HashMap<String, BlobCacheStatus>>>,
    reaction_projection_rows: Arc<RwLock<MemoryReactionProjectionRows>>,
    bookmarked_custom_reactions: Arc<RwLock<HashMap<String, BookmarkedCustomReactionRow>>>,
    bookmarked_posts: Arc<RwLock<HashMap<String, BookmarkedPostRow>>>,
    direct_message_conversations: Arc<RwLock<HashMap<String, DirectMessageConversationRow>>>,
    direct_message_rows: Arc<RwLock<MemoryDirectMessageRows>>,
    direct_message_outbox_rows: Arc<RwLock<MemoryDirectMessageOutboxRows>>,
    direct_message_tombstones: Arc<RwLock<MemoryDirectMessageTombstones>>,
    notification_rows: Arc<RwLock<MemoryNotificationRows>>,
}

#[async_trait]
impl Store for MemoryStore {
    async fn put_envelope(&self, envelope: KukuriEnvelope) -> Result<()> {
        let topic_id = envelope.topic_id().map(|topic| topic.0);
        let thread_ref = envelope.thread_ref();
        self.envelopes
            .write()
            .await
            .insert(envelope.id.clone(), envelope.clone());

        if let Some(topic_id) = topic_id {
            self.topic_objects
                .write()
                .await
                .entry(topic_id.clone())
                .or_default()
                .push(envelope.id.clone());

            let root = thread_ref
                .as_ref()
                .map(|thread| thread.root.clone())
                .unwrap_or_else(|| envelope.id.clone());
            self.object_threads
                .write()
                .await
                .entry(topic_id)
                .or_default()
                .insert(envelope.id.0.clone(), root);
        }

        if let Some(profile) = parse_profile(&envelope)? {
            self.upsert_profile(profile).await?;
        }
        if let Some(edge) = parse_follow_edge(&envelope)? {
            self.upsert_follow_edge(edge).await?;
        }

        Ok(())
    }

    async fn get_envelope(&self, envelope_id: &EnvelopeId) -> Result<Option<KukuriEnvelope>> {
        Ok(self.envelopes.read().await.get(envelope_id).cloned())
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        let envelopes = self.envelopes.read().await;
        let mut items = self
            .topic_objects
            .read()
            .await
            .get(topic_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|object_id| envelopes.get(&object_id).cloned())
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        let filtered = apply_desc_cursor(items, cursor, limit);
        Ok(filtered)
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        let envelopes = self.envelopes.read().await;
        let roots = self.object_threads.read().await;
        let mut items = roots
            .get(topic_id)
            .into_iter()
            .flat_map(|entries| entries.keys())
            .filter_map(|object_id| {
                envelopes
                    .get(&EnvelopeId::from(object_id.as_str()))
                    .cloned()
            })
            .filter(|envelope| {
                envelope.id == *thread_root_object_id
                    || envelope
                        .thread_ref()
                        .map(|thread| thread.root == *thread_root_object_id)
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let left_root = left.id == *thread_root_object_id;
            let right_root = right.id == *thread_root_object_id;
            left_root
                .cmp(&right_root)
                .reverse()
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        let filtered = apply_asc_cursor(items, cursor, limit);
        Ok(filtered)
    }

    async fn upsert_profile(&self, profile: Profile) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        match profiles.get(profile.pubkey.as_str()) {
            Some(existing) if existing.updated_at > profile.updated_at => {}
            _ => {
                profiles.insert(profile.pubkey.0.clone(), profile);
            }
        }
        Ok(())
    }

    async fn get_profile(&self, pubkey: &str) -> Result<Option<Profile>> {
        Ok(self.profiles.read().await.get(pubkey).cloned())
    }

    async fn upsert_follow_edge(&self, edge: FollowEdge) -> Result<()> {
        let key = (
            edge.subject_pubkey.as_str().to_string(),
            edge.target_pubkey.as_str().to_string(),
        );
        let mut follow_edges = self.follow_edges.write().await;
        match follow_edges.get(&key) {
            Some(existing) if existing.updated_at > edge.updated_at => {}
            _ => {
                follow_edges.insert(key, edge);
            }
        }
        Ok(())
    }

    async fn list_follow_edges_by_subject(&self, subject_pubkey: &str) -> Result<Vec<FollowEdge>> {
        let mut items = self
            .follow_edges
            .read()
            .await
            .values()
            .filter(|edge| edge.subject_pubkey.as_str() == subject_pubkey)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.target_pubkey.cmp(&right.target_pubkey))
        });
        Ok(items)
    }

    async fn list_follow_edges_by_target(&self, target_pubkey: &str) -> Result<Vec<FollowEdge>> {
        let mut items = self
            .follow_edges
            .read()
            .await
            .values()
            .filter(|edge| edge.target_pubkey.as_str() == target_pubkey)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.subject_pubkey.cmp(&right.subject_pubkey))
        });
        Ok(items)
    }
}

#[async_trait]
impl ProjectionStore for MemoryStore {
    async fn put_object_projection(&self, row: ObjectProjectionRow) -> Result<()> {
        self.object_projection_rows
            .write()
            .await
            .insert(row.object_id.clone(), row);
        Ok(())
    }

    async fn get_object_projection(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<ObjectProjectionRow>> {
        Ok(self
            .object_projection_rows
            .read()
            .await
            .get(object_id)
            .cloned())
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let mut items = self
            .object_projection_rows
            .read()
            .await
            .values()
            .filter(|row| row.topic_id == topic_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.object_id.cmp(&left.object_id))
        });
        Ok(apply_desc_projection_cursor(items, cursor, limit))
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let mut items = self
            .object_projection_rows
            .read()
            .await
            .values()
            .filter(|row| {
                row.topic_id == topic_id
                    && (row.object_id == *thread_root_object_id
                        || row
                            .root_object_id
                            .as_ref()
                            .is_some_and(|root| root == thread_root_object_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let left_root = left.object_id == *thread_root_object_id;
            let right_root = right.object_id == *thread_root_object_id;
            left_root
                .cmp(&right_root)
                .reverse()
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.object_id.cmp(&right.object_id))
        });
        Ok(apply_asc_projection_cursor(items, cursor, limit))
    }

    async fn upsert_profile_cache(&self, profile: Profile) -> Result<()> {
        self.upsert_profile(profile).await
    }

    async fn upsert_live_session_cache(&self, row: LiveSessionProjectionRow) -> Result<()> {
        self.live_session_rows
            .write()
            .await
            .insert(row.session_id.clone(), row);
        Ok(())
    }

    async fn list_topic_live_sessions(
        &self,
        topic_id: &str,
    ) -> Result<Vec<LiveSessionProjectionRow>> {
        let presence = self.live_presence.read().await;
        let mut items = self
            .live_session_rows
            .read()
            .await
            .values()
            .filter(|row| row.topic_id == topic_id)
            .cloned()
            .collect::<Vec<_>>();
        for row in &mut items {
            row.viewer_count = if row.status == LiveSessionStatus::Ended {
                0
            } else {
                presence
                    .iter()
                    .filter(
                        |((presence_channel, session_id, _), (presence_topic, _, _, _))| {
                            presence_channel == &row.channel_id
                                && session_id == &row.session_id
                                && presence_topic == topic_id
                        },
                    )
                    .count()
            };
        }
        items.sort_by(|left, right| {
            right
                .started_at
                .cmp(&left.started_at)
                .then_with(|| right.session_id.cmp(&left.session_id))
        });
        Ok(items)
    }

    async fn upsert_game_room_cache(&self, row: GameRoomProjectionRow) -> Result<()> {
        self.game_room_rows
            .write()
            .await
            .insert(row.room_id.clone(), row);
        Ok(())
    }

    async fn list_topic_game_rooms(&self, topic_id: &str) -> Result<Vec<GameRoomProjectionRow>> {
        let mut items = self
            .game_room_rows
            .read()
            .await
            .values()
            .filter(|row| row.topic_id == topic_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.room_id.cmp(&left.room_id))
        });
        Ok(items)
    }

    async fn get_author_relationship(
        &self,
        local_author_pubkey: &str,
        author_pubkey: &str,
    ) -> Result<Option<AuthorRelationshipProjectionRow>> {
        Ok(self
            .author_relationship_rows
            .read()
            .await
            .get(&(local_author_pubkey.to_string(), author_pubkey.to_string()))
            .cloned())
    }

    async fn rebuild_author_relationships(
        &self,
        local_author_pubkey: &str,
        rows: Vec<AuthorRelationshipProjectionRow>,
    ) -> Result<()> {
        let mut guard = self.author_relationship_rows.write().await;
        guard.retain(|(local_author, _), _| local_author != local_author_pubkey);
        for row in rows {
            guard.insert(
                (row.local_author_pubkey.clone(), row.author_pubkey.clone()),
                row,
            );
        }
        Ok(())
    }

    async fn put_muted_author(&self, row: MutedAuthorRow) -> Result<()> {
        self.muted_authors
            .write()
            .await
            .insert(row.author_pubkey.clone(), row);
        Ok(())
    }

    async fn get_muted_author(&self, author_pubkey: &str) -> Result<Option<MutedAuthorRow>> {
        Ok(self.muted_authors.read().await.get(author_pubkey).cloned())
    }

    async fn list_muted_authors(&self) -> Result<Vec<MutedAuthorRow>> {
        let mut items = self
            .muted_authors
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .muted_at
                .cmp(&left.muted_at)
                .then_with(|| left.author_pubkey.cmp(&right.author_pubkey))
        });
        Ok(items)
    }

    async fn remove_muted_author(&self, author_pubkey: &str) -> Result<()> {
        self.muted_authors.write().await.remove(author_pubkey);
        Ok(())
    }

    async fn upsert_live_presence(
        &self,
        topic_id: &str,
        channel_id: &str,
        session_id: &str,
        author_pubkey: &str,
        expires_at: i64,
        updated_at: i64,
    ) -> Result<()> {
        self.live_presence.write().await.insert(
            (
                channel_id.to_string(),
                session_id.to_string(),
                author_pubkey.to_string(),
            ),
            (
                topic_id.to_string(),
                channel_id.to_string(),
                expires_at,
                updated_at,
            ),
        );
        Ok(())
    }

    async fn clear_expired_live_presence(&self, now_ms: i64) -> Result<()> {
        self.live_presence
            .write()
            .await
            .retain(|_, (_, _, expires_at, _)| *expires_at > now_ms);
        Ok(())
    }

    async fn clear_topic_live_presence(&self, topic_id: &str) -> Result<()> {
        self.live_presence
            .write()
            .await
            .retain(|_, (presence_topic, _, _, _)| presence_topic != topic_id);
        Ok(())
    }

    async fn mark_blob_status(&self, hash: &BlobHash, status: BlobCacheStatus) -> Result<()> {
        self.blob_statuses
            .write()
            .await
            .insert(hash.as_str().to_string(), status);
        Ok(())
    }

    async fn upsert_reaction_cache(&self, row: ReactionProjectionRow) -> Result<()> {
        self.reaction_projection_rows.write().await.insert(
            (
                row.source_replica_id.as_str().to_string(),
                row.target_object_id.as_str().to_string(),
                row.reaction_id.as_str().to_string(),
            ),
            row,
        );
        Ok(())
    }

    async fn get_reaction_cache(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
        reaction_id: &EnvelopeId,
    ) -> Result<Option<ReactionProjectionRow>> {
        Ok(self
            .reaction_projection_rows
            .read()
            .await
            .get(&(
                source_replica_id.as_str().to_string(),
                target_object_id.as_str().to_string(),
                reaction_id.as_str().to_string(),
            ))
            .cloned())
    }

    async fn list_reaction_cache_for_target(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
    ) -> Result<Vec<ReactionProjectionRow>> {
        let mut items = self
            .reaction_projection_rows
            .read()
            .await
            .values()
            .filter(|row| {
                row.source_replica_id == *source_replica_id
                    && row.target_object_id == *target_object_id
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.normalized_reaction_key
                .cmp(&right.normalized_reaction_key)
                .then_with(|| left.reaction_id.cmp(&right.reaction_id))
        });
        Ok(items)
    }

    async fn list_recent_reaction_cache_by_author(
        &self,
        author_pubkey: &str,
    ) -> Result<Vec<ReactionProjectionRow>> {
        let mut items = self
            .reaction_projection_rows
            .read()
            .await
            .values()
            .filter(|row| row.author_pubkey == author_pubkey)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.reaction_id.cmp(&left.reaction_id))
        });
        Ok(items)
    }

    async fn put_bookmarked_custom_reaction(&self, row: BookmarkedCustomReactionRow) -> Result<()> {
        self.bookmarked_custom_reactions
            .write()
            .await
            .insert(row.asset_id.clone(), row);
        Ok(())
    }

    async fn list_bookmarked_custom_reactions(&self) -> Result<Vec<BookmarkedCustomReactionRow>> {
        let mut items = self
            .bookmarked_custom_reactions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .bookmarked_at
                .cmp(&left.bookmarked_at)
                .then_with(|| right.asset_id.cmp(&left.asset_id))
        });
        Ok(items)
    }

    async fn remove_bookmarked_custom_reaction(&self, asset_id: &str) -> Result<()> {
        self.bookmarked_custom_reactions
            .write()
            .await
            .remove(asset_id);
        Ok(())
    }

    async fn put_bookmarked_post(&self, row: BookmarkedPostRow) -> Result<()> {
        self.bookmarked_posts
            .write()
            .await
            .insert(row.source_object_id.as_str().to_string(), row);
        Ok(())
    }

    async fn list_bookmarked_posts(&self) -> Result<Vec<BookmarkedPostRow>> {
        let mut items = self
            .bookmarked_posts
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .bookmarked_at
                .cmp(&left.bookmarked_at)
                .then_with(|| right.source_object_id.cmp(&left.source_object_id))
        });
        Ok(items)
    }

    async fn remove_bookmarked_post(&self, source_object_id: &EnvelopeId) -> Result<()> {
        self.bookmarked_posts
            .write()
            .await
            .remove(source_object_id.as_str());
        Ok(())
    }

    async fn upsert_direct_message_conversation(
        &self,
        row: DirectMessageConversationRow,
    ) -> Result<()> {
        self.direct_message_conversations
            .write()
            .await
            .insert(row.dm_id.clone(), row);
        Ok(())
    }

    async fn get_direct_message_conversation_by_peer(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        Ok(self
            .direct_message_conversations
            .read()
            .await
            .values()
            .find(|row| row.peer_pubkey == peer_pubkey)
            .cloned())
    }

    async fn get_direct_message_conversation_by_dm_id(
        &self,
        dm_id: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        Ok(self
            .direct_message_conversations
            .read()
            .await
            .get(dm_id)
            .cloned())
    }

    async fn list_direct_message_conversations(&self) -> Result<Vec<DirectMessageConversationRow>> {
        let mut items = self
            .direct_message_conversations
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.dm_id.cmp(&left.dm_id))
        });
        Ok(items)
    }

    async fn put_direct_message_message(&self, row: DirectMessageMessageRow) -> Result<()> {
        if self
            .direct_message_tombstones
            .read()
            .await
            .contains_key(&(row.dm_id.clone(), row.message_id.clone()))
        {
            return Ok(());
        }
        self.direct_message_rows
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    async fn get_direct_message_message(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageMessageRow>> {
        Ok(self
            .direct_message_rows
            .read()
            .await
            .get(&(dm_id.to_string(), message_id.to_string()))
            .cloned())
    }

    async fn list_direct_message_messages(
        &self,
        dm_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<DirectMessageMessageRow>> {
        let mut items = self
            .direct_message_rows
            .read()
            .await
            .values()
            .filter(|row| row.dm_id == dm_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.message_id.cmp(&left.message_id))
        });
        Ok(apply_desc_direct_message_cursor(items, cursor, limit))
    }

    async fn set_direct_message_acked_at(
        &self,
        dm_id: &str,
        message_id: &str,
        acked_at: i64,
    ) -> Result<()> {
        if let Some(row) = self
            .direct_message_rows
            .write()
            .await
            .get_mut(&(dm_id.to_string(), message_id.to_string()))
        {
            row.acked_at = Some(acked_at);
        }
        Ok(())
    }

    async fn put_direct_message_outbox(&self, row: DirectMessageOutboxRow) -> Result<()> {
        self.direct_message_outbox_rows
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    async fn get_direct_message_outbox(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageOutboxRow>> {
        Ok(self
            .direct_message_outbox_rows
            .read()
            .await
            .get(&(dm_id.to_string(), message_id.to_string()))
            .cloned())
    }

    async fn list_direct_message_outbox(&self) -> Result<Vec<DirectMessageOutboxRow>> {
        let mut items = self
            .direct_message_outbox_rows
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.message_id.cmp(&right.message_id))
        });
        Ok(items)
    }

    async fn touch_direct_message_outbox_attempt(
        &self,
        dm_id: &str,
        message_id: &str,
        attempted_at: i64,
    ) -> Result<()> {
        if let Some(row) = self
            .direct_message_outbox_rows
            .write()
            .await
            .get_mut(&(dm_id.to_string(), message_id.to_string()))
        {
            row.last_attempt_at = Some(attempted_at);
        }
        Ok(())
    }

    async fn remove_direct_message_outbox(&self, dm_id: &str, message_id: &str) -> Result<()> {
        self.direct_message_outbox_rows
            .write()
            .await
            .remove(&(dm_id.to_string(), message_id.to_string()));
        Ok(())
    }

    async fn put_direct_message_tombstone(&self, row: DirectMessageTombstoneRow) -> Result<()> {
        self.direct_message_tombstones
            .write()
            .await
            .insert((row.dm_id.clone(), row.message_id.clone()), row);
        Ok(())
    }

    async fn list_direct_message_tombstones(
        &self,
        dm_id: &str,
    ) -> Result<Vec<DirectMessageTombstoneRow>> {
        let mut items = self
            .direct_message_tombstones
            .read()
            .await
            .values()
            .filter(|row| row.dm_id == dm_id)
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .deleted_at
                .cmp(&left.deleted_at)
                .then_with(|| right.message_id.cmp(&left.message_id))
        });
        Ok(items)
    }

    async fn has_direct_message_tombstone(&self, dm_id: &str, message_id: &str) -> Result<bool> {
        Ok(self
            .direct_message_tombstones
            .read()
            .await
            .contains_key(&(dm_id.to_string(), message_id.to_string())))
    }

    async fn delete_direct_message_message_local(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<()> {
        self.direct_message_rows
            .write()
            .await
            .remove(&(dm_id.to_string(), message_id.to_string()));
        self.direct_message_outbox_rows
            .write()
            .await
            .remove(&(dm_id.to_string(), message_id.to_string()));
        Ok(())
    }

    async fn clear_direct_message_local(&self, dm_id: &str) -> Result<()> {
        self.direct_message_rows
            .write()
            .await
            .retain(|(row_dm_id, _), _| row_dm_id != dm_id);
        self.direct_message_outbox_rows
            .write()
            .await
            .retain(|(row_dm_id, _), _| row_dm_id != dm_id);
        self.direct_message_conversations
            .write()
            .await
            .remove(dm_id);
        Ok(())
    }

    async fn put_notification_if_absent(&self, row: NotificationRow) -> Result<bool> {
        let mut notifications = self.notification_rows.write().await;
        if notifications.contains_key(row.notification_id.as_str()) {
            return Ok(false);
        }
        notifications.insert(row.notification_id.clone(), row);
        Ok(true)
    }

    async fn list_notifications(&self) -> Result<Vec<NotificationRow>> {
        let mut items = self
            .notification_rows
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .received_at
                .cmp(&left.received_at)
                .then_with(|| right.notification_id.cmp(&left.notification_id))
        });
        Ok(items)
    }

    async fn mark_notification_read(&self, notification_id: &str, read_at: i64) -> Result<()> {
        if let Some(row) = self
            .notification_rows
            .write()
            .await
            .get_mut(notification_id)
        {
            row.read_at.get_or_insert(read_at);
        }
        Ok(())
    }

    async fn mark_all_notifications_read(&self, read_at: i64) -> Result<()> {
        for row in self.notification_rows.write().await.values_mut() {
            row.read_at.get_or_insert(read_at);
        }
        Ok(())
    }

    async fn count_unread_notifications(&self) -> Result<usize> {
        Ok(self
            .notification_rows
            .read()
            .await
            .values()
            .filter(|row| row.read_at.is_none())
            .count())
    }

    async fn rebuild_object_projections(&self, rows: Vec<ObjectProjectionRow>) -> Result<()> {
        let mut guard = self.object_projection_rows.write().await;
        guard.clear();
        for row in rows {
            guard.insert(row.object_id.clone(), row);
        }
        self.live_session_rows.write().await.clear();
        self.game_room_rows.write().await.clear();
        self.live_presence.write().await.clear();
        self.reaction_projection_rows.write().await.clear();
        Ok(())
    }
}
