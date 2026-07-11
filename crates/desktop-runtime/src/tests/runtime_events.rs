use super::*;

#[test]
fn sync_status_changed_event_wire_shape_is_stable() {
    let event = RuntimeEvent::SyncStatusChanged {
        sync_status: None,
        community_node_statuses: None,
    };

    assert_eq!(
        serde_json::to_value(event).expect("serialize runtime event"),
        serde_json::json!({
            "type": "sync_status_changed",
            "sync_status": null,
            "community_node_statuses": null,
        })
    );
}

#[tokio::test]
async fn sync_status_observer_emits_only_changed_snapshot_parts_and_stops_on_shutdown() {
    let _resource = lock_test_resource(TestResource::CommunityNodeServer).await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("runtime-events.db");
    let runtime = Arc::new(
        DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime"),
    );
    let mut events = runtime.subscribe_events();

    runtime
        .start_sync_status_observer_with_interval(Duration::from_millis(25))
        .await;
    runtime
        .start_sync_status_observer_with_interval(Duration::from_millis(25))
        .await;

    let first = timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("initial event timeout")
        .expect("initial event");
    match first {
        RuntimeEvent::SyncStatusChanged {
            sync_status,
            community_node_statuses,
        } => {
            assert!(sync_status.is_some());
            assert_eq!(community_node_statuses, Some(Vec::new()));
        }
        other => panic!("unexpected initial event: {other:?}"),
    }

    assert!(
        timeout(Duration::from_millis(150), events.recv())
            .await
            .is_err(),
        "unchanged snapshots and a duplicate start must not emit"
    );

    runtime
        .set_topic_gossip_enabled(SetTopicGossipEnabledRequest {
            topic: "kukuri:topic:q2b-observer".to_string(),
            enabled: false,
        })
        .await
        .expect("disable topic gossip");
    let sync_changed = timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("sync change timeout")
        .expect("sync change event");
    match sync_changed {
        RuntimeEvent::SyncStatusChanged {
            sync_status,
            community_node_statuses,
        } => {
            assert!(
                sync_status
                    .expect("changed sync status")
                    .gossip_disabled_topics
                    .contains(&"kukuri:topic:q2b-observer".to_string())
            );
            assert!(community_node_statuses.is_none());
        }
        other => panic!("unexpected sync event: {other:?}"),
    }

    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: "http://127.0.0.1:9".to_string(),
            auto_approve: false,
            resolved_urls: None,
        }],
    };
    let community_node_changed = timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("community-node change timeout")
        .expect("community-node change event");
    match community_node_changed {
        RuntimeEvent::SyncStatusChanged {
            sync_status,
            community_node_statuses,
        } => {
            assert!(sync_status.is_none());
            let statuses = community_node_statuses.expect("changed community-node statuses");
            assert_eq!(statuses.len(), 1);
            assert_eq!(statuses[0].base_url, "http://127.0.0.1:9");
        }
        other => panic!("unexpected community-node event: {other:?}"),
    }

    runtime.shutdown().await;
    assert!(runtime.sync_status_observer_task.lock().await.is_none());
}
