CREATE SCHEMA IF NOT EXISTS cn_search;

CREATE TABLE IF NOT EXISTS cn_search.graph_sync_offsets (
    consumer TEXT PRIMARY KEY,
    last_seq BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_search.user_community_affinity (
    user_id TEXT NOT NULL,
    community_id TEXT NOT NULL,
    relation_score DOUBLE PRECISION NOT NULL,
    signals_json JSONB NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, community_id)
);

CREATE INDEX IF NOT EXISTS user_community_affinity_score_idx
    ON cn_search.user_community_affinity (user_id, relation_score DESC, computed_at DESC);
