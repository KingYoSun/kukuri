use super::*;

impl SqliteStore {
    pub(super) async fn projection_upsert_direct_message_conversation_impl(
        &self,
        row: DirectMessageConversationRow,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO dm_conversations (
              dm_id, peer_pubkey, updated_at, last_message_at, last_message_id, last_message_preview
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(dm_id) DO UPDATE SET
              peer_pubkey = excluded.peer_pubkey,
              updated_at = excluded.updated_at,
              last_message_at = excluded.last_message_at,
              last_message_id = excluded.last_message_id,
              last_message_preview = excluded.last_message_preview
            "#,
        )
        .bind(row.dm_id.as_str())
        .bind(row.peer_pubkey.as_str())
        .bind(row.updated_at)
        .bind(row.last_message_at)
        .bind(row.last_message_id.as_deref())
        .bind(row.last_message_preview.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_get_direct_message_conversation_by_peer_impl(
        &self,
        peer_pubkey: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        let row = sqlx::query(
            r#"
            SELECT dm_id, peer_pubkey, updated_at, last_message_at, last_message_id, last_message_preview
            FROM dm_conversations
            WHERE peer_pubkey = ?1
            "#,
        )
        .bind(peer_pubkey)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_direct_message_conversation).transpose()
    }

    pub(super) async fn projection_get_direct_message_conversation_by_dm_id_impl(
        &self,
        dm_id: &str,
    ) -> Result<Option<DirectMessageConversationRow>> {
        let row = sqlx::query(
            r#"
            SELECT dm_id, peer_pubkey, updated_at, last_message_at, last_message_id, last_message_preview
            FROM dm_conversations
            WHERE dm_id = ?1
            "#,
        )
        .bind(dm_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_direct_message_conversation).transpose()
    }

    pub(super) async fn projection_list_direct_message_conversations_impl(
        &self,
    ) -> Result<Vec<DirectMessageConversationRow>> {
        let rows = sqlx::query(
            r#"
            SELECT dm_id, peer_pubkey, updated_at, last_message_at, last_message_id, last_message_preview
            FROM dm_conversations
            ORDER BY updated_at DESC, dm_id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(row_to_direct_message_conversation)
            .collect()
    }

    pub(super) async fn projection_put_direct_message_message_impl(
        &self,
        row: DirectMessageMessageRow,
    ) -> Result<()> {
        let tombstoned = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM dm_message_tombstones
            WHERE dm_id = ?1 AND message_id = ?2
            LIMIT 1
            "#,
        )
        .bind(row.dm_id.as_str())
        .bind(row.message_id.as_str())
        .fetch_optional(&self.pool)
        .await?
        .is_some();
        if tombstoned {
            return Ok(());
        }
        let attachment_manifest_json = row
            .attachment_manifest
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        sqlx::query(
            r#"
            INSERT INTO dm_messages (
              dm_id, message_id, sender_pubkey, recipient_pubkey, created_at, text,
              reply_to_message_id, attachment_manifest_json, outgoing, acked_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(dm_id, message_id) DO UPDATE SET
              sender_pubkey = excluded.sender_pubkey,
              recipient_pubkey = excluded.recipient_pubkey,
              created_at = excluded.created_at,
              text = excluded.text,
              reply_to_message_id = excluded.reply_to_message_id,
              attachment_manifest_json = excluded.attachment_manifest_json,
              outgoing = excluded.outgoing,
              acked_at = excluded.acked_at
            "#,
        )
        .bind(row.dm_id.as_str())
        .bind(row.message_id.as_str())
        .bind(row.sender_pubkey.as_str())
        .bind(row.recipient_pubkey.as_str())
        .bind(row.created_at)
        .bind(row.text.as_deref())
        .bind(row.reply_to_message_id.as_deref())
        .bind(attachment_manifest_json.as_deref())
        .bind(if row.outgoing { 1_i64 } else { 0_i64 })
        .bind(row.acked_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_get_direct_message_message_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageMessageRow>> {
        let row = sqlx::query(
            r#"
            SELECT dm_id, message_id, sender_pubkey, recipient_pubkey, created_at, text,
                   reply_to_message_id, attachment_manifest_json, outgoing, acked_at
            FROM dm_messages
            WHERE dm_id = ?1 AND message_id = ?2
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_direct_message_message).transpose()
    }

    pub(super) async fn projection_list_direct_message_messages_impl(
        &self,
        dm_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<DirectMessageMessageRow>> {
        let rows = sqlx::query(
            r#"
            SELECT dm_id, message_id, sender_pubkey, recipient_pubkey, created_at, text,
                   reply_to_message_id, attachment_manifest_json, outgoing, acked_at
            FROM dm_messages
            WHERE dm_id = ?1
              AND (
                ?2 IS NULL
                OR created_at < ?2
                OR (created_at = ?2 AND message_id < ?3)
              )
            ORDER BY created_at DESC, message_id DESC
            LIMIT ?4
            "#,
        )
        .bind(dm_id)
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.object_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        direct_message_page_from_rows(rows, limit)
    }

    pub(super) async fn projection_set_direct_message_acked_at_impl(
        &self,
        dm_id: &str,
        message_id: &str,
        acked_at: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE dm_messages
            SET acked_at = ?3
            WHERE dm_id = ?1 AND message_id = ?2
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .bind(acked_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_put_direct_message_outbox_impl(
        &self,
        row: DirectMessageOutboxRow,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO dm_outbox (
              dm_id, message_id, peer_pubkey, frame_blob_hash, created_at, last_attempt_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(dm_id, message_id) DO UPDATE SET
              peer_pubkey = excluded.peer_pubkey,
              frame_blob_hash = excluded.frame_blob_hash,
              created_at = excluded.created_at,
              last_attempt_at = excluded.last_attempt_at
            "#,
        )
        .bind(row.dm_id.as_str())
        .bind(row.message_id.as_str())
        .bind(row.peer_pubkey.as_str())
        .bind(row.frame_blob_hash.as_str())
        .bind(row.created_at)
        .bind(row.last_attempt_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_get_direct_message_outbox_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<Option<DirectMessageOutboxRow>> {
        let row = sqlx::query(
            r#"
            SELECT dm_id, message_id, peer_pubkey, frame_blob_hash, created_at, last_attempt_at
            FROM dm_outbox
            WHERE dm_id = ?1 AND message_id = ?2
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_direct_message_outbox).transpose()
    }

    pub(super) async fn projection_list_direct_message_outbox_impl(
        &self,
    ) -> Result<Vec<DirectMessageOutboxRow>> {
        let rows = sqlx::query(
            r#"
            SELECT dm_id, message_id, peer_pubkey, frame_blob_hash, created_at, last_attempt_at
            FROM dm_outbox
            ORDER BY created_at ASC, message_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_direct_message_outbox).collect()
    }

    pub(super) async fn projection_touch_direct_message_outbox_attempt_impl(
        &self,
        dm_id: &str,
        message_id: &str,
        attempted_at: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE dm_outbox
            SET last_attempt_at = ?3
            WHERE dm_id = ?1 AND message_id = ?2
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .bind(attempted_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_remove_direct_message_outbox_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM dm_outbox
            WHERE dm_id = ?1 AND message_id = ?2
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_put_direct_message_tombstone_impl(
        &self,
        row: DirectMessageTombstoneRow,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO dm_message_tombstones (dm_id, message_id, deleted_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(dm_id, message_id) DO UPDATE SET
              deleted_at = excluded.deleted_at
            "#,
        )
        .bind(row.dm_id.as_str())
        .bind(row.message_id.as_str())
        .bind(row.deleted_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_list_direct_message_tombstones_impl(
        &self,
        dm_id: &str,
    ) -> Result<Vec<DirectMessageTombstoneRow>> {
        let rows = sqlx::query(
            r#"
            SELECT dm_id, message_id, deleted_at
            FROM dm_message_tombstones
            WHERE dm_id = ?1
            ORDER BY deleted_at DESC, message_id DESC
            "#,
        )
        .bind(dm_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(row_to_direct_message_tombstone)
            .collect()
    }

    pub(super) async fn projection_has_direct_message_tombstone_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM dm_message_tombstones
            WHERE dm_id = ?1 AND message_id = ?2
            LIMIT 1
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?
        .is_some();
        Ok(exists)
    }

    pub(super) async fn projection_delete_direct_message_message_local_impl(
        &self,
        dm_id: &str,
        message_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM dm_messages
            WHERE dm_id = ?1 AND message_id = ?2
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            DELETE FROM dm_outbox
            WHERE dm_id = ?1 AND message_id = ?2
            "#,
        )
        .bind(dm_id)
        .bind(message_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_clear_direct_message_local_impl(
        &self,
        dm_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM dm_messages
            WHERE dm_id = ?1
            "#,
        )
        .bind(dm_id)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            DELETE FROM dm_outbox
            WHERE dm_id = ?1
            "#,
        )
        .bind(dm_id)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            DELETE FROM dm_conversations
            WHERE dm_id = ?1
            "#,
        )
        .bind(dm_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
