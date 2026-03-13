CREATE SCHEMA IF NOT EXISTS cn_index;

CREATE TABLE IF NOT EXISTS cn_index.reindex_jobs (
    job_id TEXT PRIMARY KEY,
    topic_id TEXT NULL,
    status TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    error_message TEXT NULL,
    total_events BIGINT NULL,
    processed_events BIGINT NOT NULL DEFAULT 0,
    cutoff_seq BIGINT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS reindex_jobs_status_idx
    ON cn_index.reindex_jobs (status, requested_at DESC);

CREATE TABLE IF NOT EXISTS cn_index.expired_events (
    event_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    expired_at BIGINT NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, topic_id)
);
