CREATE SCHEMA IF NOT EXISTS cn_search;

CREATE TABLE IF NOT EXISTS cn_search.backfill_jobs (
    job_id TEXT PRIMARY KEY,
    target TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'failed')),
    high_watermark_seq BIGINT NULL,
    processed_rows BIGINT NOT NULL DEFAULT 0,
    error_message TEXT NULL,
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS backfill_jobs_target_status_idx
    ON cn_search.backfill_jobs (target, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS cn_search.backfill_checkpoints (
    job_id TEXT NOT NULL,
    shard_key TEXT NOT NULL,
    last_cursor TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (job_id, shard_key),
    CONSTRAINT backfill_checkpoints_job_id_fkey
        FOREIGN KEY (job_id) REFERENCES cn_search.backfill_jobs (job_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS cn_search.shadow_read_logs (
    id BIGSERIAL PRIMARY KEY,
    endpoint TEXT NOT NULL,
    user_id TEXT NOT NULL,
    query_norm TEXT NOT NULL,
    meili_ids TEXT[] NOT NULL,
    pg_ids TEXT[] NOT NULL,
    overlap_at_10 DOUBLE PRECISION NOT NULL,
    latency_meili_ms INT NOT NULL,
    latency_pg_ms INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS shadow_read_logs_endpoint_created_idx
    ON cn_search.shadow_read_logs (endpoint, created_at DESC);
