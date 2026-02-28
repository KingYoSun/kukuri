use super::offline_service::{OfflineServiceTrait, SaveOfflineActionParams};
use super::p2p_service::P2PServiceTrait;
use crate::application::ports::repositories::{
    PendingTopicRepository, TopicMetricsRepository, TopicRepository,
};
use crate::domain::constants::DEFAULT_PUBLIC_TOPIC_ID;
use crate::domain::entities::offline::OfflineActionRecord;
use crate::domain::entities::{
    PendingTopic, PendingTopicStatus, Topic, TopicMetricsRecord, TopicVisibility,
};
use crate::domain::value_objects::event_gateway::PublicKey;
use crate::domain::value_objects::offline::{
    EntityId, EntityType, OfflineActionType, OfflinePayload,
};
use crate::shared::{ValidationFailureKind, error::AppError};
use chrono::Utc;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

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

pub struct EnqueuedTopicCreation {
    pub pending_topic: PendingTopic,
    pub offline_action: OfflineActionRecord,
}

pub struct TopicService {
    repository: Arc<dyn TopicRepository>,
    pending_repository: Arc<dyn PendingTopicRepository>,
    metrics_repository: Arc<dyn TopicMetricsRepository>,
    metrics_enabled: bool,
    p2p: Arc<dyn P2PServiceTrait>,
    offline_service: Arc<dyn OfflineServiceTrait>,
}

impl TopicService {
    pub fn new(
        repository: Arc<dyn TopicRepository>,
        pending_repository: Arc<dyn PendingTopicRepository>,
        metrics_repository: Arc<dyn TopicMetricsRepository>,
        metrics_enabled: bool,
        p2p: Arc<dyn P2PServiceTrait>,
        offline_service: Arc<dyn OfflineServiceTrait>,
    ) -> Self {
        Self {
            repository,
            pending_repository,
            metrics_repository,
            metrics_enabled,
            p2p,
            offline_service,
        }
    }

    pub async fn create_topic(
        &self,
        name: String,
        description: Option<String>,
        visibility: TopicVisibility,
        creator_pubkey: &str,
    ) -> Result<Topic, AppError> {
        let mut topic = Topic::new(name, description);
        topic.visibility = visibility;
        self.repository.create_topic(&topic).await?;
        self.join_topic(&topic.id, creator_pubkey).await?;

        if let Some(mut stored) = self.get_topic(&topic.id).await? {
            stored.is_joined = true;
            return Ok(stored);
        }

        topic.is_joined = true;
        topic.member_count = topic.member_count.saturating_add(1);
        Ok(topic)
    }

    pub async fn get_topic(&self, id: &str) -> Result<Option<Topic>, AppError> {
        self.repository.get_topic(id).await
    }

    pub async fn get_all_topics(&self) -> Result<Vec<Topic>, AppError> {
        self.repository.get_all_topics().await
    }

    pub async fn list_topics_with_membership(
        &self,
        user_pubkey: Option<&str>,
    ) -> Result<Vec<Topic>, AppError> {
        let mut topics = self.repository.get_all_topics().await?;

        if let Some(pubkey) = user_pubkey {
            let joined = self.repository.get_joined_topics(pubkey).await?;
            let joined_ids: HashSet<String> = joined.into_iter().map(|topic| topic.id).collect();

            for topic in topics.iter_mut() {
                if joined_ids.contains(&topic.id) {
                    topic.is_joined = true;
                }
            }
        }

        Ok(topics)
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
        if id == DEFAULT_PUBLIC_TOPIC_ID {
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
        if self
            .repository
            .get_topic(DEFAULT_PUBLIC_TOPIC_ID)
            .await?
            .is_none()
        {
            let public_topic = Topic::public_topic();
            self.repository.create_topic(&public_topic).await?;
        }
        Ok(())
    }

    pub async fn enqueue_topic_creation(
        &self,
        user_pubkey: &str,
        name: String,
        description: Option<String>,
        visibility: TopicVisibility,
    ) -> Result<EnqueuedTopicCreation, AppError> {
        let public_key = PublicKey::from_hex_str(user_pubkey).map_err(|err| {
            AppError::validation(
                ValidationFailureKind::Generic,
                format!("Invalid pubkey: {err}"),
            )
        })?;

        let pending_id = Uuid::new_v4().to_string();
        let payload = OfflinePayload::new(json!({
            "pendingId": pending_id,
            "name": name,
            "description": description,
            "visibility": visibility.as_str(),
        }))
        .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
        let action_type = OfflineActionType::new("topic_create".to_string())
            .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
        let entity_type = EntityType::new("topic".to_string())
            .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;
        let entity_id = EntityId::new(pending_id.clone())
            .map_err(AppError::validation_mapper(ValidationFailureKind::Generic))?;

        let saved = self
            .offline_service
            .save_action(SaveOfflineActionParams {
                user_pubkey: public_key.clone(),
                action_type,
                entity_type,
                entity_id,
                payload,
            })
            .await?;

        let now = Utc::now();
        let pending_topic = PendingTopic::new(
            pending_id,
            public_key.as_hex().to_string(),
            name,
            description,
            PendingTopicStatus::Queued,
            saved.local_id.to_string(),
            None,
            None,
            now,
            now,
        );

        self.pending_repository
            .insert_pending_topic(&pending_topic)
            .await?;

        Ok(EnqueuedTopicCreation {
            pending_topic,
            offline_action: saved.action,
        })
    }

    pub async fn list_pending_topics(
        &self,
        user_pubkey: &str,
    ) -> Result<Vec<PendingTopic>, AppError> {
        self.pending_repository
            .list_pending_topics(user_pubkey)
            .await
    }

    pub async fn get_pending_topic(
        &self,
        pending_id: &str,
    ) -> Result<Option<PendingTopic>, AppError> {
        self.pending_repository.get_pending_topic(pending_id).await
    }

    pub async fn mark_pending_topic_synced(
        &self,
        pending_id: &str,
        topic_id: &str,
    ) -> Result<(), AppError> {
        self.pending_repository
            .update_pending_topic_status(
                pending_id,
                PendingTopicStatus::Synced,
                Some(topic_id),
                None,
            )
            .await
    }

    pub async fn mark_pending_topic_failed(
        &self,
        pending_id: &str,
        error_message: Option<String>,
    ) -> Result<(), AppError> {
        self.pending_repository
            .update_pending_topic_status(
                pending_id,
                PendingTopicStatus::Failed,
                None,
                error_message.as_deref(),
            )
            .await
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

        if self.metrics_enabled
            && let Some(snapshot) = self.metrics_repository.list_recent_metrics(limit).await?
        {
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
        PendingTopicRepository as PortPendingTopicRepository,
        TopicMetricsRepository as PortTopicMetricsRepository,
        TopicRepository as PortTopicRepository,
    };
    use crate::application::services::offline_service::{
        OfflineActionsQuery, OfflineServiceTrait, SaveOfflineActionParams,
    };
    use crate::application::services::p2p_service::{P2PServiceTrait, P2PStatus};
    use crate::domain::constants::DEFAULT_PUBLIC_TOPIC_ID;
    use crate::domain::entities::offline::{
        CacheMetadataUpdate, CacheStatusSnapshot, OfflineActionRecord, OptimisticUpdateDraft,
        SavedOfflineAction, SyncQueueItem, SyncQueueItemDraft, SyncResult, SyncStatusUpdate,
    };
    use crate::domain::entities::{
        MetricsWindow, PendingTopic, TopicActivityRow, TopicMetricsSnapshot, TopicMetricsUpsert,
        TopicVisibility,
    };
    use crate::domain::value_objects::event_gateway::PublicKey;
    use crate::domain::value_objects::offline::{OfflinePayload, OptimisticUpdateId, SyncQueueId};
    use crate::shared::config::BootstrapSource;
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
            async fn connect_to_peer(&self, peer_address: &str) -> Result<(), AppError>;
            async fn get_status(&self) -> Result<P2PStatus, AppError>;
            async fn get_node_addresses(&self) -> Result<Vec<String>, AppError>;
            fn generate_topic_id(&self, topic_name: &str) -> String;
            async fn apply_bootstrap_nodes(
                &self,
                nodes: Vec<String>,
                source: BootstrapSource,
            ) -> Result<(), AppError>;
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

    #[derive(Clone)]
    struct NoopPendingRepo;

    #[async_trait]
    impl PortPendingTopicRepository for NoopPendingRepo {
        async fn insert_pending_topic(&self, _topic: &PendingTopic) -> Result<(), AppError> {
            Ok(())
        }

        async fn list_pending_topics(
            &self,
            _user_pubkey: &str,
        ) -> Result<Vec<PendingTopic>, AppError> {
            Ok(vec![])
        }

        async fn get_pending_topic(
            &self,
            _pending_id: &str,
        ) -> Result<Option<PendingTopic>, AppError> {
            Ok(None)
        }

        async fn update_pending_topic_status(
            &self,
            _pending_id: &str,
            _status: PendingTopicStatus,
            _synced_topic_id: Option<&str>,
            _error_message: Option<&str>,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn delete_pending_topic(&self, _pending_id: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct StubOfflineService;

    #[async_trait]
    impl OfflineServiceTrait for StubOfflineService {
        async fn save_action(
            &self,
            _params: SaveOfflineActionParams,
        ) -> Result<SavedOfflineAction, AppError> {
            Err(AppError::NotImplemented("stub offline service".to_string()))
        }

        async fn list_actions(
            &self,
            _query: OfflineActionsQuery,
        ) -> Result<Vec<OfflineActionRecord>, AppError> {
            Ok(vec![])
        }

        async fn sync_actions(&self, _user_pubkey: PublicKey) -> Result<SyncResult, AppError> {
            Err(AppError::NotImplemented("stub offline service".to_string()))
        }

        async fn cache_status(&self) -> Result<CacheStatusSnapshot, AppError> {
            Err(AppError::NotImplemented("stub offline service".to_string()))
        }

        async fn enqueue_sync(&self, _draft: SyncQueueItemDraft) -> Result<SyncQueueId, AppError> {
            Err(AppError::NotImplemented("stub offline service".to_string()))
        }

        async fn recent_sync_queue_items(
            &self,
            _limit: Option<u32>,
        ) -> Result<Vec<SyncQueueItem>, AppError> {
            Ok(vec![])
        }

        async fn upsert_cache_metadata(
            &self,
            _update: CacheMetadataUpdate,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn save_optimistic_update(
            &self,
            _draft: OptimisticUpdateDraft,
        ) -> Result<OptimisticUpdateId, AppError> {
            Err(AppError::NotImplemented("stub offline service".to_string()))
        }

        async fn confirm_optimistic_update(
            &self,
            _update_id: OptimisticUpdateId,
        ) -> Result<(), AppError> {
            Ok(())
        }

        async fn rollback_optimistic_update(
            &self,
            _update_id: OptimisticUpdateId,
        ) -> Result<Option<OfflinePayload>, AppError> {
            Ok(None)
        }

        async fn cleanup_expired_cache(&self) -> Result<u32, AppError> {
            Ok(0)
        }

        async fn update_sync_status(&self, _update: SyncStatusUpdate) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn build_topic_service(
        repo: Arc<dyn PortTopicRepository>,
        metrics_repo: Arc<dyn PortTopicMetricsRepository>,
        p2p: Arc<dyn P2PServiceTrait>,
        metrics_enabled: bool,
    ) -> TopicService {
        let pending_repo: Arc<dyn PortPendingTopicRepository> = Arc::new(NoopPendingRepo);
        let offline_service: Arc<dyn OfflineServiceTrait> = Arc::new(StubOfflineService);
        TopicService::new(
            repo,
            pending_repo,
            metrics_repo,
            metrics_enabled,
            p2p,
            offline_service,
        )
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
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, false);

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
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, false);

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
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, false);

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
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, true);

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
    async fn list_topics_with_membership_marks_joined_flags() {
        let mut repo = MockTopicRepo::new();
        let public = topic_with_counts(DEFAULT_PUBLIC_TOPIC_ID, "Public", 0, 0);
        let private = topic_with_counts("private", "Private", 0, 0);

        let public_for_all = public.clone();
        let private_for_all = private.clone();
        repo.expect_get_all_topics()
            .times(1)
            .returning(move || Ok(vec![public_for_all.clone(), private_for_all.clone()]));
        let public_for_joined = public.clone();
        repo.expect_get_joined_topics()
            .with(eq("pubkey1"))
            .times(1)
            .returning(move |_| Ok(vec![public_for_joined.clone()]));

        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> =
            Arc::new(MockTopicMetricsRepo::new());
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(MockP2P::new());
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, false);

        let topics = service
            .list_topics_with_membership(Some("pubkey1"))
            .await
            .expect("topics with membership");

        assert_eq!(topics.len(), 2);
        assert!(
            topics
                .iter()
                .any(|topic| topic.id == DEFAULT_PUBLIC_TOPIC_ID && topic.is_joined)
        );
        assert!(
            topics
                .iter()
                .any(|topic| topic.id == "private" && !topic.is_joined)
        );
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
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, false);
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
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, false);

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
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, true);

        assert_eq!(
            service.latest_metrics_generated_at().await.unwrap(),
            Some(999)
        );
    }

    #[tokio::test]
    async fn create_topic_returns_joined_topic_with_repo_state() {
        let mut repo = MockTopicRepo::new();
        repo.expect_create_topic().times(1).returning(|_| Ok(()));
        repo.expect_join_topic()
            .times(1)
            .withf(|topic_id, user| !topic_id.is_empty() && user == "creator")
            .return_once(|_, _| Ok(()));
        repo.expect_get_topic().times(1).returning(|id| {
            let mut topic = Topic::new("My Topic".to_string(), Some("desc".to_string()));
            topic.id = id.to_string();
            topic.member_count = 1;
            topic.visibility = TopicVisibility::Public;
            Ok(Some(topic))
        });

        let repo_arc: Arc<dyn PortTopicRepository> = Arc::new(repo);
        let metrics_arc: Arc<dyn PortTopicMetricsRepository> =
            Arc::new(MockTopicMetricsRepo::new());
        let mut p2p = MockP2P::new();
        p2p.expect_join_topic()
            .times(1)
            .withf(|topic_id, peers| !topic_id.is_empty() && peers.is_empty())
            .return_once(|_, _| Ok(()));
        let p2p_arc: Arc<dyn P2PServiceTrait> = Arc::new(p2p);
        let service = build_topic_service(repo_arc, metrics_arc, p2p_arc, false);

        let topic = service
            .create_topic(
                "My Topic".to_string(),
                Some("desc".to_string()),
                TopicVisibility::Public,
                "creator",
            )
            .await
            .expect("topic created");

        assert!(topic.is_joined);
        assert!(topic.member_count >= 1);
    }
}
