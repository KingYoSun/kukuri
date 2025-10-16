use crate::application::services::UserService;
use crate::domain::entities::UserMetadata;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_user(
    npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<Option<serde_json::Value>, String> {
    let user = user_service
        .get_user(&npub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(user.map(|u| serde_json::to_value(u).unwrap()))
}

#[tauri::command]
pub async fn get_user_by_pubkey(
    pubkey: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<Option<serde_json::Value>, String> {
    let user = user_service
        .get_user_by_pubkey(&pubkey)
        .await
        .map_err(|e| e.to_string())?;

    Ok(user.map(|u| serde_json::to_value(u).unwrap()))
}

#[tauri::command]
pub async fn update_profile(
    npub: String,
    metadata: UserMetadata,
    user_service: State<'_, Arc<UserService>>,
) -> Result<(), String> {
    user_service
        .update_profile(&npub, metadata)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn follow_user(
    follower_npub: String,
    target_npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<(), String> {
    user_service
        .follow_user(&follower_npub, &target_npub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unfollow_user(
    follower_npub: String,
    target_npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<(), String> {
    user_service
        .unfollow_user(&follower_npub, &target_npub)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_followers(
    npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<Vec<serde_json::Value>, String> {
    let followers = user_service
        .get_followers(&npub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(followers
        .into_iter()
        .map(|u| serde_json::to_value(u).unwrap())
        .collect())
}

#[tauri::command]
pub async fn get_following(
    npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<Vec<serde_json::Value>, String> {
    let following = user_service
        .get_following(&npub)
        .await
        .map_err(|e| e.to_string())?;

    Ok(following
        .into_iter()
        .map(|u| serde_json::to_value(u).unwrap())
        .collect())
}
