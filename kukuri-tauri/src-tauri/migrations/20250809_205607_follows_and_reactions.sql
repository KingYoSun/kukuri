-- フォロー関係テーブル
CREATE TABLE IF NOT EXISTS follows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    follower_pubkey TEXT NOT NULL,
    followed_pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(follower_pubkey, followed_pubkey)
);

-- リアクションテーブル
CREATE TABLE IF NOT EXISTS reactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reactor_pubkey TEXT NOT NULL,
    target_event_id TEXT NOT NULL,
    reaction_content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(reactor_pubkey, target_event_id),
    FOREIGN KEY (target_event_id) REFERENCES events(event_id)
);

-- インデックス作成
CREATE INDEX idx_follows_follower ON follows(follower_pubkey);
CREATE INDEX idx_follows_followed ON follows(followed_pubkey);
CREATE INDEX idx_reactions_target ON reactions(target_event_id);
CREATE INDEX idx_reactions_reactor ON reactions(reactor_pubkey);