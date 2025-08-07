-- オフラインサポートのためのテーブル作成

-- 同期キューテーブル
-- オンラインになったときに同期すべきアクションを管理
CREATE TABLE IF NOT EXISTS sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_type TEXT NOT NULL, -- 'create_post', 'like', 'boost', 'follow', 'unfollow', 'topic_join', 'topic_leave' など
    payload TEXT NOT NULL, -- JSON形式のアクションデータ
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'syncing', 'failed', 'completed'
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    synced_at INTEGER,
    error_message TEXT
);

-- オフラインアクションログテーブル
-- ユーザーのオフライン中のアクションを記録
CREATE TABLE IF NOT EXISTS offline_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_pubkey TEXT NOT NULL,
    action_type TEXT NOT NULL,
    target_id TEXT, -- 対象となるID（投稿ID、ユーザーID、トピックIDなど）
    action_data TEXT NOT NULL, -- JSON形式の詳細データ
    local_id TEXT NOT NULL UNIQUE, -- ローカルで生成したユニークID
    remote_id TEXT, -- 同期後のリモートID
    is_synced INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    synced_at INTEGER
);

-- キャッシュメタデータテーブル
-- キャッシュされたデータの状態を管理
CREATE TABLE IF NOT EXISTS cache_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cache_key TEXT NOT NULL UNIQUE,
    cache_type TEXT NOT NULL, -- 'posts', 'topics', 'users', 'reactions' など
    last_synced_at INTEGER,
    last_accessed_at INTEGER,
    data_version INTEGER NOT NULL DEFAULT 1,
    is_stale INTEGER NOT NULL DEFAULT 0,
    expiry_time INTEGER, -- キャッシュの有効期限（UNIX時間）
    metadata TEXT -- JSON形式の追加メタデータ
);

-- 楽観的更新用の一時データテーブル
-- UIに即座に反映させるための一時的なデータを保存
CREATE TABLE IF NOT EXISTS optimistic_updates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    update_id TEXT NOT NULL UNIQUE,
    entity_type TEXT NOT NULL, -- 'post', 'reaction', 'topic_membership' など
    entity_id TEXT NOT NULL,
    original_data TEXT, -- JSON形式の元データ（ロールバック用）
    updated_data TEXT NOT NULL, -- JSON形式の更新データ
    is_confirmed INTEGER NOT NULL DEFAULT 0, -- サーバーから確認されたか
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    confirmed_at INTEGER
);

-- 同期状態テーブル
-- 各エンティティの同期状態を追跡
CREATE TABLE IF NOT EXISTS sync_status (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    local_version INTEGER NOT NULL DEFAULT 1,
    remote_version INTEGER,
    last_local_update INTEGER NOT NULL DEFAULT (unixepoch()),
    last_remote_sync INTEGER,
    sync_status TEXT NOT NULL DEFAULT 'synced', -- 'synced', 'pending', 'conflict', 'error'
    conflict_data TEXT, -- JSON形式の競合データ
    UNIQUE(entity_type, entity_id)
);

-- インデックスの作成
CREATE INDEX IF NOT EXISTS idx_sync_queue_status ON sync_queue(status);
CREATE INDEX IF NOT EXISTS idx_sync_queue_created_at ON sync_queue(created_at);
CREATE INDEX IF NOT EXISTS idx_offline_actions_user_pubkey ON offline_actions(user_pubkey);
CREATE INDEX IF NOT EXISTS idx_offline_actions_is_synced ON offline_actions(is_synced);
CREATE INDEX IF NOT EXISTS idx_offline_actions_local_id ON offline_actions(local_id);
CREATE INDEX IF NOT EXISTS idx_cache_metadata_cache_type ON cache_metadata(cache_type);
CREATE INDEX IF NOT EXISTS idx_cache_metadata_last_synced_at ON cache_metadata(last_synced_at);
CREATE INDEX IF NOT EXISTS idx_optimistic_updates_entity ON optimistic_updates(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_optimistic_updates_is_confirmed ON optimistic_updates(is_confirmed);
CREATE INDEX IF NOT EXISTS idx_sync_status_entity ON sync_status(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_sync_status_sync_status ON sync_status(sync_status);