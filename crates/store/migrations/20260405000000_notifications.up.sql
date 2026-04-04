CREATE TABLE IF NOT EXISTS notifications (
    notification_id TEXT PRIMARY KEY,
    recipient_pubkey TEXT NOT NULL,
    kind TEXT NOT NULL,
    actor_pubkey TEXT NOT NULL,
    source_envelope_id TEXT,
    source_replica_id TEXT,
    topic_id TEXT,
    channel_id TEXT,
    object_id TEXT,
    dm_id TEXT,
    message_id TEXT,
    preview_text TEXT,
    created_at INTEGER NOT NULL,
    received_at INTEGER NOT NULL,
    read_at INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_notifications_docs_dedupe
    ON notifications(recipient_pubkey, kind, source_envelope_id)
    WHERE source_envelope_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_notifications_dm_dedupe
    ON notifications(recipient_pubkey, kind, dm_id, message_id)
    WHERE dm_id IS NOT NULL AND message_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_notifications_inbox
    ON notifications(received_at DESC, notification_id DESC);

CREATE INDEX IF NOT EXISTS idx_notifications_unread
    ON notifications(received_at DESC, notification_id DESC)
    WHERE read_at IS NULL;
