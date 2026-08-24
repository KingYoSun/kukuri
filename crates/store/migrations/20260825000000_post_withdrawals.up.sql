CREATE TABLE IF NOT EXISTS post_withdrawals (
    target_object_id TEXT PRIMARY KEY NOT NULL,
    target_author_pubkey TEXT NOT NULL,
    source_replica_id TEXT NOT NULL,
    withdrawal_envelope_id TEXT NOT NULL,
    withdrawn_at INTEGER NOT NULL,
    generation INTEGER NOT NULL CHECK (generation > 0),
    replacement_object_id TEXT,
    reason_visibility TEXT NOT NULL CHECK (reason_visibility IN ('public', 'private')),
    reason TEXT CHECK (reason IS NULL OR reason IN ('author_request', 'correction', 'privacy', 'other'))
);

CREATE INDEX IF NOT EXISTS idx_post_withdrawals_author
    ON post_withdrawals (target_author_pubkey, withdrawn_at DESC);
