CREATE TABLE IF NOT EXISTS content_observations (
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    node_base_url TEXT NOT NULL,
    capability TEXT NOT NULL,
    observed_at INTEGER NOT NULL,
    PRIMARY KEY (subject_kind, subject_id, node_base_url, capability),
    CHECK (subject_kind IN ('post', 'profile'))
);

CREATE INDEX IF NOT EXISTS idx_content_observations_subject
    ON content_observations (subject_kind, subject_id, observed_at DESC);

CREATE INDEX IF NOT EXISTS idx_content_observations_observed_at
    ON content_observations (observed_at ASC);
