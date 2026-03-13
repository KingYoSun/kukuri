DROP TABLE IF EXISTS bookmarks;

CREATE TABLE IF NOT EXISTS bookmarks (
    id TEXT PRIMARY KEY,
    user_pubkey TEXT NOT NULL,
    post_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(user_pubkey, post_id)
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_user_pubkey ON bookmarks(user_pubkey);
CREATE INDEX IF NOT EXISTS idx_bookmarks_post_id ON bookmarks(post_id);
CREATE INDEX IF NOT EXISTS idx_bookmarks_created_at ON bookmarks(created_at);
