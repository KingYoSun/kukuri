use crate::{
    infrastructure::p2p::metrics::GossipMetricDetails,
    presentation::dto::{
        ApiResponse,
        p2p::{
            BootstrapConfigResponse, BroadcastRequest, GossipMetricDetailsResponse,
            GossipMetricsResponse, JoinTopicByNameRequest, JoinTopicByNameResponse,
            JoinTopicRequest, LeaveTopicRequest, MainlineMetricsResponse, NodeAddressResponse,
            P2PMetricsResponse, P2PStatusResponse,
        },
    },
    shared::AppError,
    state::AppState,
};
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

/// トピック名で参加
#[tauri::command]
pub async fn join_topic_by_name(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicName: String,
    #[allow(non_snake_case)] initialPeers: Vec<String>,
) -> Result<ApiResponse<JoinTopicByNameResponse>, AppError> {
    let request = JoinTopicByNameRequest {
        topic_name: topicName,
        initial_peers: initialPeers,
    };
    match state.p2p_handler.join_topic_by_name(request).await {
        Ok(response) => {
            if let Err(e) = state.ensure_ui_subscription(&response.topic_id).await {
                tracing::warn!(
                    "Failed to ensure UI subscription for {}: {}",
                    response.topic_id,
                    e
                );
            }
            Ok(ApiResponse::success(response))
        }
        Err(err) => Ok(ApiResponse::from_app_error(err)),
    }
}

// ================= Bootstrap UI コマンド =================

#[tauri::command]
pub async fn get_bootstrap_config() -> Result<ApiResponse<BootstrapConfigResponse>, AppError> {
    use crate::infrastructure::p2p::bootstrap_config;
    let nodes = bootstrap_config::load_user_bootstrap_nodes();
    let mode = if nodes.is_empty() {
        "default".to_string()
    } else {
        "custom".to_string()
    };
    Ok(ApiResponse::success(BootstrapConfigResponse {
        mode,
        nodes,
    }))
}

#[tauri::command]
pub async fn set_bootstrap_nodes(nodes: Vec<String>) -> Result<ApiResponse<()>, AppError> {
    use crate::infrastructure::p2p::bootstrap_config;
    bootstrap_config::save_user_bootstrap_nodes(&nodes)
        .map_err(|e| AppError::ConfigurationError(e.to_string()))?;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn clear_bootstrap_nodes() -> Result<ApiResponse<()>, AppError> {
    use crate::infrastructure::p2p::bootstrap_config;
    bootstrap_config::clear_user_bootstrap_nodes()
        .map_err(|e| AppError::ConfigurationError(e.to_string()))?;
    Ok(ApiResponse::success(()))
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
