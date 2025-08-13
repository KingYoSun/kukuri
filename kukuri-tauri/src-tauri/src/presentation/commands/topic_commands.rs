use crate::application::services::TopicService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicStats {
    pub member_count: u32,
    pub post_count: u32,
}

// v2コマンドに移行
// #[tauri::command]
// pub async fn create_topic(
//     request: CreateTopicRequest,
//     topic_service: State<'_, Arc<TopicService>>,
// ) -> Result<serde_json::Value, String> {
//     let topic = topic_service
//         .create_topic(request.name, request.description)
//         .await
//         .map_err(|e| e.to_string())?;

//     serde_json::to_value(topic).map_err(|e| e.to_string())
// }

#[tauri::command]
pub async fn get_topic(
    id: String,
    topic_service: State<'_, Arc<TopicService>>,
) -> Result<Option<serde_json::Value>, String> {
    let topic = topic_service
        .get_topic(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(topic.map(|t| serde_json::to_value(t).unwrap()))
}

#[tauri::command]
pub async fn get_all_topics(
    topic_service: State<'_, Arc<TopicService>>,
) -> Result<Vec<serde_json::Value>, String> {
    let topics = topic_service
        .get_all_topics()
        .await
        .map_err(|e| e.to_string())?;

    Ok(topics
        .into_iter()
        .map(|t| serde_json::to_value(t).unwrap())
        .collect())
}

#[tauri::command]
pub async fn get_joined_topics(
    topic_service: State<'_, Arc<TopicService>>,
) -> Result<Vec<serde_json::Value>, String> {
    let topics = topic_service
        .get_joined_topics()
        .await
        .map_err(|e| e.to_string())?;

    Ok(topics
        .into_iter()
        .map(|t| serde_json::to_value(t).unwrap())
        .collect())
}

#[tauri::command]
pub async fn join_topic(
    id: String,
    topic_service: State<'_, Arc<TopicService>>,
) -> Result<(), String> {
    topic_service
        .join_topic(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn leave_topic(
    id: String,
    topic_service: State<'_, Arc<TopicService>>,
) -> Result<(), String> {
    topic_service
        .leave_topic(&id)
        .await
        .map_err(|e| e.to_string())
}

// v2コマンドに移行
// #[tauri::command]
// pub async fn delete_topic(
//     id: String,
//     topic_service: State<'_, Arc<TopicService>>,
// ) -> Result<(), String> {
//     topic_service
//         .delete_topic(&id)
//         .await
//         .map_err(|e| e.to_string())
// }

#[tauri::command]
pub async fn get_topic_stats(
    id: String,
    topic_service: State<'_, Arc<TopicService>>,
) -> Result<TopicStats, String> {
    let (member_count, post_count) = topic_service
        .get_topic_stats(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TopicStats {
        member_count,
        post_count,
    })
}