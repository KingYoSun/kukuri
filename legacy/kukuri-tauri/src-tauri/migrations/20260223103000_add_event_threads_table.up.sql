CREATE TABLE IF NOT EXISTS event_threads (
    event_id TEXT PRIMARY KEY NOT NULL,
    topic_id TEXT NOT NULL,
    thread_namespace TEXT NOT NULL,
    thread_uuid TEXT NOT NULL,
    root_event_id TEXT NOT NULL,
    parent_event_id TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (event_id) REFERENCES events(event_id)
);

CREATE INDEX IF NOT EXISTS idx_event_threads_topic_thread
    ON event_threads (topic_id, thread_uuid, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_event_threads_root_event_id
    ON event_threads (root_event_id);

CREATE INDEX IF NOT EXISTS idx_event_threads_parent_event_id
    ON event_threads (parent_event_id);
