use super::*;

#[test]
fn public_replication_direction_prefers_direct_connected_subscriber_when_requested() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer"]);
    let subscriber_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);

    assert!(should_publish_from_direct_connected_subscriber(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
        PublicReplicationDirection::PreferDirectConnectedSubscriber,
    ));
}
#[test]
fn public_replication_direction_keeps_original_publisher_by_default() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer"]);
    let subscriber_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);

    assert!(!should_publish_from_direct_connected_subscriber(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
        PublicReplicationDirection::PreferOriginalPublisher,
    ));
}

#[test]
fn public_replication_retry_flips_to_subscriber_when_both_sides_are_assist_only() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer-a"]);
    let subscriber_status = sync_status_with_topic(topic, &[], &["assist-peer-b"]);

    assert!(!should_retry_public_replication_from_subscriber(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
        PublicReplicationDirection::PreferDirectConnectedSubscriber,
        1,
    ));
    assert!(should_retry_public_replication_from_subscriber(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
        PublicReplicationDirection::PreferDirectConnectedSubscriber,
        2,
    ));
    assert!(!should_retry_public_replication_from_subscriber(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
        PublicReplicationDirection::PreferDirectConnectedSubscriber,
        3,
    ));
}

#[test]
fn public_feature_selection_prefers_direct_connected_subscriber_when_available() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer"]);
    let subscriber_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);

    let selection =
        select_public_feature_strategy(&publisher_status, &subscriber_status, topic, 1, 1);

    assert!(selection.select_subscriber);
    assert!(selection.require_direct_subscriber);
}

#[test]
fn public_feature_selection_retry_flips_to_subscriber_without_direct_requirement() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer-a"]);
    let subscriber_status = sync_status_with_topic(topic, &[], &["assist-peer-b"]);

    let selection =
        select_public_feature_strategy(&publisher_status, &subscriber_status, topic, 1, 2);

    assert!(selection.select_subscriber);
    assert!(!selection.require_direct_subscriber);
}

#[test]
fn direct_topic_readiness_rejects_pending_join_errors() {
    let topic = "kukuri:topic:test";
    let mut status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);
    status.last_error = Some("topic join pending: timed out waiting for initial topic join".into());

    assert!(!topic_has_direct_peer_without_pending_join(
        &status, topic, 1
    ));
}
