use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use kukuri_core::{
    BlobHash, EnvelopeId, FollowEdge, FollowEdgeStatus, GameRoomStatus, GameScoreEntry,
    KukuriEnvelope, LiveSessionStatus, PayloadRef, Profile, ReplicaId, ThreadRef,
    parse_follow_edge, parse_profile,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineCursor {
    pub created_at: i64,
    pub object_id: EnvelopeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<TimelineCursor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobCacheStatus {
    Missing,
    Available,
    Pinned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectProjectionRow {
    pub object_id: EnvelopeId,
    pub topic_id: String,
    pub channel_id: String,
    pub author_pubkey: String,
    pub created_at: i64,
    pub root_object_id: Option<EnvelopeId>,
    pub reply_to_object_id: Option<EnvelopeId>,
    pub payload_ref: PayloadRef,
    pub content: Option<String>,
    pub source_replica_id: ReplicaId,
    pub source_key: String,
    pub source_envelope_id: EnvelopeId,
    pub source_blob_hash: Option<BlobHash>,
    pub derived_at: i64,
    pub projection_version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionProjectionRow {
    pub session_id: String,
    pub topic_id: String,
    pub channel_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: LiveSessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub updated_at: i64,
    pub source_replica_id: ReplicaId,
    pub source_key: String,
    pub manifest_blob_hash: BlobHash,
    pub derived_at: i64,
    pub projection_version: i64,
    pub viewer_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRoomProjectionRow {
    pub room_id: String,
    pub topic_id: String,
    pub channel_id: String,
    pub host_pubkey: String,
    pub title: String,
    pub description: String,
    pub status: GameRoomStatus,
    pub phase_label: Option<String>,
    pub scores: Vec<GameScoreEntry>,
    pub updated_at: i64,
    pub source_replica_id: ReplicaId,
    pub source_key: String,
    pub manifest_blob_hash: BlobHash,
    pub derived_at: i64,
    pub projection_version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorRelationshipProjectionRow {
    pub local_author_pubkey: String,
    pub author_pubkey: String,
    pub following: bool,
    pub followed_by: bool,
    pub mutual: bool,
    pub friend_of_friend: bool,
    pub friend_of_friend_via_pubkeys: Vec<String>,
    pub derived_at: i64,
}

type LivePresenceKey = (String, String, String);
type LivePresenceValue = (String, String, i64, i64);

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
    async fn rebuild_object_projections(&self, rows: Vec<ObjectProjectionRow>) -> Result<()>;
}

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

        sqlx::migrate!("./migrations").run(&pool).await?;

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

        sqlx::migrate!("./migrations").run(&pool).await?;

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

        envelope_page_from_rows(rows)
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

        envelope_page_from_rows(rows)
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
            INSERT INTO profiles (pubkey, name, display_name, about, picture, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(pubkey) DO UPDATE SET
              name = excluded.name,
              display_name = excluded.display_name,
              about = excluded.about,
              picture = excluded.picture,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(profile.pubkey.as_str())
        .bind(profile.name)
        .bind(profile.display_name)
        .bind(profile.about)
        .bind(profile.picture)
        .bind(profile.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_profile(&self, pubkey: &str) -> Result<Option<Profile>> {
        let row = sqlx::query(
            r#"
            SELECT pubkey, name, display_name, about, picture, updated_at
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
    live_presence: Arc<RwLock<HashMap<LivePresenceKey, LivePresenceValue>>>,
    blob_statuses: Arc<RwLock<HashMap<String, BlobCacheStatus>>>,
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
impl ProjectionStore for SqliteStore {
    async fn put_object_projection(&self, row: ObjectProjectionRow) -> Result<()> {
        let payload_json = serde_json::to_string(&row.payload_ref)?;
        sqlx::query(
            r#"
            INSERT INTO object_index_cache (
              object_id, topic_id, channel_id, author_pubkey, created_at, root_object_id,
              reply_to_object_id, payload_ref_json, content, source_replica_id, source_key,
              source_envelope_id, source_blob_hash, derived_at, projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(object_id) DO UPDATE SET
              topic_id = excluded.topic_id,
              channel_id = excluded.channel_id,
              author_pubkey = excluded.author_pubkey,
              created_at = excluded.created_at,
              root_object_id = excluded.root_object_id,
              reply_to_object_id = excluded.reply_to_object_id,
              payload_ref_json = excluded.payload_ref_json,
              content = excluded.content,
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
        .bind(row.root_object_id.as_ref().map(EnvelopeId::as_str))
        .bind(row.reply_to_object_id.as_ref().map(EnvelopeId::as_str))
        .bind(payload_json)
        .bind(row.content.as_deref())
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
            SELECT object_id, topic_id, author_pubkey, created_at, root_object_id, reply_to_object_id,
                   channel_id, payload_ref_json, content, source_replica_id, source_key,
                   source_envelope_id, source_blob_hash, derived_at, projection_version
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
            SELECT object_id, topic_id, author_pubkey, created_at, root_object_id, reply_to_object_id,
                   channel_id, payload_ref_json, content, source_replica_id, source_key,
                   source_envelope_id, source_blob_hash, derived_at, projection_version
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

        object_projection_page_from_rows(rows)
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
            SELECT oic.object_id, oic.topic_id, oic.author_pubkey, oic.created_at, oic.root_object_id,
                   oic.reply_to_object_id, oic.channel_id, oic.payload_ref_json, oic.content,
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

        object_projection_page_from_rows(rows)
    }

    async fn upsert_profile_cache(&self, profile: Profile) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO profile_cache (pubkey, name, display_name, about, picture, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(pubkey) DO UPDATE SET
              name = excluded.name,
              display_name = excluded.display_name,
              about = excluded.about,
              picture = excluded.picture,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(profile.pubkey.as_str())
        .bind(profile.name)
        .bind(profile.display_name)
        .bind(profile.about)
        .bind(profile.picture)
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
                   COALESCE((
                     SELECT COUNT(*)
                     FROM live_presence_cache lpc
                     WHERE lpc.topic_id = lsc.topic_id
                       AND lpc.channel_id = lsc.channel_id
                       AND lpc.session_id = lsc.session_id
                   ), 0) AS viewer_count
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
        sqlx::query(
            r#"
            DELETE FROM author_relationship_cache
            WHERE local_author_pubkey = ?1
            "#,
        )
        .bind(local_author_pubkey)
        .execute(&self.pool)
        .await?;

        for row in rows {
            let via_json = serde_json::to_string(&row.friend_of_friend_via_pubkeys)?;
            sqlx::query(
                r#"
                INSERT INTO author_relationship_cache (
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
            .execute(&self.pool)
            .await?;
        }

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
        for row in rows {
            self.put_object_projection(row).await?;
        }
        Ok(())
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
            row.viewer_count = presence
                .iter()
                .filter(
                    |((presence_channel, session_id, _), (presence_topic, _, _, _))| {
                        presence_channel == &row.channel_id
                            && session_id == &row.session_id
                            && presence_topic == topic_id
                    },
                )
                .count();
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

    async fn rebuild_object_projections(&self, rows: Vec<ObjectProjectionRow>) -> Result<()> {
        let mut guard = self.object_projection_rows.write().await;
        guard.clear();
        for row in rows {
            guard.insert(row.object_id.clone(), row);
        }
        self.live_session_rows.write().await.clear();
        self.game_room_rows.write().await.clear();
        self.live_presence.write().await.clear();
        Ok(())
    }
}

fn row_to_envelope(row: sqlx::sqlite::SqliteRow) -> Result<KukuriEnvelope> {
    Ok(KukuriEnvelope {
        id: row.get::<String, _>("envelope_id").into(),
        pubkey: row.get::<String, _>("pubkey").into(),
        created_at: row.get("created_at"),
        kind: row.get("kind"),
        content: row.get("content"),
        tags: serde_json::from_str(row.get::<String, _>("tags_json").as_str())?,
        sig: row.get("sig"),
    })
}

fn row_to_object_projection(row: sqlx::sqlite::SqliteRow) -> Result<ObjectProjectionRow> {
    Ok(ObjectProjectionRow {
        object_id: row.get::<String, _>("object_id").into(),
        topic_id: row.get("topic_id"),
        channel_id: row.get("channel_id"),
        author_pubkey: row.get("author_pubkey"),
        created_at: row.get("created_at"),
        root_object_id: row
            .try_get::<String, _>("root_object_id")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(EnvelopeId::from),
        reply_to_object_id: row
            .try_get::<String, _>("reply_to_object_id")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(EnvelopeId::from),
        payload_ref: serde_json::from_str(row.get::<String, _>("payload_ref_json").as_str())?,
        content: row.try_get("content").ok(),
        source_replica_id: ReplicaId::new(row.get::<String, _>("source_replica_id")),
        source_key: row.get("source_key"),
        source_envelope_id: row.get::<String, _>("source_envelope_id").into(),
        source_blob_hash: row
            .try_get::<String, _>("source_blob_hash")
            .ok()
            .map(BlobHash::new),
        derived_at: row.get("derived_at"),
        projection_version: row.get("projection_version"),
    })
}

fn row_to_follow_edge(row: sqlx::sqlite::SqliteRow) -> Result<FollowEdge> {
    Ok(FollowEdge {
        subject_pubkey: row.get::<String, _>("subject_pubkey").into(),
        target_pubkey: row.get::<String, _>("target_pubkey").into(),
        status: parse_follow_edge_status(row.get::<String, _>("status").as_str())?,
        updated_at: row.get("updated_at"),
        envelope_id: row.get::<String, _>("source_envelope_id").into(),
    })
}

fn row_to_live_session_projection(
    row: sqlx::sqlite::SqliteRow,
) -> Result<LiveSessionProjectionRow> {
    Ok(LiveSessionProjectionRow {
        session_id: row.get("session_id"),
        topic_id: row.get("topic_id"),
        channel_id: row.get("channel_id"),
        host_pubkey: row.get("host_pubkey"),
        title: row.get("title"),
        description: row.get("description"),
        status: parse_live_status(row.get::<String, _>("status").as_str())?,
        started_at: row.get("started_at"),
        ended_at: row.try_get("ended_at").ok(),
        updated_at: row.get("updated_at"),
        source_replica_id: ReplicaId::new(row.get::<String, _>("source_replica_id")),
        source_key: row.get("source_key"),
        manifest_blob_hash: BlobHash::new(row.get::<String, _>("manifest_blob_hash")),
        derived_at: row.get("derived_at"),
        projection_version: row.get("projection_version"),
        viewer_count: row.get::<i64, _>("viewer_count") as usize,
    })
}

fn row_to_game_room_projection(row: sqlx::sqlite::SqliteRow) -> Result<GameRoomProjectionRow> {
    Ok(GameRoomProjectionRow {
        room_id: row.get("room_id"),
        topic_id: row.get("topic_id"),
        channel_id: row.get("channel_id"),
        host_pubkey: row.get("host_pubkey"),
        title: row.get("title"),
        description: row.get("description"),
        status: parse_game_status(row.get::<String, _>("status").as_str())?,
        phase_label: row.try_get("phase_label").ok(),
        scores: serde_json::from_str(row.get::<String, _>("scores_json").as_str())?,
        updated_at: row.get("updated_at"),
        source_replica_id: ReplicaId::new(row.get::<String, _>("source_replica_id")),
        source_key: row.get("source_key"),
        manifest_blob_hash: BlobHash::new(row.get::<String, _>("manifest_blob_hash")),
        derived_at: row.get("derived_at"),
        projection_version: row.get("projection_version"),
    })
}

fn row_to_author_relationship_projection(
    row: sqlx::sqlite::SqliteRow,
) -> Result<AuthorRelationshipProjectionRow> {
    Ok(AuthorRelationshipProjectionRow {
        local_author_pubkey: row.get("local_author_pubkey"),
        author_pubkey: row.get("author_pubkey"),
        following: row.get("following"),
        followed_by: row.get("followed_by"),
        mutual: row.get("mutual"),
        friend_of_friend: row.get("friend_of_friend"),
        friend_of_friend_via_pubkeys: serde_json::from_str(
            row.get::<String, _>("friend_of_friend_via_pubkeys_json")
                .as_str(),
        )?,
        derived_at: row.get("derived_at"),
    })
}

fn follow_edge_status_name(status: &FollowEdgeStatus) -> &'static str {
    match status {
        FollowEdgeStatus::Active => "active",
        FollowEdgeStatus::Revoked => "revoked",
    }
}

fn parse_follow_edge_status(value: &str) -> Result<FollowEdgeStatus> {
    match value {
        "active" => Ok(FollowEdgeStatus::Active),
        "revoked" => Ok(FollowEdgeStatus::Revoked),
        _ => anyhow::bail!("unknown follow edge status: {value}"),
    }
}

fn live_status_name(status: &LiveSessionStatus) -> &'static str {
    match status {
        LiveSessionStatus::Scheduled => "scheduled",
        LiveSessionStatus::Live => "live",
        LiveSessionStatus::Paused => "paused",
        LiveSessionStatus::Ended => "ended",
    }
}

fn parse_live_status(value: &str) -> Result<LiveSessionStatus> {
    match value {
        "scheduled" => Ok(LiveSessionStatus::Scheduled),
        "live" => Ok(LiveSessionStatus::Live),
        "paused" => Ok(LiveSessionStatus::Paused),
        "ended" => Ok(LiveSessionStatus::Ended),
        _ => anyhow::bail!("unknown live session status: {value}"),
    }
}

fn game_status_name(status: &GameRoomStatus) -> &'static str {
    match status {
        GameRoomStatus::Waiting => "waiting",
        GameRoomStatus::Running => "running",
        GameRoomStatus::Paused => "paused",
        GameRoomStatus::Ended => "ended",
    }
}

fn parse_game_status(value: &str) -> Result<GameRoomStatus> {
    match value {
        "open" | "waiting" => Ok(GameRoomStatus::Waiting),
        "in_progress" | "running" => Ok(GameRoomStatus::Running),
        "paused" => Ok(GameRoomStatus::Paused),
        "finished" | "ended" => Ok(GameRoomStatus::Ended),
        _ => anyhow::bail!("unknown game room status: {value}"),
    }
}

fn envelope_page_from_rows(rows: Vec<sqlx::sqlite::SqliteRow>) -> Result<Page<KukuriEnvelope>> {
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(row_to_envelope(row)?);
    }
    let next_cursor = items.last().map(|envelope| TimelineCursor {
        created_at: envelope.created_at,
        object_id: envelope.id.clone(),
    });
    Ok(Page { items, next_cursor })
}

fn object_projection_page_from_rows(
    rows: Vec<sqlx::sqlite::SqliteRow>,
) -> Result<Page<ObjectProjectionRow>> {
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(row_to_object_projection(row)?);
    }
    let next_cursor = items.last().map(|row| TimelineCursor {
        created_at: row.created_at,
        object_id: row.object_id.clone(),
    });
    Ok(Page { items, next_cursor })
}

fn apply_desc_cursor(
    items: Vec<KukuriEnvelope>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<KukuriEnvelope> {
    let mut filtered = items
        .into_iter()
        .filter(|envelope| {
            cursor.as_ref().is_none_or(|cursor| {
                envelope.created_at < cursor.created_at
                    || (envelope.created_at == cursor.created_at && envelope.id < cursor.object_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|envelope| TimelineCursor {
        created_at: envelope.created_at,
        object_id: envelope.id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

fn apply_asc_cursor(
    items: Vec<KukuriEnvelope>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<KukuriEnvelope> {
    let mut filtered = items
        .into_iter()
        .filter(|envelope| {
            cursor.as_ref().is_none_or(|cursor| {
                envelope.created_at > cursor.created_at
                    || (envelope.created_at == cursor.created_at && envelope.id > cursor.object_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|envelope| TimelineCursor {
        created_at: envelope.created_at,
        object_id: envelope.id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

fn apply_desc_projection_cursor(
    items: Vec<ObjectProjectionRow>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<ObjectProjectionRow> {
    let mut filtered = items
        .into_iter()
        .filter(|row| {
            cursor.as_ref().is_none_or(|cursor| {
                row.created_at < cursor.created_at
                    || (row.created_at == cursor.created_at && row.object_id < cursor.object_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|row| TimelineCursor {
        created_at: row.created_at,
        object_id: row.object_id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

fn apply_asc_projection_cursor(
    items: Vec<ObjectProjectionRow>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<ObjectProjectionRow> {
    let mut filtered = items
        .into_iter()
        .filter(|row| {
            cursor.as_ref().is_none_or(|cursor| {
                row.created_at > cursor.created_at
                    || (row.created_at == cursor.created_at && row.object_id > cursor.object_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|row| TimelineCursor {
        created_at: row.created_at,
        object_id: row.object_id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kukuri_core::{
        BlobHash, FollowEdgeStatus, PayloadRef, ReplicaId, TopicId, build_follow_edge_envelope,
        build_post_envelope, generate_keys,
    };

    #[tokio::test]
    async fn store_timeline_cursor_stable() {
        let store = SqliteStore::connect_memory().await.expect("sqlite store");
        let topic = TopicId::new("kukuri:topic:timeline");
        let keys = generate_keys();

        let first = build_post_envelope(&keys, &topic, "one", None).expect("first");
        let second = build_post_envelope(&keys, &topic, "two", None).expect("second");
        let third = build_post_envelope(&keys, &topic, "three", None).expect("third");
        store
            .put_envelope(first.clone())
            .await
            .expect("insert first");
        store
            .put_envelope(second.clone())
            .await
            .expect("insert second");
        store
            .put_envelope(third.clone())
            .await
            .expect("insert third");

        let first_page = Store::list_topic_timeline(&store, topic.as_str(), None, 2)
            .await
            .expect("timeline page");
        let cursor = first_page.next_cursor.clone().expect("cursor");
        let second_page = Store::list_topic_timeline(&store, topic.as_str(), Some(cursor), 2)
            .await
            .expect("second page");

        assert_eq!(first_page.items.len(), 2);
        assert!(first_page.items[0].created_at >= first_page.items[1].created_at);
        assert!(second_page.items.len() <= 1);
        assert!(second_page.items.iter().all(|event| {
            !first_page
                .items
                .iter()
                .any(|existing| existing.id == event.id)
        }));
    }

    #[tokio::test]
    async fn store_thread_materialization() {
        let store = SqliteStore::connect_memory().await.expect("sqlite store");
        let topic = TopicId::new("kukuri:topic:thread");
        let keys = generate_keys();

        let root = build_post_envelope(&keys, &topic, "root", None).expect("root");
        let reply = build_post_envelope(&keys, &topic, "reply", Some(&root)).expect("reply");
        store.put_envelope(root.clone()).await.expect("insert root");
        store
            .put_envelope(reply.clone())
            .await
            .expect("insert reply");

        let thread = Store::list_thread(&store, topic.as_str(), &root.id, None, 10)
            .await
            .expect("thread");

        assert_eq!(thread.items.len(), 2);
        assert_eq!(thread.items[0].id, root.id);
        assert_eq!(thread.items[1].id, reply.id);
    }

    #[tokio::test]
    async fn store_profile_upsert_latest_wins() {
        let store = SqliteStore::connect_memory().await.expect("sqlite store");
        let pubkey = "f".repeat(64);

        store
            .upsert_profile(Profile {
                pubkey: pubkey.as_str().into(),
                name: Some("older".into()),
                display_name: Some("older".into()),
                about: None,
                picture: None,
                updated_at: 10,
            })
            .await
            .expect("insert older");
        store
            .upsert_profile(Profile {
                pubkey: pubkey.as_str().into(),
                name: Some("newer".into()),
                display_name: Some("newer".into()),
                about: None,
                picture: None,
                updated_at: 20,
            })
            .await
            .expect("insert newer");

        let profile = store
            .get_profile(pubkey.as_str())
            .await
            .expect("load profile")
            .expect("profile");
        assert_eq!(profile.name.as_deref(), Some("newer"));
        assert_eq!(profile.display_name.as_deref(), Some("newer"));
    }

    #[tokio::test]
    async fn projection_rebuild_from_docs_blobs_only() {
        let store = SqliteStore::connect_memory().await.expect("sqlite store");
        let topic = "kukuri:topic:projection";
        let root_id = EnvelopeId::from("object-root");
        let reply_id = EnvelopeId::from("object-reply");
        let rows = vec![
            ObjectProjectionRow {
                object_id: root_id.clone(),
                topic_id: topic.to_string(),
                channel_id: "public".into(),
                author_pubkey: "a".repeat(64),
                created_at: 10,
                root_object_id: None,
                reply_to_object_id: None,
                payload_ref: PayloadRef::BlobText {
                    hash: BlobHash::new("1".repeat(64)),
                    mime: "text/plain".into(),
                    bytes: 4,
                },
                content: Some("root".into()),
                source_replica_id: ReplicaId::new(format!("topic::{topic}")),
                source_key: "objects/object-root/header".into(),
                source_envelope_id: root_id.clone(),
                source_blob_hash: Some(BlobHash::new("1".repeat(64))),
                derived_at: 10,
                projection_version: 1,
            },
            ObjectProjectionRow {
                object_id: reply_id.clone(),
                topic_id: topic.to_string(),
                channel_id: "public".into(),
                author_pubkey: "b".repeat(64),
                created_at: 11,
                root_object_id: Some(root_id.clone()),
                reply_to_object_id: Some(root_id.clone()),
                payload_ref: PayloadRef::BlobText {
                    hash: BlobHash::new("2".repeat(64)),
                    mime: "text/plain".into(),
                    bytes: 5,
                },
                content: Some("reply".into()),
                source_replica_id: ReplicaId::new(format!("topic::{topic}")),
                source_key: "objects/object-reply/header".into(),
                source_envelope_id: reply_id.clone(),
                source_blob_hash: Some(BlobHash::new("2".repeat(64))),
                derived_at: 11,
                projection_version: 1,
            },
        ];

        ProjectionStore::rebuild_object_projections(&store, rows)
            .await
            .expect("rebuild projection");

        let timeline = ProjectionStore::list_topic_timeline(&store, topic, None, 10)
            .await
            .expect("timeline");
        let thread = ProjectionStore::list_thread(&store, topic, &root_id, None, 10)
            .await
            .expect("thread");

        assert_eq!(timeline.items.len(), 2);
        assert_eq!(timeline.items[0].object_id, reply_id);
        assert_eq!(thread.items.len(), 2);
        assert_eq!(thread.items[0].object_id, root_id);
    }

    #[tokio::test]
    async fn store_follow_edge_latest_wins() {
        let store = SqliteStore::connect_memory().await.expect("sqlite store");
        let subject_keys = generate_keys();
        let target_keys = generate_keys();
        let active = build_follow_edge_envelope(
            &subject_keys,
            &target_keys.public_key(),
            FollowEdgeStatus::Active,
        )
        .expect("active edge");
        let mut revoked = build_follow_edge_envelope(
            &subject_keys,
            &target_keys.public_key(),
            FollowEdgeStatus::Revoked,
        )
        .expect("revoked edge");
        revoked.created_at = active.created_at + 1;

        store
            .put_envelope(active.clone())
            .await
            .expect("insert active edge");
        store
            .put_envelope(revoked.clone())
            .await
            .expect("insert revoked edge");
        store
            .put_envelope(active)
            .await
            .expect("reinsert older edge");

        let edges = store
            .list_follow_edges_by_subject(subject_keys.public_key_hex().as_str())
            .await
            .expect("list edges");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].status, FollowEdgeStatus::Revoked);
    }

    #[tokio::test]
    async fn author_relationship_projection_rebuild_roundtrip() {
        let store = SqliteStore::connect_memory().await.expect("sqlite store");
        let local_author = "a".repeat(64);
        let target_author = "b".repeat(64);

        ProjectionStore::rebuild_author_relationships(
            &store,
            local_author.as_str(),
            vec![AuthorRelationshipProjectionRow {
                local_author_pubkey: local_author.clone(),
                author_pubkey: target_author.clone(),
                following: false,
                followed_by: true,
                mutual: false,
                friend_of_friend: true,
                friend_of_friend_via_pubkeys: vec!["c".repeat(64)],
                derived_at: 12,
            }],
        )
        .await
        .expect("rebuild relationships");

        let relationship = ProjectionStore::get_author_relationship(
            &store,
            local_author.as_str(),
            target_author.as_str(),
        )
        .await
        .expect("get relationship")
        .expect("relationship");
        assert!(relationship.friend_of_friend);
        assert_eq!(
            relationship.friend_of_friend_via_pubkeys,
            vec!["c".repeat(64)]
        );
        assert!(relationship.followed_by);
    }
}
