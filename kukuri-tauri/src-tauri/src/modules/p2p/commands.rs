use tauri::State;
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::modules::p2p::generate_topic_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PStatus {
    pub connected: bool,
    pub endpoint_id: String,
    pub active_topics: Vec<TopicStatus>,
    pub peer_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStatus {
    pub topic_id: String,
    pub peer_count: usize,
    pub message_count: usize,
    pub last_activity: i64,
}

#[tauri::command]
pub async fn initialize_p2p(
    state: State<'_, AppState>,
) -> Result<(), String> {
    // P2P機能は既にAppStateで初期化されているはず
    // ここでは状態確認のみ
    let p2p_state = state.p2p_state.read().await;
    
    if p2p_state.manager.is_some() {
        Ok(())
    } else {
        Err("P2P manager not initialized".to_string())
    }
}

#[tauri::command]
pub async fn join_p2p_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)]
    topicId: String,
    #[allow(non_snake_case)]
    initialPeers: Vec<String>,
) -> Result<(), String> {
    let p2p_state = state.p2p_state.read().await;
    
    match &p2p_state.manager {
        Some(manager) => {
            manager.join_topic(&topicId, initialPeers)
                .await
                .map_err(|e| e.to_string())
        },
        None => Err("P2P manager not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn leave_p2p_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)]
    topicId: String,
) -> Result<(), String> {
    let p2p_state = state.p2p_state.read().await;
    
    match &p2p_state.manager {
        Some(manager) => {
            manager.leave_topic(&topicId)
                .await
                .map_err(|e| e.to_string())
        },
        None => Err("P2P manager not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn broadcast_to_topic(
    state: State<'_, AppState>,
    #[allow(non_snake_case)]
    topicId: String,
    content: String,
) -> Result<(), String> {
    let p2p_state = state.p2p_state.read().await;
    
    match &p2p_state.manager {
        Some(manager) => {
            // メッセージの作成と署名はGossipManager内で行う
            // 現時点では簡単な実装
            use crate::modules::p2p::message::{GossipMessage, MessageType};
            
            let message = GossipMessage::new(
                MessageType::NostrEvent,
                content.into_bytes(),
                vec![], // 送信者の公開鍵はbroadcast時に自動設定される
            );
            
            manager.broadcast(&topicId, message)
                .await
                .map_err(|e| e.to_string())
        },
        None => Err("P2P manager not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_p2p_status(
    state: State<'_, AppState>,
) -> Result<P2PStatus, String> {
    let p2p_state = state.p2p_state.read().await;
    
    match &p2p_state.manager {
        Some(manager) => {
            let topic_stats = manager.get_all_topic_stats().await;
            let mut topic_statuses = Vec::new();
            let mut total_peer_count = 0;
            
            for (topic_id, stats) in topic_stats {
                topic_statuses.push(TopicStatus {
                    topic_id,
                    peer_count: stats.peer_count,
                    message_count: stats.message_count,
                    last_activity: stats.last_activity,
                });
                total_peer_count += stats.peer_count;
            }
            
            Ok(P2PStatus {
                connected: true,
                endpoint_id: manager.node_id(),
                active_topics: topic_statuses,
                peer_count: total_peer_count,
            })
        },
        None => {
            Ok(P2PStatus {
                connected: false,
                endpoint_id: String::new(),
                active_topics: vec![],
                peer_count: 0,
            })
        }
    }
}

#[tauri::command]
pub async fn get_node_address(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let p2p_state = state.p2p_state.read().await;
    
    match &p2p_state.manager {
        Some(manager) => {
            manager.node_addr()
                .await
                .map_err(|e| e.to_string())
        },
        None => Err("P2P manager not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn join_topic_by_name(
    state: State<'_, AppState>,
    #[allow(non_snake_case)]
    topicName: String,
    #[allow(non_snake_case)]
    initialPeers: Vec<String>,
) -> Result<(), String> {
    let topic_id = generate_topic_id(&topicName);
    join_p2p_topic(state, topic_id, initialPeers).await
}