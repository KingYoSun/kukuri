use crate::modules::database::connection::DbPool;
use crate::modules::database::models;
use anyhow::Result;
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// イベントコールバックの型エイリアス
type EventCallback = Box<dyn Fn(Event) + Send + Sync>;

/// Nostrイベントハンドラー
pub struct EventHandler {
    event_callbacks: Arc<RwLock<Vec<EventCallback>>>,
    db_pool: Option<Arc<DbPool>>,
}

impl EventHandler {
    /// 新しいEventHandlerインスタンスを作成
    pub fn new() -> Self {
        Self {
            event_callbacks: Arc::new(RwLock::new(Vec::new())),
            db_pool: None,
        }
    }

    /// データベースプールを設定
    pub fn set_db_pool(&mut self, db_pool: Arc<DbPool>) {
        self.db_pool = Some(db_pool);
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
        if let Some(pool) = &self.db_pool {
            let event_model = models::Event {
                id: 0, // Auto-increment
                event_id: event.id.to_string(),
                public_key: event.pubkey.to_string(),
                created_at: event.created_at.as_u64() as i64,
                kind: event.kind.as_u16() as i64,
                content: event.content.clone(),
                tags: serde_json::to_string(&event.tags)?,
                sig: event.sig.to_string(),
                saved_at: chrono::Utc::now().timestamp(),
            };

            sqlx::query!(
                r#"
                INSERT INTO events (event_id, public_key, created_at, kind, content, tags, sig, saved_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(event_id) DO NOTHING
                "#,
                event_model.event_id,
                event_model.public_key,
                event_model.created_at,
                event_model.kind,
                event_model.content,
                event_model.tags,
                event_model.sig,
                event_model.saved_at
            )
            .execute(pool.as_ref())
            .await?;

            debug!("Text note saved to database: {}", event.id);

            // イベントのタグからトピックIDを抽出し、マッピングを保存（冪等）
            // 対象: Hashtag("t") と Custom("topic")
            for tag in event.tags.iter() {
                if let Some(std) = tag.as_standardized() {
                    if let nostr_sdk::TagStandard::Hashtag(topic) = std {
                        let _ = sqlx::query(
                            r#"INSERT OR IGNORE INTO event_topics (event_id, topic_id, created_at) VALUES (?1, ?2, ?3)"#,
                        )
                        .bind(event.id.to_string())
                        .bind(topic)
                        .bind(chrono::Utc::now().timestamp_millis())
                        .execute(pool.as_ref())
                        .await;
                    }
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
                        .execute(pool.as_ref())
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
        if let Some(pool) = &self.db_pool {
            let metadata: serde_json::Value = serde_json::from_str(&event.content)?;

            let profile = models::Profile {
                id: 0, // Auto-increment
                public_key: event.pubkey.to_string(),
                display_name: metadata
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                about: metadata
                    .get("about")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                picture_url: metadata
                    .get("picture")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                banner_url: metadata
                    .get("banner")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                nip05: metadata
                    .get("nip05")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                created_at: event.created_at.as_u64() as i64,
                updated_at: chrono::Utc::now().timestamp(),
            };

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
                profile.public_key,
                profile.display_name,
                profile.about,
                profile.picture_url,
                profile.banner_url,
                profile.nip05,
                profile.created_at,
                profile.updated_at
            )
            .execute(pool.as_ref())
            .await?;

            debug!("Profile metadata saved to database for: {}", event.pubkey);
        }

        Ok(())
    }

    /// コンタクトリストイベントを処理
    async fn handle_contact_list(&self, event: &Event) -> Result<()> {
        info!("Received contact list from {}", event.pubkey);

        // フォロー関係をデータベースに保存
        if let Some(pool) = &self.db_pool {
            // 既存のフォロー関係を削除
            let follower_pubkey = event.pubkey.to_string();
            sqlx::query!(
                "DELETE FROM follows WHERE follower_pubkey = ?1",
                follower_pubkey
            )
            .execute(pool.as_ref())
            .await?;

            // 新しいフォロー関係を保存
            let follower_pubkey_str = event.pubkey.to_string();
            for tag in event.tags.iter() {
                if let Some(nostr_sdk::TagStandard::PublicKey {
                    public_key: pubkey, ..
                }) = tag.as_standardized()
                {
                    let followed_pubkey = pubkey.to_hex();
                    let created_at = chrono::Utc::now().timestamp();

                    sqlx::query!(
                        r#"
                        INSERT INTO follows (follower_pubkey, followed_pubkey, created_at)
                        VALUES (?1, ?2, ?3)
                        "#,
                        follower_pubkey_str,
                        followed_pubkey,
                        created_at
                    )
                    .execute(pool.as_ref())
                    .await?;
                }
            }

            debug!("Contact list saved to database for: {}", event.pubkey);
        }

        Ok(())
    }

    /// リアクションイベントを処理
    async fn handle_reaction(&self, event: &Event) -> Result<()> {
        info!("Received reaction from {}: {}", event.pubkey, event.content);

        // リアクションをデータベースに保存
        if let Some(pool) = &self.db_pool {
            // リアクション対象のイベントIDを取得
            let mut target_event_id: Option<String> = None;
            for tag in event.tags.iter() {
                if let Some(nostr_sdk::TagStandard::Event { event_id, .. }) = tag.as_standardized()
                {
                    target_event_id = Some(event_id.to_hex());
                    break;
                }
            }

            if let Some(target_id) = target_event_id {
                let reactor_pubkey = event.pubkey.to_string();
                let reaction_content = event.content.clone();
                let created_at = event.created_at.as_u64() as i64;

                sqlx::query!(
                    r#"
                    INSERT INTO reactions (reactor_pubkey, target_event_id, reaction_content, created_at)
                    VALUES (?1, ?2, ?3, ?4)
                    ON CONFLICT(reactor_pubkey, target_event_id) DO UPDATE SET
                        reaction_content = excluded.reaction_content,
                        created_at = excluded.created_at
                    "#,
                    reactor_pubkey,
                    target_id,
                    reaction_content,
                    created_at
                )
                .execute(pool.as_ref())
                .await?;

                debug!(
                    "Reaction saved to database: {} -> {}",
                    event.pubkey, target_id
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_handler_creation() {
        let handler = EventHandler::new();
        assert!(handler.event_callbacks.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_handle_text_note() {
        let handler = EventHandler::new();
        let keys = Keys::generate();

        let event = EventBuilder::text_note("Test text note")
            .sign_with_keys(&keys)
            .unwrap();

        // テキストノートの処理が正常に完了することを確認
        assert!(handler.handle_event(event).await.is_ok());
    }

    #[tokio::test]
    async fn test_handle_metadata() {
        let handler = EventHandler::new();
        let keys = Keys::generate();

        let metadata = Metadata::new().name("Test User").about("Test about");

        let event = EventBuilder::metadata(&metadata)
            .sign_with_keys(&keys)
            .unwrap();

        // メタデータイベントの処理が正常に完了することを確認
        assert!(handler.handle_event(event).await.is_ok());
    }

    #[tokio::test]
    async fn test_handle_reaction() {
        let handler = EventHandler::new();
        let keys = Keys::generate();
        let _target_event_id = EventId::from_slice(&[1; 32]).unwrap();

        // リアクション用の疑似イベントを作成
        let target_event = EventBuilder::text_note("dummy")
            .sign_with_keys(&keys)
            .unwrap();
        let event = EventBuilder::reaction(&target_event, "+")
            .sign_with_keys(&keys)
            .unwrap();

        // リアクションイベントの処理が正常に完了することを確認
        assert!(handler.handle_event(event).await.is_ok());
    }
}
