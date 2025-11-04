use crate::application::services::{ProfileAvatarService, UploadProfileAvatarInput, UserService};
use crate::domain::entities::UserMetadata;
use crate::presentation::dto::{
    ApiResponse,
    profile_avatar_dto::{
        FetchProfileAvatarRequest, FetchProfileAvatarResponse, UploadProfileAvatarRequest,
        UploadProfileAvatarResponse,
    },
    user_dto::{
        GetFollowersRequest, GetFollowingRequest, PaginatedUserProfiles,
        UserProfile as UserProfileDto,
    },
};
use crate::shared::AppError;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

fn map_user_to_profile(user: crate::domain::entities::User) -> UserProfileDto {
    UserProfileDto {
        npub: user.npub,
        pubkey: user.pubkey,
        name: user.name,
        display_name: Some(user.profile.display_name),
        about: Some(user.profile.bio),
        picture: user.profile.avatar_url,
        banner: None,
        website: None,
        nip05: user.nip05,
    }
}

fn user_to_value(user: crate::domain::entities::User) -> Result<Value, AppError> {
    serde_json::to_value(map_user_to_profile(user)).map_err(AppError::from)
}

#[tauri::command]
pub async fn get_user(
    npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<Option<Value>>, AppError> {
    let result = user_service.get_user(&npub).await.and_then(|user| {
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
        .and_then(|user| {
            user.map(|u| serde_json::to_value(u).map_err(AppError::from))
                .transpose()
        });
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn search_users(
    query: String,
    limit: Option<u32>,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<Vec<Value>>, AppError> {
    let trimmed = query.trim().to_string();
    if trimmed.is_empty() {
        return Ok(ApiResponse::from_result(Ok(Vec::new())));
    }

    let limit = limit.unwrap_or(20).min(100) as usize;
    let result = user_service
        .search_users(&trimmed, limit)
        .await
        .and_then(|users| {
            users
                .into_iter()
                .map(user_to_value)
                .collect::<Result<Vec<_>, _>>()
        });

    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn update_profile(
    npub: String,
    metadata: UserMetadata,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = user_service.update_profile(&npub, metadata).await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn follow_user(
    follower_npub: String,
    target_npub: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<()>, AppError> {
    let result = user_service.follow_user(&follower_npub, &target_npub).await;
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
        .await;
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_followers(
    request: GetFollowersRequest,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<PaginatedUserProfiles>, AppError> {
    let limit = request.limit.unwrap_or(25).min(100) as usize;
    let result = user_service
        .get_followers_paginated(&request.npub, request.cursor.as_deref(), limit)
        .await
        .map(|page| PaginatedUserProfiles {
            items: page.users.into_iter().map(map_user_to_profile).collect(),
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        });
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_following(
    request: GetFollowingRequest,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<PaginatedUserProfiles>, AppError> {
    let limit = request.limit.unwrap_or(25).min(100) as usize;
    let result = user_service
        .get_following_paginated(&request.npub, request.cursor.as_deref(), limit)
        .await
        .map(|page| PaginatedUserProfiles {
            items: page.users.into_iter().map(map_user_to_profile).collect(),
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        });
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn upload_profile_avatar(
    request: UploadProfileAvatarRequest,
    avatar_service: State<'_, Arc<ProfileAvatarService>>,
) -> Result<ApiResponse<UploadProfileAvatarResponse>, AppError> {
    let input = UploadProfileAvatarInput {
        npub: request.npub,
        bytes: request.bytes,
        format: request.format,
        access_level: request.access_level,
    };
    let result = avatar_service
        .upload_avatar(input)
        .await
        .map(UploadProfileAvatarResponse::from);
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn fetch_profile_avatar(
    request: FetchProfileAvatarRequest,
    avatar_service: State<'_, Arc<ProfileAvatarService>>,
) -> Result<ApiResponse<FetchProfileAvatarResponse>, AppError> {
    let result = avatar_service
        .fetch_avatar(&request.npub)
        .await
        .map(FetchProfileAvatarResponse::from);
    Ok(ApiResponse::from_result(result))
}
