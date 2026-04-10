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
