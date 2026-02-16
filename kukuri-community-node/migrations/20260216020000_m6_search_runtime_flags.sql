CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS pgroonga;
CREATE EXTENSION IF NOT EXISTS age;

CREATE SCHEMA IF NOT EXISTS cn_search;

CREATE TABLE IF NOT EXISTS cn_search.runtime_flags (
    flag_name TEXT PRIMARY KEY,
    flag_value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT NOT NULL
);

INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by)
VALUES
    ('search_read_backend', 'meili', 'migration'),
    ('search_write_mode', 'meili_only', 'migration'),
    ('suggest_read_backend', 'legacy', 'migration'),
    ('shadow_sample_rate', '0', 'migration')
ON CONFLICT (flag_name) DO NOTHING;
