use super::p2p_service::P2PServiceTrait;
use crate::application::ports::repositories::{TopicMetricsRepository, TopicRepository};
use crate::domain::entities::{Topic, TopicMetricsRecord};
use crate::shared::error::AppError;
use chrono::Utc;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TopicTrendingEntry {
    pub topic: Topic,
    pub trending_score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendingDataSource {
    Metrics,
    Legacy,
}

pub struct TrendingTopicsResult {
    pub generated_at: i64,
    pub entries: Vec<TopicTrendingEntry>,
    pub data_source: TrendingDataSource,
}

pub struct TopicService {
    repository: Arc<dyn TopicRepository>,
    metrics_repository: Arc<dyn TopicMetricsRepository>,
    metrics_enabled: bool,
    p2p: Arc<dyn P2PServiceTrait>,
}

impl TopicService {
    pub fn new(
        repository: Arc<dyn TopicRepository>,
        metrics_repository: Arc<dyn TopicMetricsRepository>,
        metrics_enabled: bool,
        p2p: Arc<dyn P2PServiceTrait>,
    ) -> Self {
        Self {
            repository,
            metrics_repository,
            metrics_enabled,
            p2p,
        }
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
    ) -> Result<TrendingTopicsResult, AppError> {
        if limit == 0 {
            return Ok(TrendingTopicsResult {
                generated_at: Utc::now().timestamp_millis(),
                entries: Vec::new(),
                data_source: TrendingDataSource::Legacy,
            });
        }

        if self.metrics_enabled {
            if let Some(snapshot) = self.metrics_repository.list_recent_metrics(limit).await? {
                let entries = self
                    .build_entries_from_metrics(&snapshot.metrics, limit)
                    .await?;

                if !entries.is_empty() || !snapshot.metrics.is_empty() {
                    return Ok(TrendingTopicsResult {
                        generated_at: snapshot.window_end,
                        entries,
                        data_source: TrendingDataSource::Metrics,
                    });
                }
            }
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
        Ok(TrendingTopicsResult {
            generated_at: Utc::now().timestamp_millis(),
            entries,
            data_source: TrendingDataSource::Legacy,
        })
    }

    pub async fn latest_metrics_generated_at(&self) -> Result<Option<i64>, AppError> {
        if !self.metrics_enabled {
            return Ok(None);
        }
        self.metrics_repository.latest_window_end().await
    }

    async fn build_entries_from_metrics(
        &self,
        metrics: &[TopicMetricsRecord],
        limit: usize,
    ) -> Result<Vec<TopicTrendingEntry>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for record in metrics {
            if entries.len() >= limit {
                break;
            }
            if let Some(topic) = self.repository.get_topic(&record.topic_id).await? {
                entries.push(TopicTrendingEntry {
                    trending_score: record.score_24h,
                    topic,
                });
            }
        }
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
    use crate::application::ports::repositories::{
        TopicMetricsRepository as PortTopicMetricsRepository,
        TopicRepository as PortTopicRepository,
    };
    use crate::application::services::p2p_service::{P2PServiceTrait, P2PStatus};
    use crate::domain::entities::{
        MetricsWindow, TopicActivityRow, TopicMetricsSnapshot, TopicMetricsUpsert,
    };
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

    mock! {
        pub TopicMetricsRepo {}

        #[async_trait]
        impl PortTopicMetricsRepository for TopicMetricsRepo {
            async fn upsert_metrics(&self, metrics: TopicMetricsUpsert) -> Result<(), AppError>;
            async fn cleanup_expired(&self, cutoff_millis: i64) -> Result<u64, AppError>;
            async fn collect_activity(
                &self,
                window: MetricsWindow,
            ) -> Result<Vec<TopicActivityRow>, AppError>;
            async fn latest_window_end(&self) -> Result<Option<i64>, AppError>;
            async fn list_recent_metrics(
                &self,
                limit: usize,
            ) -> Result<Option<TopicMetricsSnapshot>, AppError>;
        }
    }

    fn topic_with_counts(id: &str, name: &str, members: u32, posts: u32) -> Topic {
        let mut topic = Topic::new(name.to_string(), Some(format!("{name} desc")));
        topic.id = id.to_string();
        topic.member_count = members;
        topic.post_count = posts;
        topic
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
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> =
            Arc::new(MockTopicMetricsRepo::new());
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(p2p);
        let service = TopicService::new(repo_arc, metrics_arc, false, p2p_arc);

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
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> =
            Arc::new(MockTopicMetricsRepo::new());
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(p2p);
        let service = TopicService::new(repo_arc, metrics_arc, false, p2p_arc);

        let result = service.leave_topic("tech", "pubkey1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_trending_topics_orders_by_score_without_metrics() {
        let mut repo = MockTopicRepo::new();
        let topic_alpha = topic_with_counts("alpha", "Alpha", 5, 20);
        let topic_beta = topic_with_counts("beta", "Beta", 15, 10);
        let topic_gamma = topic_with_counts("gamma", "Gamma", 2, 5);

        repo.expect_get_all_topics().times(1).returning(move || {
            Ok(vec![
                topic_alpha.clone(),
                topic_beta.clone(),
                topic_gamma.clone(),
            ])
        });

        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> =
            Arc::new(MockTopicMetricsRepo::new());
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = TopicService::new(repo_arc, metrics_arc, false, p2p_arc);

        let result = service
            .list_trending_topics(3)
            .await
            .expect("trending topics");

        assert_eq!(result.entries.len(), 3);
        assert_eq!(result.entries[0].topic.id, "alpha");
        assert!(result.entries[0].trending_score >= result.entries[1].trending_score);
        assert_eq!(result.data_source, TrendingDataSource::Legacy);
    }

    #[tokio::test]
    async fn list_trending_topics_prefers_metrics_snapshot() {
        let mut repo = MockTopicRepo::new();
        let topic_a = topic_with_counts("alpha", "Alpha", 5, 10);
        let topic_b = topic_with_counts("beta", "Beta", 3, 15);
        repo.expect_get_topic()
            .with(eq("alpha"))
            .return_once(move |_| Ok(Some(topic_a.clone())));
        repo.expect_get_topic()
            .with(eq("beta"))
            .return_once(move |_| Ok(Some(topic_b.clone())));
        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);

        let mut metrics_repo = MockTopicMetricsRepo::new();
        metrics_repo
            .expect_list_recent_metrics()
            .with(eq(2))
            .return_once(|_| {
                Ok(Some(TopicMetricsSnapshot {
                    window_start: 100,
                    window_end: 200,
                    metrics: vec![
                        TopicMetricsRecord {
                            topic_id: "alpha".into(),
                            window_start: 100,
                            window_end: 200,
                            posts_24h: 10,
                            posts_6h: 4,
                            unique_authors: 3,
                            boosts: 1,
                            replies: 0,
                            bookmarks: 0,
                            participant_delta: 1,
                            score_24h: 42.0,
                            score_6h: 21.0,
                            updated_at: 200,
                        },
                        TopicMetricsRecord {
                            topic_id: "beta".into(),
                            window_start: 100,
                            window_end: 200,
                            posts_24h: 5,
                            posts_6h: 2,
                            unique_authors: 2,
                            boosts: 0,
                            replies: 0,
                            bookmarks: 0,
                            participant_delta: 0,
                            score_24h: 30.0,
                            score_6h: 15.0,
                            updated_at: 200,
                        },
                    ],
                }))
            });
        metrics_repo
            .expect_latest_window_end()
            .returning(|| Ok(Some(200)));

        let metrics_arc: Arc<dyn PortTopicMetricsRepository> = Arc::new(metrics_repo);
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = TopicService::new(repo_arc, metrics_arc, true, p2p_arc);

        let result = service
            .list_trending_topics(2)
            .await
            .expect("trending topics");

        assert_eq!(result.generated_at, 200);
        assert_eq!(result.data_source, TrendingDataSource::Metrics);
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].topic.id, "alpha");
        assert_eq!(result.entries[0].trending_score, 42.0);
    }

    #[tokio::test]
    async fn test_get_joined_topics_passes_user_pubkey() {
        let mut repo = MockTopicRepo::new();
        repo.expect_get_joined_topics()
            .with(eq("pubkey1"))
            .times(1)
            .returning(|_| Ok(vec![]));
        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> =
            Arc::new(MockTopicMetricsRepo::new());
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = TopicService::new(repo_arc, metrics_arc, false, p2p_arc);
        let result = service.get_joined_topics("pubkey1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn latest_metrics_generated_at_handles_disabled_metrics() {
        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(MockTopicRepo::new());
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> =
            Arc::new(MockTopicMetricsRepo::new());
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = TopicService::new(repo_arc, metrics_arc, false, p2p_arc);

        assert!(
            service
                .latest_metrics_generated_at()
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn latest_metrics_generated_at_reads_from_repo() {
        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(MockTopicRepo::new());
        let mut metrics_repo = MockTopicMetricsRepo::new();
        metrics_repo
            .expect_latest_window_end()
            .return_once(|| Ok(Some(999)));
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> = Arc::new(metrics_repo);
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = TopicService::new(repo_arc, metrics_arc, true, p2p_arc);

        assert_eq!(
            service.latest_metrics_generated_at().await.unwrap(),
            Some(999)
        );
    }
}
