CREATE TABLE object_index_cache_revert (
    object_id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    author_pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    root_object_id TEXT,
    reply_to_object_id TEXT,
    payload_ref_json TEXT NOT NULL,
    content TEXT,
    source_replica_id TEXT NOT NULL,
    source_key TEXT NOT NULL,
    source_envelope_id TEXT NOT NULL,
    source_blob_hash TEXT,
    derived_at INTEGER NOT NULL,
    projection_version INTEGER NOT NULL,
    channel_id TEXT NOT NULL DEFAULT 'public'
);

INSERT INTO object_index_cache_revert (
    object_id,
    topic_id,
    author_pubkey,
    created_at,
    root_object_id,
    reply_to_object_id,
    payload_ref_json,
    content,
    source_replica_id,
    source_key,
    source_envelope_id,
    source_blob_hash,
    derived_at,
    projection_version,
    channel_id
)
SELECT
    object_id,
    topic_id,
    author_pubkey,
    created_at,
    root_object_id,
    reply_to_object_id,
    payload_ref_json,
    content,
    source_replica_id,
    source_key,
    source_envelope_id,
    source_blob_hash,
    derived_at,
    projection_version,
    channel_id
FROM object_index_cache;

DROP TABLE object_index_cache;

ALTER TABLE object_index_cache_revert RENAME TO object_index_cache;

DROP INDEX IF EXISTS idx_object_index_cache_topic_created;
CREATE INDEX IF NOT EXISTS idx_object_index_cache_topic_created
    ON object_index_cache(topic_id, channel_id, created_at DESC, object_id DESC);
