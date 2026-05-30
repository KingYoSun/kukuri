use super::super::*;

#[test]
fn community_node_config_normalizes_base_urls_and_connectivity_urls() {
    let config = normalize_community_node_config(CommunityNodeConfig {
        nodes: vec![
            CommunityNodeNodeConfig {
                base_url: "https://community.example.com/".into(),
                auto_approve: false,
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://public.example.com/",
                        vec![
                            "https://relay-b.example.com/".into(),
                            "https://relay-a.example.com/".into(),
                            "https://relay-a.example.com/".into(),
                        ],
                        vec![CommunityNodeSeedPeer::new("peer-b", None).expect("seed peer")],
                    )
                    .expect("resolved urls"),
                ),
            },
            CommunityNodeNodeConfig {
                base_url: "https://community.example.com".into(),
                auto_approve: true,
                resolved_urls: None,
            },
        ],
    })
    .expect("normalized config");

    assert_eq!(config.nodes.len(), 1);
    assert_eq!(config.nodes[0].base_url, "https://community.example.com");
    assert!(config.nodes[0].auto_approve);
    assert_eq!(
        config.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .connectivity_urls,
        vec![
            "https://relay-a.example.com".to_string(),
            "https://relay-b.example.com".to_string(),
        ]
    );
    assert_eq!(
        config.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![CommunityNodeSeedPeer::new("peer-b", None).expect("seed peer")]
    );
}

#[test]
fn community_node_config_preserves_public_kukuri_urls() {
    let config = normalize_community_node_config(CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: "https://api.kukuri.app/".into(),
            auto_approve: true,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(
                    "https://api.kukuri.app/",
                    vec!["https://iroh-relay.kukuri.app/".into()],
                    Vec::new(),
                )
                .expect("resolved urls"),
            ),
        }],
    })
    .expect("normalized config");

    let resolved = config.nodes[0]
        .resolved_urls
        .as_ref()
        .expect("resolved urls");

    assert_eq!(config.nodes[0].base_url, "https://api.kukuri.app");
    assert_eq!(resolved.public_base_url, "https://api.kukuri.app");
    assert_eq!(
        resolved.connectivity_urls,
        vec!["https://iroh-relay.kukuri.app".to_string()]
    );
    assert!(
        resolved
            .connectivity_urls
            .iter()
            .all(|url| !url.contains("api.kukuri.app/relay"))
    );
}

#[tokio::test]
async fn local_community_node_seed_peer_includes_addr_hint() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-seed-peer-addr-hint.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");

    let seed_peer = runtime
        .local_community_node_seed_peer("test")
        .await
        .expect("seed peer");

    assert!(seed_peer.addr_hint.is_some());

    runtime.shutdown().await;
}

#[tokio::test]
async fn local_community_node_seed_peer_keeps_addr_hint_when_relay_urls_exist() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-seed-peer-relay-auto-hint.db");
    let runtime = DesktopRuntime::new_with_config_and_identity(
        &db_path,
        TransportNetworkConfig::default(),
        IdentityStorageMode::FileOnly,
    )
    .await
    .expect("runtime");
    *runtime.community_node_config.lock().await = CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: "https://api.example.com".to_string(),
            auto_approve: false,
            resolved_urls: Some(
                CommunityNodeResolvedUrls::new(
                    "https://api.example.com",
                    vec!["https://relay.example.com".to_string()],
                    Vec::new(),
                )
                .expect("resolved urls"),
            ),
        }],
    };

    let seed_peer = runtime
        .local_community_node_seed_peer("test")
        .await
        .expect("seed peer");

    assert!(seed_peer.addr_hint.is_some());

    runtime.shutdown().await;
}

#[test]
fn stored_community_node_config_restores_cached_connectivity_union() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-relay.db");
    save_community_node_config(
        &db_path,
        &CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: "https://community.example.com".into(),
                auto_approve: false,
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://public.example.com",
                        vec!["https://relay.example.com".into()],
                        vec![CommunityNodeSeedPeer::new("peer-a", None).expect("seed peer")],
                    )
                    .expect("resolved urls"),
                ),
            }],
        },
    )
    .expect("save community node config");
    let restored = load_community_node_config_from_file(&db_path)
        .expect("load community node config")
        .expect("community node config");
    let relay_config = relay_config_from_community_node_config(&restored);

    assert_eq!(relay_config.connect_mode(), ConnectMode::DirectOrRelay);
    assert_eq!(
        relay_config.iroh_relay_urls,
        vec!["https://relay.example.com".to_string()]
    );
    assert_eq!(
        restored.nodes[0]
            .resolved_urls
            .as_ref()
            .expect("resolved urls")
            .seed_peers,
        vec![CommunityNodeSeedPeer::new("peer-a", None).expect("seed peer")]
    );
}

#[test]
fn default_preview_community_node_config_marks_preloaded_node_auto_approve() {
    let config = default_preview_community_node_config();
    assert_eq!(config.nodes.len(), 1);
    assert_eq!(config.nodes[0].base_url, "https://api.kukuri.app");
    assert!(config.nodes[0].auto_approve);
}

#[tokio::test]
async fn runtime_preloads_preview_community_node_when_config_file_is_missing() {
    let _serial = acquire_async_test_lock().await;
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("community-preview-preload.db");

    let runtime = DesktopRuntime::new_with_config_and_identity_and_discovery(
        &db_path,
        TransportNetworkConfig::loopback(),
        IdentityStorageMode::FileOnly,
        DiscoveryConfig::static_peer_default(),
        DhtDiscoveryOptions::disabled(),
        true,
    )
    .await
    .expect("runtime");

    let config = runtime
        .get_community_node_config()
        .await
        .expect("community node config");
    assert_eq!(config.nodes.len(), 1);
    assert_eq!(config.nodes[0].base_url, "https://api.kukuri.app");
    assert!(config.nodes[0].auto_approve);
    assert!(
        community_node_config_path(&db_path).exists(),
        "preloaded preview config should be persisted"
    );

    runtime.shutdown().await;
}
