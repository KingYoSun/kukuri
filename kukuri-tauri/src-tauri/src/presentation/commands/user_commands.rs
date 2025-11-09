use crate::application::ports::repositories::FollowListSort;
use crate::application::services::{
    ProfileAvatarService, UploadProfileAvatarInput, UserSearchService, UserService,
};
use crate::application::services::user_search_service::{
    SearchSort, SearchUsersParams, DEFAULT_LIMIT as SEARCH_DEFAULT_LIMIT,
    MAX_LIMIT as SEARCH_MAX_LIMIT,
};
use crate::domain::entities::UserMetadata;
use crate::presentation::dto::{
    ApiResponse, Validate,
    profile_avatar_dto::{
        FetchProfileAvatarRequest, FetchProfileAvatarResponse, UploadProfileAvatarRequest,
        UploadProfileAvatarResponse,
    },
    user_dto::{
        GetFollowersRequest, GetFollowingRequest, PaginatedUserProfiles, SearchUsersRequest,
        SearchUsersResponse, UpdatePrivacySettingsRequest, UserProfile as UserProfileDto,
    },
};
use crate::shared::AppError;
use nostr_sdk::prelude::{FromBech32, PublicKey};
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
        is_profile_public: Some(user.public_profile),
        show_online_status: Some(user.show_online_status),
    }
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
    request: SearchUsersRequest,
    user_search_service: State<'_, Arc<UserSearchService>>,
) -> Result<ApiResponse<SearchUsersResponse>, AppError> {
    request.validate()?;

    let trimmed_query = request.query.trim().to_string();
    let limit = request
        .limit
        .map(|value| value as usize)
        .unwrap_or(SEARCH_DEFAULT_LIMIT)
        .clamp(1, SEARCH_MAX_LIMIT);
    let sort = SearchSort::try_from_str(request.sort.as_deref())?;
    let viewer_pubkey = if let Some(npub) = request.viewer_npub.as_deref() {
        Some(
            PublicKey::from_bech32(npub)
                .map_err(|_| AppError::InvalidInput("Invalid viewer npub".into()))?
                .to_hex(),
        )
    } else {
        None
    };

    let params = SearchUsersParams {
        query: trimmed_query,
        cursor: request.cursor.clone(),
        limit,
        sort,
        allow_incomplete: request.allow_incomplete.unwrap_or(false),
        viewer_pubkey,
    };

    let result = user_search_service.search(params).await?;
    let response = SearchUsersResponse {
        items: result.users.into_iter().map(map_user_to_profile).collect(),
        next_cursor: result.next_cursor,
        has_more: result.has_more,
        total_count: result.total_count as u64,
        took_ms: result.took_ms.min(u64::MAX as u128) as u64,
    };

    Ok(ApiResponse::success(response))
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
pub async fn update_privacy_settings(
    request: UpdatePrivacySettingsRequest,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<()>, AppError> {
    request.validate()?;

    let result = user_service
        .update_privacy_settings(
            &request.npub,
            request.public_profile,
            request.show_online_status,
        )
        .await;
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
    let sort = match request.sort.as_deref() {
        Some(value) => FollowListSort::try_from(value)
            .map_err(|_| AppError::InvalidInput(format!("Unsupported followers sort: {value}")))?,
        None => FollowListSort::Recent,
    };
    let search = request
        .search
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let result = user_service
        .get_followers_paginated(
            &request.npub,
            request.cursor.as_deref(),
            limit,
            sort,
            search,
        )
        .await
        .map(|page| PaginatedUserProfiles {
            items: page.users.into_iter().map(map_user_to_profile).collect(),
            next_cursor: page.next_cursor,
            has_more: page.has_more,
            total_count: page.total_count,
        });
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_following(
    request: GetFollowingRequest,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<PaginatedUserProfiles>, AppError> {
    let limit = request.limit.unwrap_or(25).min(100) as usize;
    let sort = match request.sort.as_deref() {
        Some(value) => FollowListSort::try_from(value)
            .map_err(|_| AppError::InvalidInput(format!("Unsupported following sort: {value}")))?,
        None => FollowListSort::Recent,
    };
    let search = request
        .search
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let result = user_service
        .get_following_paginated(
            &request.npub,
            request.cursor.as_deref(),
            limit,
            sort,
            search,
        )
        .await
        .map(|page| PaginatedUserProfiles {
            items: page.users.into_iter().map(map_user_to_profile).collect(),
            next_cursor: page.next_cursor,
            has_more: page.has_more,
            total_count: page.total_count,
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
