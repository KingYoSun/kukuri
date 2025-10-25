use super::*;
use crate::application::ports::key_manager::KeyManager;
use crate::domain::p2p::user_topic_id;
use crate::infrastructure::{crypto::DefaultKeyManager, p2p::GossipService};
use nostr_sdk::prelude::*;
use std::sync::Arc;

mod support;

use support::mocks::TestGossipService;

#[tokio::test]
async fn test_event_manager_initialization() {
    let manager = EventManager::new();
    let key_manager = DefaultKeyManager::new();

    key_manager.generate_keypair().await.unwrap();

    assert!(
        manager
            .initialize_with_key_manager(&key_manager)
            .await
            .is_ok()
    );
    assert!(manager.get_public_key().await.is_some());
}

#[tokio::test]
async fn test_event_manager_not_initialized() {
    let manager = EventManager::new();

    assert!(manager.publish_text_note("test").await.is_err());
    assert!(
        manager
            .publish_topic_post("topic", "content", None)
            .await
            .is_err()
    );
    assert!(manager.subscribe_to_topic("topic", None).await.is_err());
}

#[tokio::test]
async fn test_initialize_and_disconnect() {
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
async fn test_get_public_key() {
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
async fn test_create_events() {
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
async fn test_ensure_initialized() {
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
async fn test_default_topics_api() {
    let manager = EventManager::new();

    let mut topics = manager.list_default_p2p_topics().await;
    topics.sort();
    assert_eq!(topics, vec!["public".to_string()]);

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
async fn test_routing_non_topic_includes_user_and_defaults() {
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
