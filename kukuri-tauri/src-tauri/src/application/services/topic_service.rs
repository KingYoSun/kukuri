use super::p2p_service::P2PServiceTrait;
use crate::application::ports::repositories::TopicRepository;
use crate::domain::entities::Topic;
use crate::shared::error::AppError;
use std::sync::Arc;

pub struct TopicService {
    repository: Arc<dyn TopicRepository>,
    p2p: Arc<dyn P2PServiceTrait>,
}

impl TopicService {
    pub fn new(repository: Arc<dyn TopicRepository>, p2p: Arc<dyn P2PServiceTrait>) -> Self {
        Self { repository, p2p }
    }

    pub async fn create_topic(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Topic, AppError> {
        let topic = Topic::new(name, description);
        self.repository.create_topic(&topic).await?;

        // Join gossip topic
        self.p2p.join_topic(&topic.id, vec![]).await?;

        Ok(topic)
    }

    pub async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError> {
        self.repository.get_topic(id).await
    }

    pub async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError> {
        self.repository.get_all_topics().await
    }

    pub async fn get_joined_topics(&self, user_pubkey: &str) -> Result<Vec<Topic>, AppError> {
        self.repository.get_joined_topics(user_pubkey).await
    }

    pub async fn join_topic(&self, id: &str, user_pubkey: &str) -> Result<(), AppError> {
        self.repository.join_topic(id, user_pubkey).await?;
        self.p2p.join_topic(id, Vec::new()).await?;
        Ok(())
    }

    pub async fn leave_topic(&self, id: &str, user_pubkey: &str) -> Result<(), AppError> {
        self.repository.leave_topic(id, user_pubkey).await?;
        self.p2p.leave_topic(id).await?;
        Ok(())
    }

    pub async fn update_topic(&self, topic: &Topic) -> Result<(), AppError> {
        self.repository.update_topic(topic).await
    }

    pub async fn delete_topic(&self, id: &str) -> Result<(), AppError> {
        // Prevent deletion of public topic
        if id == "public" {
            return Err("Cannot delete public topic".into());
        }

        self.p2p.leave_topic(id).await?;
        self.repository.delete_topic(id).await
    }

    pub async fn get_topic_stats(&self, id: &str) -> Result<(u32, u32), AppError> {
        if let Some(topic) = self.repository.get_topic(id).await? {
            Ok((topic.member_count, topic.post_count))
        } else {
            Ok((0, 0))
        }
    }

    pub async fn ensure_public_topic(&self) -> Result<(), AppError> {
        if self.repository.get_topic("public").await?.is_none() {
            let public_topic = Topic::public_topic();
            self.repository.create_topic(&public_topic).await?;
            self.p2p.join_topic("public", Vec::new()).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::repositories::TopicRepository as PortTopicRepository;
    use crate::application::services::p2p_service::{P2PServiceTrait, P2PStatus};
    use async_trait::async_trait;
    use mockall::{mock, predicate::*};

    mock! {
        pub TopicRepo {}

        #[async_trait]
        impl PortTopicRepository for TopicRepo {
            async fn create_topic(&self, topic: &Topic) -> Result<(), AppError>;
            async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError>;
            async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError>;
            async fn get_joined_topics(&self, user_pubkey: &str) -> Result<Vec<Topic>, AppError>;
            async fn update_topic(&self, topic: &Topic) -> Result<(), AppError>;
            async fn delete_topic(&self, id: &str) -> Result<(), AppError>;
            async fn join_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError>;
            async fn leave_topic(&self, topic_id: &str, user_pubkey: &str) -> Result<(), AppError>;
            async fn update_topic_stats(
                &self,
                topic_id: &str,
                member_count: u32,
                post_count: u32,
            ) -> Result<(), AppError>;
        }
    }

    mock! {
        pub P2P {}

        #[async_trait]
        impl P2PServiceTrait for P2P {
            async fn initialize(&self) -> Result<(), AppError>;
            async fn join_topic(&self, topic_id: &str, initial_peers: Vec<String>) -> Result<(), AppError>;
            async fn leave_topic(&self, topic_id: &str) -> Result<(), AppError>;
            async fn broadcast_message(&self, topic_id: &str, content: &str) -> Result<(), AppError>;
            async fn get_status(&self) -> Result<P2PStatus, AppError>;
            async fn get_node_addresses(&self) -> Result<Vec<String>, AppError>;
            fn generate_topic_id(&self, topic_name: &str) -> String;
        }
    }

    #[tokio::test]
    async fn test_join_topic_calls_repository_and_gossip() {
        let mut repo = MockTopicRepo::new();
        repo.expect_join_topic()
            .with(eq("tech"), eq("pubkey1"))
            .times(1)
            .returning(|_, _| Ok(()));
        let mut p2p = MockP2P::new();
        p2p.expect_join_topic()
            .with(eq("tech"), eq(Vec::<String>::new()))
            .times(1)
            .returning(|_, _| Ok(()));

        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(p2p);
        let service = TopicService::new(repo_arc, p2p_arc);

        let result = service.join_topic("tech", "pubkey1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_leave_topic_calls_repository_and_gossip() {
        let mut repo = MockTopicRepo::new();
        repo.expect_leave_topic()
            .with(eq("tech"), eq("pubkey1"))
            .times(1)
            .returning(|_, _| Ok(()));
        let mut p2p = MockP2P::new();
        p2p.expect_leave_topic()
            .with(eq("tech"))
            .times(1)
            .returning(|_| Ok(()));

        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(p2p);
        let service = TopicService::new(repo_arc, p2p_arc);

        let result = service.leave_topic("tech", "pubkey1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_joined_topics_passes_user_pubkey() {
        let mut repo = MockTopicRepo::new();
        repo.expect_get_joined_topics()
            .with(eq("pubkey1"))
            .times(1)
            .returning(|_| Ok(vec![]));
        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = TopicService::new(repo_arc, p2p_arc);
        let result = service.get_joined_topics("pubkey1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
