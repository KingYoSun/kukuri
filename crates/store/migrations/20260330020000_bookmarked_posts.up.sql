CREATE TABLE IF NOT EXISTS bookmarked_posts (
    source_object_id TEXT PRIMARY KEY,
    source_envelope_id TEXT NOT NULL,
    source_replica_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    author_pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    object_kind TEXT NOT NULL,
    payload_ref_json TEXT NOT NULL,
    content TEXT,
    attachments_json TEXT NOT NULL,
    reply_to_object_id TEXT,
    root_object_id TEXT,
    repost_of_json TEXT,
    bookmarked_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bookmarked_posts_bookmarked_at
    ON bookmarked_posts(bookmarked_at DESC, source_object_id DESC);
