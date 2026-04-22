use super::*;

impl SqliteStore {
    pub(super) async fn projection_upsert_live_session_cache_impl(
        &self,
        row: LiveSessionProjectionRow,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO live_session_cache (
              session_id, topic_id, channel_id, host_pubkey, title, description, status,
              started_at, ended_at, updated_at, source_replica_id, source_key, manifest_blob_hash,
              derived_at, projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(session_id) DO UPDATE SET
              topic_id = excluded.topic_id,
              channel_id = excluded.channel_id,
              host_pubkey = excluded.host_pubkey,
              title = excluded.title,
              description = excluded.description,
              status = excluded.status,
              started_at = excluded.started_at,
              ended_at = excluded.ended_at,
              updated_at = excluded.updated_at,
              source_replica_id = excluded.source_replica_id,
              source_key = excluded.source_key,
              manifest_blob_hash = excluded.manifest_blob_hash,
              derived_at = excluded.derived_at,
              projection_version = excluded.projection_version
            "#,
        )
        .bind(row.session_id.as_str())
        .bind(row.topic_id.as_str())
        .bind(row.channel_id.as_str())
        .bind(row.host_pubkey.as_str())
        .bind(row.title.as_str())
        .bind(row.description.as_str())
        .bind(live_status_name(&row.status))
        .bind(row.started_at)
        .bind(row.ended_at)
        .bind(row.updated_at)
        .bind(row.source_replica_id.as_str())
        .bind(row.source_key.as_str())
        .bind(row.manifest_blob_hash.as_str())
        .bind(row.derived_at)
        .bind(row.projection_version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_list_topic_live_sessions_impl(
        &self,
        topic_id: &str,
    ) -> Result<Vec<LiveSessionProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT lsc.session_id, lsc.topic_id, lsc.host_pubkey, lsc.title, lsc.description,
                   lsc.channel_id, lsc.status, lsc.started_at, lsc.ended_at, lsc.updated_at,
                   lsc.source_replica_id, lsc.source_key, lsc.manifest_blob_hash, lsc.derived_at,
                   lsc.projection_version,
                   CASE
                     WHEN lsc.status = 'ended' THEN 0
                     ELSE COALESCE((
                       SELECT COUNT(*)
                       FROM live_presence_cache lpc
                       WHERE lpc.topic_id = lsc.topic_id
                         AND lpc.channel_id = lsc.channel_id
                         AND lpc.session_id = lsc.session_id
                     ), 0)
                   END AS viewer_count
            FROM live_session_cache lsc
            WHERE lsc.topic_id = ?1
            ORDER BY lsc.started_at DESC, lsc.session_id DESC
            "#,
        )
        .bind(topic_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(row_to_live_session_projection)
            .collect()
    }

    pub(super) async fn projection_upsert_game_room_cache_impl(
        &self,
        row: GameRoomProjectionRow,
    ) -> Result<()> {
        let scores_json = serde_json::to_string(&row.scores)?;
        sqlx::query(
            r#"
            INSERT INTO game_room_cache (
              room_id, topic_id, channel_id, host_pubkey, title, description, status, phase_label,
              scores_json, updated_at, source_replica_id, source_key, manifest_blob_hash,
              derived_at, projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(room_id) DO UPDATE SET
              topic_id = excluded.topic_id,
              channel_id = excluded.channel_id,
              host_pubkey = excluded.host_pubkey,
              title = excluded.title,
              description = excluded.description,
              status = excluded.status,
              phase_label = excluded.phase_label,
              scores_json = excluded.scores_json,
              updated_at = excluded.updated_at,
              source_replica_id = excluded.source_replica_id,
              source_key = excluded.source_key,
              manifest_blob_hash = excluded.manifest_blob_hash,
              derived_at = excluded.derived_at,
              projection_version = excluded.projection_version
            "#,
        )
        .bind(row.room_id.as_str())
        .bind(row.topic_id.as_str())
        .bind(row.channel_id.as_str())
        .bind(row.host_pubkey.as_str())
        .bind(row.title.as_str())
        .bind(row.description.as_str())
        .bind(game_status_name(&row.status))
        .bind(row.phase_label.as_deref())
        .bind(scores_json)
        .bind(row.updated_at)
        .bind(row.source_replica_id.as_str())
        .bind(row.source_key.as_str())
        .bind(row.manifest_blob_hash.as_str())
        .bind(row.derived_at)
        .bind(row.projection_version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_list_topic_game_rooms_impl(
        &self,
        topic_id: &str,
    ) -> Result<Vec<GameRoomProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT room_id, topic_id, host_pubkey, title, description, status, phase_label,
                   channel_id, scores_json, updated_at, source_replica_id, source_key,
                   manifest_blob_hash, derived_at, projection_version
            FROM game_room_cache
            WHERE topic_id = ?1
            ORDER BY updated_at DESC, room_id DESC
            "#,
        )
        .bind(topic_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_game_room_projection).collect()
    }

    pub(super) async fn projection_upsert_live_presence_impl(
        &self,
        topic_id: &str,
        channel_id: &str,
        session_id: &str,
        author_pubkey: &str,
        expires_at: i64,
        updated_at: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO live_presence_cache (
              topic_id, channel_id, session_id, author_pubkey, expires_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(topic_id, channel_id, session_id, author_pubkey) DO UPDATE SET
              expires_at = excluded.expires_at,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(topic_id)
        .bind(channel_id)
        .bind(session_id)
        .bind(author_pubkey)
        .bind(expires_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_clear_expired_live_presence_impl(
        &self,
        now_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM live_presence_cache
            WHERE expires_at <= ?1
            "#,
        )
        .bind(now_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(super) async fn projection_clear_topic_live_presence_impl(
        &self,
        topic_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM live_presence_cache
            WHERE topic_id = ?1
            "#,
        )
        .bind(topic_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
