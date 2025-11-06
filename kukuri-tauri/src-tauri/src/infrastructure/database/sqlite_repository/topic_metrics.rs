use super::SqliteRepository;
use super::queries::{CLEANUP_TOPIC_METRICS, COLLECT_TOPIC_ACTIVITY, UPSERT_TOPIC_METRICS};
use crate::application::ports::repositories::TopicMetricsRepository;
use crate::domain::entities::{MetricsWindow, TopicActivityRow, TopicMetricsUpsert};
use crate::shared::error::AppError;
use async_trait::async_trait;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
struct DbTopicActivityRow {
    topic_id: String,
    posts_count: i64,
    unique_authors: i64,
    boosts: i64,
    replies: i64,
    bookmarks: i64,
    participant_delta: i64,
}

#[async_trait]
impl TopicMetricsRepository for SqliteRepository {
    async fn upsert_metrics(&self, metrics: TopicMetricsUpsert) -> Result<(), AppError> {
        sqlx::query(UPSERT_TOPIC_METRICS)
            .bind(&metrics.topic_id)
            .bind(metrics.window_start)
            .bind(metrics.window_end)
            .bind(metrics.posts_24h)
            .bind(metrics.posts_6h)
            .bind(metrics.unique_authors)
            .bind(metrics.boosts)
            .bind(metrics.replies)
            .bind(metrics.bookmarks)
            .bind(metrics.participant_delta)
            .bind(metrics.score_24h)
            .bind(metrics.score_6h)
            .bind(metrics.updated_at)
            .execute(self.pool.get_pool())
            .await?;

        Ok(())
    }

    async fn cleanup_expired(&self, cutoff_millis: i64) -> Result<u64, AppError> {
        let result = sqlx::query(CLEANUP_TOPIC_METRICS)
            .bind(cutoff_millis)
            .execute(self.pool.get_pool())
            .await?;

        Ok(result.rows_affected())
    }

    async fn collect_activity(
        &self,
        window: MetricsWindow,
    ) -> Result<Vec<TopicActivityRow>, AppError> {
        let rows: Vec<DbTopicActivityRow> = sqlx::query_as(COLLECT_TOPIC_ACTIVITY)
            .bind(window.start)
            .bind(window.end)
            .fetch_all(self.pool.get_pool())
            .await?;

        let mut activities = Vec::with_capacity(rows.len());
        for row in rows {
            activities.push(TopicActivityRow {
                topic_id: row.topic_id,
                posts_count: row.posts_count,
                unique_authors: row.unique_authors,
                boosts: row.boosts,
                replies: row.replies,
                bookmarks: row.bookmarks,
                participant_delta: row.participant_delta,
            });
        }

        Ok(activities)
    }
}
