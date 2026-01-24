CREATE EXTENSION IF NOT EXISTS age;

CREATE SCHEMA IF NOT EXISTS cn_trust;

CREATE TABLE IF NOT EXISTS cn_trust.report_events (
    event_id TEXT PRIMARY KEY,
    subject_pubkey TEXT NOT NULL,
    reporter_pubkey TEXT NULL,
    target TEXT NOT NULL,
    reason TEXT NULL,
    label TEXT NULL,
    confidence DOUBLE PRECISION NULL,
    label_exp BIGINT NULL,
    source_kind INT NOT NULL,
    topic_id TEXT NULL,
    created_at BIGINT NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS trust_report_events_subject_idx
    ON cn_trust.report_events (subject_pubkey, created_at DESC);

CREATE INDEX IF NOT EXISTS trust_report_events_kind_idx
    ON cn_trust.report_events (source_kind, created_at DESC);

CREATE TABLE IF NOT EXISTS cn_trust.interactions (
    event_id TEXT NOT NULL,
    actor_pubkey TEXT NOT NULL,
    target_pubkey TEXT NOT NULL,
    weight DOUBLE PRECISION NOT NULL,
    topic_id TEXT NULL,
    created_at BIGINT NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, target_pubkey)
);

CREATE INDEX IF NOT EXISTS trust_interactions_actor_idx
    ON cn_trust.interactions (actor_pubkey, created_at DESC);

CREATE INDEX IF NOT EXISTS trust_interactions_target_idx
    ON cn_trust.interactions (target_pubkey, created_at DESC);

CREATE TABLE IF NOT EXISTS cn_trust.report_scores (
    subject_pubkey TEXT PRIMARY KEY,
    score DOUBLE PRECISION NOT NULL,
    report_count BIGINT NOT NULL,
    label_count BIGINT NOT NULL,
    window_start BIGINT NOT NULL,
    window_end BIGINT NOT NULL,
    attestation_id TEXT NULL,
    attestation_exp BIGINT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_trust.communication_scores (
    subject_pubkey TEXT PRIMARY KEY,
    score DOUBLE PRECISION NOT NULL,
    interaction_count BIGINT NOT NULL,
    peer_count BIGINT NOT NULL,
    window_start BIGINT NOT NULL,
    window_end BIGINT NOT NULL,
    attestation_id TEXT NULL,
    attestation_exp BIGINT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_trust.attestations (
    attestation_id TEXT PRIMARY KEY,
    subject TEXT NOT NULL,
    claim TEXT NOT NULL,
    score DOUBLE PRECISION NOT NULL,
    exp BIGINT NOT NULL,
    topic_id TEXT NULL,
    issuer_pubkey TEXT NOT NULL,
    value_json JSONB NULL,
    evidence_json JSONB NULL,
    context_json JSONB NULL,
    event_json JSONB NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS trust_attestations_subject_claim_idx
    ON cn_trust.attestations (subject, claim, exp DESC);

CREATE INDEX IF NOT EXISTS trust_attestations_exp_idx
    ON cn_trust.attestations (exp DESC);

CREATE TABLE IF NOT EXISTS cn_trust.jobs (
    job_id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,
    subject_pubkey TEXT NULL,
    status TEXT NOT NULL,
    total_targets BIGINT NULL,
    processed_targets BIGINT NOT NULL DEFAULT 0,
    requested_by TEXT NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    error_message TEXT NULL,
    params_json JSONB NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS trust_jobs_status_idx
    ON cn_trust.jobs (status, requested_at DESC);

CREATE INDEX IF NOT EXISTS trust_jobs_type_idx
    ON cn_trust.jobs (job_type, requested_at DESC);

CREATE TABLE IF NOT EXISTS cn_trust.job_schedules (
    job_type TEXT PRIMARY KEY,
    interval_seconds BIGINT NOT NULL,
    next_run_at TIMESTAMPTZ NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS trust_job_schedules_next_idx
    ON cn_trust.job_schedules (next_run_at);
