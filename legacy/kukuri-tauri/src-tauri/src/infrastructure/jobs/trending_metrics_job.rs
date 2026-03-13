use super::trending_metrics_metrics::TrendingMetricsRecorder;
use crate::application::ports::repositories::TopicMetricsRepository;
use crate::domain::entities::{MetricsWindow, ScoreWeights, TopicActivityRow, TopicMetricsUpsert};
use crate::shared::error::AppError;
use chrono::Duration;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Default, Clone)]
struct AggregatedTopicMetrics {
    posts_24h: i64,
    posts_6h: i64,
    unique_authors: i64,
    boosts: i64,
    replies: i64,
    bookmarks: i64,
    participant_delta: i64,
}

pub struct TrendingMetricsJob {
    metrics_repository: Arc<dyn TopicMetricsRepository>,
    score_weights: ScoreWeights,
    ttl_hours: u64,
    metrics_recorder: Option<Arc<TrendingMetricsRecorder>>,
}

#[derive(Debug, Clone, Copy)]
pub struct TrendingMetricsRunStats {
    pub topics_upserted: u64,
    pub expired_records: u64,
    pub cutoff_millis: i64,
    pub window_start_millis: i64,
    pub window_end_millis: i64,
    pub lag_millis: i64,
    pub score_weights: ScoreWeights,
}

impl TrendingMetricsJob {
    pub fn new(
        metrics_repository: Arc<dyn TopicMetricsRepository>,
        score_weights: Option<ScoreWeights>,
        ttl_hours: u64,
        metrics_recorder: Option<Arc<TrendingMetricsRecorder>>,
    ) -> Self {
        Self {
            metrics_repository,
            score_weights: score_weights.unwrap_or_default(),
            ttl_hours,
            metrics_recorder,
        }
    }

    pub async fn run_once(&self) -> Result<(), AppError> {
        let started = Instant::now();
        let result = self.execute_once().await;
        let duration = started.elapsed();
        let duration_ms = duration.as_millis().min(u128::from(u64::MAX)) as u64;

        if let Some(recorder) = &self.metrics_recorder {
            match &result {
                Ok(stats) => recorder.record_success(duration, stats),
                Err(_) => recorder.record_failure(duration),
            }
        }

        if let Ok(stats) = &result {
            tracing::info!(
                target: "metrics::trending",
                topics_upserted = stats.topics_upserted,
                cutoff_millis = stats.cutoff_millis,
                removed_records = stats.expired_records,
                window_start_millis = stats.window_start_millis,
                window_end_millis = stats.window_end_millis,
                lag_millis = stats.lag_millis,
                score_weight_posts = stats.score_weights.posts,
                score_weight_unique_authors = stats.score_weights.unique_authors,
                score_weight_boosts = stats.score_weights.boosts,
                duration_ms,
                "trending metrics job completed"
            );
        }

        result.map(|_| ())
    }

    async fn execute_once(&self) -> Result<TrendingMetricsRunStats, AppError> {
        let now = Utc::now().timestamp_millis();
        let window_24h = MetricsWindow::new(now - Duration::hours(24).num_milliseconds(), now);
        let window_6h = MetricsWindow::new(now - Duration::hours(6).num_milliseconds(), now);

        let activity_24h = self.metrics_repository.collect_activity(window_24h).await?;
        let activity_6h = self.metrics_repository.collect_activity(window_6h).await?;

        let aggregated = merge_activity(activity_24h, activity_6h);

        let mut upserted = 0usize;
        for (topic_id, metrics) in aggregated {
            let score_24h =
                self.score_weights
                    .score(metrics.posts_24h, metrics.unique_authors, metrics.boosts);
            let score_6h =
                self.score_weights
                    .score(metrics.posts_6h, metrics.unique_authors, metrics.boosts);

            let upsert = TopicMetricsUpsert {
                topic_id,
                window_start: window_24h.start,
                window_end: window_24h.end,
                posts_24h: metrics.posts_24h,
                posts_6h: metrics.posts_6h,
                unique_authors: metrics.unique_authors,
                boosts: metrics.boosts,
                replies: metrics.replies,
                bookmarks: metrics.bookmarks,
                participant_delta: metrics.participant_delta,
                score_24h,
                score_6h,
                updated_at: now,
            };

            self.metrics_repository.upsert_metrics(upsert).await?;
            upserted += 1;
        }

        let cutoff = now - (self.ttl_hours as i64 * Duration::hours(1).num_milliseconds());
        let removed = self.metrics_repository.cleanup_expired(cutoff).await?;
        let lag_millis = now.saturating_sub(window_24h.end).max(0);

        Ok(TrendingMetricsRunStats {
            topics_upserted: upserted as u64,
            expired_records: removed,
            cutoff_millis: cutoff,
            window_start_millis: window_24h.start,
            window_end_millis: window_24h.end,
            lag_millis,
            score_weights: self.score_weights,
        })
    }
}

fn merge_activity(
    activity_24h: Vec<TopicActivityRow>,
    activity_6h: Vec<TopicActivityRow>,
) -> HashMap<String, AggregatedTopicMetrics> {
    let mut aggregated: HashMap<String, AggregatedTopicMetrics> = HashMap::new();

    for row in activity_24h {
        let entry = aggregated.entry(row.topic_id.clone()).or_default();
        entry.posts_24h = row.posts_count;
        entry.unique_authors = row.unique_authors;
        entry.boosts = row.boosts;
        entry.replies = row.replies;
        entry.bookmarks = row.bookmarks;
        entry.participant_delta = row.participant_delta;
    }

    for row in activity_6h {
        let entry = aggregated.entry(row.topic_id.clone()).or_default();
        entry.posts_6h = row.posts_count;

        if entry.unique_authors == 0 {
            entry.unique_authors = row.unique_authors;
        }
        if entry.boosts == 0 {
            entry.boosts = row.boosts;
        }
        if entry.replies == 0 {
            entry.replies = row.replies;
        }
        if entry.bookmarks == 0 {
            entry.bookmarks = row.bookmarks;
        }
        if entry.participant_delta == 0 {
            entry.participant_delta = row.participant_delta;
        }
    }

    aggregated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_activity_prefers_24h_metrics() {
        let topic = "topic-1";
        let activity_24h = vec![TopicActivityRow {
            topic_id: topic.to_string(),
            posts_count: 12,
            unique_authors: 5,
            boosts: 3,
            replies: 1,
            bookmarks: 2,
            participant_delta: 1,
        }];

        let activity_6h = vec![TopicActivityRow {
            topic_id: topic.to_string(),
            posts_count: 4,
            unique_authors: 2,
            boosts: 1,
            replies: 0,
            bookmarks: 0,
            participant_delta: 0,
        }];

        let merged = merge_activity(activity_24h, activity_6h);
        let metrics = merged.get(topic).expect("metrics");
        assert_eq!(metrics.posts_24h, 12);
        assert_eq!(metrics.posts_6h, 4);
        assert_eq!(metrics.unique_authors, 5);
        assert_eq!(metrics.boosts, 3);
        assert_eq!(metrics.replies, 1);
        assert_eq!(metrics.bookmarks, 2);
    }

    #[test]
    fn merge_activity_falls_back_to_6h_when_24h_missing() {
        let topic = "topic-2";
        let activity_6h = vec![TopicActivityRow {
            topic_id: topic.to_string(),
            posts_count: 2,
            unique_authors: 1,
            boosts: 0,
            replies: 0,
            bookmarks: 0,
            participant_delta: 0,
        }];

        let merged = merge_activity(vec![], activity_6h);
        let metrics = merged.get(topic).expect("metrics");
        assert_eq!(metrics.posts_24h, 0);
        assert_eq!(metrics.posts_6h, 2);
        assert_eq!(metrics.unique_authors, 1);
    }
}
