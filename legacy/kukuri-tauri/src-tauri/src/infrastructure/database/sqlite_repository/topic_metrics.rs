use super::SqliteRepository;
use super::queries::{
    CLEANUP_TOPIC_METRICS, COLLECT_TOPIC_ACTIVITY, SELECT_LATEST_METRICS_WINDOW_END,
    SELECT_METRICS_BY_WINDOW, UPSERT_TOPIC_METRICS,
};
use crate::application::ports::repositories::TopicMetricsRepository;
use crate::domain::entities::{
    MetricsWindow, TopicActivityRow, TopicMetricsRecord, TopicMetricsSnapshot, TopicMetricsUpsert,
};
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

#[derive(Debug, FromRow)]
struct DbTopicMetricsRow {
    topic_id: String,
    window_start: i64,
    window_end: i64,
    posts_24h: i64,
    posts_6h: i64,
    unique_authors: i64,
    boosts: i64,
    replies: i64,
    bookmarks: i64,
    participant_delta: i64,
    score_24h: f64,
    score_6h: f64,
    updated_at: i64,
}

impl From<DbTopicMetricsRow> for TopicMetricsRecord {
    fn from(value: DbTopicMetricsRow) -> Self {
        Self {
            topic_id: value.topic_id,
            window_start: value.window_start,
            window_end: value.window_end,
            posts_24h: value.posts_24h,
            posts_6h: value.posts_6h,
            unique_authors: value.unique_authors,
            boosts: value.boosts,
            replies: value.replies,
            bookmarks: value.bookmarks,
            participant_delta: value.participant_delta,
            score_24h: value.score_24h,
            score_6h: value.score_6h,
            updated_at: value.updated_at,
        }
    }
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

    async fn latest_window_end(&self) -> Result<Option<i64>, AppError> {
        let result: Option<(i64,)> = sqlx::query_as(SELECT_LATEST_METRICS_WINDOW_END)
            .fetch_optional(self.pool.get_pool())
            .await?;
        Ok(result.map(|row| row.0))
    }

    async fn list_recent_metrics(
        &self,
        limit: usize,
    ) -> Result<Option<TopicMetricsSnapshot>, AppError> {
        let Some(window_end) = self.latest_window_end().await? else {
            return Ok(None);
        };

        let fetch_limit = if limit == 0 { 1 } else { limit }.min(i64::MAX as usize);
        let rows: Vec<DbTopicMetricsRow> = sqlx::query_as(SELECT_METRICS_BY_WINDOW)
            .bind(window_end)
            .bind(fetch_limit as i64)
            .fetch_all(self.pool.get_pool())
            .await?;

        if rows.is_empty() {
            return Ok(Some(TopicMetricsSnapshot {
                window_start: window_end,
                window_end,
                metrics: Vec::new(),
            }));
        }

        let window_start = rows
            .first()
            .map(|row| row.window_start)
            .unwrap_or(window_end);

        let metrics = if limit == 0 {
            Vec::new()
        } else {
            rows.into_iter()
                .take(limit)
                .map(TopicMetricsRecord::from)
                .collect()
        };

        Ok(Some(TopicMetricsSnapshot {
            window_start,
            window_end,
            metrics,
        }))
    }
}
