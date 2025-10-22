use anyhow::Result;
use nostr_sdk::prelude::*;
use tracing::{debug, info};

/// Nostr イベントの生成を担う共通パブリッシャー。
#[derive(Default)]
pub struct EventPublisher {
    keys: Option<Keys>,
}

impl EventPublisher {
    /// 新しい EventPublisher インスタンスを作成
    pub fn new() -> Self {
        Self::default()
    }

    /// 鍵を設定
    pub fn set_keys(&mut self, keys: Keys) {
        self.keys = Some(keys);
    }

    /// テキストノートイベントを作成
    pub fn create_text_note(&self, content: &str, tags: Vec<Tag>) -> Result<Event> {
        let keys = self
            .keys
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;

        let event = EventBuilder::text_note(content)
            .tags(tags)
            .sign_with_keys(keys)?;

        debug!("Created text note event: {}", event.id);
        Ok(event)
    }

    /// メタデータイベントを作成
    pub fn create_metadata(&self, metadata: Metadata) -> Result<Event> {
        let keys = self
            .keys
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;

        let event = EventBuilder::metadata(&metadata).sign_with_keys(keys)?;

        debug!("Created metadata event: {}", event.id);
        Ok(event)
    }

    /// リアクションイベントを作成
    pub fn create_reaction(&self, event_id: &EventId, reaction: &str) -> Result<Event> {
        let keys = self
            .keys
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;

        // リアクションイベント用のタグを作成
        let tags = vec![Tag::event(*event_id), Tag::public_key(keys.public_key())];

        let event = EventBuilder::new(Kind::Reaction, reaction)
            .tags(tags)
            .sign_with_keys(keys)?;

        debug!("Created reaction event: {}", event.id);
        Ok(event)
    }

    /// 削除イベントを作成
    pub fn create_deletion(&self, event_ids: Vec<EventId>, reason: Option<&str>) -> Result<Event> {
        let keys = self
            .keys
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;

        // 削除イベント用のタグを作成
        let tags: Vec<Tag> = event_ids.iter().map(|id| Tag::event(*id)).collect();

        let content = reason.unwrap_or("");

        let event = EventBuilder::new(Kind::EventDeletion, content)
            .tags(tags)
            .sign_with_keys(keys)?;

        debug!("Created deletion event: {}", event.id);
        Ok(event)
    }

    /// トピック投稿イベントを作成（kukuri 独自実装）
    pub fn create_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<EventId>,
    ) -> Result<Event> {
        let keys = self
            .keys
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keys not set"))?;

        let mut tags = vec![
            Tag::hashtag(topic_id),
            Tag::custom(TagKind::Custom("topic".into()), vec![topic_id.to_string()]),
        ];

        if let Some(reply_id) = reply_to {
            tags.push(Tag::event(reply_id));
            tags.push(Tag::custom(
                TagKind::Custom("reply".into()),
                vec![reply_id.to_string()],
            ));
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
    fn test_set_keys() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        let public_key = keys.public_key();

        publisher.set_keys(keys);
        assert_eq!(publisher.get_public_key(), Some(public_key));
    }

    #[test]
    fn test_create_text_note() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);

        let event = publisher.create_text_note("Hello, Nostr!", vec![]).unwrap();
        assert_eq!(event.content, "Hello, Nostr!");
        assert_eq!(event.kind, Kind::TextNote);
    }

    #[test]
    fn test_create_text_note_with_tags() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);

        let tags = vec![Tag::hashtag("nostr"), Tag::hashtag("test")];

        let event = publisher
            .create_text_note("Hello with tags!", tags.clone())
            .unwrap();
        assert_eq!(event.content, "Hello with tags!");

        // タグが含まれていることを確認
        let event_tags: Vec<_> = event.tags.into_iter().collect();
        assert!(event_tags.iter().any(|t| matches!(t.as_standardized(), Some(nostr_sdk::TagStandard::Hashtag(h)) if h == "nostr")));
        assert!(event_tags.iter().any(|t| matches!(t.as_standardized(), Some(nostr_sdk::TagStandard::Hashtag(h)) if h == "test")));
    }

    #[test]
    fn test_create_metadata() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);

        let metadata = Metadata::new()
            .name("Test User")
            .about("Test about")
            .picture(Url::parse("https://example.com/pic.jpg").unwrap());

        let event = publisher.create_metadata(metadata).unwrap();
        assert_eq!(event.kind, Kind::Metadata);
        assert!(event.content.contains("Test User"));
    }

    #[test]
    fn test_create_reaction() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys.clone());

        let event_id = EventId::from_slice(&[1; 32]).unwrap();
        let event = publisher.create_reaction(&event_id, "+").unwrap();

        assert_eq!(event.kind, Kind::Reaction);
        assert_eq!(event.content, "+");

        // タグにイベントIDが含まれていることを確認
        let tags: Vec<_> = event.tags.into_iter().collect();
        assert!(tags.iter().any(|t| matches!(t.as_standardized(), Some(nostr_sdk::TagStandard::Event { event_id: id, .. }) if id == &event_id)));
    }

    #[test]
    fn test_create_deletion() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);

        let event_ids = vec![
            EventId::from_slice(&[1; 32]).unwrap(),
            EventId::from_slice(&[2; 32]).unwrap(),
        ];

        let event = publisher
            .create_deletion(event_ids.clone(), Some("Spam"))
            .unwrap();
        assert_eq!(event.kind, Kind::EventDeletion);
        assert_eq!(event.content, "Spam");

        // 削除対象のイベントIDが含まれていることを確認
        let tags: Vec<_> = event.tags.into_iter().collect();
        for id in &event_ids {
            assert!(tags.iter().any(|t| matches!(t.as_standardized(), Some(nostr_sdk::TagStandard::Event { event_id, .. }) if event_id == id)));
        }
    }

    #[test]
    fn test_create_topic_post() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);

        let event = publisher
            .create_topic_post("bitcoin", "Let's discuss Bitcoin!", None)
            .unwrap();
        assert!(event.content.contains("Let's discuss Bitcoin!"));

        // タグを確認
        let tags: Vec<_> = event.tags.into_iter().collect();
        assert!(tags.iter().any(|t| matches!(t.as_standardized(), Some(nostr_sdk::TagStandard::Hashtag(h)) if h == "bitcoin")));
        assert!(
            tags.iter()
                .any(|t| t.kind().to_string() == "topic" && t.content().is_some())
        );
    }

    #[test]
    fn test_create_topic_post_with_reply() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);

        let reply_to = EventId::from_slice(&[3; 32]).unwrap();
        let event = publisher
            .create_topic_post("nostr", "Reply to thread", Some(reply_to))
            .unwrap();

        // タグにリプライ情報が含まれていることを確認
        let tags: Vec<_> = event.tags.into_iter().collect();
        assert!(tags.iter().any(|t| matches!(t.as_standardized(), Some(nostr_sdk::TagStandard::Event { event_id, .. }) if event_id == &reply_to)));
        assert!(
            tags.iter()
                .any(|t| t.kind().to_string() == "reply" && t.content().is_some())
        );
    }

    #[test]
    fn test_no_keys_error() {
        let publisher = EventPublisher::new();

        // 鍵が設定されていない状態で各メソッドを呼び出すとエラーになることを確認
        assert!(publisher.create_text_note("test", vec![]).is_err());
        assert!(publisher.create_metadata(Metadata::new()).is_err());
        assert!(
            publisher
                .create_reaction(&EventId::from_slice(&[1; 32]).unwrap(), "+")
                .is_err()
        );
        assert!(publisher.create_deletion(vec![], None).is_err());
        assert!(
            publisher
                .create_topic_post("topic", "content", None)
                .is_err()
        );
    }

    #[test]
    fn test_event_signature_verification() {
        let mut publisher = EventPublisher::new();
        let keys = Keys::generate();
        publisher.set_keys(keys);

        // 各種イベントを作成して署名が正しいことを確認
        let events = vec![
            publisher.create_text_note("test", vec![]).unwrap(),
            publisher
                .create_metadata(Metadata::new().name("test"))
                .unwrap(),
            publisher
                .create_reaction(&EventId::from_slice(&[1; 32]).unwrap(), "+")
                .unwrap(),
            publisher
                .create_topic_post("test", "content", None)
                .unwrap(),
        ];

        for event in events {
            assert!(
                event.verify().is_ok(),
                "Event signature verification failed"
            );
        }
    }
}
