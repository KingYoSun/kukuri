-- 初期スキーマ作成

-- プロファイルテーブル
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

-- イベントテーブル
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    public_key TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL, -- JSON形式で保存
    sig TEXT NOT NULL,
    saved_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (public_key) REFERENCES profiles(public_key)
);

-- トピックテーブル
CREATE TABLE IF NOT EXISTS topics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- イベントとトピックの関連テーブル
CREATE TABLE IF NOT EXISTS event_topics (
    event_id TEXT NOT NULL,
    topic_id INTEGER NOT NULL,
    PRIMARY KEY (event_id, topic_id),
    FOREIGN KEY (event_id) REFERENCES events(event_id),
    FOREIGN KEY (topic_id) REFERENCES topics(id)
);

-- リレーテーブル
CREATE TABLE IF NOT EXISTS relays (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    name TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- インデックス作成
CREATE INDEX idx_events_public_key ON events(public_key);
CREATE INDEX idx_events_created_at ON events(created_at);
CREATE INDEX idx_events_kind ON events(kind);
CREATE INDEX idx_event_topics_topic_id ON event_topics(topic_id);