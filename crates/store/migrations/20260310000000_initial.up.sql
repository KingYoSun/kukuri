CREATE TABLE IF NOT EXISTS envelopes (
    envelope_id TEXT PRIMARY KEY,
    pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    tags_json TEXT NOT NULL,
    sig TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS topic_objects (
    topic_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (topic_id, object_id),
    FOREIGN KEY (object_id) REFERENCES envelopes (envelope_id)
);

CREATE INDEX IF NOT EXISTS idx_topic_objects_timeline
    ON topic_objects (topic_id, created_at DESC, object_id DESC);

CREATE TABLE IF NOT EXISTS object_threads (
    topic_id TEXT NOT NULL,
    object_id TEXT PRIMARY KEY,
    root_object_id TEXT NOT NULL,
    reply_to_object_id TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (object_id) REFERENCES envelopes (envelope_id)
);

CREATE INDEX IF NOT EXISTS idx_object_threads_root
    ON object_threads (topic_id, root_object_id, created_at ASC, object_id ASC);

CREATE TABLE IF NOT EXISTS profiles (
    pubkey TEXT PRIMARY KEY,
    name TEXT,
    display_name TEXT,
    about TEXT,
    picture TEXT,
    updated_at INTEGER NOT NULL
);
