CREATE TABLE IF NOT EXISTS live_session_cache (
    session_id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    host_pubkey TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    updated_at INTEGER NOT NULL,
    source_replica_id TEXT NOT NULL,
    source_key TEXT NOT NULL,
    manifest_blob_hash TEXT NOT NULL,
    derived_at INTEGER NOT NULL,
    projection_version INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_session_cache_topic_started
    ON live_session_cache(topic_id, started_at DESC, session_id DESC);

CREATE TABLE IF NOT EXISTS game_room_cache (
    room_id TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL,
    host_pubkey TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL,
    phase_label TEXT,
    scores_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    source_replica_id TEXT NOT NULL,
    source_key TEXT NOT NULL,
    manifest_blob_hash TEXT NOT NULL,
    derived_at INTEGER NOT NULL,
    projection_version INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_game_room_cache_topic_updated
    ON game_room_cache(topic_id, updated_at DESC, room_id DESC);

CREATE TABLE IF NOT EXISTS live_presence_cache (
    topic_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    author_pubkey TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (topic_id, session_id, author_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_live_presence_cache_expiry
    ON live_presence_cache(expires_at ASC);
