# kukuriデータストレージ実装ガイドライン

## 作成日：2025年07月25日

## 概要

本ドキュメントは、kukuriプロジェクトにおけるsqlx（SQLite）を使用したデータストレージ層の実装ガイドラインです。

## 1. 即座に実装すべき事項

### 1.1 データベース初期化

```rust
// src-tauri/src/db/mod.rs
use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous}};
use std::str::FromStr;

pub async fn init_database(db_path: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    // 接続プール設定
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path))?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)  // Write-Ahead Logging
                .synchronous(SqliteSynchronous::Normal) // パフォーマンス最適化
                .foreign_keys(true)                     // 外部キー制約有効化
        )
        .await?;
    
    // マイグレーション実行
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}
```

### 1.2 SQLiteスキーマ定義

```sql
-- migrations/001_initial_schema.sql

-- ユーザーテーブル
CREATE TABLE IF NOT EXISTS users (
    pubkey TEXT PRIMARY KEY,
    privkey_encrypted TEXT,
    name TEXT,
    about TEXT,
    picture TEXT,
    nip05 TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- トピックテーブル
CREATE TABLE IF NOT EXISTS topics (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    creator TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]', -- JSON配列
    category TEXT,
    members_count INTEGER DEFAULT 0,
    FOREIGN KEY (creator) REFERENCES users(pubkey)
);

-- Nostrイベントテーブル
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]', -- JSON配列
    content TEXT NOT NULL,
    sig TEXT NOT NULL,
    topic_id TEXT,
    deleted_at INTEGER, -- ソフトデリート用
    FOREIGN KEY (pubkey) REFERENCES users(pubkey),
    FOREIGN KEY (topic_id) REFERENCES topics(id)
);

-- パフォーマンス最適化インデックス
CREATE INDEX idx_events_created_at_desc ON events(created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_events_pubkey_created ON events(pubkey, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_events_kind_topic ON events(kind, topic_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_events_topic_created ON events(topic_id, created_at DESC) WHERE deleted_at IS NULL;

-- JSON検索用の仮想列とインデックス（SQLite 3.38.0+）
CREATE INDEX idx_events_tags ON events(json_extract(tags, '$[0]')) WHERE deleted_at IS NULL;

-- トピックメンバーシップ
CREATE TABLE IF NOT EXISTS topic_members (
    topic_id TEXT NOT NULL,
    pubkey TEXT NOT NULL,
    joined_at INTEGER NOT NULL,
    role TEXT DEFAULT 'member', -- 'owner', 'moderator', 'member'
    PRIMARY KEY (topic_id, pubkey),
    FOREIGN KEY (topic_id) REFERENCES topics(id),
    FOREIGN KEY (pubkey) REFERENCES users(pubkey)
);

-- 既読管理
CREATE TABLE IF NOT EXISTS read_markers (
    pubkey TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    last_read_at INTEGER NOT NULL,
    last_event_id TEXT,
    PRIMARY KEY (pubkey, topic_id),
    FOREIGN KEY (pubkey) REFERENCES users(pubkey),
    FOREIGN KEY (topic_id) REFERENCES topics(id)
);

-- ブックマーク管理（2025年08月03日追加）
CREATE TABLE IF NOT EXISTS bookmarks (
    id TEXT PRIMARY KEY,
    user_pubkey TEXT NOT NULL,
    post_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(user_pubkey, post_id)
);

-- ブックマークのインデックス
CREATE INDEX idx_bookmarks_user_pubkey ON bookmarks(user_pubkey);
CREATE INDEX idx_bookmarks_post_id ON bookmarks(post_id);
CREATE INDEX idx_bookmarks_created_at ON bookmarks(created_at DESC);
```

### 1.3 Nostrイベント処理実装

```rust
// src-tauri/src/nostr/event_store.rs
use sqlx::{SqlitePool, Row};
use serde_json;

#[derive(Debug, Clone)]
pub struct NostrEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: i32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

pub struct EventStore {
    pool: SqlitePool,
}

impl EventStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    
    // イベント保存（署名検証後）
    pub async fn save_event(&self, event: &NostrEvent, topic_id: Option<&str>) -> Result<(), sqlx::Error> {
        let tags_json = serde_json::to_string(&event.tags)?;
        
        sqlx::query!(
            r#"
            INSERT INTO events (id, pubkey, created_at, kind, tags, content, sig, topic_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO NOTHING
            "#,
            event.id,
            event.pubkey,
            event.created_at,
            event.kind,
            tags_json,
            event.content,
            event.sig,
            topic_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    // タイムライン取得（最適化済み）
    pub async fn get_timeline(
        &self, 
        topic_id: &str, 
        limit: i64,
        until: Option<i64>
    ) -> Result<Vec<NostrEvent>, sqlx::Error> {
        let until = until.unwrap_or(i64::MAX);
        
        let rows = sqlx::query!(
            r#"
            SELECT id, pubkey, created_at, kind, tags, content, sig
            FROM events
            WHERE topic_id = ?1 
                AND created_at < ?2
                AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT ?3
            "#,
            topic_id,
            until,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        
        let events = rows.into_iter()
            .map(|row| NostrEvent {
                id: row.id,
                pubkey: row.pubkey,
                created_at: row.created_at,
                kind: row.kind,
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
                content: row.content,
                sig: row.sig,
            })
            .collect();
        
        Ok(events)
    }
}
```

### 1.4 Tauriコマンド実装

```rust
// src-tauri/src/commands/events.rs
use tauri::State;
use crate::nostr::{EventStore, NostrEvent};

#[tauri::command]
pub async fn get_topic_timeline(
    event_store: State<'_, EventStore>,
    topic_id: String,
    limit: Option<i64>,
    until: Option<i64>
) -> Result<Vec<NostrEvent>, String> {
    let limit = limit.unwrap_or(50).min(100); // 最大100件
    
    event_store
        .get_timeline(&topic_id, limit, until)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn post_to_topic(
    event_store: State<'_, EventStore>,
    topic_id: String,
    content: String,
    tags: Vec<Vec<String>>
) -> Result<NostrEvent, String> {
    // イベント作成と署名
    let event = create_and_sign_event(content, tags, 30001)?; // KukuriEventKind::TOPIC_POST
    
    // 保存
    event_store
        .save_event(&event, Some(&topic_id))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(event)
}
```

## 2. パフォーマンス最適化

### 2.1 クエリ最適化

```rust
// バッチ挿入
pub async fn save_events_batch(&self, events: &[NostrEvent]) -> Result<(), sqlx::Error> {
    let mut tx = self.pool.begin().await?;
    
    for event in events {
        let tags_json = serde_json::to_string(&event.tags)?;
        sqlx::query!(
            "INSERT INTO events (id, pubkey, created_at, kind, tags, content, sig, topic_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO NOTHING",
            event.id, event.pubkey, event.created_at, event.kind,
            tags_json, event.content, event.sig, event.topic_id
        )
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    Ok(())
}
```

### 2.2 キャッシュ戦略

```typescript
// フロントエンド側（Tanstack Query）
import { useQuery } from '@tanstack/react-query';

export function useTopicTimeline(topicId: string) {
  return useQuery({
    queryKey: ['timeline', topicId],
    queryFn: () => invoke('get_topic_timeline', { topicId, limit: 50 }),
    staleTime: 30 * 1000, // 30秒間は再フェッチしない
    gcTime: 5 * 60 * 1000, // 5分間キャッシュ保持
  });
}
```

## 3. セキュリティ実装

### 3.1 秘密鍵の暗号化

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};

pub fn encrypt_private_key(
    private_key: &str,
    password: &str
) -> Result<String, Box<dyn std::error::Error>> {
    // パスワードからキー導出
    let salt = generate_salt();
    let key = derive_key_from_password(password, &salt)?;
    
    // AES-256-GCM暗号化
    let cipher = Aes256Gcm::new(Key::from_slice(&key));
    let nonce = generate_nonce();
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), private_key.as_bytes())?;
    
    // Base64エンコードして保存
    let encrypted = base64::encode([&salt[..], &nonce[..], &ciphertext[..]].concat());
    Ok(encrypted)
}
```

## 4. 同期メカニズム（Phase 2準備）

### 4.1 差分同期の準備

```rust
// 同期状態テーブル（将来の実装用）
CREATE TABLE IF NOT EXISTS sync_state (
    peer_id TEXT PRIMARY KEY,
    last_sync_at INTEGER NOT NULL,
    last_event_id TEXT,
    sync_status TEXT DEFAULT 'pending'
);

// イベント同期用メタデータ
ALTER TABLE events ADD COLUMN synced_at INTEGER;
ALTER TABLE events ADD COLUMN sync_version INTEGER DEFAULT 1;
```

## 5. 監視とメンテナンス

### 5.1 データベース最適化

```rust
// 定期的なVACUUM実行
#[tauri::command]
pub async fn optimize_database(pool: State<'_, SqlitePool>) -> Result<(), String> {
    sqlx::query!("VACUUM")
        .execute(&**pool)
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query!("ANALYZE")
        .execute(&**pool)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

### 5.2 ストレージ容量監視

```rust
pub async fn get_database_stats(&self) -> Result<DatabaseStats, sqlx::Error> {
    let size = sqlx::query!("SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()")
        .fetch_one(&self.pool)
        .await?;
    
    let event_count = sqlx::query!("SELECT COUNT(*) as count FROM events WHERE deleted_at IS NULL")
        .fetch_one(&self.pool)
        .await?;
    
    Ok(DatabaseStats {
        size_bytes: size.size.unwrap_or(0),
        event_count: event_count.count,
    })
}
```

## 6. テスト戦略

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .unwrap();
        
        pool
    }
    
    #[tokio::test]
    async fn test_save_and_retrieve_event() {
        let pool = setup_test_db().await;
        let store = EventStore::new(pool);
        
        // テストイベント作成
        let event = create_test_event();
        
        // 保存
        store.save_event(&event, Some("test_topic")).await.unwrap();
        
        // 取得
        let timeline = store.get_timeline("test_topic", 10, None).await.unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].id, event.id);
    }
}
```

## 7. 移行パス（将来の拡張）

### Phase 2でのiroh統合準備

```rust
// ハイブリッドストレージインターフェース
trait StorageBackend {
    async fn store_event(&self, event: &NostrEvent) -> Result<(), Error>;
    async fn get_event(&self, id: &str) -> Result<Option<NostrEvent>, Error>;
    async fn sync_with_peer(&self, peer_id: &str) -> Result<SyncResult, Error>;
}

// SQLiteとirohの両方に対応
struct HybridStorage {
    sqlite: SqliteBackend,
    iroh: Option<IrohBackend>, // Phase 2で追加
}
```

## まとめ

この実装ガイドラインに従うことで：

1. **即座に動作するMVPを構築**
2. **Nostrプロトコルとの完全な互換性を確保**
3. **優れたパフォーマンスとユーザー体験を提供**
4. **将来の拡張に対応できる柔軟な設計**

を実現できます。