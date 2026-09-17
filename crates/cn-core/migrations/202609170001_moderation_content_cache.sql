-- #1060: node-local normalized content assessments, expiring execution claims,
-- and restart-safe shared API reservations. No media, text, credentials or raw responses.
CREATE TABLE cn_safety.content_scan_cache (
    cache_key TEXT PRIMARY KEY CHECK (cache_key ~ '^[0-9a-f]{64}$'),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    scan_results JSONB NOT NULL CHECK (jsonb_typeof(scan_results) = 'array'),
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (jsonb_array_length(scan_results) BETWEEN 1 AND 16),
    CHECK (octet_length(scan_results::text) <= 65536)
);
CREATE TABLE cn_safety.content_scan_claims (
    cache_key TEXT PRIMARY KEY CHECK (cache_key ~ '^[0-9a-f]{64}$'),
    owner UUID NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_cn_content_scan_claim_expiry ON cn_safety.content_scan_claims (expires_at);

CREATE TABLE cn_safety.moderation_budget_reservations (
    id BIGSERIAL PRIMARY KEY,
    budget_key TEXT NOT NULL,
    tokens INTEGER NOT NULL CHECK (tokens > 0 AND tokens <= 10000),
    admitted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX idx_cn_moderation_budget_window
    ON cn_safety.moderation_budget_reservations (budget_key, admitted_at);
