-- 完全なスキーマ定義（v2アーキテクチャ対応）

-- ユーザーテーブル
CREATE TABLE IF NOT EXISTS users (
    npub TEXT PRIMARY KEY NOT NULL,
    pubkey TEXT NOT NULL UNIQUE,
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- イベントテーブル（Nostrイベント用）
CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY NOT NULL,
    public_key TEXT NOT NULL,
    content TEXT NOT NULL,
    kind INTEGER NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]', -- JSON形式
    created_at INTEGER NOT NULL,
    sig TEXT NOT NULL DEFAULT '',
    saved_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    deleted INTEGER DEFAULT 0,
    updated_at INTEGER,
    sync_status INTEGER DEFAULT 0,
    sync_event_id TEXT,
    synced_at INTEGER
);

-- トピックテーブル
CREATE TABLE IF NOT EXISTS topics (
    topic_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    member_count INTEGER DEFAULT 0,
    post_count INTEGER DEFAULT 0
);

-- ユーザーとトピックの関連テーブル
CREATE TABLE IF NOT EXISTS user_topics (
    topic_id TEXT NOT NULL,
    user_pubkey TEXT,
    is_joined INTEGER DEFAULT 0,
    joined_at INTEGER,
    left_at INTEGER,
    PRIMARY KEY (topic_id, user_pubkey),
    FOREIGN KEY (topic_id) REFERENCES topics(topic_id)
);

-- フォロー関係テーブル
CREATE TABLE IF NOT EXISTS follows (
    follower_pubkey TEXT NOT NULL,
    followed_pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (follower_pubkey, followed_pubkey)
);

-- リアクションテーブル
CREATE TABLE IF NOT EXISTS reactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_event_id TEXT NOT NULL,
    reactor_pubkey TEXT NOT NULL,
    reaction_content TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    UNIQUE(reactor_pubkey, target_event_id)
);

-- ブックマークテーブル
CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- オフラインアクションテーブル
CREATE TABLE IF NOT EXISTS offline_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_type TEXT NOT NULL,
    payload TEXT NOT NULL, -- JSON形式
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    synced INTEGER DEFAULT 0,
    synced_at INTEGER,
    error TEXT
);

-- キャッシュテーブル
CREATE TABLE IF NOT EXISTS cache_entries (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL, -- JSON形式
    category TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    expires_at INTEGER,
    access_count INTEGER DEFAULT 0,
    last_accessed INTEGER
);

-- 楽観的更新テーブル
CREATE TABLE IF NOT EXISTS optimistic_updates (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    update_data TEXT NOT NULL, -- JSON形式
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    confirmed INTEGER DEFAULT 0,
    confirmed_at INTEGER,
    rollback_data TEXT -- JSON形式
);

-- 同期キューテーブル
CREATE TABLE IF NOT EXISTS sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_type TEXT NOT NULL,
    item_id TEXT NOT NULL,
    priority INTEGER DEFAULT 0,
    retry_count INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    next_retry_at INTEGER,
    error_message TEXT
);

-- リレーテーブル（Nostrリレー管理用）
CREATE TABLE IF NOT EXISTS relays (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    name TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- プロファイルテーブル（旧アーキテクチャとの互換性）
CREATE TABLE IF NOT EXISTS profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    public_key TEXT NOT NULL UNIQUE,
    display_name TEXT,
    about TEXT,
    picture_url TEXT,
    banner_url TEXT,
    nip05 TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- インデックス作成
CREATE INDEX IF NOT EXISTS idx_users_pubkey ON users(pubkey);
CREATE INDEX IF NOT EXISTS idx_events_public_key ON events(public_key);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at);
CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
CREATE INDEX IF NOT EXISTS idx_events_sync_status ON events(sync_status);
CREATE INDEX IF NOT EXISTS idx_topics_name ON topics(name);
CREATE INDEX IF NOT EXISTS idx_user_topics_user ON user_topics(user_pubkey);
CREATE INDEX IF NOT EXISTS idx_follows_follower ON follows(follower_pubkey);
CREATE INDEX IF NOT EXISTS idx_follows_followed ON follows(followed_pubkey);
CREATE INDEX IF NOT EXISTS idx_bookmarks_created_at ON bookmarks(created_at);
CREATE INDEX IF NOT EXISTS idx_offline_actions_synced ON offline_actions(synced);
CREATE INDEX IF NOT EXISTS idx_cache_entries_category ON cache_entries(category);
CREATE INDEX IF NOT EXISTS idx_cache_entries_expires_at ON cache_entries(expires_at);
CREATE INDEX IF NOT EXISTS idx_sync_queue_item_type ON sync_queue(item_type);
CREATE INDEX IF NOT EXISTS idx_sync_queue_next_retry ON sync_queue(next_retry_at);

-- デフォルトデータの挿入
INSERT OR IGNORE INTO topics (topic_id, name, description, created_at, updated_at)
VALUES ('public', '#public', 'パブリックトピック - すべての投稿が表示されます', unixepoch() * 1000, unixepoch() * 1000);
