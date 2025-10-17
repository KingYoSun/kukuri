use crate::infrastructure::p2p::metrics::GossipMetricDetails;
use crate::presentation::dto::p2p::{
    BroadcastRequest, GossipMetricDetailsResponse, GossipMetricsResponse, JoinTopicRequest,
    LeaveTopicRequest, MainlineMetricsResponse, NodeAddressResponse, P2PMetricsResponse,
    P2PStatusResponse,
};
use crate::state::AppState;
use tauri::State;

/// P2P機能を初期化
#[tauri::command]
pub async fn initialize_p2p(state: State<'_, AppState>) -> Result<String, String> {
    state
        .p2p_handler
        .initialize_p2p()
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// P2Pトピックに参加
#[tauri::command]
pub async fn join_p2p_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
    #[allow(non_snake_case)] initialPeers: Vec<String>,
) -> Result<String, String> {
    let request = JoinTopicRequest {
        topic_id: topicId.clone(),
        initial_peers: initialPeers,
    };

    state
        .p2p_handler
        .join_topic(request)
        .await
        .map_err(|e| e.to_string())?;

    // UI購読導線を確立（冪等）
    if let Err(e) = state.ensure_ui_subscription(&topicId).await {
        tracing::warn!("Failed to ensure UI subscription for {}: {}", &topicId, e);
    }

    Ok(serde_json::to_string(&serde_json::json!({ "success": true })).unwrap())
}

/// P2Pトピックから離脱
#[tauri::command]
pub async fn leave_p2p_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
) -> Result<String, String> {
    let request = LeaveTopicRequest {
        topic_id: topicId.clone(),
    };

    state
        .p2p_handler
        .leave_topic(request)
        .await
        .map_err(|e| e.to_string())?;

    // UI購読導線を停止（冪等）
    if let Err(e) = state.stop_ui_subscription(&topicId).await {
        tracing::warn!("Failed to stop UI subscription for {}: {}", &topicId, e);
    }

    Ok(serde_json::to_string(&serde_json::json!({ "success": true })).unwrap())
}

/// トピックにメッセージをブロードキャスト
#[tauri::command]
pub async fn broadcast_to_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
    content: String,
) -> Result<String, String> {
    let request = BroadcastRequest {
        topic_id: topicId,
        content,
    };

    state
        .p2p_handler
        .broadcast_to_topic(request)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// P2Pステータスを取得
#[tauri::command]
pub async fn get_p2p_status(state: State<'_, AppState>) -> Result<P2PStatusResponse, String> {
    state
        .p2p_handler
        .get_p2p_status()
        .await
        .map_err(|e| e.to_string())
}

/// ノードアドレスを取得
#[tauri::command]
pub async fn get_node_address(state: State<'_, AppState>) -> Result<NodeAddressResponse, String> {
    state
        .p2p_handler
        .get_node_address()
        .await
        .map_err(|e| e.to_string())
}

/// トピック名で参加
#[tauri::command]
pub async fn join_topic_by_name(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicName: String,
    #[allow(non_snake_case)] initialPeers: Vec<String>,
) -> Result<String, String> {
    let res = state
        .p2p_handler
        .join_topic_by_name(topicName, initialPeers)
        .await
        .map_err(|e| e.to_string())?;

    // レスポンスからtopic_idを取り出してUI購読を確立
    if let Some(topic_id) = res.get("topic_id").and_then(|t| t.as_str()) {
        if let Err(e) = state.ensure_ui_subscription(topic_id).await {
            tracing::warn!("Failed to ensure UI subscription for {}: {}", topic_id, e);
        }
    }
    Ok(serde_json::to_string(&res).unwrap())
}

// ================= Bootstrap UI コマンド =================

#[tauri::command]
pub async fn get_bootstrap_config() -> Result<String, String> {
    use crate::infrastructure::p2p::bootstrap_config;
    let user_nodes = bootstrap_config::load_user_bootstrap_nodes();
    let mode = if user_nodes.is_empty() {
        "default"
    } else {
        "custom"
    };
    let json = serde_json::json!({
        "mode": mode,
        "nodes": user_nodes,
    });
    Ok(serde_json::to_string(&json).unwrap())
}

#[tauri::command]
pub async fn set_bootstrap_nodes(nodes: Vec<String>) -> Result<String, String> {
    use crate::infrastructure::p2p::bootstrap_config;
    bootstrap_config::save_user_bootstrap_nodes(&nodes).map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&serde_json::json!({"success": true})).unwrap())
}

#[tauri::command]
pub async fn clear_bootstrap_nodes() -> Result<String, String> {
    use crate::infrastructure::p2p::bootstrap_config;
    bootstrap_config::clear_user_bootstrap_nodes().map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&serde_json::json!({"success": true})).unwrap())
}

/// Gossipメトリクスを取得
#[tauri::command]
pub async fn get_p2p_metrics() -> Result<P2PMetricsResponse, String> {
    use crate::infrastructure::p2p::metrics;
    let snap = metrics::snapshot_full();
    Ok(P2PMetricsResponse {
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
    })
}

fn to_response_details(details: &GossipMetricDetails) -> GossipMetricDetailsResponse {
    GossipMetricDetailsResponse {
        total: details.total,
        failures: details.failures,
        last_success_ms: details.last_success_ms,
        last_failure_ms: details.last_failure_ms,
    }
}
