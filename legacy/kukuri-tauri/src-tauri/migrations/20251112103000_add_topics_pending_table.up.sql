-- Pending topics queue for offline topic creation replays
CREATE TABLE IF NOT EXISTS topics_pending (
    pending_id TEXT PRIMARY KEY NOT NULL,
    user_pubkey TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'queued',
    offline_action_id TEXT NOT NULL,
    synced_topic_id TEXT,
    error_message TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_topics_pending_user_status
    ON topics_pending(user_pubkey, status);

CREATE INDEX IF NOT EXISTS idx_topics_pending_offline_action
    ON topics_pending(offline_action_id);
