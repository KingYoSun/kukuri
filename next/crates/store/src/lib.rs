use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use next_core::{Event, EventId, Profile, ThreadRef, parse_profile};
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
            ORDER BY e.created_at ASC, e.event_id ASC
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
            .flat_map(|entries| entries.values())
            .filter_map(|root_id| events.get(root_id).cloned())
            .filter(|event| {
                event.id == *thread_id
                    || event
                        .thread_ref()
                        .map(|thread| thread.root == *thread_id)
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
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

#[cfg(test)]
mod tests {
    use super::*;
    use next_core::{TopicId, build_text_note, generate_keys};

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

        let first_page = store
            .list_topic_timeline(topic.as_str(), None, 2)
            .await
            .expect("timeline page");
        let cursor = first_page.next_cursor.clone().expect("cursor");
        let second_page = store
            .list_topic_timeline(topic.as_str(), Some(cursor), 2)
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

        let thread = store
            .list_thread(topic.as_str(), &root.id, None, 10)
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
}
