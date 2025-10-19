use super::*;
use crate::application::services::subscription_state::{
    SubscriptionRecord, SubscriptionStateStore, SubscriptionStatus, SubscriptionTarget,
};
use crate::modules::event::manager::EventManager;
use crate::shared::error::AppError;
use mockall::predicate::*;
use std::sync::Arc;

mod support;

use support::fixtures::create_test_event;
use support::mocks::{
    MockEventDist, MockEventRepo, MockSignatureServ, MockSubscriptionInvokerMock,
    MockSubscriptionStateMock,
};

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

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let result = service
        .create_event(
            1,
            "Test content".to_string(),
            "test_pubkey".to_string(),
            "test_private_key",
        )
        .await;

    assert!(result.is_ok());
    let event = result.unwrap();
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

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let event = create_test_event();
    let result = service.process_received_event(event).await;

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

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let event = create_test_event();
    let result = service.process_received_event(event).await;

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
    let mock_repo = MockEventRepo::new();
    let mock_signature = MockSignatureServ::new();
    let mock_distributor = MockEventDist::new();

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let result = service.delete_events(vec!["abc".to_string()], None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::ConfigurationError(_)));
}

#[tokio::test]
async fn test_delete_events_with_invalid_id() {
    let mock_repo = MockEventRepo::new();
    let mock_signature = MockSignatureServ::new();
    let mock_distributor = MockEventDist::new();

    let mut service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    service.set_event_manager(Arc::new(EventManager::new()));

    let result = service
        .delete_events(vec!["invalid".to_string()], None)
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::ValidationError(_)));
}

#[tokio::test]
async fn test_delete_events_event_manager_failure_maps_to_nostr_error() {
    let mock_repo = MockEventRepo::new();
    let mock_signature = MockSignatureServ::new();
    let mock_distributor = MockEventDist::new();

    let mut service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    service.set_event_manager(Arc::new(EventManager::new()));

    let valid_id = format!("{:064x}", 1);
    let result = service
        .delete_events(vec![valid_id], Some("cleanup".to_string()))
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, AppError::NostrError(_)));
}

#[tokio::test]
async fn test_get_event() {
    let mut mock_repo = MockEventRepo::new();
    let test_event = create_test_event();
    let test_event_clone = test_event.clone();

    mock_repo
        .expect_get_event()
        .with(eq("test_id"))
        .times(1)
        .returning(move |_| Ok(Some(test_event_clone.clone())));

    let mock_signature = MockSignatureServ::new();
    let mock_distributor = MockEventDist::new();

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let result = service.get_event("test_id").await;

    assert!(result.is_ok());
    let event_opt = result.unwrap();
    assert!(event_opt.is_some());
    let event = event_opt.unwrap();
    assert_eq!(event.content, "Test content");
}

#[tokio::test]
async fn test_get_events_by_kind() {
    let mut mock_repo = MockEventRepo::new();
    let test_events = vec![create_test_event(), create_test_event()];
    let test_events_clone = test_events.clone();

    mock_repo
        .expect_get_events_by_kind()
        .with(eq(1u32), eq(10usize))
        .times(1)
        .returning(move |_, _| Ok(test_events_clone.clone()));

    let mock_signature = MockSignatureServ::new();
    let mock_distributor = MockEventDist::new();

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let result = service.get_events_by_kind(1, 10).await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_get_events_by_author() {
    let mut mock_repo = MockEventRepo::new();
    let test_events = vec![create_test_event()];
    let test_events_clone = test_events.clone();

    mock_repo
        .expect_get_events_by_author()
        .with(eq("test_pubkey"), eq(5usize))
        .times(1)
        .returning(move |_, _| Ok(test_events_clone.clone()));

    let mock_signature = MockSignatureServ::new();
    let mock_distributor = MockEventDist::new();

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let result = service.get_events_by_author("test_pubkey", 5).await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].pubkey, "test_pubkey");
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

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let result = service
        .delete_event(
            "event_to_delete",
            "test_pubkey".to_string(),
            "test_private_key",
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_pending_events() {
    let mut mock_repo = MockEventRepo::new();
    let test_events = vec![create_test_event(), create_test_event()];
    let test_events_clone = test_events.clone();

    mock_repo
        .expect_get_unsync_events()
        .times(1)
        .returning(move || Ok(test_events_clone.clone()));

    mock_repo
        .expect_mark_event_synced()
        .times(2)
        .returning(|_| Ok(()));

    let mock_signature = MockSignatureServ::new();

    let mut mock_distributor = MockEventDist::new();
    mock_distributor
        .expect_distribute()
        .times(2)
        .returning(|_, _| Ok(()));

    let service = EventService::new(
        Arc::new(mock_repo),
        Arc::new(mock_signature),
        Arc::new(mock_distributor),
        Arc::new(MockSubscriptionStateMock::new()) as Arc<dyn SubscriptionStateStore>,
    );

    let result = service.sync_pending_events().await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
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

    let service = {
        let mock_repo = MockEventRepo::new();
        let mock_signature = MockSignatureServ::new();
        let mock_distributor = MockEventDist::new();
        let mut service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
            Arc::new(mock_state) as Arc<dyn SubscriptionStateStore>,
        );
        service.set_subscription_invoker(Arc::new(mock_invoker));
        service
    };

    let result = service.subscribe_to_topic("topic").await;
    assert!(result.is_ok());
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

    let service = {
        let mock_repo = MockEventRepo::new();
        let mock_signature = MockSignatureServ::new();
        let mock_distributor = MockEventDist::new();
        let mut service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
            Arc::new(mock_state) as Arc<dyn SubscriptionStateStore>,
        );
        service.set_subscription_invoker(Arc::new(mock_invoker));
        service
    };

    let result = service.subscribe_to_topic("topic").await;
    assert!(result.is_err());
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
    let topic_record_for_list = topic_record.clone();
    let user_record_for_list = user_record.clone();
    let user_record_for_predicate = user_record.clone();

    let mut mock_state = MockSubscriptionStateMock::new();
    mock_state
        .expect_list_for_restore()
        .times(1)
        .return_once(move || Ok(vec![topic_record_for_list, user_record_for_list]));
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
                    == user_record_for_predicate
                        .last_synced_at
                        .map(|value| (value - 300) as u64)
        })
        .return_once(|_, _| Ok(()));

    let service = {
        let mock_repo = MockEventRepo::new();
        let mock_signature = MockSignatureServ::new();
        let mock_distributor = MockEventDist::new();
        let mut service = EventService::new(
            Arc::new(mock_repo),
            Arc::new(mock_signature),
            Arc::new(mock_distributor),
            Arc::new(mock_state) as Arc<dyn SubscriptionStateStore>,
        );
        service.set_subscription_invoker(Arc::new(mock_invoker));
        service
    };

    let result = service.handle_network_connected().await;
    assert!(result.is_ok());
}
