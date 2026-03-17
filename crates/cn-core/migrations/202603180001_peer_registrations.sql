CREATE TABLE IF NOT EXISTS cn_bootstrap.peer_registrations (
    subscriber_pubkey TEXT PRIMARY KEY
        REFERENCES cn_user.subscriber_accounts (subscriber_pubkey) ON DELETE CASCADE,
    endpoint_id TEXT NOT NULL,
    addr_hint TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cn_bootstrap_peer_registrations_updated_at
    ON cn_bootstrap.peer_registrations (updated_at DESC);
