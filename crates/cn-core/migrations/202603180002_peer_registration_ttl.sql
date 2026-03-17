ALTER TABLE cn_bootstrap.peer_registrations
    RENAME TO peer_registrations_legacy;

CREATE TABLE cn_bootstrap.peer_registrations (
    subscriber_pubkey TEXT NOT NULL
        REFERENCES cn_user.subscriber_accounts (subscriber_pubkey) ON DELETE CASCADE,
    endpoint_id TEXT NOT NULL,
    addr_hint TEXT,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '90 seconds'),
    PRIMARY KEY (subscriber_pubkey, endpoint_id)
);

INSERT INTO cn_bootstrap.peer_registrations
    (subscriber_pubkey, endpoint_id, addr_hint, registered_at, last_seen_at, expires_at)
SELECT
    subscriber_pubkey,
    endpoint_id,
    addr_hint,
    updated_at,
    updated_at,
    updated_at + INTERVAL '90 seconds'
FROM cn_bootstrap.peer_registrations_legacy;

DROP TABLE cn_bootstrap.peer_registrations_legacy;

CREATE INDEX idx_cn_bootstrap_peer_registrations_last_seen_at
    ON cn_bootstrap.peer_registrations (last_seen_at DESC);

CREATE INDEX idx_cn_bootstrap_peer_registrations_expires_at
    ON cn_bootstrap.peer_registrations (expires_at DESC);
