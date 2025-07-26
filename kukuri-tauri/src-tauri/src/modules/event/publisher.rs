
use nostr_sdk::prelude::*;
use anyhow::Result;
use chrono::Utc;
use tracing::{info, debug};

/// Nostrイベント発行者
pub struct EventPublisher {
    keys: Option<Keys>,
}

impl EventPublisher {
    /// 新しいEventPublisherインスタンスを作成
    pub fn new() -> Self {
        Self { keys: None }
    }

    /// 鍵を設定
    pub fn set_keys(&mut self, keys: Keys) {
        self.keys = Some(keys);
    }

    /// テキストノートイベントを作成
    pub fn create_text_note(&self, content: &str, tags: Vec<Tag>) -> Result<Event> {
        let keys = self.keys.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
        
        let event = EventBuilder::text_note(content)
            .tags(tags)
            .sign_with_keys(keys)?;
        
        debug!("Created text note event: {}", event.id);
        Ok(event)
    }

    /// メタデータイベントを作成
    pub fn create_metadata(&self, metadata: Metadata) -> Result<Event> {
        let keys = self.keys.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
        
        let event = EventBuilder::metadata(&metadata)
            .sign_with_keys(keys)?;
        
        debug!("Created metadata event: {}", event.id);
        Ok(event)
    }

    /// リアクションイベントを作成
    pub fn create_reaction(&self, event_id: &EventId, reaction: &str) -> Result<Event> {
        let keys = self.keys.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
        
        // リアクションイベント用のタグを作成
        let tags = vec![
            Tag::event(*event_id),
            Tag::public_key(keys.public_key()),
        ];
        
        let event = EventBuilder::new(Kind::Reaction, reaction)
            .tags(tags)
            .sign_with_keys(keys)?;
        
        debug!("Created reaction event: {}", event.id);
        Ok(event)
    }

    /// リポストイベントを作成
    pub fn create_repost(&self, event_id: &EventId, relay_url: Option<String>) -> Result<Event> {
        let keys = self.keys.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
        
        // リポストイベント用のタグを作成
        let tags = vec![
            Tag::event(*event_id),
            Tag::public_key(keys.public_key()),
        ];
        
        let mut tags_with_relay = tags;
        
        if let Some(url) = relay_url {
            tags_with_relay.push(Tag::custom(TagKind::Custom("relay".into()), vec![url]));
        }
        
        let event = EventBuilder::new(Kind::Repost, "")
            .tags(tags_with_relay)
            .sign_with_keys(keys)?;
        
        debug!("Created repost event: {}", event.id);
        Ok(event)
    }

    /// カスタムイベントを作成
    pub fn create_custom_event(&self, kind: Kind, content: &str, tags: Vec<Tag>) -> Result<Event> {
        let keys = self.keys.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
        
        let timestamp = Timestamp::from(Utc::now().timestamp() as u64);
        
        let event = EventBuilder::new(kind, content)
            .tags(tags)
            .custom_created_at(timestamp)
            .sign_with_keys(keys)?;
        
        debug!("Created custom event: {} (kind: {})", event.id, kind);
        Ok(event)
    }

    /// 削除イベントを作成
    pub fn create_deletion(&self, event_ids: Vec<EventId>, reason: Option<&str>) -> Result<Event> {
        let keys = self.keys.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
        
        // 削除イベント用のタグを作成
        let tags: Vec<Tag> = event_ids.iter()
            .map(|id| Tag::event(*id))
            .collect();
        
        let content = reason.unwrap_or("");
        
        let event = EventBuilder::new(Kind::EventDeletion, content)
            .tags(tags)
            .sign_with_keys(keys)?;
        
        debug!("Created deletion event: {}", event.id);
        Ok(event)
    }

    /// トピック投稿イベントを作成（kukuri独自実装）
    pub fn create_topic_post(&self, topic_id: &str, content: &str, reply_to: Option<EventId>) -> Result<Event> {
        let keys = self.keys.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;
        
        let mut tags = vec![
            Tag::hashtag(topic_id),
            Tag::custom(TagKind::Custom("topic".into()), vec![topic_id.to_string()]),
        ];
        
        if let Some(reply_id) = reply_to {
            tags.push(Tag::event(reply_id));
            tags.push(Tag::custom(TagKind::Custom("reply".into()), vec![reply_id.to_string()]));
        }
        
        let event = EventBuilder::text_note(content)
            .tags(tags)
            .sign_with_keys(keys)?;
        
        info!("Created topic post for topic: {}", topic_id);
        Ok(event)
    }

    /// 公開鍵を取得
    pub fn get_public_key(&self) -> Option<PublicKey> {
        self.keys.as_ref().map(|k| k.public_key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_publisher_creation() {
        let publisher = EventPublisher::new();
        assert!(publisher.keys.is_none());
    }

    #[test]
    fn test_create_text_note() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);
        
        let event = publisher.create_text_note("Hello, Nostr!", vec![]).unwrap();
        assert_eq!(event.content(), "Hello, Nostr!");
        assert_eq!(event.kind(), Kind::TextNote);
    }

    #[test]
    fn test_create_topic_post() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);
        
        let event = publisher.create_topic_post("bitcoin", "Let's discuss Bitcoin!", None).unwrap();
        assert!(event.content().contains("Let's discuss Bitcoin!"));
        
        // タグを確認
        let tags: Vec<_> = event.tags().into_iter().collect();
        assert!(tags.iter().any(|t| matches!(t, Tag::Hashtag(h) if h == "bitcoin")));
    }
}
