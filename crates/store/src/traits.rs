use anyhow::Result;
use async_trait::async_trait;
use kukuri_core::{BlobHash, EnvelopeId, FollowEdge, KukuriEnvelope, Profile, ReplicaId};

use crate::models::{
    AuthorRelationshipProjectionRow, BlobCacheStatus, BookmarkedCustomReactionRow,
    BookmarkedPostRow, DirectMessageConversationRow, DirectMessageMessageRow,
    DirectMessageOutboxRow, DirectMessageTombstoneRow, GameRoomProjectionRow,
    LiveSessionProjectionRow, MutedAuthorRow, NotificationRow, ObjectProjectionRow, Page,
    ReactionProjectionRow, TimelineCursor,
};

#[async_trait]
pub trait Store: Send + Sync {
    async fn put_envelope(&self, envelope: KukuriEnvelope) -> Result<()>;
    async fn get_envelope(&self, envelope_id: &EnvelopeId) -> Result<Option<KukuriEnvelope>>;
    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>>;
    async fn list_thread(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>>;
    async fn upsert_profile(&self, profile: Profile) -> Result<()>;
    async fn get_profile(&self, pubkey: &str) -> Result<Option<Profile>>;
    async fn upsert_follow_edge(&self, edge: FollowEdge) -> Result<()>;
    async fn list_follow_edges_by_subject(&self, subject_pubkey: &str) -> Result<Vec<FollowEdge>>;
    async fn list_follow_edges_by_target(&self, target_pubkey: &str) -> Result<Vec<FollowEdge>>;
}

#[async_trait]
pub trait ProjectionStore: Send + Sync {
    async fn put_object_projection(&self, row: ObjectProjectionRow) -> Result<()>;
    async fn get_object_projection(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<ObjectProjectionRow>>;
    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>>;
    async fn list_thread(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>>;
    async fn upsert_profile_cache(&self, profile: Profile) -> Result<()>;
    async fn upsert_live_session_cache(&self, row: LiveSessionProjectionRow) -> Result<()>;
    async fn list_topic_live_sessions(
        &self,
        topic_id: &str,
    ) -> Result<Vec<LiveSessionProjectionRow>>;
    async fn upsert_game_room_cache(&self, row: GameRoomProjectionRow) -> Result<()>;
    async fn list_topic_game_rooms(&self, topic_id: &str) -> Result<Vec<GameRoomProjectionRow>>;
    async fn get_author_relationship(
        &self,
        local_author_pubkey: &str,
        author_pubkey: &str,
    ) -> Result<Option<AuthorRelationshipProjectionRow>>;
    async fn rebuild_author_relationships(
        &self,
        local_author_pubkey: &str,
        rows: Vec<AuthorRelationshipProjectionRow>,
    ) -> Result<()>;
    async fn put_muted_author(&self, row: MutedAuthorRow) -> Result<()>;
    async fn get_muted_author(&self, author_pubkey: &str) -> Result<Option<MutedAuthorRow>>;
    async fn list_muted_authors(&self) -> Result<Vec<MutedAuthorRow>>;
    async fn remove_muted_author(&self, author_pubkey: &str) -> Result<()>;
    async fn upsert_live_presence(
        &self,
        topic_id: &str,
        channel_id: &str,
        session_id: &str,
        author_pubkey: &str,
        expires_at: i64,
        updated_at: i64,
    ) -> Result<()>;
    async fn clear_topic_live_presence(&self, topic_id: &str) -> Result<()>;
    async fn clear_expired_live_presence(&self, now_ms: i64) -> Result<()>;
    async fn mark_blob_status(&self, hash: &BlobHash, status: BlobCacheStatus) -> Result<()>;
    async fn upsert_reaction_cache(&self, row: ReactionProjectionRow) -> Result<()>;
    async fn get_reaction_cache(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
        reaction_id: &EnvelopeId,
    ) -> Result<Option<ReactionProjectionRow>>;
    async fn list_reaction_cache_for_target(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
    ) -> Result<Vec<ReactionProjectionRow>>;
    async fn list_recent_reaction_cache_by_author(
        &self,
        author_pubkey: &str,
    ) -> Result<Vec<ReactionProjectionRow>>;
    async fn put_bookmarked_custom_reaction(&self, row: BookmarkedCustomReactionRow) -> Result<()>;
    async fn list_bookmarked_custom_reactions(&self) -> Result<Vec<BookmarkedCustomReactionRow>>;
    async fn remove_bookmarked_custom_reaction(&self, asset_id: &str) -> Result<()>;
    async fn put_bookmarked_post(&self, row: BookmarkedPostRow) -> Result<()>;
    async fn list_bookmarked_posts(&self) -> Result<Vec<BookmarkedPostRow>>;
    async fn remove_bookmarked_post(&self, source_object_id: &EnvelopeId) -> Result<()>;
    async fn upsert_direct_message_conversation(
        &self,
        row: DirectMessageConversationRow,
    ) -> Result<()>;
    async fn get_direct_message_conversation_by_peer(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<DirectMessageConversationRow>>;
    async fn get_direct_message_conversation_by_dm_id(
        &self,
        dm_id: &str,
    ) -> Result<Option<DirectMessageConversationRow>>;
    async fn list_direct_message_conversations(&self) -> Result<Vec<DirectMessageConversationRow>>;
    async fn put_direct_message_message(&self, row: DirectMessageMessageRow) -> Result<()>;
    async fn get_direct_message_message(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageMessageRow>>;
    async fn list_direct_message_messages(
        &self,
        dm_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<DirectMessageMessageRow>>;
    async fn set_direct_message_acked_at(
        &self,
        dm_id: &str,
        message_id: &str,
        acked_at: i64,
    ) -> Result<()>;
    async fn put_direct_message_outbox(&self, row: DirectMessageOutboxRow) -> Result<()>;
    async fn get_direct_message_outbox(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageOutboxRow>>;
    async fn list_direct_message_outbox(&self) -> Result<Vec<DirectMessageOutboxRow>>;
    async fn touch_direct_message_outbox_attempt(
        &self,
        dm_id: &str,
        message_id: &str,
        attempted_at: i64,
    ) -> Result<()>;
    async fn remove_direct_message_outbox(&self, dm_id: &str, message_id: &str) -> Result<()>;
    async fn put_direct_message_tombstone(&self, row: DirectMessageTombstoneRow) -> Result<()>;
    async fn list_direct_message_tombstones(
        &self,
        dm_id: &str,
    ) -> Result<Vec<DirectMessageTombstoneRow>>;
    async fn has_direct_message_tombstone(&self, dm_id: &str, message_id: &str) -> Result<bool>;
    async fn delete_direct_message_message_local(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<()>;
    async fn clear_direct_message_local(&self, dm_id: &str) -> Result<()>;
    async fn put_notification_if_absent(&self, row: NotificationRow) -> Result<bool>;
    async fn list_notifications(&self) -> Result<Vec<NotificationRow>>;
    async fn mark_notification_read(&self, notification_id: &str, read_at: i64) -> Result<()>;
    async fn mark_all_notifications_read(&self, read_at: i64) -> Result<()>;
    async fn count_unread_notifications(&self) -> Result<usize>;
    async fn rebuild_object_projections(&self, rows: Vec<ObjectProjectionRow>) -> Result<()>;
}
