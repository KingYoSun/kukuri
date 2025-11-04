use super::p2p_service::P2PServiceTrait;
use crate::application::ports::repositories::TopicRepository;
use crate::domain::entities::Topic;
use crate::shared::error::AppError;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TopicTrendingEntry {
    pub topic: Topic,
    pub trending_score: f64,
}

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

    pub async fn list_trending_topics(
        &self,
        limit: usize,
    ) -> Result<Vec<TopicTrendingEntry>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut entries: Vec<TopicTrendingEntry> = self
            .repository
            .get_all_topics()
            .await?
            .into_iter()
            .map(|topic| TopicTrendingEntry {
                trending_score: Self::calculate_trending_score(&topic),
                topic,
            })
            .collect();

        entries.sort_by(|a, b| {
            b.trending_score
                .partial_cmp(&a.trending_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.topic.updated_at.cmp(&a.topic.updated_at))
                .then_with(|| a.topic.name.cmp(&b.topic.name))
        });

        entries.truncate(limit);
        Ok(entries)
    }

    fn calculate_trending_score(topic: &Topic) -> f64 {
        if topic.member_count == 0 && topic.post_count == 0 {
            0.0
        } else {
            (topic.post_count as f64 * 0.6) + (topic.member_count as f64 * 0.4)
        }
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
    async fn list_trending_topics_orders_by_score() {
        let mut repo = MockTopicRepo::new();
        let mut topic_alpha = Topic::new("Alpha".into(), Some("A".into()));
        topic_alpha.id = "alpha".into();
        topic_alpha.member_count = 5;
        topic_alpha.post_count = 20;

        let mut topic_beta = Topic::new("Beta".into(), Some("B".into()));
        topic_beta.id = "beta".into();
        topic_beta.member_count = 15;
        topic_beta.post_count = 10;

        let mut topic_gamma = Topic::new("Gamma".into(), Some("G".into()));
        topic_gamma.id = "gamma".into();
        topic_gamma.member_count = 2;
        topic_gamma.post_count = 5;

        repo.expect_get_all_topics().times(1).returning(move || {
            Ok(vec![
                topic_alpha.clone(),
                topic_beta.clone(),
                topic_gamma.clone(),
            ])
        });

        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = TopicService::new(repo_arc, p2p_arc);

        let result = service
            .list_trending_topics(3)
            .await
            .expect("trending topics");

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].topic.id, "alpha");
        assert!(result[0].trending_score >= result[1].trending_score);
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
