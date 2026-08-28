CREATE TABLE cn_metaverse.dome_blob_cache (
    blob_hash TEXT PRIMARY KEY,
    bytes BIGINT NOT NULL CHECK (bytes >= 0),
    data BYTEA NOT NULL,
    last_accessed_at BIGINT NOT NULL,
    unreferenced_at BIGINT
);

CREATE TABLE cn_metaverse.dome_blob_pins (
    blob_hash TEXT NOT NULL REFERENCES cn_metaverse.dome_blob_cache(blob_hash) ON DELETE CASCADE,
    reason TEXT NOT NULL CHECK (reason IN ('current', 'active_lease', 'staging', 'rollback')),
    reference_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (blob_hash, reason, reference_id)
);

CREATE INDEX idx_dome_blob_cache_gc
    ON cn_metaverse.dome_blob_cache (unreferenced_at, last_accessed_at)
    WHERE unreferenced_at IS NOT NULL;

CREATE INDEX idx_dome_blob_pins_reference
    ON cn_metaverse.dome_blob_pins (reference_id, reason);
