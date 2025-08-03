-- ブックマークテーブルの作成
CREATE TABLE IF NOT EXISTS bookmarks (
    id TEXT PRIMARY KEY,
    user_pubkey TEXT NOT NULL,
    post_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    -- ユニーク制約：同じユーザーが同じ投稿を複数回ブックマークできない
    UNIQUE(user_pubkey, post_id)
);

-- インデックスの作成
CREATE INDEX IF NOT EXISTS idx_bookmarks_user_pubkey ON bookmarks(user_pubkey);
CREATE INDEX IF NOT EXISTS idx_bookmarks_post_id ON bookmarks(post_id);
CREATE INDEX IF NOT EXISTS idx_bookmarks_created_at ON bookmarks(created_at DESC);