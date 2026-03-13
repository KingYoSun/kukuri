use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MetricsWindow {
    pub start: i64,
    pub end: i64,
}

impl MetricsWindow {
    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }

    pub fn duration_millis(&self) -> i64 {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ScoreWeights {
    pub posts: f64,
    pub unique_authors: f64,
    pub boosts: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            posts: 0.6,
            unique_authors: 0.3,
            boosts: 0.1,
        }
    }
}

impl ScoreWeights {
    pub fn score(&self, posts: i64, unique_authors: i64, boosts: i64) -> f64 {
        (posts as f64 * self.posts)
            + (unique_authors as f64 * self.unique_authors)
            + (boosts as f64 * self.boosts)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicActivityRow {
    pub topic_id: String,
    pub posts_count: i64,
    pub unique_authors: i64,
    pub boosts: i64,
    pub replies: i64,
    pub bookmarks: i64,
    pub participant_delta: i64,
}

impl TopicActivityRow {
    pub fn empty(topic_id: impl Into<String>) -> Self {
        Self {
            topic_id: topic_id.into(),
            posts_count: 0,
            unique_authors: 0,
            boosts: 0,
            replies: 0,
            bookmarks: 0,
            participant_delta: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicMetricsUpsert {
    pub topic_id: String,
    pub window_start: i64,
    pub window_end: i64,
    pub posts_24h: i64,
    pub posts_6h: i64,
    pub unique_authors: i64,
    pub boosts: i64,
    pub replies: i64,
    pub bookmarks: i64,
    pub participant_delta: i64,
    pub score_24h: f64,
    pub score_6h: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicMetricsRecord {
    pub topic_id: String,
    pub window_start: i64,
    pub window_end: i64,
    pub posts_24h: i64,
    pub posts_6h: i64,
    pub unique_authors: i64,
    pub boosts: i64,
    pub replies: i64,
    pub bookmarks: i64,
    pub participant_delta: i64,
    pub score_24h: f64,
    pub score_6h: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicMetricsSnapshot {
    pub window_start: i64,
    pub window_end: i64,
    pub metrics: Vec<TopicMetricsRecord>,
}
