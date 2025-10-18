-- Nostr購読状態テーブルの追加
CREATE TABLE IF NOT EXISTS nostr_subscriptions (
    target TEXT NOT NULL,
    target_type TEXT NOT NULL,
    status TEXT NOT NULL,
    last_synced_at INTEGER,
    last_attempt_at INTEGER,
    failure_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (target, target_type)
);

CREATE INDEX IF NOT EXISTS idx_nostr_subscriptions_status ON nostr_subscriptions(status);
