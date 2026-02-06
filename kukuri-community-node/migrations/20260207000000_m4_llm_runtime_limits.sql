CREATE TABLE IF NOT EXISTS cn_moderation.llm_daily_usage (
    usage_day DATE PRIMARY KEY,
    requests_count BIGINT NOT NULL DEFAULT 0 CHECK (requests_count >= 0),
    estimated_cost DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (estimated_cost >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS llm_daily_usage_updated_idx
    ON cn_moderation.llm_daily_usage (updated_at DESC);

CREATE TABLE IF NOT EXISTS cn_moderation.llm_inflight (
    request_id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    estimated_cost DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (estimated_cost >= 0),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS llm_inflight_expires_idx
    ON cn_moderation.llm_inflight (expires_at);
