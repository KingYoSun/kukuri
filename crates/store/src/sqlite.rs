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
use sqlx::{Pool, Row, Sqlite};

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

pub(crate) static STORE_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct SqliteStore {
    pool: Pool<Sqlite>,
}

impl SqliteStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
            .with_context(|| format!("failed to connect sqlite database: {database_url}"))?;

        run_store_migrations(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn connect_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .with_context(|| format!("failed to connect sqlite database: {}", path.display()))?;

        run_store_migrations(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn connect_memory() -> Result<Self> {
        Self::connect("sqlite::memory:").await
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

async fn run_store_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    repair_line_ending_only_migration_checksums(pool).await?;
    STORE_MIGRATOR.run(pool).await?;
    Ok(())
}

async fn repair_line_ending_only_migration_checksums(pool: &Pool<Sqlite>) -> Result<()> {
    if !sqlite_table_exists(pool, "_sqlx_migrations").await? {
        return Ok(());
    }

    let applied_migrations = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await?;

    for (version, applied_checksum) in applied_migrations {
        let Some(migration) = STORE_MIGRATOR.iter().find(|migration| {
            migration.version == version && !migration.migration_type.is_down_migration()
        }) else {
            continue;
        };

        if applied_checksum.as_slice() == migration.checksum.as_ref() {
            continue;
        }

        if checksum_matches_line_ending_variant(&applied_checksum, migration.sql.as_ref()) {
            repair_migration_checksum(pool, version).await?;
        }
    }

    Ok(())
}

async fn repair_migration_checksum(pool: &Pool<Sqlite>, version: i64) -> Result<()> {
    let migration = STORE_MIGRATOR
        .iter()
        .find(|migration| {
            migration.version == version && !migration.migration_type.is_down_migration()
        })
        .with_context(|| format!("embedded migration {version} is missing"))?;
    let result = sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = ?2")
        .bind(migration.checksum.as_ref())
        .bind(version)
        .execute(pool)
        .await?;
    if result.rows_affected() != 1 {
        anyhow::bail!("expected to repair one migration row for version {version}");
    }
    Ok(())
}

async fn sqlite_table_exists(pool: &Pool<Sqlite>, name: &str) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?
    .is_some();
    Ok(exists)
}

fn checksum_matches_line_ending_variant(applied_checksum: &[u8], sql: &str) -> bool {
    let lf_sql = normalize_sql_line_endings(sql);
    let lf_checksum = migration_checksum(lf_sql.as_str());
    if applied_checksum == lf_checksum {
        return true;
    }

    let crlf_sql = lf_sql.replace('\n', "\r\n");
    let crlf_checksum = migration_checksum(crlf_sql.as_str());
    applied_checksum == crlf_checksum
}

#[cfg(test)]
pub(crate) fn alternate_line_ending_checksum(
    sql: &str,
    current_checksum: &[u8],
) -> Option<Vec<u8>> {
    let lf_sql = normalize_sql_line_endings(sql);
    let lf_checksum = migration_checksum(lf_sql.as_str());
    if lf_checksum != current_checksum {
        return Some(lf_checksum);
    }

    let crlf_sql = lf_sql.replace('\n', "\r\n");
    let crlf_checksum = migration_checksum(crlf_sql.as_str());
    if crlf_checksum != current_checksum {
        return Some(crlf_checksum);
    }

    None
}

fn normalize_sql_line_endings(sql: &str) -> String {
    sql.replace("\r\n", "\n").replace('\r', "\n")
}

fn migration_checksum(sql: &str) -> Vec<u8> {
    Vec::from(Sha384::digest(sql.as_bytes()).as_slice())
}

#[async_trait]
impl Store for SqliteStore {
    async fn put_envelope(&self, envelope: KukuriEnvelope) -> Result<()> {
        let tags_json = serde_json::to_string(&envelope.tags)?;

        sqlx::query(
            r#"
            INSERT INTO envelopes (envelope_id, pubkey, created_at, kind, content, tags_json, sig)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(envelope_id) DO UPDATE SET
              pubkey = excluded.pubkey,
              created_at = excluded.created_at,
              kind = excluded.kind,
              content = excluded.content,
              tags_json = excluded.tags_json,
              sig = excluded.sig
            "#,
        )
        .bind(envelope.id.as_str())
        .bind(envelope.pubkey.as_str())
        .bind(envelope.created_at)
        .bind(envelope.kind.as_str())
        .bind(envelope.content.as_str())
        .bind(tags_json)
        .bind(envelope.sig.as_str())
        .execute(&self.pool)
        .await?;

        if let Some(topic_id) = envelope.topic_id() {
            sqlx::query(
                r#"
                INSERT INTO topic_objects (topic_id, object_id, created_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(topic_id, object_id) DO UPDATE SET created_at = excluded.created_at
                "#,
            )
            .bind(topic_id.as_str())
            .bind(envelope.id.as_str())
            .bind(envelope.created_at)
            .execute(&self.pool)
            .await?;

            let thread_ref = envelope.thread_ref().unwrap_or(ThreadRef {
                root: envelope.id.clone(),
                reply_to: None,
            });
            sqlx::query(
                r#"
                INSERT INTO object_threads (
                  topic_id, object_id, root_object_id, reply_to_object_id, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(object_id) DO UPDATE SET
                  topic_id = excluded.topic_id,
                  root_object_id = excluded.root_object_id,
                  reply_to_object_id = excluded.reply_to_object_id,
                  created_at = excluded.created_at
                "#,
            )
            .bind(topic_id.as_str())
            .bind(envelope.id.as_str())
            .bind(thread_ref.root.as_str())
            .bind(thread_ref.reply_to.as_ref().map(EnvelopeId::as_str))
            .bind(envelope.created_at)
            .execute(&self.pool)
            .await?;
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
        let row = sqlx::query(
            r#"
            SELECT envelope_id, pubkey, created_at, kind, content, tags_json, sig
            FROM envelopes
            WHERE envelope_id = ?1
            "#,
        )
        .bind(envelope_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_envelope).transpose()
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        let rows = sqlx::query(
            r#"
            SELECT e.envelope_id, e.pubkey, e.created_at, e.kind, e.content, e.tags_json, e.sig
            FROM topic_objects tp
            INNER JOIN envelopes e ON e.envelope_id = tp.object_id
            WHERE tp.topic_id = ?1
              AND (
                ?2 IS NULL
                OR e.created_at < ?2
                OR (e.created_at = ?2 AND e.envelope_id < ?3)
              )
            ORDER BY e.created_at DESC, e.envelope_id DESC
            LIMIT ?4
            "#,
        )
        .bind(topic_id)
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.object_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        envelope_page_from_rows(rows, limit)
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_root_object_id: &EnvelopeId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<KukuriEnvelope>> {
        let rows = sqlx::query(
            r#"
            SELECT e.envelope_id, e.pubkey, e.created_at, e.kind, e.content, e.tags_json, e.sig
            FROM object_threads te
            INNER JOIN envelopes e ON e.envelope_id = te.object_id
            WHERE te.topic_id = ?1
              AND te.root_object_id = ?2
              AND (
                ?3 IS NULL
                OR e.created_at > ?3
                OR (e.created_at = ?3 AND e.envelope_id > ?4)
              )
            ORDER BY
              CASE WHEN e.envelope_id = te.root_object_id THEN 0 ELSE 1 END ASC,
              e.created_at ASC,
              e.envelope_id ASC
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

        envelope_page_from_rows(rows, limit)
    }

    async fn upsert_profile(&self, profile: Profile) -> Result<()> {
        let existing = self.get_profile(profile.pubkey.as_str()).await?;
        if let Some(existing) = existing
            && existing.updated_at > profile.updated_at
        {
            return Ok(());
        }

        sqlx::query(
            r#"
            INSERT INTO profiles (
              pubkey, name, display_name, about, picture,
              picture_blob_hash, picture_mime, picture_bytes, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(pubkey) DO UPDATE SET
              name = excluded.name,
              display_name = excluded.display_name,
              about = excluded.about,
              picture = excluded.picture,
              picture_blob_hash = excluded.picture_blob_hash,
              picture_mime = excluded.picture_mime,
              picture_bytes = excluded.picture_bytes,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(profile.pubkey.as_str())
        .bind(profile.name.clone())
        .bind(profile.display_name.clone())
        .bind(profile.about.clone())
        .bind(profile.picture.clone())
        .bind(
            profile
                .picture_asset
                .as_ref()
                .map(|asset| asset.hash.as_str().to_string()),
        )
        .bind(
            profile
                .picture_asset
                .as_ref()
                .map(|asset| asset.mime.clone()),
        )
        .bind(
            profile
                .picture_asset
                .as_ref()
                .map(|asset| asset.bytes as i64),
        )
        .bind(profile.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_profile(&self, pubkey: &str) -> Result<Option<Profile>> {
        let row = sqlx::query(
            r#"
            SELECT
              pubkey, name, display_name, about, picture,
              picture_blob_hash, picture_mime, picture_bytes, updated_at
            FROM profiles
            WHERE pubkey = ?1
            "#,
        )
        .bind(pubkey)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Profile {
            pubkey: row.get::<String, _>("pubkey").into(),
            name: row.try_get("name").ok(),
            display_name: row.try_get("display_name").ok(),
            about: row.try_get("about").ok(),
            picture: row.try_get("picture").ok(),
            picture_asset: row
                .try_get::<String, _>("picture_blob_hash")
                .ok()
                .map(|hash| kukuri_core::AssetRef {
                    hash: kukuri_core::BlobHash::new(hash),
                    mime: row
                        .try_get::<String, _>("picture_mime")
                        .ok()
                        .unwrap_or_else(|| "application/octet-stream".into()),
                    bytes: row
                        .try_get::<i64, _>("picture_bytes")
                        .ok()
                        .unwrap_or_default() as u64,
                    role: kukuri_core::AssetRole::ProfileAvatar,
                }),
            updated_at: row.get("updated_at"),
        }))
    }

    async fn upsert_follow_edge(&self, edge: FollowEdge) -> Result<()> {
        let existing_updated_at = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT updated_at
            FROM follow_edges
            WHERE subject_pubkey = ?1 AND target_pubkey = ?2
            "#,
        )
        .bind(edge.subject_pubkey.as_str())
        .bind(edge.target_pubkey.as_str())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(updated_at) = existing_updated_at
            && updated_at > edge.updated_at
        {
            return Ok(());
        }

        sqlx::query(
            r#"
            INSERT INTO follow_edges (
              subject_pubkey, target_pubkey, status, updated_at, source_envelope_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(subject_pubkey, target_pubkey) DO UPDATE SET
              status = excluded.status,
              updated_at = excluded.updated_at,
              source_envelope_id = excluded.source_envelope_id
            "#,
        )
        .bind(edge.subject_pubkey.as_str())
        .bind(edge.target_pubkey.as_str())
        .bind(follow_edge_status_name(&edge.status))
        .bind(edge.updated_at)
        .bind(edge.envelope_id.as_str())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_follow_edges_by_subject(&self, subject_pubkey: &str) -> Result<Vec<FollowEdge>> {
        let rows = sqlx::query(
            r#"
            SELECT subject_pubkey, target_pubkey, status, updated_at, source_envelope_id
            FROM follow_edges
            WHERE subject_pubkey = ?1
            ORDER BY updated_at DESC, target_pubkey ASC
            "#,
        )
        .bind(subject_pubkey)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_follow_edge).collect()
    }

    async fn list_follow_edges_by_target(&self, target_pubkey: &str) -> Result<Vec<FollowEdge>> {
        let rows = sqlx::query(
            r#"
            SELECT subject_pubkey, target_pubkey, status, updated_at, source_envelope_id
            FROM follow_edges
            WHERE target_pubkey = ?1
            ORDER BY updated_at DESC, subject_pubkey ASC
            "#,
        )
        .bind(target_pubkey)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_follow_edge).collect()
    }
}

#[async_trait]
impl ProjectionStore for SqliteStore {
    async fn put_object_projection(&self, row: ObjectProjectionRow) -> Result<()> {
        let payload_json = serde_json::to_string(&row.payload_ref)?;
        let repost_json = row
            .repost_of
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        sqlx::query(
            r#"
            INSERT INTO object_index_cache (
              object_id, topic_id, channel_id, author_pubkey, created_at, object_kind,
              root_object_id, reply_to_object_id, payload_ref_json, content, repost_of_json,
              source_replica_id, source_key, source_envelope_id, source_blob_hash, derived_at,
              projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
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
        .bind(repost_json.as_deref())
        .bind(row.source_replica_id.as_str())
        .bind(row.source_key.as_str())
        .bind(row.source_envelope_id.as_str())
        .bind(row.source_blob_hash.as_ref().map(BlobHash::as_str))
        .bind(row.derived_at)
        .bind(row.projection_version)
        .execute(&self.pool)
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_object_projection(
        &self,
        object_id: &EnvelopeId,
    ) -> Result<Option<ObjectProjectionRow>> {
        let row = sqlx::query(
            r#"
            SELECT object_id, topic_id, author_pubkey, created_at, object_kind, root_object_id,
                   reply_to_object_id, channel_id, payload_ref_json, content, repost_of_json,
                   source_replica_id, source_key, source_envelope_id, source_blob_hash, derived_at,
                   projection_version
            FROM object_index_cache
            WHERE object_id = ?1
            "#,
        )
        .bind(object_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_object_projection).transpose()
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<ObjectProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT object_id, topic_id, author_pubkey, created_at, object_kind, root_object_id,
                   reply_to_object_id, channel_id, payload_ref_json, content, repost_of_json,
                   source_replica_id, source_key, source_envelope_id, source_blob_hash, derived_at,
                   projection_version
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

    async fn list_thread(
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
                   oic.payload_ref_json, oic.content, oic.repost_of_json, oic.source_replica_id,
                   oic.source_key, oic.source_envelope_id, oic.source_blob_hash, oic.derived_at,
                   oic.projection_version
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

    async fn upsert_profile_cache(&self, profile: Profile) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO profile_cache (
              pubkey, name, display_name, about, picture,
              picture_blob_hash, picture_mime, picture_bytes, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(pubkey) DO UPDATE SET
              name = excluded.name,
              display_name = excluded.display_name,
              about = excluded.about,
              picture = excluded.picture,
              picture_blob_hash = excluded.picture_blob_hash,
              picture_mime = excluded.picture_mime,
              picture_bytes = excluded.picture_bytes,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(profile.pubkey.as_str())
        .bind(profile.name)
        .bind(profile.display_name)
        .bind(profile.about)
        .bind(profile.picture)
        .bind(
            profile
                .picture_asset
                .as_ref()
                .map(|asset| asset.hash.as_str().to_string()),
        )
        .bind(
            profile
                .picture_asset
                .as_ref()
                .map(|asset| asset.mime.clone()),
        )
        .bind(
            profile
                .picture_asset
                .as_ref()
                .map(|asset| asset.bytes as i64),
        )
        .bind(profile.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_live_session_cache(&self, row: LiveSessionProjectionRow) -> Result<()> {
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

    async fn list_topic_live_sessions(
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

    async fn upsert_game_room_cache(&self, row: GameRoomProjectionRow) -> Result<()> {
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

    async fn list_topic_game_rooms(&self, topic_id: &str) -> Result<Vec<GameRoomProjectionRow>> {
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

    async fn get_author_relationship(
        &self,
        local_author_pubkey: &str,
        author_pubkey: &str,
    ) -> Result<Option<AuthorRelationshipProjectionRow>> {
        let row = sqlx::query(
            r#"
            SELECT local_author_pubkey, author_pubkey, following, followed_by, mutual,
                   friend_of_friend, friend_of_friend_via_pubkeys_json, derived_at
            FROM author_relationship_cache
            WHERE local_author_pubkey = ?1 AND author_pubkey = ?2
            "#,
        )
        .bind(local_author_pubkey)
        .bind(author_pubkey)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_author_relationship_projection).transpose()
    }

    async fn rebuild_author_relationships(
        &self,
        local_author_pubkey: &str,
        rows: Vec<AuthorRelationshipProjectionRow>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            DELETE FROM author_relationship_cache
            WHERE local_author_pubkey = ?1
            "#,
        )
        .bind(local_author_pubkey)
        .execute(&mut *tx)
        .await?;

        for row in rows {
            let via_json = serde_json::to_string(&row.friend_of_friend_via_pubkeys)?;
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO author_relationship_cache (
                  local_author_pubkey, author_pubkey, following, followed_by, mutual,
                  friend_of_friend, friend_of_friend_via_pubkeys_json, derived_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
            )
            .bind(row.local_author_pubkey.as_str())
            .bind(row.author_pubkey.as_str())
            .bind(row.following)
            .bind(row.followed_by)
            .bind(row.mutual)
            .bind(row.friend_of_friend)
            .bind(via_json)
            .bind(row.derived_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn put_muted_author(&self, row: MutedAuthorRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO muted_authors (author_pubkey, muted_at)
            VALUES (?1, ?2)
            ON CONFLICT(author_pubkey) DO UPDATE SET
              muted_at = excluded.muted_at
            "#,
        )
        .bind(row.author_pubkey.as_str())
        .bind(row.muted_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_muted_author(&self, author_pubkey: &str) -> Result<Option<MutedAuthorRow>> {
        let row = sqlx::query(
            r#"
            SELECT author_pubkey, muted_at
            FROM muted_authors
            WHERE author_pubkey = ?1
            "#,
        )
        .bind(author_pubkey)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_muted_author).transpose()
    }

    async fn list_muted_authors(&self) -> Result<Vec<MutedAuthorRow>> {
        let rows = sqlx::query(
            r#"
            SELECT author_pubkey, muted_at
            FROM muted_authors
            ORDER BY muted_at DESC, author_pubkey ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_muted_author).collect()
    }

    async fn remove_muted_author(&self, author_pubkey: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM muted_authors
            WHERE author_pubkey = ?1
            "#,
        )
        .bind(author_pubkey)
        .execute(&self.pool)
        .await?;
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

    async fn clear_expired_live_presence(&self, now_ms: i64) -> Result<()> {
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

    async fn clear_topic_live_presence(&self, topic_id: &str) -> Result<()> {
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

    async fn mark_blob_status(&self, hash: &BlobHash, status: BlobCacheStatus) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO blob_objects (blob_hash, status)
            VALUES (?1, ?2)
            ON CONFLICT(blob_hash) DO UPDATE SET status = excluded.status
            "#,
        )
        .bind(hash.as_str())
        .bind(match status {
            BlobCacheStatus::Missing => "missing",
            BlobCacheStatus::Available => "available",
            BlobCacheStatus::Pinned => "pinned",
        })
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_reaction_cache(&self, row: ReactionProjectionRow) -> Result<()> {
        let snapshot_json = row
            .custom_asset_snapshot
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        sqlx::query(
            r#"
            INSERT INTO reaction_cache (
              source_replica_id, target_object_id, reaction_id, author_pubkey, created_at,
              updated_at, reaction_key_kind, normalized_reaction_key, emoji, custom_asset_id,
              custom_asset_snapshot_json, status, source_key, source_envelope_id, derived_at,
              projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(source_replica_id, target_object_id, reaction_id) DO UPDATE SET
              author_pubkey = excluded.author_pubkey,
              created_at = excluded.created_at,
              updated_at = excluded.updated_at,
              reaction_key_kind = excluded.reaction_key_kind,
              normalized_reaction_key = excluded.normalized_reaction_key,
              emoji = excluded.emoji,
              custom_asset_id = excluded.custom_asset_id,
              custom_asset_snapshot_json = excluded.custom_asset_snapshot_json,
              status = excluded.status,
              source_key = excluded.source_key,
              source_envelope_id = excluded.source_envelope_id,
              derived_at = excluded.derived_at,
              projection_version = excluded.projection_version
            "#,
        )
        .bind(row.source_replica_id.as_str())
        .bind(row.target_object_id.as_str())
        .bind(row.reaction_id.as_str())
        .bind(row.author_pubkey.as_str())
        .bind(row.created_at)
        .bind(row.updated_at)
        .bind(reaction_key_kind_name(&row.reaction_key_kind))
        .bind(row.normalized_reaction_key.as_str())
        .bind(row.emoji.as_deref())
        .bind(row.custom_asset_id.as_deref())
        .bind(snapshot_json.as_deref())
        .bind(object_status_name(&row.status))
        .bind(row.source_key.as_str())
        .bind(row.source_envelope_id.as_str())
        .bind(row.derived_at)
        .bind(row.projection_version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_reaction_cache(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
        reaction_id: &EnvelopeId,
    ) -> Result<Option<ReactionProjectionRow>> {
        let row = sqlx::query(
            r#"
            SELECT source_replica_id, target_object_id, reaction_id, author_pubkey, created_at,
                   updated_at, reaction_key_kind, normalized_reaction_key, emoji,
                   custom_asset_id, custom_asset_snapshot_json, status, source_key,
                   source_envelope_id, derived_at, projection_version
            FROM reaction_cache
            WHERE source_replica_id = ?1 AND target_object_id = ?2 AND reaction_id = ?3
            "#,
        )
        .bind(source_replica_id.as_str())
        .bind(target_object_id.as_str())
        .bind(reaction_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_reaction_projection).transpose()
    }

    async fn list_reaction_cache_for_target(
        &self,
        source_replica_id: &ReplicaId,
        target_object_id: &EnvelopeId,
    ) -> Result<Vec<ReactionProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT source_replica_id, target_object_id, reaction_id, author_pubkey, created_at,
                   updated_at, reaction_key_kind, normalized_reaction_key, emoji,
                   custom_asset_id, custom_asset_snapshot_json, status, source_key,
                   source_envelope_id, derived_at, projection_version
            FROM reaction_cache
            WHERE source_replica_id = ?1 AND target_object_id = ?2
            ORDER BY normalized_reaction_key ASC, reaction_id ASC
            "#,
        )
        .bind(source_replica_id.as_str())
        .bind(target_object_id.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_reaction_projection).collect()
    }

    async fn list_recent_reaction_cache_by_author(
        &self,
        author_pubkey: &str,
    ) -> Result<Vec<ReactionProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT source_replica_id, target_object_id, reaction_id, author_pubkey, created_at,
                   updated_at, reaction_key_kind, normalized_reaction_key, emoji,
                   custom_asset_id, custom_asset_snapshot_json, status, source_key,
                   source_envelope_id, derived_at, projection_version
            FROM reaction_cache
            WHERE author_pubkey = ?1
            ORDER BY updated_at DESC, reaction_id DESC
            "#,
        )
        .bind(author_pubkey)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_reaction_projection).collect()
    }

    async fn put_bookmarked_custom_reaction(&self, row: BookmarkedCustomReactionRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO bookmarked_custom_reactions (
              asset_id, owner_pubkey, blob_hash, search_key, mime, bytes, width, height, bookmarked_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(asset_id) DO UPDATE SET
              owner_pubkey = excluded.owner_pubkey,
              blob_hash = excluded.blob_hash,
              search_key = excluded.search_key,
              mime = excluded.mime,
              bytes = excluded.bytes,
              width = excluded.width,
              height = excluded.height,
              bookmarked_at = excluded.bookmarked_at
            "#,
        )
        .bind(row.asset_id.as_str())
        .bind(row.owner_pubkey.as_str())
        .bind(row.blob_hash.as_str())
        .bind(row.search_key.as_str())
        .bind(row.mime.as_str())
        .bind(row.bytes as i64)
        .bind(i64::from(row.width))
        .bind(i64::from(row.height))
        .bind(row.bookmarked_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_bookmarked_custom_reactions(&self) -> Result<Vec<BookmarkedCustomReactionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT asset_id, owner_pubkey, blob_hash, search_key, mime, bytes, width, height, bookmarked_at
            FROM bookmarked_custom_reactions
            ORDER BY bookmarked_at DESC, asset_id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(row_to_bookmarked_custom_reaction)
            .collect()
    }

    async fn remove_bookmarked_custom_reaction(&self, asset_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM bookmarked_custom_reactions
            WHERE asset_id = ?1
            "#,
        )
        .bind(asset_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn put_bookmarked_post(&self, row: BookmarkedPostRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO bookmarked_posts (
              source_object_id,
              source_envelope_id,
              source_replica_id,
              topic_id,
              channel_id,
              author_pubkey,
              created_at,
              object_kind,
              payload_ref_json,
              content,
              attachments_json,
              reply_to_object_id,
              root_object_id,
              repost_of_json,
              bookmarked_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(source_object_id) DO UPDATE SET
              source_envelope_id = excluded.source_envelope_id,
              source_replica_id = excluded.source_replica_id,
              topic_id = excluded.topic_id,
              channel_id = excluded.channel_id,
              author_pubkey = excluded.author_pubkey,
              created_at = excluded.created_at,
              object_kind = excluded.object_kind,
              payload_ref_json = excluded.payload_ref_json,
              content = excluded.content,
              attachments_json = excluded.attachments_json,
              reply_to_object_id = excluded.reply_to_object_id,
              root_object_id = excluded.root_object_id,
              repost_of_json = excluded.repost_of_json,
              bookmarked_at = excluded.bookmarked_at
            "#,
        )
        .bind(row.source_object_id.as_str())
        .bind(row.source_envelope_id.as_str())
        .bind(row.source_replica_id.as_str())
        .bind(row.topic_id.as_str())
        .bind(row.channel_id.as_str())
        .bind(row.author_pubkey.as_str())
        .bind(row.created_at)
        .bind(row.object_kind.as_str())
        .bind(serde_json::to_string(&row.payload_ref)?)
        .bind(row.content.as_deref())
        .bind(serde_json::to_string(&row.attachments)?)
        .bind(row.reply_to_object_id.as_ref().map(EnvelopeId::as_str))
        .bind(row.root_object_id.as_ref().map(EnvelopeId::as_str))
        .bind(
            row.repost_of
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
        )
        .bind(row.bookmarked_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_bookmarked_posts(&self) -> Result<Vec<BookmarkedPostRow>> {
        let rows = sqlx::query(
            r#"
            SELECT
              source_object_id,
              source_envelope_id,
              source_replica_id,
              topic_id,
              channel_id,
              author_pubkey,
              created_at,
              object_kind,
              payload_ref_json,
              content,
              attachments_json,
              reply_to_object_id,
              root_object_id,
              repost_of_json,
              bookmarked_at
            FROM bookmarked_posts
            ORDER BY bookmarked_at DESC, source_object_id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_bookmarked_post).collect()
    }

    async fn remove_bookmarked_post(&self, source_object_id: &EnvelopeId) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM bookmarked_posts
            WHERE source_object_id = ?1
            "#,
        )
        .bind(source_object_id.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_direct_message_conversation(
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

    async fn get_direct_message_conversation_by_peer(
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

    async fn get_direct_message_conversation_by_dm_id(
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

    async fn list_direct_message_conversations(&self) -> Result<Vec<DirectMessageConversationRow>> {
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

    async fn put_direct_message_message(&self, row: DirectMessageMessageRow) -> Result<()> {
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

    async fn get_direct_message_message(
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

    async fn list_direct_message_messages(
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

    async fn set_direct_message_acked_at(
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

    async fn put_direct_message_outbox(&self, row: DirectMessageOutboxRow) -> Result<()> {
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

    async fn get_direct_message_outbox(
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

    async fn list_direct_message_outbox(&self) -> Result<Vec<DirectMessageOutboxRow>> {
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

    async fn touch_direct_message_outbox_attempt(
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

    async fn remove_direct_message_outbox(&self, dm_id: &str, message_id: &str) -> Result<()> {
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

    async fn put_direct_message_tombstone(&self, row: DirectMessageTombstoneRow) -> Result<()> {
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

    async fn list_direct_message_tombstones(
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

    async fn has_direct_message_tombstone(&self, dm_id: &str, message_id: &str) -> Result<bool> {
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

    async fn delete_direct_message_message_local(
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

    async fn clear_direct_message_local(&self, dm_id: &str) -> Result<()> {
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

    async fn put_notification_if_absent(&self, row: NotificationRow) -> Result<bool> {
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

    async fn list_notifications(&self) -> Result<Vec<NotificationRow>> {
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

    async fn mark_notification_read(&self, notification_id: &str, read_at: i64) -> Result<()> {
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

    async fn mark_all_notifications_read(&self, read_at: i64) -> Result<()> {
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

    async fn count_unread_notifications(&self) -> Result<usize> {
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

    async fn rebuild_object_projections(&self, rows: Vec<ObjectProjectionRow>) -> Result<()> {
        sqlx::query("DELETE FROM object_thread_cache")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM object_index_cache")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM live_session_cache")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM game_room_cache")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM live_presence_cache")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM reaction_cache")
            .execute(&self.pool)
            .await?;
        for row in rows {
            self.put_object_projection(row).await?;
        }
        Ok(())
    }
}
