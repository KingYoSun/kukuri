use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use async_trait::async_trait;
use kukuri_core::{
    BlobHash, EnvelopeId, FollowEdge, KukuriEnvelope, Profile, ReplicaId, ThreadRef,
    parse_follow_edge, parse_profile,
};
use sha2::{Digest, Sha384};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, QueryBuilder, Row, Sqlite};

use crate::models::{
    AuthorRelationshipProjectionRow, BlobCacheStatus, BookmarkedCustomReactionRow,
    BookmarkedPostRow, DirectMessageConversationRow, DirectMessageMessageRow,
    DirectMessageOutboxRow, DirectMessageTombstoneRow, GameRoomProjectionRow,
    LiveSessionProjectionRow, MutedAuthorRow, NotificationRow, ObjectProjectionRow, Page,
    ReactionProjectionRow, TimelineCursor,
};
use crate::pagination::{
    direct_message_page_from_rows, envelope_page_from_rows, object_projection_page_from_rows,
};
use crate::row_mapping::{
    follow_edge_status_name, game_status_name, live_status_name, notification_kind_name,
    object_status_name, reaction_key_kind_name, row_to_author_relationship_projection,
    row_to_bookmarked_custom_reaction, row_to_bookmarked_post, row_to_direct_message_conversation,
    row_to_direct_message_message, row_to_direct_message_outbox, row_to_direct_message_tombstone,
    row_to_envelope, row_to_follow_edge, row_to_game_room_projection,
    row_to_live_session_projection, row_to_muted_author, row_to_notification,
    row_to_object_projection, row_to_reaction_projection,
};
use crate::traits::{ProjectionStore, Store};

mod bookmarks;
mod connection;
mod direct_messages;
mod envelopes;
mod live_game;
mod notifications;
mod projections;
mod social;

#[derive(Clone)]
pub struct SqliteStore {
    pool: Pool<Sqlite>,
}

#[cfg(test)]
pub(crate) use connection::STORE_MIGRATOR;
#[cfg(test)]
pub(crate) use connection::alternate_line_ending_checksum;

#[async_trait]
impl Store for SqliteStore {
    async fn put_envelope(&self, envelope: KukuriEnvelope) -> Result<()> {
        self.store_put_envelope_impl(envelope).await
    }

    async fn get_envelope(&self, envelope_id: &EnvelopeId) -> Result<Option<KukuriEnvelope>> {
        self.store_get_envelope_impl(envelope_id).await
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        self.store_list_topic_timeline_impl(topic_id, cursor, limit)
            .await
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        self.store_list_thread_impl(topic_id, thread_root_object_id, cursor, limit)
            .await
    }

    async fn upsert_profile(&self, profile: Profile) -> Result<()> {
        self.store_upsert_profile_impl(profile).await
    }

    async fn get_profile(&self, pubkey: &str) -> Result<Option<Profile>> {
        self.store_get_profile_impl(pubkey).await
    }

    async fn get_profiles(
        &self,
        pubkeys: &[String],
    ) -> Result<std::collections::HashMap<String, Profile>> {
        self.store_get_profiles_impl(pubkeys).await
    }

    async fn upsert_follow_edge(&self, edge: FollowEdge) -> Result<()> {
        self.store_upsert_follow_edge_impl(edge).await
    }

    async fn list_follow_edges_by_subject(&self, subject_pubkey: &str) -> Result<Vec<FollowEdge>> {
        self.store_list_follow_edges_by_subject_impl(subject_pubkey)
            .await
    }

    async fn list_follow_edges_by_target(&self, target_pubkey: &str) -> Result<Vec<FollowEdge>> {
        self.store_list_follow_edges_by_target_impl(target_pubkey)
            .await
    }
}

#[async_trait]
impl ProjectionStore for SqliteStore {
    async fn put_object_projection(&self, row: ObjectProjectionRow) -> Result<()> {
        self.projection_put_object_projection_impl(row).await
    }

    async fn put_object_projections(&self, rows: Vec<ObjectProjectionRow>) -> Result<()> {
        self.projection_put_object_projections_impl(rows).await
    }

    async fn get_object_projection(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<ObjectProjectionRow>> {
        self.projection_get_object_projection_impl(object_id).await
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        self.projection_list_topic_timeline_impl(topic_id, cursor, limit)
            .await
    }

    async fn list_topic_timeline_filtered(
        &self,
        topic_id: &str,
        allowed_channels: &std::collections::BTreeSet<String>,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        self.projection_list_topic_timeline_filtered_impl(topic_id, allowed_channels, cursor, limit)
            .await
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        self.projection_list_thread_impl(topic_id, thread_root_object_id, cursor, limit)
            .await
    }

    async fn list_thread_filtered(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        allowed_channel: Option<&str>,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        self.projection_list_thread_filtered_impl(
            topic_id,
            thread_root_object_id,
            allowed_channel,
            cursor,
            limit,
        )
        .await
    }

    async fn upsert_profile_cache(&self, profile: Profile) -> Result<()> {
        self.projection_upsert_profile_cache_impl(profile).await
    }

    async fn upsert_live_session_cache(&self, row: LiveSessionProjectionRow) -> Result<()> {
        self.projection_upsert_live_session_cache_impl(row).await
    }

    async fn list_topic_live_sessions(
        &self,
        topic_id: &str,
    ) -> Result<Vec<LiveSessionProjectionRow>> {
        self.projection_list_topic_live_sessions_impl(topic_id)
            .await
    }

    async fn upsert_game_room_cache(&self, row: GameRoomProjectionRow) -> Result<()> {
        self.projection_upsert_game_room_cache_impl(row).await
    }

    async fn list_topic_game_rooms(&self, topic_id: &str) -> Result<Vec<GameRoomProjectionRow>> {
        self.projection_list_topic_game_rooms_impl(topic_id).await
    }

    async fn get_author_relationship(
        &self,
        local_author_pubkey: &str,
        author_pubkey: &str,
    ) -> Result<Option<AuthorRelationshipProjectionRow>> {
        self.projection_get_author_relationship_impl(local_author_pubkey, author_pubkey)
            .await
    }

    async fn list_author_relationships(
        &self,
        local_author_pubkey: &str,
        author_pubkeys: &[String],
    ) -> Result<std::collections::HashMap<String, AuthorRelationshipProjectionRow>> {
        self.projection_list_author_relationships_impl(local_author_pubkey, author_pubkeys)
            .await
    }

    async fn rebuild_author_relationships(
        &self,
        local_author_pubkey: &str,
        rows: Vec<AuthorRelationshipProjectionRow>,
    ) -> Result<()> {
        self.projection_rebuild_author_relationships_impl(local_author_pubkey, rows)
            .await
    }

    async fn put_muted_author(&self, row: MutedAuthorRow) -> Result<()> {
        self.projection_put_muted_author_impl(row).await
    }

    async fn get_muted_author(&self, author_pubkey: &str) -> Result<Option<MutedAuthorRow>> {
        self.projection_get_muted_author_impl(author_pubkey).await
    }

    async fn list_muted_authors(&self) -> Result<Vec<MutedAuthorRow>> {
        self.projection_list_muted_authors_impl().await
    }

    async fn remove_muted_author(&self, author_pubkey: &str) -> Result<()> {
        self.projection_remove_muted_author_impl(author_pubkey)
            .await
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
        self.projection_upsert_live_presence_impl(
            topic_id,
            channel_id,
            session_id,
            author_pubkey,
            expires_at,
            updated_at,
        )
        .await
    }

    async fn clear_expired_live_presence(&self, now_ms: i64) -> Result<()> {
        self.projection_clear_expired_live_presence_impl(now_ms)
            .await
    }

    async fn clear_topic_live_presence(&self, topic_id: &str) -> Result<()> {
        self.projection_clear_topic_live_presence_impl(topic_id)
            .await
    }

    async fn mark_blob_status(&self, hash: &BlobHash, status: BlobCacheStatus) -> Result<()> {
        self.projection_mark_blob_status_impl(hash, status).await
    }

    async fn mark_blob_statuses(&self, rows: Vec<(BlobHash, BlobCacheStatus)>) -> Result<()> {
        self.projection_mark_blob_statuses_impl(rows).await
    }

    async fn upsert_reaction_cache(&self, row: ReactionProjectionRow) -> Result<()> {
        self.projection_upsert_reaction_cache_impl(row).await
    }

    async fn get_reaction_cache(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
        reaction_id: &EnvelopeId,
    ) -> Result<Option<ReactionProjectionRow>> {
        self.projection_get_reaction_cache_impl(source_replica_id, target_object_id, reaction_id)
            .await
    }

    async fn list_reaction_cache_for_target(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
    ) -> Result<Vec<ReactionProjectionRow>> {
        self.projection_list_reaction_cache_for_target_impl(source_replica_id, target_object_id)
            .await
    }

    async fn list_reaction_cache_for_targets(
        &self,
        source_replica_id: &ReplicaId,
        target_object_ids: &[EnvelopeId],
    ) -> Result<std::collections::HashMap<String, Vec<ReactionProjectionRow>>> {
        self.projection_list_reaction_cache_for_targets_impl(source_replica_id, target_object_ids)
            .await
    }

    async fn list_recent_reaction_cache_by_author(
        &self,
        author_pubkey: &str,
    ) -> Result<Vec<ReactionProjectionRow>> {
        self.projection_list_recent_reaction_cache_by_author_impl(author_pubkey)
            .await
    }

    async fn put_bookmarked_custom_reaction(&self, row: BookmarkedCustomReactionRow) -> Result<()> {
        self.projection_put_bookmarked_custom_reaction_impl(row)
            .await
    }

    async fn list_bookmarked_custom_reactions(&self) -> Result<Vec<BookmarkedCustomReactionRow>> {
        self.projection_list_bookmarked_custom_reactions_impl()
            .await
    }

    async fn remove_bookmarked_custom_reaction(&self, asset_id: &str) -> Result<()> {
        self.projection_remove_bookmarked_custom_reaction_impl(asset_id)
            .await
    }

    async fn put_bookmarked_post(&self, row: BookmarkedPostRow) -> Result<()> {
        self.projection_put_bookmarked_post_impl(row).await
    }

    async fn list_bookmarked_posts(&self) -> Result<Vec<BookmarkedPostRow>> {
        self.projection_list_bookmarked_posts_impl().await
    }

    async fn remove_bookmarked_post(&self, source_object_id: &EnvelopeId) -> Result<()> {
        self.projection_remove_bookmarked_post_impl(source_object_id)
            .await
    }

    async fn upsert_direct_message_conversation(
        &self,
        row: DirectMessageConversationRow,
    ) -> Result<()> {
        self.projection_upsert_direct_message_conversation_impl(row)
            .await
    }

    async fn get_direct_message_conversation_by_peer(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        self.projection_get_direct_message_conversation_by_peer_impl(peer_pubkey)
            .await
    }

    async fn get_direct_message_conversation_by_dm_id(
        &self,
        dm_id: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        self.projection_get_direct_message_conversation_by_dm_id_impl(dm_id)
            .await
    }

    async fn list_direct_message_conversations(&self) -> Result<Vec<DirectMessageConversationRow>> {
        self.projection_list_direct_message_conversations_impl()
            .await
    }

    async fn put_direct_message_message(&self, row: DirectMessageMessageRow) -> Result<()> {
        self.projection_put_direct_message_message_impl(row).await
    }

    async fn get_direct_message_message(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageMessageRow>> {
        self.projection_get_direct_message_message_impl(dm_id, message_id)
            .await
    }

    async fn list_direct_message_messages(
        &self,
        dm_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<DirectMessageMessageRow>> {
        self.projection_list_direct_message_messages_impl(dm_id, cursor, limit)
            .await
    }

    async fn set_direct_message_acked_at(
        &self,
        dm_id: &str,
        message_id: &str,
        acked_at: i64,
    ) -> Result<()> {
        self.projection_set_direct_message_acked_at_impl(dm_id, message_id, acked_at)
            .await
    }

    async fn put_direct_message_outbox(&self, row: DirectMessageOutboxRow) -> Result<()> {
        self.projection_put_direct_message_outbox_impl(row).await
    }

    async fn get_direct_message_outbox(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageOutboxRow>> {
        self.projection_get_direct_message_outbox_impl(dm_id, message_id)
            .await
    }

    async fn list_direct_message_outbox(&self) -> Result<Vec<DirectMessageOutboxRow>> {
        self.projection_list_direct_message_outbox_impl().await
    }

    async fn touch_direct_message_outbox_attempt(
        &self,
        dm_id: &str,
        message_id: &str,
        attempted_at: i64,
    ) -> Result<()> {
        self.projection_touch_direct_message_outbox_attempt_impl(dm_id, message_id, attempted_at)
            .await
    }

    async fn remove_direct_message_outbox(&self, dm_id: &str, message_id: &str) -> Result<()> {
        self.projection_remove_direct_message_outbox_impl(dm_id, message_id)
            .await
    }

    async fn put_direct_message_tombstone(&self, row: DirectMessageTombstoneRow) -> Result<()> {
        self.projection_put_direct_message_tombstone_impl(row).await
    }

    async fn list_direct_message_tombstones(
        &self,
        dm_id: &str,
    ) -> Result<Vec<DirectMessageTombstoneRow>> {
        self.projection_list_direct_message_tombstones_impl(dm_id)
            .await
    }

    async fn has_direct_message_tombstone(&self, dm_id: &str, message_id: &str) -> Result<bool> {
        self.projection_has_direct_message_tombstone_impl(dm_id, message_id)
            .await
    }

    async fn delete_direct_message_message_local(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<()> {
        self.projection_delete_direct_message_message_local_impl(dm_id, message_id)
            .await
    }

    async fn clear_direct_message_local(&self, dm_id: &str) -> Result<()> {
        self.projection_clear_direct_message_local_impl(dm_id).await
    }

    async fn put_notification_if_absent(&self, row: NotificationRow) -> Result<bool> {
        self.projection_put_notification_if_absent_impl(row).await
    }

    async fn list_notifications(&self) -> Result<Vec<NotificationRow>> {
        self.projection_list_notifications_impl().await
    }

    async fn mark_notification_read(&self, notification_id: &str, read_at: i64) -> Result<()> {
        self.projection_mark_notification_read_impl(notification_id, read_at)
            .await
    }

    async fn mark_all_notifications_read(&self, read_at: i64) -> Result<()> {
        self.projection_mark_all_notifications_read_impl(read_at)
            .await
    }

    async fn count_unread_notifications(&self) -> Result<usize> {
        self.projection_count_unread_notifications_impl().await
    }

    async fn rebuild_object_projections(&self, rows: Vec<ObjectProjectionRow>) -> Result<()> {
        self.projection_rebuild_object_projections_impl(rows).await
    }
}
