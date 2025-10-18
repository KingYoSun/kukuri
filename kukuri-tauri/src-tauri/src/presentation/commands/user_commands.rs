use crate::application::services::UserService;
use crate::domain::entities::UserMetadata;
use crate::presentation::dto::ApiResponse;
use crate::shared::AppError;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_user(
    npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<Option<Value>>, AppError> {
    let result = user_service
        .get_user(&npub)
        .await
        .map_err(AppError::from)
        .and_then(|user| {
            user.map(|u| serde_json::to_value(u).map_err(AppError::from))
                .transpose()
        });
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_user_by_pubkey(
    pubkey: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<Option<Value>>, AppError> {
    let result = user_service
        .get_user_by_pubkey(&pubkey)
        .await
        .map_err(AppError::from)
        .and_then(|user| {
            user.map(|u| serde_json::to_value(u).map_err(AppError::from))
                .transpose()
        });
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn update_profile(
    npub: String,
    metadata: UserMetadata,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = user_service
        .update_profile(&npub, metadata)
        .await
        .map_err(AppError::from);
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn follow_user(
    follower_npub: String,
    target_npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = user_service
        .follow_user(&follower_npub, &target_npub)
        .await
        .map_err(AppError::from);
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn unfollow_user(
    follower_npub: String,
    target_npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = user_service
        .unfollow_user(&follower_npub, &target_npub)
        .await
        .map_err(AppError::from);
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_followers(
    npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<Vec<Value>>, AppError> {
    let result = user_service
        .get_followers(&npub)
        .await
        .map_err(AppError::from)
        .and_then(|followers| {
            followers
                .into_iter()
                .map(|u| serde_json::to_value(u).map_err(AppError::from))
                .collect::<Result<Vec<_>, _>>()
        });
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_following(
    npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<Vec<Value>>, AppError> {
    let result = user_service
        .get_following(&npub)
        .await
        .map_err(AppError::from)
        .and_then(|following| {
            following
                .into_iter()
                .map(|u| serde_json::to_value(u).map_err(AppError::from))
                .collect::<Result<Vec<_>, _>>()
        });
    Ok(ApiResponse::from_result(result))
}
