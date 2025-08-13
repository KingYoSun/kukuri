use crate::presentation::dto::p2p::{
    JoinTopicRequest, LeaveTopicRequest, BroadcastRequest,
    P2PStatusResponse, NodeAddressResponse
};
use crate::state::AppState;
use tauri::State;

/// P2P機能を初期化
#[tauri::command]
pub async fn initialize_p2p_v2(state: State<'_, AppState>) -> Result<String, String> {
    state
        .p2p_handler
        .initialize_p2p()
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// P2Pトピックに参加
#[tauri::command]
pub async fn join_p2p_topic_v2(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
    #[allow(non_snake_case)] initialPeers: Vec<String>,
) -> Result<String, String> {
    let request = JoinTopicRequest {
        topic_id: topicId,
        initial_peers: initialPeers,
    };
    
    state
        .p2p_handler
        .join_topic(request)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// P2Pトピックから離脱
#[tauri::command]
pub async fn leave_p2p_topic_v2(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicId: String,
) -> Result<String, String> {
    let request = LeaveTopicRequest {
        topic_id: topicId,
    };
    
    state
        .p2p_handler
        .leave_topic(request)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}

/// トピックにメッセージをブロードキャスト
#[tauri::command]
pub async fn broadcast_to_topic_v2(
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
pub async fn get_p2p_status_v2(state: State<'_, AppState>) -> Result<P2PStatusResponse, String> {
    state
        .p2p_handler
        .get_p2p_status()
        .await
        .map_err(|e| e.to_string())
}

/// ノードアドレスを取得
#[tauri::command]
pub async fn get_node_address_v2(state: State<'_, AppState>) -> Result<NodeAddressResponse, String> {
    state
        .p2p_handler
        .get_node_address()
        .await
        .map_err(|e| e.to_string())
}

/// トピック名で参加
#[tauri::command]
pub async fn join_topic_by_name_v2(
    state: State<'_, AppState>,
    #[allow(non_snake_case)] topicName: String,
    #[allow(non_snake_case)] initialPeers: Vec<String>,
) -> Result<String, String> {
    state
        .p2p_handler
        .join_topic_by_name(topicName, initialPeers)
        .await
        .map(|response| serde_json::to_string(&response).unwrap())
        .map_err(|e| e.to_string())
}