use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use kukuri_core::{
    BlobHash, Event, EventId, GameRoomStatus, GameScoreEntry, LiveSessionStatus, PayloadRef,
    Profile, ReplicaId, ThreadRef, parse_profile,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineCursor {
    pub created_at: i64,
    pub event_id: EventId,
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
pub struct EventProjectionRow {
    pub event_id: EventId,
    pub topic_id: String,
    pub author_pubkey: String,
    pub created_at: i64,
    pub root_id: Option<EventId>,
    pub reply_to: Option<EventId>,
    pub payload_ref: PayloadRef,
    pub content: Option<String>,
    pub source_replica_id: ReplicaId,
    pub source_key: String,
    pub source_event_id: EventId,
    pub source_blob_hash: Option<BlobHash>,
    pub derived_at: i64,
    pub projection_version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSessionProjectionRow {
    pub session_id: String,
    pub topic_id: String,
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

type LivePresenceKey = (String, String);
type LivePresenceValue = (String, i64, i64);

#[async_trait]
pub trait Store: Send + Sync {
    async fn put_event(&self, event: Event) -> Result<()>;
    async fn get_event(&self, event_id: &EventId) -> Result<Option<Event>>;
    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<Event>>;
    async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &EventId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<Event>>;
    async fn upsert_profile(&self, profile: Profile) -> Result<()>;
    async fn get_profile(&self, pubkey: &str) -> Result<Option<Profile>>;
}

#[async_trait]
pub trait ProjectionStore: Send + Sync {
    async fn put_projection_row(&self, row: EventProjectionRow) -> Result<()>;
    async fn get_event_projection(&self, event_id: &EventId) -> Result<Option<EventProjectionRow>>;
    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<EventProjectionRow>>;
    async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &EventId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<EventProjectionRow>>;
    async fn upsert_profile_cache(&self, profile: Profile) -> Result<()>;
    async fn upsert_live_session_cache(&self, row: LiveSessionProjectionRow) -> Result<()>;
    async fn list_topic_live_sessions(
        &self,
        topic_id: &str,
    ) -> Result<Vec<LiveSessionProjectionRow>>;
    async fn upsert_game_room_cache(&self, row: GameRoomProjectionRow) -> Result<()>;
    async fn list_topic_game_rooms(&self, topic_id: &str) -> Result<Vec<GameRoomProjectionRow>>;
    async fn upsert_live_presence(
        &self,
        topic_id: &str,
        session_id: &str,
        author_pubkey: &str,
        expires_at: i64,
        updated_at: i64,
    ) -> Result<()>;
    async fn clear_topic_live_presence(&self, topic_id: &str) -> Result<()>;
    async fn clear_expired_live_presence(&self, now_ms: i64) -> Result<()>;
    async fn mark_blob_status(&self, hash: &BlobHash, status: BlobCacheStatus) -> Result<()>;
    async fn rebuild_from_docs_blobs(&self, rows: Vec<EventProjectionRow>) -> Result<()>;
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
    async fn put_event(&self, event: Event) -> Result<()> {
        let tags_json = serde_json::to_string(&event.tags)?;

        sqlx::query(
            r#"
            INSERT INTO events (event_id, pubkey, created_at, kind, content, tags_json, sig)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(event_id) DO UPDATE SET
              pubkey = excluded.pubkey,
              created_at = excluded.created_at,
              kind = excluded.kind,
              content = excluded.content,
              tags_json = excluded.tags_json,
              sig = excluded.sig
            "#,
        )
        .bind(event.id.as_str())
        .bind(event.pubkey.as_str())
        .bind(event.created_at)
        .bind(i64::from(event.kind))
        .bind(event.content.as_str())
        .bind(tags_json)
        .bind(event.sig.as_str())
        .execute(&self.pool)
        .await?;

        if let Some(topic_id) = event.topic_id() {
            sqlx::query(
                r#"
                INSERT INTO topic_posts (topic_id, event_id, created_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(topic_id, event_id) DO UPDATE SET created_at = excluded.created_at
                "#,
            )
            .bind(topic_id.as_str())
            .bind(event.id.as_str())
            .bind(event.created_at)
            .execute(&self.pool)
            .await?;

            let thread_ref = event.thread_ref().unwrap_or(ThreadRef {
                root: event.id.clone(),
                reply_to: None,
            });
            sqlx::query(
                r#"
                INSERT INTO thread_edges (topic_id, event_id, root_event_id, parent_event_id, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(event_id) DO UPDATE SET
                  topic_id = excluded.topic_id,
                  root_event_id = excluded.root_event_id,
                  parent_event_id = excluded.parent_event_id,
                  created_at = excluded.created_at
                "#,
            )
            .bind(topic_id.as_str())
            .bind(event.id.as_str())
            .bind(thread_ref.root.as_str())
            .bind(thread_ref.reply_to.as_ref().map(EventId::as_str))
            .bind(event.created_at)
            .execute(&self.pool)
            .await?;
        }

        if let Some(profile) = parse_profile(&event)? {
            self.upsert_profile(profile).await?;
        }

        Ok(())
    }

    async fn get_event(&self, event_id: &EventId) -> Result<Option<Event>> {
        let row = sqlx::query(
            r#"
            SELECT event_id, pubkey, created_at, kind, content, tags_json, sig
            FROM events
            WHERE event_id = ?1
            "#,
        )
        .bind(event_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_event).transpose()
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT e.event_id, e.pubkey, e.created_at, e.kind, e.content, e.tags_json, e.sig
            FROM topic_posts tp
            INNER JOIN events e ON e.event_id = tp.event_id
            WHERE tp.topic_id = ?1
              AND (
                ?2 IS NULL
                OR e.created_at < ?2
                OR (e.created_at = ?2 AND e.event_id < ?3)
              )
            ORDER BY e.created_at DESC, e.event_id DESC
            LIMIT ?4
            "#,
        )
        .bind(topic_id)
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.event_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        page_from_rows(rows)
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &EventId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<Event>> {
        let rows = sqlx::query(
            r#"
            SELECT e.event_id, e.pubkey, e.created_at, e.kind, e.content, e.tags_json, e.sig
            FROM thread_edges te
            INNER JOIN events e ON e.event_id = te.event_id
            WHERE te.topic_id = ?1
              AND te.root_event_id = ?2
              AND (
                ?3 IS NULL
                OR e.created_at > ?3
                OR (e.created_at = ?3 AND e.event_id > ?4)
              )
            ORDER BY
              CASE WHEN e.event_id = te.root_event_id THEN 0 ELSE 1 END ASC,
              e.created_at ASC,
              e.event_id ASC
            LIMIT ?5
            "#,
        )
        .bind(topic_id)
        .bind(thread_id.as_str())
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.event_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        page_from_rows(rows)
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
}

#[derive(Clone, Default)]
pub struct MemoryStore {
    events: Arc<RwLock<HashMap<EventId, Event>>>,
    topic_posts: Arc<RwLock<HashMap<String, Vec<EventId>>>>,
    thread_edges: Arc<RwLock<HashMap<String, BTreeMap<String, EventId>>>>,
    profiles: Arc<RwLock<HashMap<String, Profile>>>,
    projection_rows: Arc<RwLock<HashMap<EventId, EventProjectionRow>>>,
    live_session_rows: Arc<RwLock<HashMap<String, LiveSessionProjectionRow>>>,
    game_room_rows: Arc<RwLock<HashMap<String, GameRoomProjectionRow>>>,
    live_presence: Arc<RwLock<HashMap<LivePresenceKey, LivePresenceValue>>>,
    blob_statuses: Arc<RwLock<HashMap<String, BlobCacheStatus>>>,
}

#[async_trait]
impl Store for MemoryStore {
    async fn put_event(&self, event: Event) -> Result<()> {
        let topic_id = event.topic_id().map(|topic| topic.0);
        let thread_ref = event.thread_ref();
        self.events
            .write()
            .await
            .insert(event.id.clone(), event.clone());

        if let Some(topic_id) = topic_id {
            self.topic_posts
                .write()
                .await
                .entry(topic_id.clone())
                .or_default()
                .push(event.id.clone());

            let root = thread_ref
                .as_ref()
                .map(|thread| thread.root.clone())
                .unwrap_or_else(|| event.id.clone());
            self.thread_edges
                .write()
                .await
                .entry(topic_id)
                .or_default()
                .insert(event.id.0.clone(), root);
        }

        if let Some(profile) = parse_profile(&event)? {
            self.upsert_profile(profile).await?;
        }

        Ok(())
    }

    async fn get_event(&self, event_id: &EventId) -> Result<Option<Event>> {
        Ok(self.events.read().await.get(event_id).cloned())
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<Event>> {
        let events = self.events.read().await;
        let mut items = self
            .topic_posts
            .read()
            .await
            .get(topic_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|event_id| events.get(&event_id).cloned())
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
        thread_id: &EventId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<Event>> {
        let events = self.events.read().await;
        let roots = self.thread_edges.read().await;
        let mut items = roots
            .get(topic_id)
            .into_iter()
            .flat_map(|entries| entries.keys())
            .filter_map(|event_id| events.get(&EventId::from(event_id.as_str())).cloned())
            .filter(|event| {
                event.id == *thread_id
                    || event
                        .thread_ref()
                        .map(|thread| thread.root == *thread_id)
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let left_root = left.id == *thread_id;
            let right_root = right.id == *thread_id;
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
}

#[async_trait]
impl ProjectionStore for SqliteStore {
    async fn put_projection_row(&self, row: EventProjectionRow) -> Result<()> {
        let payload_json = serde_json::to_string(&row.payload_ref)?;
        sqlx::query(
            r#"
            INSERT INTO topic_index_cache (
              event_id, topic_id, author_pubkey, created_at, root_event_id, reply_to_event_id,
              payload_ref_json, content, source_replica_id, source_key, source_event_id,
              source_blob_hash, derived_at, projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(event_id) DO UPDATE SET
              topic_id = excluded.topic_id,
              author_pubkey = excluded.author_pubkey,
              created_at = excluded.created_at,
              root_event_id = excluded.root_event_id,
              reply_to_event_id = excluded.reply_to_event_id,
              payload_ref_json = excluded.payload_ref_json,
              content = excluded.content,
              source_replica_id = excluded.source_replica_id,
              source_key = excluded.source_key,
              source_event_id = excluded.source_event_id,
              source_blob_hash = excluded.source_blob_hash,
              derived_at = excluded.derived_at,
              projection_version = excluded.projection_version
            "#,
        )
        .bind(row.event_id.as_str())
        .bind(row.topic_id.as_str())
        .bind(row.author_pubkey.as_str())
        .bind(row.created_at)
        .bind(row.root_id.as_ref().map(EventId::as_str))
        .bind(row.reply_to.as_ref().map(EventId::as_str))
        .bind(payload_json)
        .bind(row.content.as_deref())
        .bind(row.source_replica_id.as_str())
        .bind(row.source_key.as_str())
        .bind(row.source_event_id.as_str())
        .bind(row.source_blob_hash.as_ref().map(BlobHash::as_str))
        .bind(row.derived_at)
        .bind(row.projection_version)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO thread_cache (
              event_id, topic_id, root_event_id, created_at
            )
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(event_id) DO UPDATE SET
              topic_id = excluded.topic_id,
              root_event_id = excluded.root_event_id,
              created_at = excluded.created_at
            "#,
        )
        .bind(row.event_id.as_str())
        .bind(row.topic_id.as_str())
        .bind(row.root_id.as_ref().unwrap_or(&row.event_id).as_str())
        .bind(row.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_event_projection(&self, event_id: &EventId) -> Result<Option<EventProjectionRow>> {
        let row = sqlx::query(
            r#"
            SELECT event_id, topic_id, author_pubkey, created_at, root_event_id, reply_to_event_id,
                   payload_ref_json, content, source_replica_id, source_key, source_event_id,
                   source_blob_hash, derived_at, projection_version
            FROM topic_index_cache
            WHERE event_id = ?1
            "#,
        )
        .bind(event_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_projection).transpose()
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<EventProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, topic_id, author_pubkey, created_at, root_event_id, reply_to_event_id,
                   payload_ref_json, content, source_replica_id, source_key, source_event_id,
                   source_blob_hash, derived_at, projection_version
            FROM topic_index_cache
            WHERE topic_id = ?1
              AND (
                ?2 IS NULL
                OR created_at < ?2
                OR (created_at = ?2 AND event_id < ?3)
              )
            ORDER BY created_at DESC, event_id DESC
            LIMIT ?4
            "#,
        )
        .bind(topic_id)
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.event_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        projection_page_from_rows(rows)
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &EventId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<EventProjectionRow>> {
        let rows = sqlx::query(
            r#"
            SELECT tic.event_id, tic.topic_id, tic.author_pubkey, tic.created_at, tic.root_event_id,
                   tic.reply_to_event_id, tic.payload_ref_json, tic.content, tic.source_replica_id,
                   tic.source_key, tic.source_event_id, tic.source_blob_hash, tic.derived_at,
                   tic.projection_version
            FROM thread_cache tc
            INNER JOIN topic_index_cache tic ON tic.event_id = tc.event_id
            WHERE tc.topic_id = ?1
              AND tc.root_event_id = ?2
              AND (
                ?3 IS NULL
                OR tic.created_at > ?3
                OR (tic.created_at = ?3 AND tic.event_id > ?4)
              )
            ORDER BY
              CASE WHEN tic.event_id = tc.root_event_id THEN 0 ELSE 1 END ASC,
              tic.created_at ASC,
              tic.event_id ASC
            LIMIT ?5
            "#,
        )
        .bind(topic_id)
        .bind(thread_id.as_str())
        .bind(cursor.as_ref().map(|value| value.created_at))
        .bind(cursor.as_ref().map(|value| value.event_id.as_str()))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        projection_page_from_rows(rows)
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
              session_id, topic_id, host_pubkey, title, description, status, started_at, ended_at,
              updated_at, source_replica_id, source_key, manifest_blob_hash, derived_at,
              projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(session_id) DO UPDATE SET
              topic_id = excluded.topic_id,
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
                   lsc.status, lsc.started_at, lsc.ended_at, lsc.updated_at, lsc.source_replica_id,
                   lsc.source_key, lsc.manifest_blob_hash, lsc.derived_at, lsc.projection_version,
                   COALESCE((
                     SELECT COUNT(*)
                     FROM live_presence_cache lpc
                     WHERE lpc.topic_id = lsc.topic_id
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
              room_id, topic_id, host_pubkey, title, description, status, phase_label,
              scores_json, updated_at, source_replica_id, source_key, manifest_blob_hash,
              derived_at, projection_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(room_id) DO UPDATE SET
              topic_id = excluded.topic_id,
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
                   scores_json, updated_at, source_replica_id, source_key, manifest_blob_hash,
                   derived_at, projection_version
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

    async fn upsert_live_presence(
        &self,
        topic_id: &str,
        session_id: &str,
        author_pubkey: &str,
        expires_at: i64,
        updated_at: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO live_presence_cache (
              topic_id, session_id, author_pubkey, expires_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(topic_id, session_id, author_pubkey) DO UPDATE SET
              expires_at = excluded.expires_at,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(topic_id)
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

    async fn rebuild_from_docs_blobs(&self, rows: Vec<EventProjectionRow>) -> Result<()> {
        sqlx::query("DELETE FROM thread_cache")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM topic_index_cache")
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
            self.put_projection_row(row).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ProjectionStore for MemoryStore {
    async fn put_projection_row(&self, row: EventProjectionRow) -> Result<()> {
        self.projection_rows
            .write()
            .await
            .insert(row.event_id.clone(), row);
        Ok(())
    }

    async fn get_event_projection(&self, event_id: &EventId) -> Result<Option<EventProjectionRow>> {
        Ok(self.projection_rows.read().await.get(event_id).cloned())
    }

    async fn list_topic_timeline(
        &self,
        topic_id: &str,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<EventProjectionRow>> {
        let mut items = self
            .projection_rows
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
                .then_with(|| right.event_id.cmp(&left.event_id))
        });
        Ok(apply_desc_projection_cursor(items, cursor, limit))
    }

    async fn list_thread(
        &self,
        topic_id: &str,
        thread_id: &EventId,
        cursor: Option<TimelineCursor>,
        limit: usize,
    ) -> Result<Page<EventProjectionRow>> {
        let mut items = self
            .projection_rows
            .read()
            .await
            .values()
            .filter(|row| {
                row.topic_id == topic_id
                    && (row.event_id == *thread_id
                        || row.root_id.as_ref().is_some_and(|root| root == thread_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            let left_root = left.event_id == *thread_id;
            let right_root = right.event_id == *thread_id;
            left_root
                .cmp(&right_root)
                .reverse()
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.event_id.cmp(&right.event_id))
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
                .filter(|((session_id, _), (presence_topic, _, _))| {
                    session_id == &row.session_id && presence_topic == topic_id
                })
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

    async fn upsert_live_presence(
        &self,
        topic_id: &str,
        session_id: &str,
        author_pubkey: &str,
        expires_at: i64,
        updated_at: i64,
    ) -> Result<()> {
        self.live_presence.write().await.insert(
            (session_id.to_string(), author_pubkey.to_string()),
            (topic_id.to_string(), expires_at, updated_at),
        );
        Ok(())
    }

    async fn clear_expired_live_presence(&self, now_ms: i64) -> Result<()> {
        self.live_presence
            .write()
            .await
            .retain(|_, (_, expires_at, _)| *expires_at > now_ms);
        Ok(())
    }

    async fn clear_topic_live_presence(&self, topic_id: &str) -> Result<()> {
        self.live_presence
            .write()
            .await
            .retain(|_, (presence_topic, _, _)| presence_topic != topic_id);
        Ok(())
    }

    async fn mark_blob_status(&self, hash: &BlobHash, status: BlobCacheStatus) -> Result<()> {
        self.blob_statuses
            .write()
            .await
            .insert(hash.as_str().to_string(), status);
        Ok(())
    }

    async fn rebuild_from_docs_blobs(&self, rows: Vec<EventProjectionRow>) -> Result<()> {
        let mut guard = self.projection_rows.write().await;
        guard.clear();
        for row in rows {
            guard.insert(row.event_id.clone(), row);
        }
        self.live_session_rows.write().await.clear();
        self.game_room_rows.write().await.clear();
        self.live_presence.write().await.clear();
        Ok(())
    }
}

fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<Event> {
    Ok(Event {
        id: row.get::<String, _>("event_id").into(),
        pubkey: row.get::<String, _>("pubkey").into(),
        created_at: row.get("created_at"),
        kind: row.get::<i64, _>("kind") as u16,
        content: row.get("content"),
        tags: serde_json::from_str(row.get::<String, _>("tags_json").as_str())?,
        sig: row.get("sig"),
    })
}

fn row_to_projection(row: sqlx::sqlite::SqliteRow) -> Result<EventProjectionRow> {
    Ok(EventProjectionRow {
        event_id: row.get::<String, _>("event_id").into(),
        topic_id: row.get("topic_id"),
        author_pubkey: row.get("author_pubkey"),
        created_at: row.get("created_at"),
        root_id: row
            .try_get::<String, _>("root_event_id")
            .ok()
            .map(EventId::from),
        reply_to: row
            .try_get::<String, _>("reply_to_event_id")
            .ok()
            .map(EventId::from),
        payload_ref: serde_json::from_str(row.get::<String, _>("payload_ref_json").as_str())?,
        content: row.try_get("content").ok(),
        source_replica_id: ReplicaId::new(row.get::<String, _>("source_replica_id")),
        source_key: row.get("source_key"),
        source_event_id: row.get::<String, _>("source_event_id").into(),
        source_blob_hash: row
            .try_get::<String, _>("source_blob_hash")
            .ok()
            .map(BlobHash::new),
        derived_at: row.get("derived_at"),
        projection_version: row.get("projection_version"),
    })
}

fn row_to_live_session_projection(
    row: sqlx::sqlite::SqliteRow,
) -> Result<LiveSessionProjectionRow> {
    Ok(LiveSessionProjectionRow {
        session_id: row.get("session_id"),
        topic_id: row.get("topic_id"),
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

fn live_status_name(status: &LiveSessionStatus) -> &'static str {
    match status {
        LiveSessionStatus::Live => "live",
        LiveSessionStatus::Ended => "ended",
    }
}

fn parse_live_status(value: &str) -> Result<LiveSessionStatus> {
    match value {
        "live" => Ok(LiveSessionStatus::Live),
        "ended" => Ok(LiveSessionStatus::Ended),
        _ => anyhow::bail!("unknown live session status: {value}"),
    }
}

fn game_status_name(status: &GameRoomStatus) -> &'static str {
    match status {
        GameRoomStatus::Open => "open",
        GameRoomStatus::InProgress => "in_progress",
        GameRoomStatus::Finished => "finished",
    }
}

fn parse_game_status(value: &str) -> Result<GameRoomStatus> {
    match value {
        "open" => Ok(GameRoomStatus::Open),
        "in_progress" => Ok(GameRoomStatus::InProgress),
        "finished" => Ok(GameRoomStatus::Finished),
        _ => anyhow::bail!("unknown game room status: {value}"),
    }
}

fn page_from_rows(rows: Vec<sqlx::sqlite::SqliteRow>) -> Result<Page<Event>> {
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(row_to_event(row)?);
    }
    let next_cursor = items.last().map(|event| TimelineCursor {
        created_at: event.created_at,
        event_id: event.id.clone(),
    });
    Ok(Page { items, next_cursor })
}

fn projection_page_from_rows(
    rows: Vec<sqlx::sqlite::SqliteRow>,
) -> Result<Page<EventProjectionRow>> {
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(row_to_projection(row)?);
    }
    let next_cursor = items.last().map(|event| TimelineCursor {
        created_at: event.created_at,
        event_id: event.event_id.clone(),
    });
    Ok(Page { items, next_cursor })
}

fn apply_desc_cursor(
    items: Vec<Event>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<Event> {
    let mut filtered = items
        .into_iter()
        .filter(|event| {
            cursor.as_ref().is_none_or(|cursor| {
                event.created_at < cursor.created_at
                    || (event.created_at == cursor.created_at && event.id < cursor.event_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|event| TimelineCursor {
        created_at: event.created_at,
        event_id: event.id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

fn apply_asc_cursor(
    items: Vec<Event>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<Event> {
    let mut filtered = items
        .into_iter()
        .filter(|event| {
            cursor.as_ref().is_none_or(|cursor| {
                event.created_at > cursor.created_at
                    || (event.created_at == cursor.created_at && event.id > cursor.event_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|event| TimelineCursor {
        created_at: event.created_at,
        event_id: event.id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

fn apply_desc_projection_cursor(
    items: Vec<EventProjectionRow>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<EventProjectionRow> {
    let mut filtered = items
        .into_iter()
        .filter(|event| {
            cursor.as_ref().is_none_or(|cursor| {
                event.created_at < cursor.created_at
                    || (event.created_at == cursor.created_at && event.event_id < cursor.event_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|event| TimelineCursor {
        created_at: event.created_at,
        event_id: event.event_id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

fn apply_asc_projection_cursor(
    items: Vec<EventProjectionRow>,
    cursor: Option<TimelineCursor>,
    limit: usize,
) -> Page<EventProjectionRow> {
    let mut filtered = items
        .into_iter()
        .filter(|event| {
            cursor.as_ref().is_none_or(|cursor| {
                event.created_at > cursor.created_at
                    || (event.created_at == cursor.created_at && event.event_id > cursor.event_id)
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let next_cursor = filtered.last().map(|event| TimelineCursor {
        created_at: event.created_at,
        event_id: event.event_id.clone(),
    });
    Page {
        items: std::mem::take(&mut filtered),
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kukuri_core::{BlobHash, PayloadRef, ReplicaId, TopicId, build_text_note, generate_keys};

    #[tokio::test]
    async fn store_timeline_cursor_stable() {
        let store = SqliteStore::connect_memory().await.expect("sqlite store");
        let topic = TopicId::new("kukuri:topic:timeline");
        let keys = generate_keys();

        let first = build_text_note(&keys, &topic, "one", None).expect("first");
        let second = build_text_note(&keys, &topic, "two", None).expect("second");
        let third = build_text_note(&keys, &topic, "three", None).expect("third");
        store.put_event(first.clone()).await.expect("insert first");
        store
            .put_event(second.clone())
            .await
            .expect("insert second");
        store.put_event(third.clone()).await.expect("insert third");

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

        let root = build_text_note(&keys, &topic, "root", None).expect("root");
        let reply = build_text_note(&keys, &topic, "reply", Some(&root)).expect("reply");
        store.put_event(root.clone()).await.expect("insert root");
        store.put_event(reply.clone()).await.expect("insert reply");

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
        let root_id = EventId::from("event-root");
        let reply_id = EventId::from("event-reply");
        let rows = vec![
            EventProjectionRow {
                event_id: root_id.clone(),
                topic_id: topic.to_string(),
                author_pubkey: "a".repeat(64),
                created_at: 10,
                root_id: None,
                reply_to: None,
                payload_ref: PayloadRef::BlobText {
                    hash: BlobHash::new("1".repeat(64)),
                    mime: "text/plain".into(),
                    bytes: 4,
                },
                content: Some("root".into()),
                source_replica_id: ReplicaId::new(format!("topic::{topic}")),
                source_key: "post/event-root/header".into(),
                source_event_id: root_id.clone(),
                source_blob_hash: Some(BlobHash::new("1".repeat(64))),
                derived_at: 10,
                projection_version: 1,
            },
            EventProjectionRow {
                event_id: reply_id.clone(),
                topic_id: topic.to_string(),
                author_pubkey: "b".repeat(64),
                created_at: 11,
                root_id: Some(root_id.clone()),
                reply_to: Some(root_id.clone()),
                payload_ref: PayloadRef::BlobText {
                    hash: BlobHash::new("2".repeat(64)),
                    mime: "text/plain".into(),
                    bytes: 5,
                },
                content: Some("reply".into()),
                source_replica_id: ReplicaId::new(format!("topic::{topic}")),
                source_key: "post/event-reply/header".into(),
                source_event_id: reply_id.clone(),
                source_blob_hash: Some(BlobHash::new("2".repeat(64))),
                derived_at: 11,
                projection_version: 1,
            },
        ];

        ProjectionStore::rebuild_from_docs_blobs(&store, rows)
            .await
            .expect("rebuild projection");

        let timeline = ProjectionStore::list_topic_timeline(&store, topic, None, 10)
            .await
            .expect("timeline");
        let thread = ProjectionStore::list_thread(&store, topic, &root_id, None, 10)
            .await
            .expect("thread");

        assert_eq!(timeline.items.len(), 2);
        assert_eq!(timeline.items[0].event_id, reply_id);
        assert_eq!(thread.items.len(), 2);
        assert_eq!(thread.items[0].event_id, root_id);
    }
}
