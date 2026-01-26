use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use kukuri_lib::domain::constants::DEFAULT_PUBLIC_TOPIC_ID;
use kukuri_lib::test_support::application::ports::event_topic_store::EventTopicStore;
use kukuri_lib::test_support::application::ports::key_manager::KeyManager;
use kukuri_lib::test_support::domain::entities::Event as DomainEvent;
use kukuri_lib::test_support::infrastructure::crypto::DefaultKeyManager;
use kukuri_lib::test_support::infrastructure::database::connection_pool::ConnectionPool;
use kukuri_lib::test_support::infrastructure::database::repository::Repository;
use kukuri_lib::test_support::infrastructure::database::sqlite_repository::SqliteRepository;
use kukuri_lib::test_support::infrastructure::event::{
    EventManagerHandle, LegacyEventManagerHandle, RepositoryEventTopicStore,
};
use kukuri_lib::test_support::infrastructure::p2p::GossipService;
use kukuri_lib::test_support::shared::error::AppError;
use nostr_sdk::prelude::*;
use sqlx::Row;
use tempfile::TempDir;
use tokio::sync::Mutex;

#[tokio::test]
async fn handle_p2p_event_persists_rows() -> Result<()> {
    let ctx = TestContext::setup().await?;

    let keys = Keys::generate();
    let event = EventBuilder::text_note("phase5-incoming-event")
        .tag(Tag::hashtag(DEFAULT_PUBLIC_TOPIC_ID))
        .sign_with_keys(&keys)?;

    ctx.manager.handle_p2p_event(event.clone()).await?;

    let stored = sqlx::query("SELECT content FROM events WHERE event_id = ?1")
        .bind(event.id.to_hex())
        .fetch_optional(ctx.pool.get_pool())
        .await?;
    let row = stored.expect("event row should exist after handler");
    let stored_content: String = row.try_get("content")?;
    assert_eq!(stored_content, "phase5-incoming-event");

    let topics = sqlx::query("SELECT topic_id FROM event_topics WHERE event_id = ?1")
        .bind(event.id.to_hex())
        .fetch_all(ctx.pool.get_pool())
        .await?;
    let topic_ids: Vec<String> = topics
        .iter()
        .map(|row| {
            row.try_get::<String, _>("topic_id")
                .expect("topic id column")
        })
        .collect();
    assert!(
        topic_ids
            .iter()
            .any(|topic| topic == DEFAULT_PUBLIC_TOPIC_ID),
        "expected hashtag mapping for public"
    );

    ctx.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn publish_topic_post_broadcasts_and_links_topics() -> Result<()> {
    let ctx = TestContext::setup().await?;
    let manager: Arc<dyn EventManagerHandle> = ctx.manager.clone();

    manager
        .set_default_p2p_topics(vec![DEFAULT_PUBLIC_TOPIC_ID.to_string()])
        .await;

    unsafe {
        std::env::set_var("KUKURI_ALLOW_NO_RELAY", "1");
    }
    let _event_id = manager
        .publish_topic_post(
            DEFAULT_PUBLIC_TOPIC_ID,
            "phase5-topic-body",
            None,
            None,
            None,
        )
        .await?;

    let joined = ctx.gossip.joined_topics().await;
    assert!(joined.iter().any(|topic| topic == DEFAULT_PUBLIC_TOPIC_ID));

    unsafe {
        std::env::remove_var("KUKURI_ALLOW_NO_RELAY");
    }
    ctx.pool.close().await;
    Ok(())
}

struct TestContext {
    _temp_dir: TempDir,
    pool: ConnectionPool,
    _repository: Arc<SqliteRepository>,
    manager: Arc<LegacyEventManagerHandle>,
    gossip: Arc<RecordingGossipService>,
}

impl TestContext {
    async fn setup() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("event_manager.db");
        let db_url = format_sqlite_url(&db_path);
        let pool = ConnectionPool::new(&db_url).await?;

        let repository = Arc::new(SqliteRepository::new(pool.clone()));
        repository.initialize().await?;

        let manager = Arc::new(LegacyEventManagerHandle::new_with_connection_pool(
            pool.clone(),
        ));
        let event_manager = manager.as_event_manager();

        let key_manager = DefaultKeyManager::new();
        key_manager.generate_keypair().await?;
        event_manager
            .initialize_with_key_manager(&key_manager)
            .await?;

        let gossip = Arc::new(RecordingGossipService::default());
        let gossip_trait: Arc<dyn GossipService> = gossip.clone();
        manager.set_gossip_service(gossip_trait).await;

        let topic_store: Arc<dyn EventTopicStore> =
            Arc::new(RepositoryEventTopicStore::new(repository.clone()));
        manager.set_event_topic_store(topic_store).await;

        Ok(Self {
            _temp_dir: temp_dir,
            pool,
            _repository: repository,
            manager,
            gossip,
        })
    }
}

#[derive(Default)]
struct RecordingGossipService {
    joined_topics: Mutex<Vec<String>>,
    broadcasts: Mutex<Vec<(String, DomainEvent)>>,
    messages: Mutex<Vec<(String, Vec<u8>)>>,
}

impl RecordingGossipService {
    async fn joined_topics(&self) -> Vec<String> {
        self.joined_topics.lock().await.clone()
    }
}

#[async_trait]
impl GossipService for RecordingGossipService {
    async fn join_topic(&self, topic: &str, _initial_peers: Vec<String>) -> Result<(), AppError> {
        self.joined_topics.lock().await.push(topic.to_string());
        Ok(())
    }

    async fn leave_topic(&self, _topic: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn broadcast(&self, topic: &str, event: &DomainEvent) -> Result<(), AppError> {
        self.broadcasts
            .lock()
            .await
            .push((topic.to_string(), event.clone()));
        Ok(())
    }

    async fn subscribe(
        &self,
        _topic: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<DomainEvent>, AppError> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    async fn get_joined_topics(&self) -> Result<Vec<String>, AppError> {
        Ok(self.joined_topics().await)
    }

    async fn get_topic_peers(&self, _topic: &str) -> Result<Vec<String>, AppError> {
        Ok(vec![])
    }

    async fn get_topic_stats(
        &self,
        _topic: &str,
    ) -> Result<Option<kukuri_lib::test_support::domain::p2p::TopicStats>, AppError> {
        Ok(None)
    }

    async fn broadcast_message(&self, topic: &str, message: &[u8]) -> Result<(), AppError> {
        self.messages
            .lock()
            .await
            .push((topic.to_string(), message.to_vec()));
        Ok(())
    }
}

fn format_sqlite_url(path: &Path) -> String {
    let mut value = path.to_string_lossy().to_string();
    if cfg!(windows) {
        value = value.replace('\\', "/");
    }
    format!("sqlite://{}?mode=rwc", value)
}
