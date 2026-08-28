CREATE TABLE IF NOT EXISTS block_edges (
    subject_pubkey TEXT NOT NULL,
    target_pubkey TEXT NOT NULL,
    status TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    source_envelope_id TEXT NOT NULL,
    PRIMARY KEY (subject_pubkey, target_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_block_edges_subject
    ON block_edges(subject_pubkey, updated_at DESC, target_pubkey ASC);

CREATE INDEX IF NOT EXISTS idx_block_edges_target
    ON block_edges(target_pubkey, updated_at DESC, subject_pubkey ASC);
