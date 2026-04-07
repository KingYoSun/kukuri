use std::time::Duration;

use anyhow::Result;
use iroh::RelayUrl;
use tempfile::tempdir;
use tokio::time::{sleep, timeout};

use crate::{DocOp, DocQuery, DocsSync, IrohDocsNode, IrohDocsSync, stable_key, topic_replica_id};
use kukuri_transport::{
    DhtDiscoveryOptions, SeedPeer, TransportNetworkConfig, TransportRelayConfig,
};

fn relay_seeded_public_replication_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(20)
    }
}

#[tokio::test]
async fn apply_relay_config_tolerates_relay_activation_timeout() -> Result<()> {
    let node = IrohDocsNode::memory().await?;
    let relay_url = "http://127.0.0.1:9".parse::<RelayUrl>()?;

    node.apply_relay_config(TransportRelayConfig {
        iroh_relay_urls: vec![relay_url.to_string()],
    })
    .await?;

    assert_eq!(node.relay_urls().await, vec![relay_url]);

    let docs = IrohDocsSync::new(node.clone());
    docs.shutdown().await;
    node.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn public_replica_syncs_over_custom_relay_seed_peers() -> Result<()> {
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        return Ok(());
    }
    let (_relay_map, relay_url, _guard) = iroh::test_utils::run_relay_server().await?;
    let relay_config = TransportRelayConfig {
        iroh_relay_urls: vec![relay_url.to_string()],
    }
    .normalized();
    let dir = tempdir()?;
    let node_a = IrohDocsNode::persistent_with_discovery_config(
        dir.path().join("docs-a"),
        TransportNetworkConfig::loopback(),
        DhtDiscoveryOptions::disabled(),
        relay_config.clone(),
    )
    .await?;
    let node_b = IrohDocsNode::persistent_with_discovery_config(
        dir.path().join("docs-b"),
        TransportNetworkConfig::loopback(),
        DhtDiscoveryOptions::disabled(),
        relay_config,
    )
    .await?;
    let docs_a = IrohDocsSync::new(node_a.clone());
    let docs_b = IrohDocsSync::new(node_b.clone());
    let replica = topic_replica_id("kukuri:topic:relay-seeded-docs");

    docs_a
        .set_seed_peers(vec![SeedPeer {
            endpoint_id: node_b.endpoint().id().to_string(),
            addr_hint: None,
        }])
        .await?;
    docs_b
        .set_seed_peers(vec![SeedPeer {
            endpoint_id: node_a.endpoint().id().to_string(),
            addr_hint: None,
        }])
        .await?;
    docs_a.open_replica(&replica).await?;
    docs_b.open_replica(&replica).await?;
    docs_a
        .apply_doc_op(
            &replica,
            DocOp::SetJson {
                key: stable_key("timeline", "0001-relay-event"),
                value: serde_json::json!({
                    "object_id": "relay-event-1",
                    "topic_id": "kukuri:topic:relay-seeded-docs"
                }),
            },
        )
        .await?;

    let sync_result = timeout(relay_seeded_public_replication_timeout(), async {
        loop {
            let rows = docs_b
                .query_replica(&replica, DocQuery::Prefix("timeline/".into()))
                .await
                .expect("query replica b");
            if rows
                .iter()
                .any(|row| row.key == "timeline/0001-relay-event")
            {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    let timeout_diagnostics = if let Err(error) = sync_result {
        let rows_a = docs_a
            .query_replica(&replica, DocQuery::Prefix("timeline/".into()))
            .await?
            .into_iter()
            .map(|row| row.key)
            .collect::<Vec<_>>();
        let rows_b = docs_b
            .query_replica(&replica, DocQuery::Prefix("timeline/".into()))
            .await?
            .into_iter()
            .map(|row| row.key)
            .collect::<Vec<_>>();
        let remote_info_a = node_a
            .endpoint()
            .remote_info(node_b.endpoint().id())
            .await
            .is_some();
        let remote_info_b = node_b
            .endpoint()
            .remote_info(node_a.endpoint().id())
            .await
            .is_some();
        let seed_peers_a = docs_a.available_sync_peer_ids().await;
        let seed_peers_b = docs_b.available_sync_peer_ids().await;
        Some((
            error,
            rows_a,
            rows_b,
            remote_info_a,
            remote_info_b,
            seed_peers_a,
            seed_peers_b,
        ))
    } else {
        None
    };

    if let Some((error, rows_a, rows_b, remote_info_a, remote_info_b, seed_peers_a, seed_peers_b)) =
        timeout_diagnostics
    {
        panic!(
            "relay-seeded public replica sync timeout: {error:?}; rows_a={rows_a:?}; rows_b={rows_b:?}; remote_info_a={remote_info_a}; remote_info_b={remote_info_b}; seed_peers_a={seed_peers_a:?}; seed_peers_b={seed_peers_b:?}"
        );
    }

    docs_a.shutdown().await;
    docs_b.shutdown().await;
    node_a.shutdown().await?;
    node_b.shutdown().await?;
    Ok(())
}
