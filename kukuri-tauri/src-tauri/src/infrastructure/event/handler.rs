use crate::infrastructure::database::connection_pool::ConnectionPool;
use anyhow::Result;
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// イベントコールバックの型エイリアス
type EventCallback = Arc<dyn Fn(Event) + Send + Sync>;

/// Nostrイベントハンドラー
pub struct EventHandler {
    event_callbacks: Arc<RwLock<Vec<EventCallback>>>,
    connection_pool: Option<ConnectionPool>,
}

impl EventHandler {
    /// 新しいEventHandlerインスタンスを作成
    pub fn new() -> Self {
        Self {
            event_callbacks: Arc::new(RwLock::new(Vec::new())),
            connection_pool: None,
        }
    }

    /// データベースプールを設定
    pub fn set_connection_pool(&mut self, pool: ConnectionPool) {
        self.connection_pool = Some(pool);
    }

    pub async fn register_callback(&self, callback: EventCallback) {
        let mut callbacks = self.event_callbacks.write().await;
        callbacks.push(callback);
    }

    /// イベントを処理
    pub async fn handle_event(&self, event: Event) -> Result<()> {
        debug!("Handling event: {}", event.id);

        let callbacks = self.event_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(event.clone());
        }

        // イベントの種類に応じた処理
        match event.kind {
            Kind::TextNote => {
                self.handle_text_note(&event).await?;
            }
            Kind::Metadata => {
                self.handle_metadata(&event).await?;
            }
            Kind::ContactList => {
                self.handle_contact_list(&event).await?;
            }
            Kind::Reaction => {
                self.handle_reaction(&event).await?;
            }
            _ => {
                debug!("Unhandled event kind: {:?}", event.kind);
            }
        }

        Ok(())
    }

    /// テキストノートイベントを処理
    async fn handle_text_note(&self, event: &Event) -> Result<()> {
        info!(
            "Received text note from {}: {}",
            event.pubkey, event.content
        );

        // データベースに保存
        if let Some(pool) = &self.connection_pool {
            let event_id = event.id.to_string();
            let public_key = event.pubkey.to_string();
            let created_at = event.created_at.as_secs() as i64;
            let kind = event.kind.as_u16() as i64;
            let content = event.content.clone();
            let tags = serde_json::to_string(&event.tags)?;
            let signature = event.sig.to_string();
            let saved_at = chrono::Utc::now().timestamp();

            sqlx::query!(
                r#"
                INSERT INTO events (event_id, public_key, created_at, kind, content, tags, sig, saved_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(event_id) DO NOTHING
                "#,
                event_id,
                public_key,
                created_at,
                kind,
                content,
                tags,
                signature,
                saved_at
            )
            .execute(pool.get_pool())
            .await?;

            debug!("Text note saved to database: {}", event.id);

            // イベントのタグからトピックIDを抽出し、マッピングを保存（冪等）
            // 対象: Hashtag("t") と Custom("topic")
            for tag in event.tags.iter() {
                if let Some(nostr_sdk::TagStandard::Hashtag(topic)) = tag.as_standardized() {
                    let _ = sqlx::query(
                        r#"INSERT OR IGNORE INTO event_topics (event_id, topic_id, created_at) VALUES (?1, ?2, ?3)"#,
                    )
                    .bind(event.id.to_string())
                    .bind(topic)
                    .bind(chrono::Utc::now().timestamp_millis())
                    .execute(pool.get_pool())
                    .await;
                }
                // カスタムタグ 'topic'
                if tag.kind().to_string() == "topic" {
                    if let Some(content) = tag.content() {
                        let _ = sqlx::query(
                            r#"INSERT OR IGNORE INTO event_topics (event_id, topic_id, created_at) VALUES (?1, ?2, ?3)"#,
                        )
                        .bind(event.id.to_string())
                        .bind(content)
                        .bind(chrono::Utc::now().timestamp_millis())
                        .execute(pool.get_pool())
                        .await;
                    }
                }
            }
        }

        Ok(())
    }

    /// メタデータイベントを処理
    async fn handle_metadata(&self, event: &Event) -> Result<()> {
        info!("Received metadata update from {}", event.pubkey);

        // メタデータをパースしてデータベースに保存
        if let Some(pool) = &self.connection_pool {
            let metadata: serde_json::Value = serde_json::from_str(&event.content)?;
            let display_name = metadata
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from);
            let about = metadata
                .get("about")
                .and_then(|v| v.as_str())
                .map(String::from);
            let picture_url = metadata
                .get("picture")
                .and_then(|v| v.as_str())
                .map(String::from);
            let banner_url = metadata
                .get("banner")
                .and_then(|v| v.as_str())
                .map(String::from);
            let nip05 = metadata
                .get("nip05")
                .and_then(|v| v.as_str())
                .map(String::from);
            let created_at = event.created_at.as_secs() as i64;
            let updated_at = chrono::Utc::now().timestamp();
            let public_key = event.pubkey.to_string();

            sqlx::query!(
                r#"
                INSERT INTO profiles (public_key, display_name, about, picture_url, banner_url, nip05, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(public_key) DO UPDATE SET
                    display_name = excluded.display_name,
                    about = excluded.about,
                    picture_url = excluded.picture_url,
                    banner_url = excluded.banner_url,
                    nip05 = excluded.nip05,
                    updated_at = excluded.updated_at
                "#,
                public_key,
                display_name,
                about,
                picture_url,
                banner_url,
                nip05,
                created_at,
                updated_at
            )
            .execute(pool.get_pool())
            .await?;

            debug!("Profile metadata saved to database for: {}", event.pubkey);
        }

        Ok(())
    }

    /// コンタクトリストイベントを処理
    async fn handle_contact_list(&self, event: &Event) -> Result<()> {
        info!("Received contact list from {}", event.pubkey);

        // フォロー関係をデータベースに保存
        if let Some(pool) = &self.connection_pool {
            // 既存のフォロー関係を削除
            let follower_pubkey = event.pubkey.to_string();
            sqlx::query!(
                r#"DELETE FROM follows WHERE follower_pubkey = ?"#,
                follower_pubkey
            )
            .execute(pool.get_pool())
            .await?;

            // 新しいフォロー関係を追加
            for tag in event.tags.iter() {
                if let Some(TagStandard::PublicKey { public_key, .. }) = tag.as_standardized() {
                    let followed_pubkey = public_key.to_string();
                    let created_at = chrono::Utc::now().timestamp();
                    sqlx::query!(
                        r#"
                        INSERT INTO follows (follower_pubkey, followed_pubkey, created_at)
                        VALUES (?1, ?2, ?3)
                        ON CONFLICT(follower_pubkey, followed_pubkey) DO NOTHING
                        "#,
                        follower_pubkey,
                        followed_pubkey,
                        created_at
                    )
                    .execute(pool.get_pool())
                    .await?;
                }
            }

            debug!(
                "Contact list processed and saved for: {}",
                event.pubkey.to_string()
            );
        }

        Ok(())
    }

    /// リアクションイベントを処理
    async fn handle_reaction(&self, event: &Event) -> Result<()> {
        info!("Received reaction from {}: {}", event.pubkey, event.content);

        // リアクションをデータベースに保存
        if let Some(pool) = &self.connection_pool {
            if event.tags.is_empty() || event.content.is_empty() {
                return Ok(());
            }

            let Some(first_tag) = event.tags.get(0) else {
                return Ok(());
            };
            let Some(target_event_id) = first_tag.content() else {
                return Ok(());
            };

            let reactor_pubkey = event.pubkey.to_string();
            let reaction_content = event.content.clone();
            let created_at = event.created_at.as_secs() as i64;
            let updated_at = chrono::Utc::now().timestamp();
            sqlx::query!(
                r#"
                INSERT INTO reactions (target_event_id, reactor_pubkey, reaction_content, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(reactor_pubkey, target_event_id) DO UPDATE SET
                    reaction_content = excluded.reaction_content,
                    updated_at = excluded.updated_at
                "#,
                target_event_id,
                reactor_pubkey,
                reaction_content,
                created_at,
                updated_at
            )
            .execute(pool.get_pool())
            .await?;

            debug!("Reaction saved to database: {}", event.id);
        }

        Ok(())
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}
