use super::*;

impl SqliteStore {
    pub(super) async fn projection_put_object_projection_impl(
        &self,
        row: ObjectProjectionRow,
    ) -> Result<()> {
        self.put_object_projections(vec![row]).await
    }

    pub(super) async fn projection_put_object_projections_impl(
        &self,
        rows: Vec<ObjectProjectionRow>,
    ) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        for row in rows {
            let payload_json = serde_json::to_string(&row.payload_ref)?;
            let attachments_json = serde_json::to_string(&row.attachments)?;
            let repost_json = row
                .repost_of
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?;
            sqlx::query(
                r#"
                INSERT INTO object_index_cache (
                  object_id, topic_id, channel_id, author_pubkey, created_at, object_kind,
                  root_object_id, reply_to_object_id, payload_ref_json, content, attachments_json,
                  repost_of_json, source_replica_id, source_key, source_envelope_id,
                  source_blob_hash, derived_at, projection_version
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                ON CONFLICT(object_id) DO UPDATE SET
                  topic_id = excluded.topic_id,
                  channel_id = excluded.channel_id,
                  author_pubkey = excluded.author_pubkey,
                  created_at = excluded.created_at,
                  object_kind = excluded.object_kind,
                  root_object_id = excluded.root_object_id,
                  reply_to_object_id = excluded.reply_to_object_id,
                  payload_ref_json = excluded.payload_ref_json,
                  content = excluded.content,
                  attachments_json = excluded.attachments_json,
                  repost_of_json = excluded.repost_of_json,
                  source_replica_id = excluded.source_replica_id,
                  source_key = excluded.source_key,
                  source_envelope_id = excluded.source_envelope_id,
                  source_blob_hash = excluded.source_blob_hash,
                  derived_at = excluded.derived_at,
                  projection_version = excluded.projection_version
                "#,
            )
            .bind(row.object_id.as_str())
            .bind(row.topic_id.as_str())
            .bind(row.channel_id.as_str())
            .bind(row.author_pubkey.as_str())
            .bind(row.created_at)
            .bind(row.object_kind.as_str())
            .bind(row.root_object_id.as_ref().map(EnvelopeId::as_str))
            .bind(row.reply_to_object_id.as_ref().map(EnvelopeId::as_str))
            .bind(payload_json)
            .bind(row.content.as_deref())
            .bind(attachments_json)
            .bind(repost_json.as_deref())
            .bind(row.source_replica_id.as_str())
            .bind(row.source_key.as_str())
            .bind(row.source_envelope_id.as_str())
            .bind(row.source_blob_hash.as_ref().map(BlobHash::as_str))
            .bind(row.derived_at)
            .bind(row.projection_version)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO object_thread_cache (
                  object_id, topic_id, channel_id, root_object_id, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(object_id) DO UPDATE SET
                  topic_id = excluded.topic_id,
                  channel_id = excluded.channel_id,
                  root_object_id = excluded.root_object_id,
                  created_at = excluded.created_at
                "#,
            )
            .bind(row.object_id.as_str())
            .bind(row.topic_id.as_str())
            .bind(row.channel_id.as_str())
            .bind(
                row.root_object_id
                    .as_ref()
                    .unwrap_or(&row.object_id)
                    .as_str(),
            )
            .bind(row.created_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub(super) async fn projection_get_object_projection_impl(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<ObjectProjectionRow>> {
        let row = sqlx::query(
            r#"
            SELECT object_id, topic_id, author_pubkey, created_at, object_kind, root_object_id,
                   reply_to_object_id, channel_id, payload_ref_json, content, attachments_json,
                   repost_of_json, source_replica_id, source_key, source_envelope_id,
                   source_blob_hash, derived_at, projection_version
            FROM object_index_cache
            WHERE object_id = ?1
            "#,
        )
        .bind(object_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_object_projection).transpose()
    }

    pub(super) async fn projection_list_topic_timeline_impl(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT object_id, topic_id, author_pubkey, created_at, object_kind, root_object_id,
                   reply_to_object_id, channel_id, payload_ref_json, content, attachments_json,
                   repost_of_json, source_replica_id, source_key, source_envelope_id,
                   source_blob_hash, derived_at, projection_version
            FROM object_index_cache
            WHERE topic_id = ?1
              AND (
                ?2 IS NULL
                OR created_at < ?2
                OR (created_at = ?2 AND object_id < ?3)
              )
            ORDER BY created_at DESC, object_id DESC
            LIMIT ?4
            "#,
        )
        .bind(topic_id)
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.object_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        object_projection_page_from_rows(rows, limit)
    }

    pub(super) async fn projection_list_topic_timeline_filtered_impl(
        &self,
        topic_id: &str,
        allowed_channels: &std::collections::BTreeSet<String>,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        if limit == 0 || allowed_channels.is_empty() {
            return Ok(Page {
                items: Vec::new(),
                next_cursor: cursor,
            });
        }

        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT object_id, topic_id, author_pubkey, created_at, object_kind, root_object_id,
                   reply_to_object_id, channel_id, payload_ref_json, content, attachments_json,
                   repost_of_json, source_replica_id, source_key, source_envelope_id,
                   source_blob_hash, derived_at, projection_version
            FROM object_index_cache
            WHERE topic_id = "#,
        );
        builder.push_bind(topic_id);
        builder.push(" AND channel_id IN (");
        let mut separated = builder.separated(", ");
        for channel_id in allowed_channels {
            separated.push_bind(channel_id);
        }
        separated.push_unseparated(")");
        builder.push(
            r#"
              AND (
                "#,
        );
        builder.push_bind(cursor.as_ref().map(|value| value.created_at));
        builder.push(
            r#" IS NULL
                OR created_at < "#,
        );
        builder.push_bind(cursor.as_ref().map(|value| value.created_at));
        builder.push(
            r#"
                OR (created_at = "#,
        );
        builder.push_bind(cursor.as_ref().map(|value| value.created_at));
        builder.push(" AND object_id < ");
        builder.push_bind(cursor.as_ref().map(|value| value.object_id.as_str()));
        builder.push(
            r#")
              )
            ORDER BY created_at DESC, object_id DESC
            LIMIT "#,
        );
        builder.push_bind(limit as i64);

        let rows = builder.build().fetch_all(&self.pool).await?;
        object_projection_page_from_rows(rows, limit)
    }

    pub(super) async fn projection_list_thread_impl(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT oic.object_id, oic.topic_id, oic.author_pubkey, oic.created_at, oic.object_kind,
                   oic.root_object_id, oic.reply_to_object_id, oic.channel_id,
                   oic.payload_ref_json, oic.content, oic.attachments_json, oic.repost_of_json,
                   oic.source_replica_id, oic.source_key, oic.source_envelope_id,
                   oic.source_blob_hash, oic.derived_at, oic.projection_version
            FROM object_thread_cache tc
            INNER JOIN object_index_cache oic ON oic.object_id = tc.object_id
            WHERE tc.topic_id = ?1
              AND tc.root_object_id = ?2
              AND (
                ?3 IS NULL
                OR oic.created_at > ?3
                OR (oic.created_at = ?3 AND oic.object_id > ?4)
              )
            ORDER BY
              CASE WHEN oic.object_id = tc.root_object_id THEN 0 ELSE 1 END ASC,
              oic.created_at ASC,
              oic.object_id ASC
            LIMIT ?5
            "#,
        )
        .bind(topic_id)
        .bind(thread_root_object_id.as_str())
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.object_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        object_projection_page_from_rows(rows, limit)
    }

    pub(super) async fn projection_list_thread_filtered_impl(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        allowed_channel: Option<&str>,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let Some(channel_id) = allowed_channel else {
            return ProjectionStore::list_thread(
                self,
                topic_id,
                thread_root_object_id,
                cursor,
                limit,
            )
            .await;
        };

        let rows = sqlx::query(
            r#"
            SELECT oic.object_id, oic.topic_id, oic.author_pubkey, oic.created_at, oic.object_kind,
                   oic.root_object_id, oic.reply_to_object_id, oic.channel_id,
                   oic.payload_ref_json, oic.content, oic.attachments_json, oic.repost_of_json,
                   oic.source_replica_id, oic.source_key, oic.source_envelope_id,
                   oic.source_blob_hash, oic.derived_at, oic.projection_version
            FROM object_thread_cache tc
            INNER JOIN object_index_cache oic ON oic.object_id = tc.object_id
            WHERE tc.topic_id = ?1
              AND tc.root_object_id = ?2
              AND tc.channel_id = ?3
              AND (
                ?4 IS NULL
                OR oic.created_at > ?4
                OR (oic.created_at = ?4 AND oic.object_id > ?5)
              )
            ORDER BY
              CASE WHEN oic.object_id = tc.root_object_id THEN 0 ELSE 1 END ASC,
              oic.created_at ASC,
              oic.object_id ASC
            LIMIT ?6
            "#,
        )
        .bind(topic_id)
        .bind(thread_root_object_id.as_str())
        .bind(channel_id)
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.object_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        object_projection_page_from_rows(rows, limit)
    }

    pub(super) async fn projection_rebuild_object_projections_impl(
        &self,
        rows: Vec<ObjectProjectionRow>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM object_thread_cache")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM object_index_cache")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM live_session_cache")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_room_cache")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM live_presence_cache")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM reaction_cache")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.put_object_projections(rows).await?;
        Ok(())
    }
}
