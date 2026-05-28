use super::*;

#[test]
fn shared_identity_public_replication_prefers_direct_connected_runtime() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &[], &["assist-peer"]);
    let subscriber_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);

    assert!(should_swap_shared_identity_public_replication_direction(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
    ));
}

#[test]
fn shared_identity_public_replication_keeps_original_publisher_when_it_is_direct() {
    let topic = "kukuri:topic:test";
    let publisher_status = sync_status_with_topic(topic, &["direct-peer"], &["assist-peer"]);
    let subscriber_status = sync_status_with_topic(topic, &[], &["assist-peer"]);

    assert!(!should_swap_shared_identity_public_replication_direction(
        &publisher_status,
        &subscriber_status,
        topic,
        1,
    ));
}
