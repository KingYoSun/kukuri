use super::super::*;

#[tokio::test]
async fn dm_send_requires_mutual_relationship() {
    let (app, _, _, _) = local_app_with_memory_services();
    let peer_keys = generate_keys();

    let error = app
        .send_direct_message(
            peer_keys.public_key_hex().as_str(),
            Some("hello"),
            None,
            Vec::new(),
        )
        .await
        .expect_err("direct message send should require mutual relationship");

    assert!(
        error
            .to_string()
            .contains("direct message requires a mutual relationship")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dm_status_stays_enabled_during_concurrent_relationship_rebuilds() {
    let _guard = iroh_integration_test_lock().lock_owned().await;
    let tempdir = tempdir().expect("tempdir");
    let database_path = tempdir.path().join("dm-status.db");
    let app_store = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("connect app store"),
    );
    let writer_store = Arc::new(
        SqliteStore::connect_file(&database_path)
            .await
            .expect("connect writer store"),
    );
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    let app = AppService::new_with_services(
        app_store.clone(),
        app_store.clone(),
        transport,
        Arc::new(NoopHintTransport),
        Arc::new(MemoryDocsSync::default()),
        Arc::new(MemoryBlobService::default()),
        generate_keys(),
    );
    let local_pubkey = app.current_author_pubkey();
    let peer_keys = generate_keys();
    let peer_pubkey = peer_keys.public_key_hex();

    app_store
        .put_envelope(
            build_follow_edge_envelope(
                app.keys.as_ref(),
                &Pubkey::from(peer_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build local->peer follow"),
        )
        .await
        .expect("seed local->peer follow");
    app_store
        .put_envelope(
            build_follow_edge_envelope(
                &peer_keys,
                &Pubkey::from(local_pubkey.as_str()),
                FollowEdgeStatus::Active,
            )
            .expect("build peer->local follow"),
        )
        .await
        .expect("seed peer->local follow");

    app.rebuild_author_relationships()
        .await
        .expect("seed relationship projection");
    let initial_status = app
        .direct_message_status_view(peer_pubkey.as_str())
        .await
        .expect("initial dm status");
    assert!(initial_status.send_enabled);
    assert!(initial_status.mutual);

    let mut rows = (0..512)
        .map(|index| AuthorRelationshipProjectionRow {
            local_author_pubkey: local_pubkey.clone(),
            author_pubkey: format!("{index:064x}"),
            following: true,
            followed_by: true,
            mutual: true,
            friend_of_friend: false,
            friend_of_friend_via_pubkeys: Vec::new(),
            derived_at: index,
        })
        .collect::<Vec<_>>();
    rows.push(AuthorRelationshipProjectionRow {
        local_author_pubkey: local_pubkey.clone(),
        author_pubkey: peer_pubkey.clone(),
        following: true,
        followed_by: true,
        mutual: true,
        friend_of_friend: false,
        friend_of_friend_via_pubkeys: Vec::new(),
        derived_at: 999,
    });

    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_for_task = Arc::clone(&keep_running);
    let writer_rows = rows.clone();
    let local_pubkey_for_task = local_pubkey.clone();
    let writer_task = tokio::spawn(async move {
        while keep_running_for_task.load(Ordering::SeqCst) {
            ProjectionStore::rebuild_author_relationships(
                writer_store.as_ref(),
                local_pubkey_for_task.as_str(),
                writer_rows.clone(),
            )
            .await
            .expect("rebuild relationships");
        }
    });

    let mut saw_disabled = false;
    for _ in 0..64 {
        let status = app
            .direct_message_status_view(peer_pubkey.as_str())
            .await
            .expect("dm status during rebuild");
        if !status.send_enabled || !status.mutual {
            saw_disabled = true;
            break;
        }
    }

    keep_running.store(false, Ordering::SeqCst);
    writer_task.await.expect("writer task");
    assert!(
        !saw_disabled,
        "direct message status should remain mutual while concurrent rebuilds are in flight",
    );
}
