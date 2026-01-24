CREATE SCHEMA IF NOT EXISTS cn_moderation;

CREATE TABLE IF NOT EXISTS cn_moderation.rules (
    rule_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INT NOT NULL DEFAULT 0,
    conditions_json JSONB NOT NULL,
    action_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS moderation_rules_enabled_priority_idx
    ON cn_moderation.rules (is_enabled, priority DESC);

CREATE TABLE IF NOT EXISTS cn_moderation.jobs (
    job_id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    source TEXT NOT NULL,
    status TEXT NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    max_attempts INT NOT NULL DEFAULT 3,
    last_error TEXT NULL,
    next_run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS moderation_jobs_event_topic_idx
    ON cn_moderation.jobs (event_id, topic_id);

CREATE INDEX IF NOT EXISTS moderation_jobs_status_idx
    ON cn_moderation.jobs (status, next_run_at);

CREATE TABLE IF NOT EXISTS cn_moderation.labels (
    label_id TEXT PRIMARY KEY,
    source_event_id TEXT NULL,
    target TEXT NOT NULL,
    topic_id TEXT NULL,
    label TEXT NOT NULL,
    confidence DOUBLE PRECISION NULL,
    policy_url TEXT NOT NULL,
    policy_ref TEXT NOT NULL,
    exp BIGINT NOT NULL,
    issuer_pubkey TEXT NOT NULL,
    rule_id TEXT NULL,
    source TEXT NOT NULL,
    label_event_json JSONB NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS moderation_labels_target_idx
    ON cn_moderation.labels (target);

CREATE INDEX IF NOT EXISTS moderation_labels_topic_idx
    ON cn_moderation.labels (topic_id);

CREATE INDEX IF NOT EXISTS moderation_labels_exp_idx
    ON cn_moderation.labels (exp);

CREATE UNIQUE INDEX IF NOT EXISTS moderation_labels_rule_event_idx
    ON cn_moderation.labels (source_event_id, rule_id)
    WHERE source_event_id IS NOT NULL AND rule_id IS NOT NULL;
