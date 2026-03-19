CREATE TABLE IF NOT EXISTS follow_edges (
    subject_pubkey TEXT NOT NULL,
    target_pubkey TEXT NOT NULL,
    status TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    source_envelope_id TEXT NOT NULL,
    PRIMARY KEY (subject_pubkey, target_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_follow_edges_subject
    ON follow_edges(subject_pubkey, updated_at DESC, target_pubkey ASC);

CREATE INDEX IF NOT EXISTS idx_follow_edges_target
    ON follow_edges(target_pubkey, updated_at DESC, subject_pubkey ASC);

CREATE TABLE IF NOT EXISTS author_relationship_cache (
    local_author_pubkey TEXT NOT NULL,
    author_pubkey TEXT NOT NULL,
    following INTEGER NOT NULL,
    followed_by INTEGER NOT NULL,
    mutual INTEGER NOT NULL,
    friend_of_friend INTEGER NOT NULL,
    friend_of_friend_via_pubkeys_json TEXT NOT NULL,
    derived_at INTEGER NOT NULL,
    PRIMARY KEY (local_author_pubkey, author_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_author_relationship_cache_local_author
    ON author_relationship_cache(local_author_pubkey, author_pubkey);
