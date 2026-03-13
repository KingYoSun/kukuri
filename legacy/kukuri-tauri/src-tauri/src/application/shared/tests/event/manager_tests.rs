use super::mocks::TestGossipService;
use crate::application::ports::key_manager::KeyManager;
use crate::domain::constants::DEFAULT_PUBLIC_TOPIC_ID;
use crate::domain::p2p::user_topic_id;
use crate::infrastructure::crypto::DefaultKeyManager;
use crate::infrastructure::database::connection_pool::ConnectionPool;
use crate::infrastructure::event::EventManager;
use crate::infrastructure::p2p::GossipService;
use nostr_sdk::prelude::*;
use sqlx::query;
use std::sync::Arc;

#[tokio::test]
async fn event_manager_initializes_with_key_manager() {
    let manager = EventManager::new();
    let key_manager = DefaultKeyManager::new();

    key_manager.generate_keypair().await.unwrap();

    manager
        .initialize_with_key_manager(&key_manager)
        .await
        .expect("initialization succeeds");
    assert!(manager.get_public_key().await.is_some());
}

#[tokio::test]
async fn event_manager_initializes_with_keypair_directly() {
    let manager = EventManager::new();
    let key_manager = DefaultKeyManager::new();

    let keypair = key_manager
        .generate_keypair()
        .await
        .expect("keypair generation");

    manager
        .initialize_with_keypair(&keypair)
        .await
        .expect("initialization with keypair");
    assert!(manager.get_public_key().await.is_some());
}

#[tokio::test]
async fn operations_fail_before_initialization() {
    let manager = EventManager::new();

    assert!(manager.publish_text_note("test").await.is_err());
    assert!(
        manager
            .publish_topic_post("topic", "content", None, None, None)
            .await
            .is_err()
    );
    assert!(manager.subscribe_to_topic("topic", None).await.is_err());
}

#[tokio::test]
async fn initialize_and_disconnect_cycle() {
    let manager = EventManager::new();
    let key_manager = DefaultKeyManager::new();
    key_manager.generate_keypair().await.unwrap();
    manager
        .initialize_with_key_manager(&key_manager)
        .await
        .unwrap();

    assert!(manager.ensure_initialized().await.is_ok());
    manager.disconnect().await.unwrap();
    assert!(manager.ensure_initialized().await.is_err());
}

#[tokio::test]
async fn get_public_key_matches_key_manager() {
    let manager = EventManager::new();
    let key_manager = DefaultKeyManager::new();

    assert!(manager.get_public_key().await.is_none());

    key_manager.generate_keypair().await.unwrap();
    manager
        .initialize_with_key_manager(&key_manager)
        .await
        .unwrap();

    let public_key = manager.get_public_key().await.unwrap();
    let current_pubkey = key_manager.current_keypair().await.unwrap().public_key;
    let expected = PublicKey::from_hex(&current_pubkey).expect("valid pubkey hex");
    assert_eq!(public_key, expected);
}

#[tokio::test]
async fn default_topics_api_behaves_idempotently() {
    let manager = EventManager::new();

    let mut topics = manager.list_default_p2p_topics().await;
    topics.sort();
    assert_eq!(topics, vec![DEFAULT_PUBLIC_TOPIC_ID.to_string()]);

    manager
        .set_default_p2p_topics(vec!["a".into(), "b".into()])
        .await;
    let mut topics = manager.list_default_p2p_topics().await;
    topics.sort();
    assert_eq!(topics, vec!["a".to_string(), "b".to_string()]);

    manager.add_default_p2p_topic("c").await;
    manager.remove_default_p2p_topic("b").await;
    let mut topics = manager.list_default_p2p_topics().await;
    topics.sort();
    assert_eq!(topics, vec!["a".to_string(), "c".to_string()]);
}

#[tokio::test]
async fn routing_non_topic_broadcasts_to_user_topic() {
    let manager = EventManager::new();
    let key_manager = DefaultKeyManager::new();

    key_manager.generate_keypair().await.unwrap();
    manager
        .initialize_with_key_manager(&key_manager)
        .await
        .unwrap();

    manager
        .set_default_p2p_topics(vec!["t1".into(), "t2".into()])
        .await;

    let gossip = Arc::new(TestGossipService::new());
    manager.set_gossip_service(gossip.clone()).await;

    let publisher = manager.event_publisher.read().await;
    let nostr_event = publisher.create_text_note("hello", vec![]).unwrap();
    drop(publisher);

    let mut topics = manager.list_default_p2p_topics().await;
    if let Some(pk) = manager.get_public_key().await {
        topics.push(user_topic_id(&pk.to_string()));
    }
    manager
        .broadcast_to_topics(
            &(gossip.clone() as Arc<dyn GossipService>),
            &topics,
            &nostr_event,
        )
        .await
        .unwrap();

    let joined = gossip.joined_topics().await;
    let pubkey = manager.get_public_key().await.unwrap();
    let user_topic = user_topic_id(&pubkey.to_string());
    assert!(joined.contains("t1"));
    assert!(joined.contains("t2"));
    assert!(joined.contains(&user_topic));

    let mut b = gossip.broadcasted_topics().await;
    b.sort();
    assert_eq!(b, {
        let mut v = vec!["t1".to_string(), "t2".to_string(), user_topic];
        v.sort();
        v
    });
}

#[tokio::test]
async fn publisher_creates_expected_event_kinds() {
    let manager = EventManager::new();
    let key_manager = DefaultKeyManager::new();

    key_manager.generate_keypair().await.unwrap();
    manager
        .initialize_with_key_manager(&key_manager)
        .await
        .unwrap();

    let publisher = manager.event_publisher.read().await;

    let text_event = publisher.create_text_note("Test note", vec![]).unwrap();
    assert_eq!(text_event.kind, Kind::TextNote);

    let metadata = Metadata::new().name("Test User");
    let metadata_event = publisher.create_metadata(metadata).unwrap();
    assert_eq!(metadata_event.kind, Kind::Metadata);

    let event_id = EventId::from_slice(&[1; 32]).unwrap();
    let reaction_event = publisher.create_reaction(&event_id, "+").unwrap();
    assert_eq!(reaction_event.kind, Kind::Reaction);
}

#[tokio::test]
async fn ensure_initialized_requires_keypair() {
    let manager = EventManager::new();
    assert!(manager.ensure_initialized().await.is_err());

    let key_manager = DefaultKeyManager::new();
    key_manager.generate_keypair().await.unwrap();
    manager
        .initialize_with_key_manager(&key_manager)
        .await
        .unwrap();

    assert!(manager.ensure_initialized().await.is_ok());
}

#[tokio::test]
async fn publish_topic_post_broadcasts_stored_profile_metadata_before_post() {
    let pool = ConnectionPool::new("sqlite::memory:?cache=shared")
        .await
        .expect("in-memory pool");
    query(
        r#"
        CREATE TABLE users (
            npub TEXT PRIMARY KEY NOT NULL,
            pubkey TEXT NOT NULL UNIQUE,
            name TEXT,
            display_name TEXT,
            bio TEXT,
            avatar_url TEXT,
            nip05 TEXT,
            is_profile_public INTEGER NOT NULL DEFAULT 1,
            show_online_status INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool.get_pool())
    .await
    .expect("create users table");

    let manager = EventManager::new_with_connection_pool(pool.clone());
    let key_manager = DefaultKeyManager::new();
    let keypair = key_manager
        .generate_keypair()
        .await
        .expect("keypair generation");
    manager
        .initialize_with_keypair(&keypair)
        .await
        .expect("initialize manager");

    query(
        r#"
        INSERT INTO users (
            npub,
            pubkey,
            name,
            display_name,
            bio,
            avatar_url,
            nip05,
            is_profile_public,
            show_online_status,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 0, ?8, ?8)
        "#,
    )
    .bind(&keypair.npub)
    .bind(&keypair.public_key)
    .bind("alice")
    .bind("Alice Example")
    .bind("Stored local profile")
    .bind("https://example.com/alice.png")
    .bind("alice@example.com")
    .bind(1_710_000_000_i64)
    .execute(pool.get_pool())
    .await
    .expect("insert user profile");

    let gossip = Arc::new(TestGossipService::new());
    manager.set_gossip_service(gossip.clone()).await;

    manager
        .publish_topic_post("topic-profile", "profile-aware post", None, None, None)
        .await
        .expect("publish topic post");

    let broadcasts = gossip.broadcasts().await;
    assert_eq!(broadcasts.len(), 2);
    assert_eq!(broadcasts[0].0, "topic-profile");
    assert_eq!(broadcasts[0].1.kind, 0);
    assert!(broadcasts[0].1.content.contains("Alice Example"));
    assert!(
        broadcasts[0]
            .1
            .content
            .contains("https://example.com/alice.png")
    );
    assert_eq!(broadcasts[1].0, "topic-profile");
    assert_eq!(broadcasts[1].1.kind, 1);
    assert_eq!(broadcasts[1].1.content, "profile-aware post");
}

#[tokio::test]
async fn publish_topic_post_skips_auto_profile_metadata_without_stored_profile() {
    let pool = ConnectionPool::new("sqlite::memory:?cache=shared")
        .await
        .expect("in-memory pool");
    query(
        r#"
        CREATE TABLE users (
            npub TEXT PRIMARY KEY NOT NULL,
            pubkey TEXT NOT NULL UNIQUE,
            name TEXT,
            display_name TEXT,
            bio TEXT,
            avatar_url TEXT,
            nip05 TEXT,
            is_profile_public INTEGER NOT NULL DEFAULT 1,
            show_online_status INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool.get_pool())
    .await
    .expect("create users table");

    let manager = EventManager::new_with_connection_pool(pool);
    let key_manager = DefaultKeyManager::new();
    let keypair = key_manager
        .generate_keypair()
        .await
        .expect("keypair generation");
    manager
        .initialize_with_keypair(&keypair)
        .await
        .expect("initialize manager");

    let gossip = Arc::new(TestGossipService::new());
    manager.set_gossip_service(gossip.clone()).await;

    manager
        .publish_topic_post("topic-profile", "plain post", None, None, None)
        .await
        .expect("publish topic post");

    let broadcasts = gossip.broadcasts().await;
    assert_eq!(broadcasts.len(), 1);
    assert_eq!(broadcasts[0].0, "topic-profile");
    assert_eq!(broadcasts[0].1.kind, 1);
    assert_eq!(broadcasts[0].1.content, "plain post");
}
