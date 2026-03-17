CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    tags_json TEXT NOT NULL,
    sig TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS topic_posts (
    topic_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (topic_id, event_id),
    FOREIGN KEY (event_id) REFERENCES events (event_id)
);

CREATE INDEX IF NOT EXISTS idx_topic_posts_timeline
    ON topic_posts (topic_id, created_at DESC, event_id DESC);

CREATE TABLE IF NOT EXISTS thread_edges (
    topic_id TEXT NOT NULL,
    event_id TEXT PRIMARY KEY,
    root_event_id TEXT NOT NULL,
    parent_event_id TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (event_id) REFERENCES events (event_id)
);

CREATE INDEX IF NOT EXISTS idx_thread_edges_root
    ON thread_edges (topic_id, root_event_id, created_at ASC, event_id ASC);

CREATE TABLE IF NOT EXISTS profiles (
    pubkey TEXT PRIMARY KEY,
    name TEXT,
    display_name TEXT,
    about TEXT,
    picture TEXT,
    updated_at INTEGER NOT NULL
);
