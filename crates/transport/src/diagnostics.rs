pub(crate) fn peer_status_detail(
    configured_peer_count: usize,
    connected_peer_count: usize,
    subscribed_topic_count: usize,
) -> String {
    if configured_peer_count == 0 {
        "No peers configured".to_string()
    } else if subscribed_topic_count == 0 {
        "No topics subscribed locally".to_string()
    } else if connected_peer_count == 0 {
        "Waiting for configured peers to connect".to_string()
    } else if connected_peer_count < configured_peer_count {
        "Connected to a subset of configured peers".to_string()
    } else {
        "Connected to all configured peers".to_string()
    }
}

pub(crate) fn topic_status_detail(
    configured_peer_count: usize,
    connected_peer_count: usize,
) -> String {
    if configured_peer_count == 0 {
        "No peers configured for this topic".to_string()
    } else if connected_peer_count == 0 {
        "Waiting for configured peers to join this topic".to_string()
    } else if connected_peer_count < configured_peer_count {
        "Connected to a subset of configured peers for this topic".to_string()
    } else {
        "Connected to all configured peers for this topic".to_string()
    }
}
