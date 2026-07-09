use super::super::*;

/// mock CN のカウンタが `at_least` に達するまで timeout 付きで待つ。
/// スケジューラ tick は非同期に走るため、hit 数は「以上」でのみ判定する。
async fn wait_for_counter(counter: &AtomicUsize, at_least: usize, label: &str) {
    timeout(Duration::from_secs(30), async {
        loop {
            if counter.load(Ordering::SeqCst) >= at_least {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "{label} did not reach {at_least} within timeout (current: {})",
            counter.load(Ordering::SeqCst)
        )
    });
}

/// トレイ常駐相当(getter を一切呼ばない)でも bootstrap heartbeat が継続することを固定する。
/// WP-C1 の failing test: スケジューラ導入前は heartbeat_hits が 0 のまま timeout で赤になる。
#[tokio::test]
async fn session_scheduler_keeps_bootstrap_registration_alive_without_getter_polling() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-scheduler-keepalive.db");
    let runtime = Arc::new(
        DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime"),
    );

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

    // トレイ常駐相当: get_sync_status / get_community_node_statuses は一切呼ばない。
    runtime
        .start_community_node_session_scheduler_with_interval(Duration::from_millis(200))
        .await;

    wait_for_counter(&state.heartbeat_hits, 1, "heartbeat_hits").await;

    // heartbeat 期限を過去に注入し、2 発目(維持の継続)を観測する。カウンタは mock が
    // リクエストを受けた時点で増えるが、クライアント側の次回期限書き込みはその後のため、
    // 単発注入では初回 tick の書き込みに上書きされうる。観測できるまで注入を繰り返す。
    if timeout(Duration::from_secs(30), async {
        loop {
            if let Some(entry) = runtime.community_node_sessions.lock().await.get_mut(base_url.as_str()) {
                entry.heartbeat_deadline = Utc::now().timestamp() - 1;
            }
            if state.heartbeat_hits.load(Ordering::SeqCst) >= 2 {
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .is_err()
    {
        let task_finished = runtime
            .community_node_scheduler_task
            .lock()
            .await
            .as_ref()
            .map(|handle| handle.is_finished());
        let sessions = runtime.community_node_sessions.lock().await;
        panic!(
            "heartbeat did not continue after deadline expiry\n  hits={} sessions={:?} now={} task_finished={:?}",
            state.heartbeat_hits.load(Ordering::SeqCst),
            sessions,
            Utc::now().timestamp(),
            task_finished,
        );
    }

    // shutdown でスケジューラ task が停止し、以後 heartbeat が打たれないこと。
    runtime.shutdown().await;
    let hits_after_shutdown = state.heartbeat_hits.load(Ordering::SeqCst);
    if let Some(entry) = runtime.community_node_sessions.lock().await.get_mut(base_url.as_str()) {
        entry.heartbeat_deadline = Utc::now().timestamp() - 1;
    }
    sleep(Duration::from_millis(600)).await;
    assert_eq!(
        state.heartbeat_hits.load(Ordering::SeqCst),
        hits_after_shutdown
    );

    server.abort();
}

/// get_sync_status が CN セッションの establish/refresh を一切行わない(読み取り専用)ことを
/// 固定する contract。駆動はスケジューラ(session maintenance tick)のみが担う(WP-C1 T4)。
#[tokio::test]
async fn get_sync_status_is_read_only_for_community_node_session() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-sync-status-read-only.db");
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

    // heartbeat deadline 未設定(= 常に due)の状態でも、getter は refresh を駆動しない。
    let _status = runtime.get_sync_status().await.expect("sync status");

    assert_eq!(state.heartbeat_hits.load(Ordering::SeqCst), 0);
    assert_eq!(state.bootstrap_hits.load(Ordering::SeqCst), 0);
    assert!(
        runtime
            .community_node_sessions
            .lock()
            .await
            .is_empty()
    );

    runtime.shutdown().await;
    server.abort();
}

/// トレイ常駐相当(getter を一切呼ばない)でも失効間近トークンの事前再認証が走ることを固定する。
/// WP-C1 の failing test: スケジューラ導入前は verify_hits が 0 のまま timeout で赤になる。
#[tokio::test]
async fn session_scheduler_reauthenticates_near_expiry_token_without_getter_polling() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-scheduler-reauth.db");
    let runtime = Arc::new(
        DesktopRuntime::new_with_config_and_identity(
            &db_path,
            TransportNetworkConfig::loopback(),
            IdentityStorageMode::FileOnly,
        )
        .await
        .expect("runtime"),
    );

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
        seed_peers: vec![seed_peer],
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

    // トレイ常駐相当: getter を一切呼ばない。
    runtime
        .start_community_node_session_scheduler_with_interval(Duration::from_millis(200))
        .await;

    wait_for_counter(&state.verify_hits, 1, "verify_hits").await;
    wait_for_counter(&state.heartbeat_hits, 1, "heartbeat_hits").await;

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
