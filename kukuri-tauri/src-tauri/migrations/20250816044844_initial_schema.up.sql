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

-- オフラインアクションテーブル（v2仕様）
CREATE TABLE IF NOT EXISTS offline_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_pubkey TEXT NOT NULL,
    action_type TEXT NOT NULL,
    target_id TEXT,
    action_data TEXT NOT NULL, -- JSON形式
    local_id TEXT NOT NULL,
    remote_id TEXT,
    is_synced INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    synced_at INTEGER
);

-- キャッシュメタデータ（v2仕様）
CREATE TABLE IF NOT EXISTS cache_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cache_key TEXT NOT NULL UNIQUE,
    cache_type TEXT NOT NULL,
    last_synced_at INTEGER,
    last_accessed_at INTEGER,
    data_version INTEGER DEFAULT 1,
    is_stale INTEGER DEFAULT 0,
    expiry_time INTEGER,
    metadata TEXT -- JSON形式
);

-- 楽観的更新テーブル（v2仕様）
CREATE TABLE IF NOT EXISTS optimistic_updates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    update_id TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    original_data TEXT, -- JSON形式
    updated_data TEXT NOT NULL, -- JSON形式
    is_confirmed INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    confirmed_at INTEGER
);

-- 同期キューテーブル（v2仕様）
CREATE TABLE IF NOT EXISTS sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_type TEXT NOT NULL,
    payload TEXT NOT NULL, -- JSON形式
    status TEXT NOT NULL,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    synced_at INTEGER,
    error_message TEXT
);

-- 同期状態テーブル（v2仕様）
CREATE TABLE IF NOT EXISTS sync_status (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    local_version INTEGER NOT NULL,
    last_local_update INTEGER NOT NULL,
    sync_status TEXT NOT NULL,
    conflict_data TEXT,
    PRIMARY KEY (entity_type, entity_id)
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
CREATE INDEX IF NOT EXISTS idx_offline_actions_synced ON offline_actions(is_synced);
CREATE INDEX IF NOT EXISTS idx_cache_metadata_type ON cache_metadata(cache_type);
CREATE INDEX IF NOT EXISTS idx_cache_metadata_expiry ON cache_metadata(expiry_time);
CREATE INDEX IF NOT EXISTS idx_sync_queue_status ON sync_queue(status);

-- デフォルトデータの挿入
INSERT OR IGNORE INTO topics (topic_id, name, description, created_at, updated_at)
VALUES ('kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0', '#public', 'パブリックトピック - すべての投稿が表示されます', unixepoch() * 1000, unixepoch() * 1000);
