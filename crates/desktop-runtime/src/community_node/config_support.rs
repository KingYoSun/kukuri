use super::*;

pub(crate) fn load_community_node_config_from_file(
    db_path: &Path,
) -> Result<Option<CommunityNodeConfig>> {
    let path = community_node_config_path(db_path);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read community-node config `{}`", path.display()))?;
    let config = serde_json::from_str::<CommunityNodeConfig>(&raw)
        .with_context(|| format!("failed to parse community-node config `{}`", path.display()))?;
    Ok(Some(normalize_community_node_config(config)?))
}

pub(crate) fn default_preview_community_node_config() -> CommunityNodeConfig {
    CommunityNodeConfig {
        nodes: vec![CommunityNodeNodeConfig {
            base_url: COMMUNITY_NODE_PREVIEW_BASE_URL.to_string(),
            auto_approve: true,
            resolved_urls: None,
        }],
    }
}

pub(crate) fn save_community_node_config(
    db_path: &Path,
    config: &CommunityNodeConfig,
) -> Result<()> {
    let path = community_node_config_path(db_path);
    let normalized = normalize_community_node_config(config.clone())?;
    let json = serde_json::to_vec_pretty(&normalized).with_context(|| {
        format!(
            "failed to encode community-node config `{}`",
            path.display()
        )
    })?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write community-node config `{}`", path.display()))
}

pub(crate) fn normalize_community_node_config(
    config: CommunityNodeConfig,
) -> Result<CommunityNodeConfig> {
    let mut deduped = std::collections::BTreeMap::<String, CommunityNodeNodeConfig>::new();
    for node in config.nodes {
        let base_url = normalize_http_url(node.base_url.as_str())?;
        let incoming_auto_approve = node.auto_approve;
        let incoming_resolved_urls = match node.resolved_urls {
            Some(resolved) => Some(CommunityNodeResolvedUrls::new(
                resolved.public_base_url,
                resolved.connectivity_urls,
                resolved.seed_peers,
            )?),
            None => None,
        };
        let resolved_urls = if let Some(existing) = deduped.get(&base_url) {
            merge_community_node_resolved_urls(
                existing.resolved_urls.clone(),
                incoming_resolved_urls,
            )?
        } else {
            incoming_resolved_urls
        };
        let auto_approve = deduped
            .get(&base_url)
            .map(|existing| existing.auto_approve || incoming_auto_approve)
            .unwrap_or(incoming_auto_approve);
        deduped.insert(
            base_url.clone(),
            CommunityNodeNodeConfig {
                base_url,
                auto_approve,
                resolved_urls,
            },
        );
    }
    Ok(CommunityNodeConfig {
        nodes: deduped.into_values().collect(),
    })
}

pub(crate) fn merge_community_node_resolved_urls(
    current: Option<CommunityNodeResolvedUrls>,
    incoming: Option<CommunityNodeResolvedUrls>,
) -> Result<Option<CommunityNodeResolvedUrls>> {
    match (current, incoming) {
        (None, None) => Ok(None),
        (Some(resolved), None) | (None, Some(resolved)) => Ok(Some(resolved)),
        (Some(current), Some(incoming)) => {
            let public_base_url = incoming.public_base_url;
            let connectivity_urls = current
                .connectivity_urls
                .into_iter()
                .chain(incoming.connectivity_urls)
                .collect();
            let seed_peers = current
                .seed_peers
                .into_iter()
                .chain(incoming.seed_peers)
                .collect();
            Ok(Some(CommunityNodeResolvedUrls::new(
                public_base_url,
                connectivity_urls,
                seed_peers,
            )?))
        }
    }
}

pub(crate) fn community_node_seed_peers(
    config: &CommunityNodeConfig,
) -> impl Iterator<Item = SeedPeer> + '_ {
    config
        .nodes
        .iter()
        .filter_map(|node| node.resolved_urls.as_ref())
        .flat_map(|resolved| {
            let relay_backed = !resolved.connectivity_urls.is_empty();
            resolved
                .seed_peers
                .iter()
                .filter_map(move |seed_peer| seed_peer_from_community_node(seed_peer, relay_backed))
        })
}

pub(crate) fn seed_peer_from_community_node(
    seed_peer: &CommunityNodeSeedPeer,
    relay_backed: bool,
) -> Option<SeedPeer> {
    let endpoint_id = seed_peer.endpoint_id.trim();
    if endpoint_id.is_empty() {
        return None;
    }
    Some(SeedPeer {
        endpoint_id: endpoint_id.to_string(),
        addr_hint: if relay_backed {
            None
        } else {
            seed_peer.addr_hint.clone()
        },
    })
}

pub(crate) fn relay_config_from_community_node_config(
    config: &CommunityNodeConfig,
) -> TransportRelayConfig {
    let mut iroh_relay_urls = std::collections::BTreeSet::new();
    for node in &config.nodes {
        if let Some(resolved) = node.resolved_urls.as_ref() {
            for relay_url in &resolved.connectivity_urls {
                iroh_relay_urls.insert(relay_url.clone());
            }
        }
    }
    TransportRelayConfig {
        iroh_relay_urls: iroh_relay_urls.into_iter().collect(),
    }
}

pub(crate) fn runtime_connectivity_assist_state(
    discovery_config: &DiscoveryConfig,
    community_node_config: &CommunityNodeConfig,
) -> RuntimeConnectivityAssistState {
    let relay_config = relay_config_from_community_node_config(community_node_config).normalized();
    let configured_seed_peers = normalize_seed_peers(discovery_config.seed_peers.clone());
    let bootstrap_seed_peers =
        normalize_seed_peers(community_node_seed_peers(community_node_config).collect());
    RuntimeConnectivityAssistState {
        discovery_mode: discovery_config.mode.clone(),
        discovery_env_locked: discovery_config.env_locked,
        configured_seed_peers,
        bootstrap_seed_peers,
        relay_urls: relay_config.iroh_relay_urls,
    }
}

pub(crate) fn effective_seed_peer_apply_state(
    discovery_config: &DiscoveryConfig,
    community_node_config: &CommunityNodeConfig,
) -> EffectiveSeedPeerApplyState {
    EffectiveSeedPeerApplyState {
        discovery_mode: discovery_config.mode.clone(),
        discovery_env_locked: discovery_config.env_locked,
        configured_seed_peers: normalize_seed_peers(discovery_config.seed_peers.clone()),
        bootstrap_seed_peers: normalize_seed_peers(
            community_node_seed_peers(community_node_config).collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn community_node_seed_peers_drop_addr_hints_when_relay_urls_exist() {
        let config = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: "https://community.example.com".to_string(),
                auto_approve: false,
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://community.example.com",
                        vec!["https://relay.example.com".to_string()],
                        vec![
                            CommunityNodeSeedPeer::new(
                                "peer-a",
                                Some("192.168.1.40:40123".to_string()),
                            )
                            .expect("seed peer"),
                        ],
                    )
                    .expect("resolved urls"),
                ),
            }],
        };

        let peers = community_node_seed_peers(&config).collect::<Vec<_>>();

        assert_eq!(
            peers,
            vec![SeedPeer {
                endpoint_id: "peer-a".to_string(),
                addr_hint: None,
            }]
        );
    }

    #[test]
    fn community_node_seed_peers_keep_addr_hints_without_relay_urls() {
        let config = CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: "https://community.example.com".to_string(),
                auto_approve: false,
                resolved_urls: Some(
                    CommunityNodeResolvedUrls::new(
                        "https://community.example.com",
                        Vec::new(),
                        vec![
                            CommunityNodeSeedPeer::new(
                                "peer-a",
                                Some("192.168.1.40:40123".to_string()),
                            )
                            .expect("seed peer"),
                        ],
                    )
                    .expect("resolved urls"),
                ),
            }],
        };

        let peers = community_node_seed_peers(&config).collect::<Vec<_>>();

        assert_eq!(
            peers,
            vec![SeedPeer {
                endpoint_id: "peer-a".to_string(),
                addr_hint: Some("192.168.1.40:40123".to_string()),
            }]
        );
    }
}
