use super::super::*;

#[tokio::test]
async fn community_node_status_refresh_updates_bootstrap_seed_peers() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-heartbeat-refresh.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(vec![
            CommunityNodeSeedPeer::new(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                None,
            )
            .expect("seed peer"),
        ])),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let statuses = runtime
        .get_community_node_statuses()
        .await
        .expect("community node statuses");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(statuses.len(), 1);
    assert_eq!(
        statuses[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        state.seed_peers.lock().await.clone()
    );
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        state.seed_peers.lock().await.clone()
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_sync_status_refresh_updates_bootstrap_seed_peers() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-sync-status-refresh.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(vec![
            CommunityNodeSeedPeer::new(
                "1111111111111111111111111111111111111111111111111111111111111111",
                None,
            )
            .expect("seed peer"),
        ])),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let _status = runtime.get_sync_status().await.expect("sync status");

    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        state.seed_peers.lock().await.clone()
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_metadata_refresh_heartbeats_before_bootstrap_sync_even_when_metadata_is_unchanged()
 {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-metadata-refresh-no-churn.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let seed_peer_runtime = DesktopRuntime::new_with_config_and_identity(
        dir.path()
            .join("community-metadata-refresh-no-churn-peer.db"),
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("seed peer runtime");
    let seed_peer = seed_peer_runtime
        .local_community_node_seed_peer("metadata-refresh-test")
        .await
        .expect("seed peer");
    seed_peer_runtime.shutdown().await;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(vec![seed_peer.clone()])),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let _status = runtime
        .get_sync_status()
        .await
        .expect("initial sync status");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    let runtime_connectivity_apply_version = runtime
        .runtime_connectivity_apply_version
        .load(Ordering::SeqCst);
    let effective_seed_peer_apply_version = runtime
        .effective_seed_peer_apply_version
        .load(Ordering::SeqCst);

    runtime
        .community_node_ready_refresh_pending
        .lock()
        .await
        .remove(base_url.as_str());

    let refreshed = runtime
        .refresh_community_node_metadata(CommunityNodeTargetRequest {
            base_url: base_url.clone(),
        })
        .await
        .expect("refresh metadata");

    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 2);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 2);
    assert_eq!(
        refreshed
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![seed_peer]
    );
    assert_eq!(
        runtime
            .runtime_connectivity_apply_version
            .load(Ordering::SeqCst),
        runtime_connectivity_apply_version
    );
    assert_eq!(
        runtime
            .effective_seed_peer_apply_version
            .load(Ordering::SeqCst),
        effective_seed_peer_apply_version
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_ready_transition_refreshes_bootstrap_metadata_before_next_heartbeat_due() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-ready-refresh.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let initial_seed_peer_runtime = DesktopRuntime::new_with_config_and_identity(
        dir.path().join("community-ready-refresh-initial-peer.db"),
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("initial seed peer runtime");
    let initial_seed_peer = initial_seed_peer_runtime
        .local_community_node_seed_peer("ready-refresh-initial")
        .await
        .expect("initial seed peer");
    initial_seed_peer_runtime.shutdown().await;
    let refreshed_seed_peer_runtime = DesktopRuntime::new_with_config_and_identity(
        dir.path().join("community-ready-refresh-refreshed-peer.db"),
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("refreshed seed peer runtime");
    let refreshed_seed_peer = refreshed_seed_peer_runtime
        .local_community_node_seed_peer("ready-refresh-updated")
        .await
        .expect("refreshed seed peer");
    refreshed_seed_peer_runtime.shutdown().await;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(vec![initial_seed_peer.clone()])),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let _status = runtime
        .get_sync_status()
        .await
        .expect("initial sync status");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert!(
        runtime
            .community_node_heartbeat_deadlines
            .lock()
            .await
            .get(base_url.as_str())
            .copied()
            .expect("heartbeat deadline")
            > Utc::now().timestamp()
    );
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![initial_seed_peer]
    );

    *state.seed_peers.lock().await = vec![refreshed_seed_peer.clone()];

    let _status = runtime
        .get_sync_status()
        .await
        .expect("refreshed sync status");

    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 2);
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![refreshed_seed_peer]
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_ready_transition_refreshes_bootstrap_metadata_only_once_before_next_heartbeat_due()
 {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-ready-refresh-once.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let seed_peer_runtime = DesktopRuntime::new_with_config_and_identity(
        dir.path().join("community-ready-refresh-once-peer.db"),
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("seed peer runtime");
    let seed_peer = seed_peer_runtime
        .local_community_node_seed_peer("ready-refresh-once")
        .await
        .expect("seed peer");
    seed_peer_runtime.shutdown().await;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(vec![seed_peer.clone()])),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let _status = runtime
        .get_sync_status()
        .await
        .expect("initial sync status");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);

    let _status = runtime
        .get_sync_status()
        .await
        .expect("ready refresh sync status");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 2);

    let _status = runtime
        .get_sync_status()
        .await
        .expect("steady-state sync status");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 2);

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_status_retries_bootstrap_metadata_when_seed_peers_are_empty() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-metadata-retry.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let seed_peer = CommunityNodeSeedPeer::new(
        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        None,
    )
    .expect("seed peer");
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(Vec::new())),
        heartbeat_seed_peers: Arc::new(Mutex::new(None)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let initial_statuses = runtime
        .get_community_node_statuses()
        .await
        .expect("initial community node statuses");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        initial_statuses[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        Vec::<CommunityNodeSeedPeer>::new()
    );
    assert!(
        runtime
            .community_node_metadata_refresh_deadlines
            .lock()
            .await
            .contains_key(base_url.as_str()),
        "empty bootstrap metadata should schedule a retry"
    );

    *state.seed_peers.lock().await = vec![seed_peer.clone()];
    runtime
        .community_node_metadata_refresh_deadlines
        .lock()
        .await
        .insert(base_url.clone(), Utc::now().timestamp() - 1);

    let refreshed_statuses = runtime
        .get_community_node_statuses()
        .await
        .expect("refreshed community node statuses");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 2);
    assert_eq!(
        refreshed_statuses[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![seed_peer]
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn refresh_community_node_metadata_refreshes_registration_before_bootstrap_sync() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-refresh-heartbeat.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let refreshed_seed_peer = CommunityNodeSeedPeer::new(
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
        Some("127.0.0.1:44003".into()),
    )
    .expect("refreshed seed peer");
    let state = Arc::new(MockCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: Arc::new(Mutex::new(Vec::new())),
        heartbeat_seed_peers: Arc::new(Mutex::new(Some(vec![refreshed_seed_peer.clone()]))),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route("/v1/bootstrap/heartbeat", post(mock_bootstrap_heartbeat))
        .route("/v1/bootstrap/nodes", get(mock_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let status = runtime
        .refresh_community_node_metadata(CommunityNodeTargetRequest {
            base_url: base_url.clone(),
        })
        .await
        .expect("refresh metadata");

    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert!(
        state.bootstrap_hits.load(Ordering::SeqCst) >= 1,
        "metadata refresh should fetch bootstrap nodes"
    );
    assert_eq!(
        status
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![refreshed_seed_peer.clone()]
    );
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![refreshed_seed_peer]
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn refresh_community_node_metadata_requeues_heartbeat_when_runtime_connectivity_changes_local_seed_peer()
 {
    let _serial = acquire_async_test_lock().await;
    let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
        .await
        .expect("relay server");
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-refresh-requeue-heartbeat.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let state = Arc::new(MockHeartbeatEchoCommunityNodeState {
        base_url: base_url.clone(),
        connectivity_urls: vec![relay_url.to_string()],
        seed_peers: Arc::new(Mutex::new(Vec::new())),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
    });
    let app = Router::new()
        .route("/v1/consents/status", get(mock_bootstrap_consent_status))
        .route(
            "/v1/bootstrap/heartbeat",
            post(mock_heartbeat_echo_bootstrap_heartbeat),
        )
        .route(
            "/v1/bootstrap/nodes",
            get(mock_heartbeat_echo_bootstrap_nodes),
        )
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "fake-token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist community-node token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(base_url.clone(), Vec::new(), Vec::new())
                    .expect("resolved urls"),
            ),
        }],
    };

    let initial_seed_peer = runtime
        .local_community_node_seed_peer("initial")
        .await
        .expect("initial seed peer");
    let _status = runtime
        .refresh_community_node_metadata(CommunityNodeTargetRequest {
            base_url: base_url.clone(),
        })
        .await
        .expect("refresh metadata");
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert!(
        state.bootstrap_hits.load(Ordering::SeqCst) >= 1,
        "metadata refresh should fetch bootstrap nodes"
    );

    let refreshed_seed_peer = runtime
        .local_community_node_seed_peer("after-refresh")
        .await
        .expect("refreshed seed peer");
    if refreshed_seed_peer == initial_seed_peer {
        runtime.shutdown().await;
        server.abort();
        return;
    }

    let _status = runtime
        .get_sync_status()
        .await
        .expect("sync status after refresh");

    assert_eq!(
        state.heartbeat_hits.load(Ordering::SeqCst),
        2,
        "runtime should heartbeat again after relay rebuild changes the local seed peer"
    );
    assert_eq!(
        runtime.community_node_config.lock().await.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![refreshed_seed_peer]
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn reapply_community_node_connectivity_forces_unchanged_runtime_inputs() {
    let _serial = acquire_async_test_lock().await;
    let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server()
        .await
        .expect("relay server");
    let dir = tempdir().expect("tempdir");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        dir.path().join("community-force-reapply.db"),
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let seed_peer_runtime = DesktopRuntime::new_with_config_and_identity(
        dir.path().join("community-force-reapply-peer.db"),
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("seed peer runtime");
    let endpoint_id = seed_peer_runtime
        .get_sync_status()
        .await
        .expect("seed peer status")
        .discovery
        .local_endpoint_id;

    apply_relay_backed_community_node_seed_peers(
        &runtime,
        "https://community.example.com",
        relay_url.as_str(),
        vec![CommunityNodeSeedPeer::new(endpoint_id.as_str(), None).expect("seed peer")],
    )
    .await;

    let runtime_connectivity_apply_version = runtime
        .runtime_connectivity_apply_version
        .load(Ordering::SeqCst);
    let effective_seed_peer_apply_version = runtime
        .effective_seed_peer_apply_version
        .load(Ordering::SeqCst);
    let stack_node_before = {
        let current = runtime.iroh_stack.current.lock().await;
        current
            .as_ref()
            .expect("current stack before force reapply")
            .node
            .clone()
    };

    timeout(
        Duration::from_secs(30),
        runtime.reapply_community_node_connectivity(),
    )
    .await
    .expect("force reapply timeout")
    .expect("force reapply");

    assert_eq!(
        runtime
            .runtime_connectivity_apply_version
            .load(Ordering::SeqCst),
        runtime_connectivity_apply_version + 1
    );
    let stack_node_after = {
        let current = runtime.iroh_stack.current.lock().await;
        current
            .as_ref()
            .expect("current stack after force reapply")
            .node
            .clone()
    };
    assert!(
        !Arc::ptr_eq(&stack_node_before, &stack_node_after),
        "force reapply should rebuild the iroh stack even when relay and seed inputs are unchanged"
    );
    assert_eq!(
        runtime
            .effective_seed_peer_apply_version
            .load(Ordering::SeqCst),
        effective_seed_peer_apply_version + 1
    );

    runtime.shutdown().await;
    seed_peer_runtime.shutdown().await;
}
