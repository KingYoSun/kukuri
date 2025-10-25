use chrono::Utc;
use kukuri_lib::domain::entities::event_gateway::{DomainEvent, EventTag};
use kukuri_lib::domain::entities::EventKind;
use kukuri_lib::domain::value_objects::event_gateway::PublicKey;
use kukuri_lib::domain::value_objects::EventId;
use kukuri_lib::infrastructure::database::{
    connection_pool::ConnectionPool, sqlite_repository::SqliteRepository, EventRepository,
};
use kukuri_lib::infrastructure::event::{
    EventManagerHandle, LegacyEventManagerGateway, LegacyEventManagerHandle,
};
use sqlx::Row;
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn gateway_persists_p2p_events_via_event_manager() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("event_gateway.db");
    let db_url = format_sqlite_url(&db_path);
    let pool = ConnectionPool::new(&db_url).await?;

    let repository = Arc::new(SqliteRepository::new(pool.clone()));
    repository.initialize().await?;

    let event_manager: Arc<dyn EventManagerHandle> =
        Arc::new(LegacyEventManagerHandle::new_with_connection_pool(pool.clone()));
    event_manager
        .set_event_repository(Arc::clone(&repository) as Arc<dyn EventRepository>)
        .await;

    let gateway = LegacyEventManagerGateway::new(Arc::clone(&event_manager));

    let event_id =
        EventId::from_hex(&"a".repeat(64)).expect("64 hex characters produce a valid event id");
    let public_key =
        PublicKey::from_hex_str(&"b".repeat(64)).expect("64 hex characters produce a public key");
    let topic_tag = EventTag::new("t", vec!["public".to_string()]).expect("tag construction");
    let signature = "c".repeat(128);
    let payload = DomainEvent::new(
        event_id.clone(),
        public_key.clone(),
        EventKind::TextNote,
        Utc::now(),
        "hello-mainline-gateway".to_string(),
        vec![topic_tag],
        signature,
    )
    .expect("domain event creation");

    gateway
        .handle_incoming_event(payload)
        .await
        .expect("gateway should accept domain events");

    let stored = sqlx::query("SELECT event_id, content FROM events WHERE event_id = ?1")
        .bind(event_id.to_hex())
        .fetch_optional(pool.get_pool())
        .await?;
    assert!(stored.is_some(), "event record should exist after gateway call");
    let row = stored.unwrap();
    let stored_id: String = row.try_get("event_id")?;
    let stored_content: String = row.try_get("content")?;
    assert_eq!(stored_id, event_id.to_hex());
    assert_eq!(stored_content, "hello-mainline-gateway");

    let topics = sqlx::query("SELECT topic_id FROM event_topics WHERE event_id = ?1")
        .bind(event_id.to_hex())
        .fetch_all(pool.get_pool())
        .await?;
    assert_eq!(
        topics.len(),
        1,
        "event_topics should receive a single hashtag mapping"
    );
    assert_eq!(topics[0].try_get::<String, _>("topic_id")?, "public");

    pool.close().await;
    Ok(())
}

fn format_sqlite_url(path: &Path) -> String {
    let mut value = path.to_string_lossy().to_string();
    if cfg!(windows) {
        value = value.replace('\\', "/");
    }
    format!("sqlite://{}?mode=rwc", value)
}
