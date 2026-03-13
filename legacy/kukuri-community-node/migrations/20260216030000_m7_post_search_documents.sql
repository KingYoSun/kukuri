CREATE SCHEMA IF NOT EXISTS cn_search;

CREATE TABLE IF NOT EXISTS cn_search.post_search_documents (
    post_id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    visibility TEXT NOT NULL,
    body_raw TEXT NOT NULL,
    body_norm TEXT NOT NULL,
    hashtags_norm TEXT[] NOT NULL DEFAULT '{}',
    mentions_norm TEXT[] NOT NULL DEFAULT '{}',
    community_terms_norm TEXT[] NOT NULL DEFAULT '{}',
    search_text TEXT NOT NULL,
    language_hint TEXT NULL,
    popularity_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    created_at BIGINT NOT NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    normalizer_version SMALLINT NOT NULL DEFAULT 1,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS post_search_text_pgroonga_idx
    ON cn_search.post_search_documents
    USING pgroonga (search_text);

CREATE INDEX IF NOT EXISTS post_search_topic_created_idx
    ON cn_search.post_search_documents (topic_id, created_at DESC);

CREATE INDEX IF NOT EXISTS post_search_visibility_idx
    ON cn_search.post_search_documents (visibility, is_deleted);
