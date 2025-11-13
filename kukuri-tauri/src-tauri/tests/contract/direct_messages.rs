use chrono::{TimeZone, Utc};
use kukuri_lib::test_support::application::ports::repositories::DirectMessageRepository;
use kukuri_lib::test_support::domain::entities::{MessageDirection, NewDirectMessage};
use kukuri_lib::test_support::infrastructure::database::connection_pool::ConnectionPool;
use kukuri_lib::test_support::infrastructure::database::repository::Repository;
use kukuri_lib::test_support::infrastructure::database::sqlite_repository::SqliteRepository;

const OWNER_NPUB: &str =
    "npub1contractowner00000000000000000000000000000000000000000000000000000000";
const FRIEND_NPUB: &str =
    "npub1contractfriend0000000000000000000000000000000000000000000000000000000";

#[tokio::test]
async fn direct_message_read_receipts_sync_across_devices() {
    let pool = ConnectionPool::new("sqlite::memory:?cache=shared")
        .await
        .expect("create pool");
    let repository = SqliteRepository::new(pool);
    repository.initialize().await.expect("initialize schema");

    let base_timestamp = 1_700_000_000_000i64;
    for (index, content) in ["first", "second", "third"].iter().enumerate() {
        let created_at = Utc
            .timestamp_millis_opt(base_timestamp + (index as i64 * 1_000))
            .single()
            .expect("valid timestamp");
        let new_message = NewDirectMessage {
            owner_npub: OWNER_NPUB.to_string(),
            conversation_npub: FRIEND_NPUB.to_string(),
            sender_npub: FRIEND_NPUB.to_string(),
            recipient_npub: OWNER_NPUB.to_string(),
            event_id: Some(format!("evt-{index}")),
            client_message_id: Some(format!("client-{index}")),
            payload_cipher_base64: format!("cipher-{content}"),
            created_at,
            delivered: true,
            direction: MessageDirection::Inbound,
        };
        let stored = repository
            .insert_direct_message(&new_message)
            .await
            .expect("insert direct message");
        repository
            .upsert_conversation_metadata(
                OWNER_NPUB,
                FRIEND_NPUB,
                stored.id,
                stored.created_at.timestamp_millis(),
            )
            .await
            .expect("upsert metadata");
    }

    let initial = repository
        .list_direct_message_conversations(OWNER_NPUB, 10)
        .await
        .expect("list conversations");
    assert_eq!(initial.len(), 1);
    assert_eq!(
        initial[0].unread_count, 3,
        "all inbound messages start unread"
    );
    assert_eq!(
        initial[0].last_read_at, 0,
        "last_read_at defaults to zero before any device reads"
    );

    let second_message_timestamp = base_timestamp + 1_000;
    repository
        .mark_conversation_as_read(OWNER_NPUB, FRIEND_NPUB, second_message_timestamp)
        .await
        .expect("mark read up to second message");
    let after_second = repository
        .list_direct_message_conversations(OWNER_NPUB, 10)
        .await
        .expect("list after read");
    assert_eq!(
        after_second[0].last_read_at, second_message_timestamp,
        "read receipt should store the latest acknowledged timestamp"
    );
    assert_eq!(
        after_second[0].unread_count, 1,
        "only the newest inbound message remains unread"
    );

    repository
        .mark_conversation_as_read(OWNER_NPUB, FRIEND_NPUB, base_timestamp + 500)
        .await
        .expect("stale read marker should be ignored");
    let after_stale = repository
        .list_direct_message_conversations(OWNER_NPUB, 10)
        .await
        .expect("list after stale mark");
    assert_eq!(
        after_stale[0].last_read_at, second_message_timestamp,
        "stale timestamps must not overwrite newer read positions"
    );
    assert_eq!(after_stale[0].unread_count, 1);

    let final_timestamp = base_timestamp + 2_000;
    repository
        .mark_conversation_as_read(OWNER_NPUB, FRIEND_NPUB, final_timestamp)
        .await
        .expect("mark all messages read");
    let final_state = repository
        .list_direct_message_conversations(OWNER_NPUB, 10)
        .await
        .expect("list after all read");
    assert_eq!(final_state[0].unread_count, 0);
    assert_eq!(final_state[0].last_read_at, final_timestamp);
}
