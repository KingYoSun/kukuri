use super::*;

impl SqliteStore {
    pub(super) async fn projection_put_notification_if_absent_impl(
        &self,
        row: NotificationRow,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO notifications (
              notification_id,
              recipient_pubkey,
              kind,
              actor_pubkey,
              source_envelope_id,
              source_replica_id,
              topic_id,
              channel_id,
              object_id,
              dm_id,
              message_id,
              preview_text,
              created_at,
              received_at,
              read_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
        )
        .bind(row.notification_id.as_str())
        .bind(row.recipient_pubkey.as_str())
        .bind(notification_kind_name(&row.kind))
        .bind(row.actor_pubkey.as_str())
        .bind(row.source_envelope_id.as_ref().map(EnvelopeId::as_str))
        .bind(row.source_replica_id.as_ref().map(ReplicaId::as_str))
        .bind(row.topic_id.as_deref())
        .bind(row.channel_id.as_deref())
        .bind(row.object_id.as_ref().map(EnvelopeId::as_str))
        .bind(row.dm_id.as_deref())
        .bind(row.message_id.as_deref())
        .bind(row.preview_text.as_deref())
        .bind(row.created_at)
        .bind(row.received_at)
        .bind(row.read_at)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(super) async fn projection_list_notifications_impl(&self) -> Result<Vec<NotificationRow>> {
        let rows = sqlx::query(
            r#"
            SELECT
              notification_id,
              recipient_pubkey,
              kind,
              actor_pubkey,
              source_envelope_id,
              source_replica_id,
              topic_id,
              channel_id,
              object_id,
              dm_id,
              message_id,
              preview_text,
              created_at,
              received_at,
              read_at
            FROM notifications
            ORDER BY received_at DESC, notification_id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_notification).collect()
    }

    pub(super) async fn projection_mark_notification_read_impl(
        &self,
        notification_id: &str,
        read_at: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE notifications
            SET read_at = COALESCE(read_at, ?2)
            WHERE notification_id = ?1
            "#,
        )
        .bind(notification_id)
        .bind(read_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_mark_all_notifications_read_impl(
        &self,
        read_at: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE notifications
            SET read_at = COALESCE(read_at, ?1)
            WHERE read_at IS NULL
            "#,
        )
        .bind(read_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_count_unread_notifications_impl(&self) -> Result<usize> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM notifications
            WHERE read_at IS NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count as usize)
    }
}
