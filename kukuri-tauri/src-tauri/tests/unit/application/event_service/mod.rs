#[path = "../../../common/mod.rs"]
mod common;

use std::sync::Arc;

use common::mocks::{
    event_manager_stub, MockEventDist, MockEventRepo, MockSignatureServ,
    MockSubscriptionInvokerMock, MockSubscriptionStateMock,
};
use kukuri_lib::application::services::event_service::{invoker::SubscriptionInvoker, EventService};
use kukuri_lib::application::services::subscription_state::{
    SubscriptionRecord, SubscriptionStateStore, SubscriptionStatus, SubscriptionTarget,
};
use kukuri_lib::domain::entities::Event;
use kukuri_lib::infrastructure::crypto::SignatureService;
use kukuri_lib::infrastructure::database::EventRepository;
use kukuri_lib::infrastructure::p2p::EventDistributor;
use kukuri_lib::shared::error::AppError;
use mockall::predicate::*;

fn create_test_event() -> Event {
    Event::new(1, "Test content".into(), "test_pubkey".into())
}

fn service_with_state(
    repo: MockEventRepo,
    signature: MockSignatureServ,
    distributor: MockEventDist,
    state: MockSubscriptionStateMock,
) -> EventService {
    EventService::new(
        Arc::new(repo) as Arc<dyn EventRepository>,
        Arc::new(signature) as Arc<dyn SignatureService>,
        Arc::new(distributor) as Arc<dyn EventDistributor>,
        Arc::new(state) as Arc<dyn SubscriptionStateStore>,
    )
}

#[tokio::test]
async fn test_create_event_success() {
    let mut mock_repo = MockEventRepo::new();
    mock_repo
        .expect_create_event()
        .times(1)
        .returning(|_| Ok(()));

    let mut mock_signature = MockSignatureServ::new();
    mock_signature
        .expect_sign_event()
        .times(1)
        .returning(|_, _| Ok(()));

    let mut mock_distributor = MockEventDist::new();
    mock_distributor
        .expect_distribute()
        .times(1)
        .returning(|_, _| Ok(()));

    let service = service_with_state(
        mock_repo,
        mock_signature,
        mock_distributor,
        MockSubscriptionStateMock::new(),
    );

    let event = service
        .create_event(
            1,
            "Test content".to_string(),
            "test_pubkey".to_string(),
            "test_private_key",
        )
        .await
        .expect("create_event should succeed");

    assert_eq!(event.content, "Test content");
    assert_eq!(event.pubkey, "test_pubkey");
}

#[tokio::test]
async fn test_process_received_event_valid_signature() {
    let mut mock_repo = MockEventRepo::new();
    mock_repo
        .expect_create_event()
        .times(1)
        .returning(|_| Ok(()));

    let mut mock_signature = MockSignatureServ::new();
    mock_signature
        .expect_verify_event()
        .times(1)
        .returning(|_| Ok(true));

    let mock_distributor = MockEventDist::new();

    let service = service_with_state(
        mock_repo,
        mock_signature,
        mock_distributor,
        MockSubscriptionStateMock::new(),
    );

    let result = service.process_received_event(create_test_event()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_received_event_invalid_signature() {
    let mock_repo = MockEventRepo::new();

    let mut mock_signature = MockSignatureServ::new();
    mock_signature
        .expect_verify_event()
        .times(1)
        .returning(|_| Ok(false));

    let mock_distributor = MockEventDist::new();

    let service = service_with_state(
        mock_repo,
        mock_signature,
        mock_distributor,
        MockSubscriptionStateMock::new(),
    );

    let result = service.process_received_event(create_test_event()).await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid event signature")
    );
}

#[tokio::test]
async fn test_delete_events_without_manager_fails() {
    let service = service_with_state(
        MockEventRepo::new(),
        MockSignatureServ::new(),
        MockEventDist::new(),
        MockSubscriptionStateMock::new(),
    );

    let err = service
        .delete_events(vec!["abc".to_string()], None)
        .await
        .expect_err("should fail without manager");

    assert!(matches!(err, AppError::ConfigurationError(_)));
}

#[tokio::test]
async fn test_delete_events_with_invalid_id() {
    let mut service = service_with_state(
        MockEventRepo::new(),
        MockSignatureServ::new(),
        MockEventDist::new(),
        MockSubscriptionStateMock::new(),
    );

    service.set_event_manager(event_manager_stub());

    let err = service
        .delete_events(vec!["invalid".to_string()], None)
        .await
        .expect_err("invalid id should fail");

    assert!(matches!(err, AppError::ValidationError(_)));
}

#[tokio::test]
async fn test_delete_events_event_manager_failure_maps_to_nostr_error() {
    let mut service = service_with_state(
        MockEventRepo::new(),
        MockSignatureServ::new(),
        MockEventDist::new(),
        MockSubscriptionStateMock::new(),
    );

    service.set_event_manager(event_manager_stub());

    let valid_id = format!("{:064x}", 1);
    let err = service
        .delete_events(vec![valid_id], Some("cleanup".to_string()))
        .await
        .expect_err("manager failure should map to nostr error");

    assert!(matches!(err, AppError::NostrError(_)));
}

#[tokio::test]
async fn test_get_event() {
    let mut mock_repo = MockEventRepo::new();
    let event = create_test_event();
    let cloned = event.clone();

    mock_repo
        .expect_get_event()
        .with(eq("test_id"))
        .times(1)
        .returning(move |_| Ok(Some(cloned.clone())));

    let service = service_with_state(
        mock_repo,
        MockSignatureServ::new(),
        MockEventDist::new(),
        MockSubscriptionStateMock::new(),
    );

    let fetched = service
        .get_event("test_id")
        .await
        .expect("get_event should succeed");

    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().content, "Test content");
}

#[tokio::test]
async fn test_get_events_by_kind() {
    let mut mock_repo = MockEventRepo::new();
    let events = vec![create_test_event(), create_test_event()];
    let cloned = events.clone();

    mock_repo
        .expect_get_events_by_kind()
        .with(eq(1u32), eq(10usize))
        .times(1)
        .returning(move |_, _| Ok(cloned.clone()));

    let service = service_with_state(
        mock_repo,
        MockSignatureServ::new(),
        MockEventDist::new(),
        MockSubscriptionStateMock::new(),
    );

    let fetched = service
        .get_events_by_kind(1, 10)
        .await
        .expect("get_events_by_kind should succeed");

    assert_eq!(fetched.len(), 2);
}

#[tokio::test]
async fn test_get_events_by_author() {
    let mut mock_repo = MockEventRepo::new();
    let events = vec![create_test_event()];
    let cloned = events.clone();

    mock_repo
        .expect_get_events_by_author()
        .with(eq("test_pubkey"), eq(5usize))
        .times(1)
        .returning(move |_, _| Ok(cloned.clone()));

    let service = service_with_state(
        mock_repo,
        MockSignatureServ::new(),
        MockEventDist::new(),
        MockSubscriptionStateMock::new(),
    );

    let fetched = service
        .get_events_by_author("test_pubkey", 5)
        .await
        .expect("get_events_by_author should succeed");

    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].pubkey, "test_pubkey");
}

#[tokio::test]
async fn test_delete_event() {
    let mut mock_repo = MockEventRepo::new();
    mock_repo
        .expect_create_event()
        .times(1)
        .returning(|_| Ok(()));
    mock_repo
        .expect_delete_event()
        .with(eq("event_to_delete"))
        .times(1)
        .returning(|_| Ok(()));

    let mut mock_signature = MockSignatureServ::new();
    mock_signature
        .expect_sign_event()
        .times(1)
        .returning(|_, _| Ok(()));

    let mut mock_distributor = MockEventDist::new();
    mock_distributor
        .expect_distribute()
        .times(1)
        .returning(|_, _| Ok(()));

    let service = service_with_state(
        mock_repo,
        mock_signature,
        mock_distributor,
        MockSubscriptionStateMock::new(),
    );

    service
        .delete_event(
            "event_to_delete",
            "test_pubkey".to_string(),
            "test_private_key",
        )
        .await
        .expect("delete_event should succeed");
}

#[tokio::test]
async fn test_sync_pending_events() {
    let mut mock_repo = MockEventRepo::new();
    let events = vec![create_test_event(), create_test_event()];
    let cloned = events.clone();

    mock_repo
        .expect_get_unsync_events()
        .times(1)
        .returning(move || Ok(cloned.clone()));

    mock_repo
        .expect_mark_event_synced()
        .times(events.len())
        .returning(|_| Ok(()));

    let mut mock_distributor = MockEventDist::new();
    mock_distributor
        .expect_distribute()
        .times(events.len())
        .returning(|_, _| Ok(()));

    let service = service_with_state(
        mock_repo,
        MockSignatureServ::new(),
        mock_distributor,
        MockSubscriptionStateMock::new(),
    );

    let synced = service
        .sync_pending_events()
        .await
        .expect("sync_pending_events should succeed");

    assert_eq!(synced, 2);
}

#[tokio::test]
async fn test_subscribe_to_topic_uses_state_machine_and_invoker() {
    let record = SubscriptionRecord {
        target: SubscriptionTarget::Topic("topic".into()),
        status: SubscriptionStatus::Pending,
        last_synced_at: None,
        last_attempt_at: None,
        failure_count: 0,
        error_message: None,
    };

    let mut mock_state = MockSubscriptionStateMock::new();
    mock_state
        .expect_record_request()
        .times(1)
        .withf(|target| matches!(target, SubscriptionTarget::Topic(t) if t == "topic"))
        .return_once(move |_| Ok(record.clone()));
    mock_state
        .expect_mark_subscribed()
        .times(1)
        .withf(|target, _| matches!(target, SubscriptionTarget::Topic(t) if t == "topic"))
        .return_once(|_, _| Ok(()));

    let mut mock_invoker = MockSubscriptionInvokerMock::new();
    mock_invoker
        .expect_subscribe_topic()
        .times(1)
        .with(eq("topic"), eq(None))
        .return_once(|_, _| Ok(()));

    let mut service = service_with_state(
        MockEventRepo::new(),
        MockSignatureServ::new(),
        MockEventDist::new(),
        mock_state,
    );
    service.set_subscription_invoker(Arc::new(mock_invoker) as Arc<dyn SubscriptionInvoker>);

    service
        .subscribe_to_topic("topic")
        .await
        .expect("subscribe_to_topic should succeed");
}

#[tokio::test]
async fn test_subscribe_to_topic_failure_marks_state() {
    let record = SubscriptionRecord {
        target: SubscriptionTarget::Topic("topic".into()),
        status: SubscriptionStatus::Pending,
        last_synced_at: None,
        last_attempt_at: None,
        failure_count: 0,
        error_message: None,
    };

    let mut mock_state = MockSubscriptionStateMock::new();
    mock_state
        .expect_record_request()
        .times(1)
        .return_once(move |_| Ok(record.clone()));
    mock_state
        .expect_mark_failure()
        .times(1)
        .withf(|target, message| {
            matches!(target, SubscriptionTarget::Topic(t) if t == "topic")
                && message.contains("failed")
        })
        .return_once(|_, _| Ok(()));

    let mut mock_invoker = MockSubscriptionInvokerMock::new();
    mock_invoker
        .expect_subscribe_topic()
        .times(1)
        .return_once(|_, _| Err(AppError::NostrError("failed".into())));

    let mut service = service_with_state(
        MockEventRepo::new(),
        MockSignatureServ::new(),
        MockEventDist::new(),
        mock_state,
    );
    service.set_subscription_invoker(Arc::new(mock_invoker) as Arc<dyn SubscriptionInvoker>);

    let err = service
        .subscribe_to_topic("topic")
        .await
        .expect_err("subscription failure should bubble up");

    assert!(matches!(err, AppError::NostrError(_)));
}

#[tokio::test]
async fn test_handle_network_connected_restores_subscriptions() {
    let topic_record = SubscriptionRecord {
        target: SubscriptionTarget::Topic("topic".into()),
        status: SubscriptionStatus::NeedsResync,
        last_synced_at: None,
        last_attempt_at: None,
        failure_count: 0,
        error_message: None,
    };
    let user_record = SubscriptionRecord {
        target: SubscriptionTarget::User("user".into()),
        status: SubscriptionStatus::Pending,
        last_synced_at: Some(3600),
        last_attempt_at: None,
        failure_count: 1,
        error_message: Some("previous failure".into()),
    };
    let list_topic = topic_record.clone();
    let list_user = user_record.clone();
    let predicate_user = user_record.clone();

    let mut mock_state = MockSubscriptionStateMock::new();
    mock_state
        .expect_list_for_restore()
        .times(1)
        .return_once(move || Ok(vec![list_topic, list_user]));
    mock_state
        .expect_mark_subscribed()
        .times(2)
        .returning(|_, _| Ok(()));

    let mut mock_invoker = MockSubscriptionInvokerMock::new();
    mock_invoker
        .expect_subscribe_topic()
        .times(1)
        .with(eq("topic"), eq(None))
        .return_once(|_, _| Ok(()));
    mock_invoker
        .expect_subscribe_user()
        .times(1)
        .withf(move |pubkey, since| {
            pubkey == "user"
                && since.map(|ts| ts.as_u64())
                    == predicate_user
                        .last_synced_at
                        .map(|value| (value - 300) as u64)
        })
        .return_once(|_, _| Ok(()));

    let mut service = service_with_state(
        MockEventRepo::new(),
        MockSignatureServ::new(),
        MockEventDist::new(),
        mock_state,
    );
    service.set_subscription_invoker(Arc::new(mock_invoker) as Arc<dyn SubscriptionInvoker>);

    service
        .handle_network_connected()
        .await
        .expect("handle_network_connected should succeed");
}
