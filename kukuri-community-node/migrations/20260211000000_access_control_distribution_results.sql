CREATE TABLE IF NOT EXISTS cn_user.key_envelope_distribution_results (
    topic_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    epoch INT NOT NULL,
    recipient_pubkey TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'success', 'failed')),
    reason TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (topic_id, scope, epoch, recipient_pubkey)
);

CREATE INDEX IF NOT EXISTS key_envelope_distribution_results_status_idx
    ON cn_user.key_envelope_distribution_results (status, updated_at DESC);
