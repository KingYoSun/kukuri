use crate::presentation::dto::p2p::{
    JoinTopicRequest, LeaveTopicRequest, BroadcastRequest,
    P2PStatusResponse, NodeAddressResponse
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
