use super::*;
use crate::application::ports::{
    direct_message_notifier::DirectMessageNotifier,
    repositories::{
        DirectMessageConversationRecord, DirectMessageCursor, DirectMessageListDirection,
        DirectMessagePageRaw,
    },
};
use async_trait::async_trait;
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Repo {}

    #[async_trait]
    impl DirectMessageRepository for Repo {
        async fn insert_direct_message(
            &self,
            message: &NewDirectMessage,
        ) -> Result<DirectMessage, AppError>;

        async fn list_direct_messages(
            &self,
            owner_npub: &str,
            conversation_npub: &str,
            cursor: Option<DirectMessageCursor>,
            limit: usize,
            direction: DirectMessageListDirection,
        ) -> Result<DirectMessagePageRaw, AppError>;

        async fn mark_delivered_by_client_id(
            &self,
            owner_npub: &str,
            client_message_id: &str,
            event_id: Option<String>,
            delivered: bool,
        ) -> Result<(), AppError>;

        async fn upsert_conversation_metadata(
            &self,
            owner_npub: &str,
            conversation_npub: &str,
            last_message_id: i64,
            last_message_created_at: i64,
        ) -> Result<(), AppError>;

        async fn mark_conversation_as_read(
            &self,
            owner_npub: &str,
            conversation_npub: &str,
            read_at: i64,
        ) -> Result<(), AppError>;

        async fn list_direct_message_conversations(
            &self,
            owner_npub: &str,
            limit: usize,
        ) -> Result<Vec<DirectMessageConversationRecord>, AppError>;
    }
}

mock! {
    pub Gateway {}

    #[async_trait]
    impl MessagingGateway for Gateway {
        async fn encrypt_and_send(
            &self,
            owner_npub: &str,
            recipient_npub: &str,
            plaintext: &str,
        ) -> Result<MessagingSendResult, AppError>;

        async fn encrypt_only(
            &self,
            owner_npub: &str,
            recipient_npub: &str,
            plaintext: &str,
        ) -> Result<String, AppError>;

        async fn decrypt_with_counterparty(
            &self,
            owner_npub: &str,
            counterparty_npub: &str,
            ciphertext: &str,
        ) -> Result<String, AppError>;
    }
}

mock! {
    pub Notifier {}

    #[async_trait]
    impl DirectMessageNotifier for Notifier {
        async fn notify(
            &self,
            owner_npub: &str,
            message: &DirectMessage,
        ) -> Result<(), AppError>;
    }
}

#[tokio::test]
async fn send_direct_message_success() {
    let mut repo = MockRepo::new();
    repo.expect_insert_direct_message()
        .times(1)
        .withf(|message| {
            message.owner_npub == "npub_sender"
                && message.recipient_npub == "npub_recipient"
                && message.payload_cipher_base64 == "cipher"
        })
        .returning(|message| {
            Ok(DirectMessage::new(
                1,
                message.owner_npub.clone(),
                message.conversation_npub.clone(),
                message.sender_npub.clone(),
                message.recipient_npub.clone(),
                message.event_id.clone(),
                message.client_message_id.clone(),
                message.payload_cipher_base64.clone(),
                message.created_at.timestamp_millis(),
                message.delivered,
                message.direction,
            ))
        });

    repo.expect_upsert_conversation_metadata()
        .times(1)
        .withf(|owner, conv, _, _| owner == "npub_sender" && conv == "npub_recipient")
        .returning(|_, _, _, _| Ok(()));

    let mut gateway = MockGateway::new();
    gateway
        .expect_encrypt_and_send()
        .times(1)
        .returning(|_, _, _| {
            Ok(MessagingSendResult {
                event_id: Some("event1".to_string()),
                ciphertext: "cipher".to_string(),
                created_at_millis: 1000,
                delivered: true,
            })
        });

    let repo: Arc<dyn DirectMessageRepository> = Arc::new(repo);
    let gateway: Arc<dyn MessagingGateway> = Arc::new(gateway);
    let service = DirectMessageService::new(repo, gateway, None);

    let result = service
        .send_direct_message(
            "npub_sender",
            "npub_recipient",
            " hello world ",
            Some("client-1".to_string()),
        )
        .await
        .expect("success");

    assert_eq!(result.event_id.as_deref(), Some("event1"));
    assert!(!result.queued);
    assert_eq!(result.message.decrypted_content.unwrap(), "hello world");
}

#[tokio::test]
async fn list_direct_messages_decrypts_payloads() {
    let mut repo = MockRepo::new();
    let page_raw = DirectMessagePageRaw {
        items: vec![DirectMessage::new(
            1,
            "npub_owner".to_string(),
            "npub_partner".to_string(),
            "npub_owner".to_string(),
            "npub_partner".to_string(),
            Some("event1".to_string()),
            Some("client-1".to_string()),
            "cipher".to_string(),
            1000,
            true,
            MessageDirection::Outbound,
        )],
        next_cursor: Some("1000:event1".to_string()),
        has_more: false,
    };

    repo.expect_list_direct_messages()
        .times(1)
        .returning(move |_, _, _, _, _| Ok(page_raw.clone()));

    let mut gateway = MockGateway::new();
    gateway
        .expect_decrypt_with_counterparty()
        .times(1)
        .returning(|_, _, _| Ok("decrypted".to_string()));

    let repo: Arc<dyn DirectMessageRepository> = Arc::new(repo);
    let gateway: Arc<dyn MessagingGateway> = Arc::new(gateway);
    let service = DirectMessageService::new(repo, gateway, None);

    let page = service
        .list_direct_messages(
            "npub_owner",
            "npub_partner",
            None,
            Some(20),
            MessagePageDirection::Backward,
        )
        .await
        .expect("list succeeds");

    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].decrypted_content.as_deref(),
        Some("decrypted")
    );
    assert_eq!(page.next_cursor.as_deref(), Some("1000:event1"));
    assert!(!page.has_more);
}

#[tokio::test]
async fn send_direct_message_rejects_empty_content() {
    let repo = MockRepo::new();
    let gateway = MockGateway::new();

    let repo: Arc<dyn DirectMessageRepository> = Arc::new(repo);
    let gateway: Arc<dyn MessagingGateway> = Arc::new(gateway);
    let service = DirectMessageService::new(repo, gateway, None);

    let error = service
        .send_direct_message("npub_owner", "npub_partner", "   ", None)
        .await
        .expect_err("validation error");

    assert_eq!(
        error.validation_kind(),
        Some(ValidationFailureKind::Generic)
    );
}

#[tokio::test]
async fn ingest_incoming_message_stores_and_notifies() {
    let mut repo = MockRepo::new();
    repo.expect_insert_direct_message()
        .times(1)
        .withf(|message| {
            message.owner_npub == "npub_owner"
                && message.conversation_npub == "npub_sender"
                && message.direction == MessageDirection::Inbound
                && message.delivered
        })
        .returning(|message| {
            Ok(DirectMessage::new(
                1,
                message.owner_npub.clone(),
                message.conversation_npub.clone(),
                message.sender_npub.clone(),
                message.recipient_npub.clone(),
                message.event_id.clone(),
                message.client_message_id.clone(),
                message.payload_cipher_base64.clone(),
                message.created_at.timestamp_millis(),
                message.delivered,
                message.direction,
            ))
        });

    repo.expect_upsert_conversation_metadata()
        .times(1)
        .withf(|owner, conv, _, _| owner == "npub_owner" && conv == "npub_sender")
        .returning(|_, _, _, _| Ok(()));

    let mut gateway = MockGateway::new();
    gateway
        .expect_decrypt_with_counterparty()
        .times(1)
        .returning(|_, _, _| Ok("hello inbound".to_string()));

    let mut notifier = MockNotifier::new();
    notifier
        .expect_notify()
        .times(1)
        .withf(|owner, message| {
            owner == "npub_owner"
                && message.conversation_npub == "npub_sender"
                && message.direction == MessageDirection::Inbound
        })
        .return_once(|_, _| Ok(()));

    let repo: Arc<dyn DirectMessageRepository> = Arc::new(repo);
    let gateway: Arc<dyn MessagingGateway> = Arc::new(gateway);
    let notifier: Arc<dyn DirectMessageNotifier> = Arc::new(notifier);
    let service = DirectMessageService::new(repo, gateway, Some(notifier));

    let stored = service
        .ingest_incoming_message(
            "npub_owner",
            "npub_sender",
            "ciphertext",
            Some("event1".to_string()),
            1_730_000_000_000,
        )
        .await
        .expect("ingest succeeds");

    let message = stored.expect("message stored");
    assert_eq!(message.decrypted_content.as_deref(), Some("hello inbound"));
    assert_eq!(message.direction, MessageDirection::Inbound);
}

#[tokio::test]
async fn ingest_incoming_message_ignores_duplicates() {
    let mut repo = MockRepo::new();
    repo.expect_insert_direct_message().times(1).returning(|_| {
        Err(AppError::Database(
            "UNIQUE constraint failed: direct_messages.owner_npub, event_id".to_string(),
        ))
    });

    let mut gateway = MockGateway::new();
    gateway
        .expect_decrypt_with_counterparty()
        .times(1)
        .returning(|_, _, _| Ok("ignored".to_string()));

    let repo: Arc<dyn DirectMessageRepository> = Arc::new(repo);
    let gateway: Arc<dyn MessagingGateway> = Arc::new(gateway);
    let service = DirectMessageService::new(repo, gateway, None);

    let result = service
        .ingest_incoming_message(
            "npub_owner",
            "npub_sender",
            "cipher",
            Some("evt".into()),
            10,
        )
        .await
        .expect("duplicate ignored");

    assert!(result.is_none());
}

#[tokio::test]
async fn list_direct_message_conversations_returns_decrypted_last_message() {
    let mut repo = MockRepo::new();
    repo.expect_list_direct_message_conversations()
        .times(1)
        .withf(|owner, limit| owner == "npub_owner" && *limit == 20)
        .returning(|_, _| {
            Ok(vec![DirectMessageConversationRecord {
                owner_npub: "npub_owner".into(),
                conversation_npub: "npub_friend".into(),
                last_message: Some(DirectMessage::new(
                    42,
                    "npub_owner".into(),
                    "npub_friend".into(),
                    "npub_owner".into(),
                    "npub_friend".into(),
                    Some("evt".into()),
                    Some("client".into()),
                    "cipher".into(),
                    1_700_000_000_000,
                    true,
                    MessageDirection::Outbound,
                )),
                last_read_at: 0,
                unread_count: 3,
            }])
        });

    let mut gateway = MockGateway::new();
    gateway
        .expect_decrypt_with_counterparty()
        .times(1)
        .returning(|_, _, _| Ok("hello friend".into()));

    let repo: Arc<dyn DirectMessageRepository> = Arc::new(repo);
    let gateway: Arc<dyn MessagingGateway> = Arc::new(gateway);
    let service = DirectMessageService::new(repo, gateway, None);

    let items = service
        .list_direct_message_conversations("npub_owner", Some(20))
        .await
        .expect("list succeeds");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].conversation_npub, "npub_friend");
    assert_eq!(items[0].unread_count, 3);
    assert_eq!(
        items[0]
            .last_message
            .as_ref()
            .and_then(|message| message.decrypted_content.clone()),
        Some("hello friend".into())
    );
}

#[tokio::test]
async fn mark_conversation_as_read_clamps_negative_timestamps() {
    let mut repo = MockRepo::new();
    repo.expect_mark_conversation_as_read()
        .times(1)
        .withf(|_, _, read_at| *read_at == 0)
        .returning(|_, _, _| Ok(()));

    let repo: Arc<dyn DirectMessageRepository> = Arc::new(repo);
    let gateway = MockGateway::new();
    let gateway: Arc<dyn MessagingGateway> = Arc::new(gateway);
    let service = DirectMessageService::new(repo, gateway, None);

    service
        .mark_conversation_as_read("npub_owner", "npub_friend", -100)
        .await
        .expect("mark succeeds");
}
