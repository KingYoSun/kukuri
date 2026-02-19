use crate::{
    infrastructure::p2p::{bootstrap_config, metrics::GossipMetricDetails},
    presentation::dto::{
        ApiResponse,
        p2p::{
            BootstrapConfigResponse, BootstrapMetricsResponse, BroadcastRequest,
            GossipMetricDetailsResponse, GossipMetricsResponse, JoinTopicRequest,
            LeaveTopicRequest, MainlineMetricsResponse, NodeAddressResponse, P2PMetricsResponse,
            P2PStatusResponse, RelayStatusResponse,
        },
    },
    shared::AppError,
    shared::config::BootstrapSource,
    state::AppState,
};
use std::collections::HashSet;
use tauri::State;

/// P2P機能を初期化
#[tauri::command]
pub async fn initialize_p2p(state: State<'_, AppState>) -> Result<ApiResponse<()>, AppError> {
    let result = state.p2p_handler.initialize_p2p().await;
    Ok(ApiResponse::from_result(result))
}

/// P2Pトピックに参加
#[tauri::command]
pub async fn join_p2p_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
    #[allow(non_snake_case)] initialPeers: Vec<String>,
) -> Result<ApiResponse<()>, AppError> {
    let request = JoinTopicRequest {
        topic_id: topicId.clone(),
        initial_peers: initialPeers,
    };
    let topic_id = request.topic_id.clone();

    match state.p2p_handler.join_topic(request).await {
        Ok(_) => {
            if let Err(e) = state.ensure_ui_subscription(&topic_id).await {
                tracing::warn!("Failed to ensure UI subscription for {}: {}", topic_id, e);
            }
            Ok(ApiResponse::success(()))
        }
        Err(err) => Ok(ApiResponse::from_app_error(err)),
    }
}

/// P2Pトピックから離脱
#[tauri::command]
pub async fn leave_p2p_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
) -> Result<ApiResponse<()>, AppError> {
    let request = LeaveTopicRequest {
        topic_id: topicId.clone(),
    };
    match state.p2p_handler.leave_topic(request).await {
        Ok(_) => {
            if let Err(e) = state.stop_ui_subscription(&topicId).await {
                tracing::warn!("Failed to stop UI subscription for {}: {}", topicId, e);
            }
            Ok(ApiResponse::success(()))
        }
        Err(err) => Ok(ApiResponse::from_app_error(err)),
    }
}

/// トピックにメッセージをブロードキャスト
#[tauri::command]
pub async fn broadcast_to_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
    content: String,
) -> Result<ApiResponse<()>, AppError> {
    let request = BroadcastRequest {
        topic_id: topicId,
        content,
    };
    let result = state.p2p_handler.broadcast_to_topic(request).await;
    Ok(ApiResponse::from_result(result))
}

/// P2Pステータスを取得
#[tauri::command]
pub async fn get_p2p_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<P2PStatusResponse>, AppError> {
    let result = state.p2p_handler.get_p2p_status().await;
    Ok(ApiResponse::from_result(result))
}

/// ノードアドレスを取得
#[tauri::command]
pub async fn get_node_address(
    state: State<'_, AppState>,
) -> Result<ApiResponse<NodeAddressResponse>, AppError> {
    let result = state.p2p_handler.get_node_address().await;
    Ok(ApiResponse::from_result(result))
}

// ================= Bootstrap UI コマンド =================

#[tauri::command]
pub async fn get_bootstrap_config() -> Result<ApiResponse<BootstrapConfigResponse>, AppError> {
    let user_nodes = bootstrap_config::load_user_bootstrap_nodes();
    let selection = bootstrap_config::load_effective_bootstrap_nodes();
    let env_locked = bootstrap_config::load_env_bootstrap_nodes().is_some();
    let cli_info = bootstrap_config::load_cli_bootstrap_nodes();
    let cli_nodes = cli_info
        .as_ref()
        .map(|info| info.nodes.clone())
        .unwrap_or_default();
    let cli_updated_at_ms = cli_info.and_then(|info| info.updated_at_ms);
    let mode = if env_locked {
        "custom".to_string()
    } else if user_nodes.is_empty() {
        "default".to_string()
    } else {
        "custom".to_string()
    };
    let source = match selection.source {
        crate::shared::config::BootstrapSource::Env => "env",
        crate::shared::config::BootstrapSource::User => "user",
        crate::shared::config::BootstrapSource::Bundle => "bundle",
        crate::shared::config::BootstrapSource::Fallback => "fallback",
        crate::shared::config::BootstrapSource::None => "none",
    }
    .to_string();

    Ok(ApiResponse::success(BootstrapConfigResponse {
        mode,
        nodes: user_nodes,
        effective_nodes: selection.nodes,
        source,
        env_locked,
        cli_nodes,
        cli_updated_at_ms,
    }))
}

#[tauri::command]
pub async fn set_bootstrap_nodes(nodes: Vec<String>) -> Result<ApiResponse<()>, AppError> {
    use crate::infrastructure::p2p::bootstrap_config;

    if bootstrap_config::load_env_bootstrap_nodes().is_some() {
        return Err(AppError::ConfigurationError(
            "Environment override is enabled; bootstrap nodes are read-only".to_string(),
        ));
    }

    for node in &nodes {
        if !node.contains('@') {
            return Err(AppError::ConfigurationError(format!(
                "Invalid bootstrap node format (expected node_id@host:port): {node}"
            )));
        }
    }

    bootstrap_config::save_user_bootstrap_nodes(&nodes)
        .map_err(|e| AppError::ConfigurationError(e.to_string()))?;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn clear_bootstrap_nodes() -> Result<ApiResponse<()>, AppError> {
    use crate::infrastructure::p2p::bootstrap_config;

    if bootstrap_config::load_env_bootstrap_nodes().is_some() {
        return Err(AppError::ConfigurationError(
            "Environment override is enabled; bootstrap nodes are read-only".to_string(),
        ));
    }

    bootstrap_config::clear_user_bootstrap_nodes()
        .map_err(|e| AppError::ConfigurationError(e.to_string()))?;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn apply_cli_bootstrap_nodes(
    state: State<'_, AppState>,
) -> Result<ApiResponse<BootstrapConfigResponse>, AppError> {
    if bootstrap_config::load_env_bootstrap_nodes().is_some() {
        return Err(AppError::ConfigurationError(
            "Environment override is enabled; CLI bootstrap list cannot be applied".to_string(),
        ));
    }

    let nodes = bootstrap_config::apply_cli_bootstrap_nodes()?;
    state
        .p2p_handler
        .apply_bootstrap_nodes(nodes, BootstrapSource::User)
        .await?;

    get_bootstrap_config().await
}

#[tauri::command]
pub async fn get_relay_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<RelayStatusResponse>>, AppError> {
    let p2p_status = state.p2p_handler.get_p2p_status().await?;
    let selection = bootstrap_config::load_effective_bootstrap_nodes();
    let default_status = match p2p_status.connection_status {
        crate::presentation::dto::p2p::ConnectionStatusResponse::Connected
        | crate::presentation::dto::p2p::ConnectionStatusResponse::Disconnected => "disconnected",
        crate::presentation::dto::p2p::ConnectionStatusResponse::Connecting => "connecting",
        crate::presentation::dto::p2p::ConnectionStatusResponse::Error => "error",
    };

    let mut statuses = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for node in selection.nodes.iter() {
        if seen.insert(node.clone()) {
            statuses.push(RelayStatusResponse {
                url: node.clone(),
                status: default_status.to_string(),
            });
        }
    }

    for peer in p2p_status.peers {
        let mut matched = None;
        for candidate in selection.nodes.iter() {
            if let Some((node_id, _)) = candidate.split_once('@') {
                if node_id == peer.node_id {
                    matched = Some(candidate.clone());
                    break;
                }
            } else if candidate == &peer.address || candidate == &peer.node_id {
                matched = Some(candidate.clone());
                break;
            }
        }

        if let Some(url) = matched
            && let Some(index) = statuses.iter().position(|entry| entry.url == url)
        {
            statuses[index].status = "connected".to_string();
            continue;
        }

        if seen.insert(peer.address.clone()) {
            statuses.push(RelayStatusResponse {
                url: peer.address,
                status: "connected".to_string(),
            });
        }
    }

    Ok(ApiResponse::success(statuses))
}

/// Gossipメトリクスを取得
#[tauri::command]
pub async fn get_p2p_metrics() -> Result<ApiResponse<P2PMetricsResponse>, AppError> {
    use crate::infrastructure::p2p::metrics;
    let snap = metrics::snapshot_full();
    let response = P2PMetricsResponse {
        gossip: GossipMetricsResponse {
            joins: snap.gossip.joins,
            leaves: snap.gossip.leaves,
            broadcasts_sent: snap.gossip.broadcasts_sent,
            messages_received: snap.gossip.messages_received,
            join_details: to_response_details(&snap.gossip.join_details),
            leave_details: to_response_details(&snap.gossip.leave_details),
            broadcast_details: to_response_details(&snap.gossip.broadcast_details),
            receive_details: to_response_details(&snap.gossip.receive_details),
        },
        mainline: MainlineMetricsResponse {
            connected_peers: snap.mainline.connected_peers,
            connection_attempts: snap.mainline.connection_attempts,
            connection_successes: snap.mainline.connection_successes,
            connection_failures: snap.mainline.connection_failures,
            connection_last_success_ms: snap.mainline.connection_last_success_ms,
            connection_last_failure_ms: snap.mainline.connection_last_failure_ms,
            routing_attempts: snap.mainline.routing_attempts,
            routing_successes: snap.mainline.routing_successes,
            routing_failures: snap.mainline.routing_failures,
            routing_success_rate: snap.mainline.routing_success_rate,
            routing_last_success_ms: snap.mainline.routing_last_success_ms,
            routing_last_failure_ms: snap.mainline.routing_last_failure_ms,
            reconnect_attempts: snap.mainline.reconnect_attempts,
            reconnect_successes: snap.mainline.reconnect_successes,
            reconnect_failures: snap.mainline.reconnect_failures,
            last_reconnect_success_ms: snap.mainline.last_reconnect_success_ms,
            last_reconnect_failure_ms: snap.mainline.last_reconnect_failure_ms,
            bootstrap: BootstrapMetricsResponse {
                env_uses: snap.mainline.bootstrap.env_uses,
                user_uses: snap.mainline.bootstrap.user_uses,
                bundle_uses: snap.mainline.bootstrap.bundle_uses,
                fallback_uses: snap.mainline.bootstrap.fallback_uses,
                last_source: snap.mainline.bootstrap.last_source,
                last_applied_ms: snap.mainline.bootstrap.last_applied_ms,
            },
        },
    };
    Ok(ApiResponse::success(response))
}

fn to_response_details(details: &GossipMetricDetails) -> GossipMetricDetailsResponse {
    GossipMetricDetailsResponse {
        total: details.total,
        failures: details.failures,
        last_success_ms: details.last_success_ms,
        last_failure_ms: details.last_failure_ms,
    }
}
