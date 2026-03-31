CREATE TABLE IF NOT EXISTS muted_authors (
    author_pubkey TEXT PRIMARY KEY,
    muted_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_muted_authors_muted_at
    ON muted_authors(muted_at DESC, author_pubkey ASC);
