use super::super::*;

#[tokio::test]
async fn auto_approve_node_bootstraps_session_on_status_refresh() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-auto-approve-session.db");
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
        "2222222222222222222222222222222222222222222222222222222222222222",
        Some("127.0.0.1:44001".into()),
    )
    .expect("seed peer");
    let state = Arc::new(MockManagedCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: vec![seed_peer.clone()],
        consent_accepted: Arc::new(AtomicBool::new(false)),
        current_token: Arc::new(Mutex::new(String::new())),
        challenge_hits: Arc::new(AtomicUsize::new(0)),
        verify_hits: Arc::new(AtomicUsize::new(0)),
        consent_status_hits: Arc::new(AtomicUsize::new(0)),
        consent_accept_hits: Arc::new(AtomicUsize::new(0)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
        simulate_pending_update: Arc::new(AtomicBool::new(false)),
    });
    let app = Router::new()
        .route("/v1/auth/challenge", post(mock_managed_auth_challenge))
        .route("/v1/auth/verify", post(mock_managed_auth_verify))
        .route("/v1/consents/status", get(mock_managed_consent_status))
        .route("/v1/consents", post(mock_managed_accept_consents))
        .route(
            "/v1/bootstrap/heartbeat",
            post(mock_managed_bootstrap_heartbeat),
        )
        .route("/v1/bootstrap/nodes", get(mock_managed_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: true,
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
    assert_eq!(state.challenge_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.verify_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.consent_status_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.consent_accept_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(statuses.len(), 1);
    assert!(statuses[0].auto_approve);
    assert!(statuses[0].auth_state.authenticated);
    assert_eq!(
        statuses[0].session_phase,
        crate::CommunityNodeSessionPhase::Ready
    );
    assert_eq!(statuses[0].retry_after, None);
    assert!(
        statuses[0]
            .consent_state
            .as_ref()
            .expect("consent state")
            .all_required_accepted
    );
    assert_eq!(
        statuses[0]
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
async fn near_expiry_token_triggers_proactive_community_node_reauthentication() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-proactive-reauth.db");
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
        "3333333333333333333333333333333333333333333333333333333333333333",
        None,
    )
    .expect("seed peer");
    let state = Arc::new(MockManagedCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: vec![seed_peer.clone()],
        consent_accepted: Arc::new(AtomicBool::new(true)),
        current_token: Arc::new(Mutex::new("near-expiry-token".into())),
        challenge_hits: Arc::new(AtomicUsize::new(0)),
        verify_hits: Arc::new(AtomicUsize::new(0)),
        consent_status_hits: Arc::new(AtomicUsize::new(0)),
        consent_accept_hits: Arc::new(AtomicUsize::new(0)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
        simulate_pending_update: Arc::new(AtomicBool::new(false)),
    });
    let app = Router::new()
        .route("/v1/auth/challenge", post(mock_managed_auth_challenge))
        .route("/v1/auth/verify", post(mock_managed_auth_verify))
        .route("/v1/consents/status", get(mock_managed_consent_status))
        .route("/v1/consents", post(mock_managed_accept_consents))
        .route(
            "/v1/bootstrap/heartbeat",
            post(mock_managed_bootstrap_heartbeat),
        )
        .route("/v1/bootstrap/nodes", get(mock_managed_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "near-expiry-token".into(),
            expires_at: Utc::now().timestamp() + 60,
        },
    )
    .expect("persist near-expiry token");
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
    assert_eq!(state.challenge_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.verify_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.consent_status_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.consent_accept_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 1);
    assert_eq!(
        statuses[0].session_phase,
        crate::CommunityNodeSessionPhase::Ready
    );
    assert!(statuses[0].auth_state.authenticated);

    let stored = crate::community_node::load_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
    )
    .expect("load token")
    .expect("stored token");
    assert_ne!(stored.access_token, "near-expiry-token");

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn consent_required_node_without_auto_approve_stays_pending() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-consent-pending.db");
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
    let state = Arc::new(MockManagedCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: vec![],
        consent_accepted: Arc::new(AtomicBool::new(false)),
        current_token: Arc::new(Mutex::new("manual-consent-token".into())),
        challenge_hits: Arc::new(AtomicUsize::new(0)),
        verify_hits: Arc::new(AtomicUsize::new(0)),
        consent_status_hits: Arc::new(AtomicUsize::new(0)),
        consent_accept_hits: Arc::new(AtomicUsize::new(0)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
        simulate_pending_update: Arc::new(AtomicBool::new(false)),
    });
    let app = Router::new()
        .route("/v1/auth/challenge", post(mock_managed_auth_challenge))
        .route("/v1/auth/verify", post(mock_managed_auth_verify))
        .route("/v1/consents/status", get(mock_managed_consent_status))
        .route("/v1/consents", post(mock_managed_accept_consents))
        .route(
            "/v1/bootstrap/heartbeat",
            post(mock_managed_bootstrap_heartbeat),
        )
        .route("/v1/bootstrap/nodes", get(mock_managed_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "manual-consent-token".into(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist token");
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
    assert_eq!(state.challenge_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.verify_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.consent_status_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.consent_accept_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 0);
    assert_eq!(
        statuses[0].session_phase,
        crate::CommunityNodeSessionPhase::Idle
    );
    assert!(statuses[0].auth_state.authenticated);
    assert!(
        !statuses[0]
            .consent_state
            .as_ref()
            .expect("consent state")
            .all_required_accepted
    );

    runtime.shutdown().await;
    server.abort();
}

#[tokio::test]
async fn community_node_status_does_not_require_restart_when_connectivity_is_active() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-status.db");
    let test_timeout = Duration::from_secs(15);
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    let base_url = "https://community.example.com".to_string();
    let connectivity_url = "http://127.0.0.1:9".to_string();
    let resolved_urls = CommunityNodeResolvedUrls::new(
        base_url.clone(),
        vec![connectivity_url.clone()],
        Vec::new(),
    )
    .expect("resolved urls");
    let node = CommunityNodeNodeConfig {
        base_url: base_url.clone(),
        auto_approve: false,
        resolved_urls: Some(resolved_urls.clone()),
    };
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
        nodes: vec![node.clone()],
    };
    *runtime.active_connectivity_urls.lock().await = vec![connectivity_url.clone()];

    let status = timeout(
        test_timeout,
        runtime.community_node_status(
            node,
            Some(CommunityNodeConsentStatus {
                all_required_accepted: true,
                items: vec![kukuri_cn_core::CommunityNodeConsentItem {
                    policy_slug: "community-basic".to_string(),
                    policy_version: 1,
                    title: "Community Basic".to_string(),
                    body: "Community basic policy body.".to_string(),
                    required: true,
                    accepted_at: Some(Utc::now().timestamp()),
                    previously_accepted_version: Some(1),
                }],
            }),
            None,
        ),
    )
    .await
    .expect("community-node status timeout")
    .expect("community-node status");
    assert!(status.auth_state.authenticated);
    assert!(
        status
            .consent_state
            .as_ref()
            .expect("consent state")
            .all_required_accepted
    );
    assert_eq!(
        status
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .connectivity_urls,
        vec![connectivity_url]
    );
    assert!(!status.restart_required);

    timeout(test_timeout, runtime.shutdown())
        .await
        .expect("runtime shutdown timeout");
}

#[tokio::test]
async fn auto_approve_node_does_not_silently_reaccept_policy_update() {
    // #384: auto_approve でも、版が上がった「更新」のときは黙って再受諾せず Idle に留める。
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-auto-approve-update.db");
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
    let state = Arc::new(MockManagedCommunityNodeState {
        base_url: base_url.clone(),
        seed_peers: vec![],
        consent_accepted: Arc::new(AtomicBool::new(false)),
        current_token: Arc::new(Mutex::new("update-pending-token".into())),
        challenge_hits: Arc::new(AtomicUsize::new(0)),
        verify_hits: Arc::new(AtomicUsize::new(0)),
        consent_status_hits: Arc::new(AtomicUsize::new(0)),
        consent_accept_hits: Arc::new(AtomicUsize::new(0)),
        heartbeat_hits: Arc::new(AtomicUsize::new(0)),
        bootstrap_hits: Arc::new(AtomicUsize::new(0)),
        simulate_pending_update: Arc::new(AtomicBool::new(true)),
    });
    let app = Router::new()
        .route("/v1/auth/challenge", post(mock_managed_auth_challenge))
        .route("/v1/auth/verify", post(mock_managed_auth_verify))
        .route("/v1/consents/status", get(mock_managed_consent_status))
        .route("/v1/consents", post(mock_managed_accept_consents))
        .route(
            "/v1/bootstrap/heartbeat",
            post(mock_managed_bootstrap_heartbeat),
        )
        .route("/v1/bootstrap/nodes", get(mock_managed_bootstrap_nodes))
        .with_state(state.clone());
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    persist_community_node_token(
        &db_path,
        IdentityStorageMode::FileOnly,
        base_url.as_str(),
        &StoredCommunityNodeToken {
            access_token: "update-pending-token".into(),
            expires_at: Utc::now().timestamp() + 3600,
        },
    )
    .expect("persist token");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: base_url.clone(),
            auto_approve: true,
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
    // 更新時は auto 受諾しない。
    assert_eq!(state.consent_status_hits.load(Ordering::SeqCst), 1);
    assert_eq!(state.consent_accept_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 0);
    assert_eq!(
        statuses[0].session_phase,
        crate::CommunityNodeSessionPhase::Idle
    );
    let consent_state = statuses[0].consent_state.as_ref().expect("consent state");
    assert!(!consent_state.all_required_accepted);
    // 更新（旧版同意済み・現行版未同意）が client に見えている。
    let item = &consent_state.items[0];
    assert!(item.accepted_at.is_none());
    assert_eq!(item.previously_accepted_version, Some(1));

    // ユーザーが明示的に受諾すれば ready になる。
    let accepted = runtime
        .accept_community_node_consents(crate::AcceptCommunityNodeConsentsRequest {
            base_url: base_url.clone(),
            policy_slugs: vec![],
        })
        .await
        .expect("accept consents");
    assert!(
        accepted
            .consent_state
            .as_ref()
            .expect("consent state")
            .all_required_accepted
    );
    assert_eq!(state.consent_accept_hits.load(Ordering::SeqCst), 1);

    runtime.shutdown().await;
    server.abort();
}
