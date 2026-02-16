CREATE TABLE IF NOT EXISTS cn_search.community_search_terms (
    community_id TEXT NOT NULL,
    term_type TEXT NOT NULL CHECK (term_type IN ('name', 'alias')),
    term_raw TEXT NOT NULL,
    term_norm TEXT NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (community_id, term_type, term_norm)
);

CREATE INDEX IF NOT EXISTS community_search_terms_trgm_idx
    ON cn_search.community_search_terms
    USING gin (term_norm gin_trgm_ops);

CREATE INDEX IF NOT EXISTS community_search_terms_prefix_idx
    ON cn_search.community_search_terms (term_norm text_pattern_ops);

WITH source_topics AS (
    SELECT topic_id
    FROM cn_admin.node_subscriptions
    UNION
    SELECT topic_id
    FROM cn_user.topic_subscriptions
    WHERE status = 'active'
),
normalized_terms AS (
    SELECT
        topic_id,
        TRIM(REGEXP_REPLACE(LOWER(topic_id), '[^[:alnum:]#@]+', ' ', 'g')) AS name_norm,
        TRIM(
            REGEXP_REPLACE(
                LOWER(REGEXP_REPLACE(topic_id, '^kukuri:(tauri:)?', '')),
                '[^[:alnum:]#@]+',
                ' ',
                'g'
            )
        ) AS alias_norm
    FROM source_topics
)
INSERT INTO cn_search.community_search_terms
    (community_id, term_type, term_raw, term_norm, is_primary)
SELECT
    topic_id,
    'name',
    topic_id,
    name_norm,
    TRUE
FROM normalized_terms
WHERE name_norm <> ''
ON CONFLICT (community_id, term_type, term_norm) DO NOTHING;

WITH source_topics AS (
    SELECT topic_id
    FROM cn_admin.node_subscriptions
    UNION
    SELECT topic_id
    FROM cn_user.topic_subscriptions
    WHERE status = 'active'
),
normalized_terms AS (
    SELECT
        topic_id,
        TRIM(REGEXP_REPLACE(LOWER(topic_id), '[^[:alnum:]#@]+', ' ', 'g')) AS name_norm,
        TRIM(
            REGEXP_REPLACE(
                LOWER(REGEXP_REPLACE(topic_id, '^kukuri:(tauri:)?', '')),
                '[^[:alnum:]#@]+',
                ' ',
                'g'
            )
        ) AS alias_norm
    FROM source_topics
)
INSERT INTO cn_search.community_search_terms
    (community_id, term_type, term_raw, term_norm, is_primary)
SELECT
    topic_id,
    'alias',
    topic_id,
    alias_norm,
    TRUE
FROM normalized_terms
WHERE alias_norm <> ''
  AND alias_norm <> name_norm
ON CONFLICT (community_id, term_type, term_norm) DO NOTHING;
