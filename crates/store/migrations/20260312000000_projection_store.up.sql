CREATE TABLE IF NOT EXISTS object_index_cache (
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
    projection_version INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_object_index_cache_topic_created
    ON object_index_cache(topic_id, created_at DESC, object_id DESC);

CREATE TABLE IF NOT EXISTS object_thread_cache (
    object_id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    root_object_id TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_object_thread_cache_topic_root_created
    ON object_thread_cache(topic_id, root_object_id, created_at ASC, object_id ASC);

CREATE TABLE IF NOT EXISTS profile_cache (
    pubkey TEXT PRIMARY KEY,
    name TEXT,
    display_name TEXT,
    about TEXT,
    picture TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS blob_objects (
    blob_hash TEXT PRIMARY KEY,
    status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS replica_cursors (
    replica_id TEXT PRIMARY KEY,
    cursor TEXT
);

CREATE TABLE IF NOT EXISTS ui_state (
    state_key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS download_jobs (
    blob_hash TEXT PRIMARY KEY,
    status TEXT NOT NULL
);
