PRAGMA foreign_keys=OFF;

CREATE TABLE cache_metadata_downlevel (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cache_key TEXT NOT NULL UNIQUE,
    cache_type TEXT NOT NULL,
    last_synced_at INTEGER,
    last_accessed_at INTEGER,
    data_version INTEGER DEFAULT 1,
    is_stale INTEGER DEFAULT 0,
    expiry_time INTEGER,
    metadata TEXT
);

INSERT INTO cache_metadata_downlevel (
    id,
    cache_key,
    cache_type,
    last_synced_at,
    last_accessed_at,
    data_version,
    is_stale,
    expiry_time,
    metadata
)
SELECT
    id,
    cache_key,
    cache_type,
    last_synced_at,
    last_accessed_at,
    data_version,
    is_stale,
    expiry_time,
    metadata
FROM cache_metadata;

DROP TABLE cache_metadata;
ALTER TABLE cache_metadata_downlevel RENAME TO cache_metadata;

CREATE INDEX IF NOT EXISTS idx_cache_metadata_type ON cache_metadata(cache_type);
CREATE INDEX IF NOT EXISTS idx_cache_metadata_expiry ON cache_metadata(expiry_time);

PRAGMA foreign_keys=ON;
