CREATE TABLE IF NOT EXISTS topic_metrics (
    topic_id TEXT NOT NULL,
    window_start INTEGER NOT NULL,
    window_end INTEGER NOT NULL,
    posts_24h INTEGER NOT NULL DEFAULT 0,
    posts_6h INTEGER NOT NULL DEFAULT 0,
    unique_authors INTEGER NOT NULL DEFAULT 0,
    boosts INTEGER NOT NULL DEFAULT 0,
    replies INTEGER NOT NULL DEFAULT 0,
    bookmarks INTEGER NOT NULL DEFAULT 0,
    participant_delta INTEGER NOT NULL DEFAULT 0,
    score_24h REAL NOT NULL DEFAULT 0,
    score_6h REAL NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (topic_id, window_start)
);

CREATE INDEX IF NOT EXISTS idx_topic_metrics_window_end
    ON topic_metrics (window_end);

CREATE INDEX IF NOT EXISTS idx_topic_metrics_updated_at
    ON topic_metrics (updated_at);
