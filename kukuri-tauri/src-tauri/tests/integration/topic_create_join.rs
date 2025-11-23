use async_trait::async_trait;
use kukuri_lib::application::services::offline_service::{OfflineService, OfflineServiceTrait};
use kukuri_lib::application::services::topic_service::{PendingTopicStatus, TopicService};
use kukuri_lib::domain::entities::Topic;
use kukuri_lib::infrastructure::database::{
    connection_pool::ConnectionPool, sqlite_repository::SqliteRepository,
};
use kukuri_lib::infrastructure::offline::SqliteOfflinePersistence;
use kukuri_lib::shared::{error::AppError, config::BootstrapSource};
use kukuri_lib::application::services::p2p_service::{P2PServiceTrait, P2PStatus};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
struct TestP2PService {
    joins: Mutex<Vec<String>>,
}

#[async_trait]
impl P2PServiceTrait for TestP2PService {
    async fn initialize(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn join_topic(
        &self,
        topic_id: &str,
        _initial_peers: Vec<String>,
    ) -> Result<(), AppError> {
        self.joins.lock().await.push(topic_id.to_string());
        Ok(())
    }

    async fn leave_topic(&self, _topic_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn broadcast_message(&self, _topic_id: &str, _content: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn get_status(&self) -> Result<P2PStatus, AppError> {
        Ok(P2PStatus {
            is_connected: true,
            connected_peers: vec![],
            gossip_topics: vec![],
            node_addresses: vec![],
            bootstrap_source: BootstrapSource::Bundle,
        })
    }

    async fn get_node_addresses(&self) -> Result<Vec<String>, AppError> {
        Ok(vec![])
    }

    fn generate_topic_id(&self, topic_name: &str) -> String {
        topic_name.to_string()
    }

    async fn apply_bootstrap_nodes(
        &self,
        _nodes: Vec<String>,
        _source: BootstrapSource,
    ) -> Result<(), AppError> {
        Ok(())
    }
}

fn test_pubkey() -> String {
    std::iter::repeat('a').take(64).collect()
}

#[tokio::test]
async fn enqueue_create_and_sync_pending_topic() {
    let connection_pool = ConnectionPool::from_memory()
        .await
        .expect("create memory pool");
    let repository = Arc::new(SqliteRepository::new(connection_pool.clone()));
    repository.initialize().await.expect("run migrations");

    let topic_repo: Arc<dyn kukuri_lib::application::ports::repositories::TopicRepository> =
        Arc::clone(&repository);
    let pending_repo: Arc<dyn kukuri_lib::application::ports::repositories::PendingTopicRepository> =
        Arc::clone(&repository);
    let metrics_repo: Arc<dyn kukuri_lib::application::ports::repositories::TopicMetricsRepository> =
        Arc::clone(&repository);

    let offline_persistence =
        Arc::new(SqliteOfflinePersistence::new(connection_pool.get_pool().clone()));
    let offline_service: Arc<dyn OfflineServiceTrait> =
        Arc::new(OfflineService::new(offline_persistence));

    let p2p_service = Arc::new(TestP2PService::default());

    let topic_service = TopicService::new(
        topic_repo,
        pending_repo,
        metrics_repo,
        false,
        Arc::clone(&p2p_service) as Arc<dyn P2PServiceTrait>,
        offline_service,
    );
    topic_service
        .ensure_public_topic()
        .await
        .expect("ensure public topic");

    let user_pubkey = test_pubkey();
    let enqueue = topic_service
        .enqueue_topic_creation(&user_pubkey, "offline.topic".into(), Some("desc".into()))
        .await
        .expect("enqueue topic creation");
    assert_eq!(enqueue.pending_topic.status, PendingTopicStatus::Queued);

    let created = topic_service
        .create_topic("offline.topic".into(), Some("desc".into()), TopicVisibility::Public, &user_pubkey)
        .await
        .expect("create topic");

    topic_service
        .mark_pending_topic_synced(&enqueue.pending_topic.pending_id, &created.id)
        .await
        .expect("mark pending synced");

    let synced = topic_service
        .get_pending_topic(&enqueue.pending_topic.pending_id)
        .await
        .expect("get pending topic")
        .expect("pending entry exists");
    assert_eq!(synced.status, PendingTopicStatus::Synced);
    assert_eq!(synced.synced_topic_id.as_deref(), Some(created.id.as_str()));

    let joined = repository
        .get_joined_topics(&user_pubkey)
        .await
        .expect("joined topics");
    assert!(
        joined.iter().any(|topic: &Topic| topic.id == created.id),
        "user should be joined to newly created topic"
    );

    {
        let joins = p2p_service.joins.lock().await;
        assert!(
            joins.contains(&created.id),
            "p2p service should receive join request"
        );
    }

    let failed = topic_service
        .enqueue_topic_creation(&user_pubkey, "offline.fail".into(), None)
        .await
        .expect("enqueue second topic");
    topic_service
        .mark_pending_topic_failed(&failed.pending_topic.pending_id, Some("network error".into()))
        .await
        .expect("mark pending failed");

    let failed_entry = topic_service
        .get_pending_topic(&failed.pending_topic.pending_id)
        .await
        .expect("get failed pending")
        .expect("failed entry exists");
    assert_eq!(failed_entry.status, PendingTopicStatus::Failed);
    assert_eq!(
        failed_entry.error_message.as_deref(),
        Some("network error")
    );
}