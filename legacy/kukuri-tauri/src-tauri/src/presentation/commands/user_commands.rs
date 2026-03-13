use crate::application::ports::repositories::FollowListSort;
use crate::application::services::offline_service::OfflineServiceTrait;
use crate::application::services::user_search_service::{
    DEFAULT_LIMIT as SEARCH_DEFAULT_LIMIT, MAX_LIMIT as SEARCH_MAX_LIMIT, SearchSort,
    SearchUsersParams,
};
use crate::application::services::{
    OfflineService, ProfileAvatarService, UploadProfileAvatarInput, UserSearchService, UserService,
};
use crate::domain::entities::UserMetadata;
use crate::domain::entities::offline::CacheMetadataUpdate;
use crate::domain::value_objects::offline::{CacheKey, CacheType};
use crate::presentation::dto::{
    ApiResponse, Validate,
    profile_avatar_dto::{
        FetchProfileAvatarRequest, FetchProfileAvatarResponse, ProfileAvatarSyncRequest,
        ProfileAvatarSyncResponse, UploadProfileAvatarRequest, UploadProfileAvatarResponse,
    },
    user_dto::{
        GetFollowersRequest, GetFollowingRequest, PaginatedUserProfiles, SearchUsersRequest,
        SearchUsersResponse, UpdatePrivacySettingsRequest, UpdateUserProfileRequest,
        UserProfile as UserProfileDto,
    },
};
use crate::shared::AppError;
use chrono::{Duration, Utc};
use nostr_sdk::prelude::{FromBech32, PublicKey};
use serde_json::json;
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
) -> Result<ApiResponse<Option<UserProfileDto>>, AppError> {
    let result = user_service
        .get_user(&npub)
        .await
        .map(|user| user.map(map_user_to_profile));
    Ok(ApiResponse::from_result(result))
}

#[tauri::command]
pub async fn get_user_by_pubkey(
    pubkey: String,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<Option<UserProfileDto>>, AppError> {
    let result = user_service
        .get_user_by_pubkey(&pubkey)
        .await
        .map(|user| user.map(map_user_to_profile));
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
pub async fn update_user_profile(
    request: UpdateUserProfileRequest,
    user_service: State<'_, Arc<UserService>>,
) -> Result<ApiResponse<()>, AppError> {
    request.validate()?;

    let metadata = UserMetadata {
        name: Some(request.name.trim().to_string()),
        display_name: Some(request.display_name.trim().to_string()),
        about: Some(request.about.trim().to_string()),
        picture: Some(request.picture.trim().to_string()),
        banner: None,
        nip05: Some(request.nip05.trim().to_string()),
        lud16: None,
        public_profile: None,
        show_online_status: None,
    };
    let result = user_service.update_profile(&request.npub, metadata).await;
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
            request.viewer_npub.as_deref(),
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
            request.viewer_npub.as_deref(),
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

#[tauri::command]
pub async fn profile_avatar_sync(
    request: ProfileAvatarSyncRequest,
    avatar_service: State<'_, Arc<ProfileAvatarService>>,
    offline_service: State<'_, Arc<OfflineService>>,
) -> Result<ApiResponse<ProfileAvatarSyncResponse>, AppError> {
    let npub = request.npub.clone();
    let response = match avatar_service.fetch_avatar(&request.npub).await {
        Ok(result) => {
            let current_version = result.metadata.version;
            let fetch_response = FetchProfileAvatarResponse::from(result);

            if let Some(known) = request.known_doc_version {
                if known >= current_version {
                    ProfileAvatarSyncResponse {
                        npub,
                        current_version: Some(current_version),
                        updated: false,
                        avatar: None,
                    }
                } else {
                    ProfileAvatarSyncResponse {
                        npub,
                        current_version: Some(current_version),
                        updated: true,
                        avatar: Some(fetch_response),
                    }
                }
            } else {
                ProfileAvatarSyncResponse {
                    npub,
                    current_version: Some(current_version),
                    updated: true,
                    avatar: Some(fetch_response),
                }
            }
        }
        Err(AppError::NotFound(_)) => ProfileAvatarSyncResponse {
            npub,
            current_version: None,
            updated: false,
            avatar: None,
        },
        Err(err) => return Err(err),
    };

    record_profile_avatar_sync_metadata(offline_service.as_ref(), &request, &response).await?;

    Ok(ApiResponse::success(response))
}

async fn record_profile_avatar_sync_metadata(
    offline_service: &OfflineService,
    request: &ProfileAvatarSyncRequest,
    response: &ProfileAvatarSyncResponse,
) -> Result<(), AppError> {
    let cache_key = CacheKey::new(format!("doc::profile_avatar::{}", request.npub))
        .map_err(AppError::InvalidInput)?;
    let cache_type =
        CacheType::new("profile_avatar".to_string()).map_err(AppError::InvalidInput)?;

    let payload_bytes = response
        .avatar
        .as_ref()
        .and_then(|avatar| i64::try_from(avatar.size_bytes).ok());

    let metadata = json!({
        "npub": request.npub,
        "source": request.source,
        "requestedAt": request.requested_at,
        "retryCount": request.retry_count,
        "jobId": request.job_id,
        "knownDocVersion": request.known_doc_version,
        "result": {
            "updated": response.updated,
            "currentVersion": response.current_version,
            "avatar": response.avatar.as_ref().map(|avatar| {
                json!({
                    "blobHash": avatar.blob_hash,
                    "docVersion": avatar.doc_version,
                    "sizeBytes": avatar.size_bytes,
                })
            }),
        },
        "loggedAt": Utc::now().to_rfc3339(),
    });

    let update = CacheMetadataUpdate {
        cache_key,
        cache_type,
        metadata: Some(metadata),
        expiry: Some(Utc::now() + Duration::minutes(30)),
        is_stale: Some(false),
        doc_version: response.current_version.map(|version| version as i64),
        blob_hash: response
            .avatar
            .as_ref()
            .map(|avatar| avatar.blob_hash.clone()),
        payload_bytes,
    };

    offline_service.upsert_cache_metadata(update).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::OfflineService;
    use crate::domain::entities::ProfileAvatarAccessLevel;
    use crate::infrastructure::offline::sqlite_store::SqliteOfflinePersistence;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn record_profile_avatar_sync_metadata_persists_cache_entry() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        let persistence = Arc::new(SqliteOfflinePersistence::new(pool.clone()));
        let offline_service = OfflineService::new(persistence);

        let request = ProfileAvatarSyncRequest {
            npub: "npub1test".to_string(),
            known_doc_version: Some(2),
            source: Some("profile-avatar-sync-worker:interval".to_string()),
            requested_at: Some("2025-11-12T00:00:00Z".to_string()),
            retry_count: Some(1),
            job_id: Some("job-42".to_string()),
        };

        let response = ProfileAvatarSyncResponse {
            npub: "npub1test".to_string(),
            current_version: Some(4),
            updated: true,
            avatar: Some(FetchProfileAvatarResponse {
                npub: "npub1test".to_string(),
                blob_hash: "bafy-avatar".to_string(),
                format: "image/png".to_string(),
                size_bytes: 1_024,
                access_level: ProfileAvatarAccessLevel::ContactsOnly,
                share_ticket: "ticket".to_string(),
                doc_version: 4,
                updated_at: "2025-11-12T00:00:00Z".to_string(),
                content_sha256: "abcd".to_string(),
                data_base64: "AAECAw==".to_string(),
            }),
        };

        record_profile_avatar_sync_metadata(&offline_service, &request, &response)
            .await
            .expect("metadata recorded");

        let row: (String, Option<i64>, Option<String>, Option<i64>) = sqlx::query_as(
            "SELECT cache_key, doc_version, blob_hash, payload_bytes FROM cache_metadata WHERE cache_key = ?1",
        )
        .bind("doc::profile_avatar::npub1test")
        .fetch_one(&pool)
        .await
        .expect("metadata row");

        assert_eq!(row.0, "doc::profile_avatar::npub1test");
        assert_eq!(row.1, Some(4));
        assert_eq!(row.2.as_deref(), Some("bafy-avatar"));
        assert_eq!(row.3, Some(1_024));
    }
}
