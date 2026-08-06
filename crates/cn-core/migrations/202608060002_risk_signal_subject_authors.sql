-- Preserve the relationship between a content-scoped risk signal and the
-- author whose trust score consumes it. A content item can have more than one
-- observed author (for example, a shared blob), so the relationship is
-- intentionally many-to-many.
CREATE TABLE cn_safety.risk_signal_subject_authors (
    target TEXT NOT NULL,
    target_id TEXT NOT NULL,
    author_pubkey TEXT NOT NULL,
    first_observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (target, target_id, author_pubkey),
    CHECK (target IN ('post_id', 'blob_cid')),
    CHECK (target_id <> ''),
    CHECK (author_pubkey <> '')
);

CREATE INDEX idx_cn_safety_signal_subject_authors_author
    ON cn_safety.risk_signal_subject_authors (author_pubkey, target, target_id);
