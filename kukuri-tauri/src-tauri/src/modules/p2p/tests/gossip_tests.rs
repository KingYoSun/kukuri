#[cfg(test)]
mod tests {
    use crate::modules::p2p::*;
    use iroh::SecretKey;
    
    async fn create_test_manager() -> GossipManager {
        let secret_key = SecretKey::generate(rand::thread_rng());
        GossipManager::new(secret_key).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_topic_join_leave() {
        let manager = create_test_manager().await;
        let topic_id = "test-topic";
        
        // Join topic
        let result = manager.join_topic(topic_id, vec![]).await;
        assert!(result.is_ok());
        
        // Verify topic is active
        let active_topics = manager.active_topics().await;
        assert!(active_topics.contains(&topic_id.to_string()));
        
        // Leave topic
        let result = manager.leave_topic(topic_id).await;
        assert!(result.is_ok());
        
        // Verify topic is removed
        let active_topics = manager.active_topics().await;
        assert!(!active_topics.contains(&topic_id.to_string()));
    }
    
    #[tokio::test]
    async fn test_multiple_topics() {
        let manager = create_test_manager().await;
        let topics = vec!["topic1", "topic2", "topic3"];
        
        // Join multiple topics
        for topic in &topics {
            manager.join_topic(topic, vec![]).await.unwrap();
        }
        
        let active_topics = manager.active_topics().await;
        assert_eq!(active_topics.len(), 3);
        
        // Leave one topic
        manager.leave_topic("topic2").await.unwrap();
        
        let active_topics = manager.active_topics().await;
        assert_eq!(active_topics.len(), 2);
        assert!(!active_topics.contains(&"topic2".to_string()));
    }
    
    #[tokio::test]
    async fn test_leave_nonexistent_topic() {
        let manager = create_test_manager().await;
        
        let result = manager.leave_topic("nonexistent").await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            P2PError::TopicNotFound(topic) => assert_eq!(topic, "nonexistent"),
            _ => panic!("Expected TopicNotFound error"),
        }
    }
}