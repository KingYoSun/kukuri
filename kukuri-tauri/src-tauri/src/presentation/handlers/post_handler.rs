use crate::{
    application::services::PostService,
    presentation::dto::{
        post_dto::{
            BookmarkPostRequest, CreatePostRequest, DeletePostRequest, GetPostsRequest,
            PostResponse, ReactToPostRequest,
        },
        ApiResponse, Validate,
    },
    shared::error::AppError,
};
use std::sync::Arc;

pub struct PostHandler {
    post_service: Arc<PostService>,
}

impl PostHandler {
    pub fn new(post_service: Arc<PostService>) -> Self {
        Self { post_service }
    }

    pub async fn create_post(&self, request: CreatePostRequest) -> Result<PostResponse, AppError> {
        // 入力検証
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        // サービス層を呼び出し
        let post = self
            .post_service
            .create_post(&request.content, &request.topic_id, request.media_urls)
            .await?;

        // DTOに変換
        Ok(PostResponse {
            id: post.id.to_string(),
            content: post.content,
            author_pubkey: post.author_pubkey.clone(),
            author_npub: {
                use nostr_sdk::prelude::*;
                PublicKey::from_hex(&post.author_pubkey)
                    .ok()
                    .and_then(|pk| pk.to_bech32().ok())
                    .unwrap_or_else(|| post.author_pubkey.clone())
            },
            topic_id: post.topic_id,
            created_at: post.created_at.timestamp(),
            likes: post.likes,
            boosts: post.boosts,
            replies: post.replies,
            is_synced: post.is_synced,
        })
    }

    pub async fn get_posts(&self, request: GetPostsRequest) -> Result<Vec<PostResponse>, AppError> {
        let pagination = request.pagination.unwrap_or_default();
        
        let posts = if let Some(topic_id) = request.topic_id {
            self.post_service
                .get_posts_by_topic(&topic_id, pagination.limit, pagination.offset)
                .await?
        } else if let Some(author) = request.author_pubkey {
            self.post_service
                .get_posts_by_author(&author, pagination.limit, pagination.offset)
                .await?
        } else {
            self.post_service
                .get_recent_posts(pagination.limit, pagination.offset)
                .await?
        };

        // DTOに変換
        Ok(posts
            .into_iter()
            .map(|post| PostResponse {
                id: post.id.to_string(),
                content: post.content,
                author_pubkey: post.author_pubkey.clone(),
                author_npub: crate::modules::utils::commands::pubkey_to_npub_internal(&post.author_pubkey)
                    .unwrap_or_else(|_| post.author_pubkey.clone()),
                topic_id: post.topic_id,
                created_at: post.created_at.timestamp(),
                likes: post.likes,
                boosts: post.boosts,
                replies: post.replies,
                is_synced: post.is_synced,
            })
            .collect())
    }

    pub async fn delete_post(&self, request: DeletePostRequest) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .delete_post(&request.post_id, request.reason.as_deref())
            .await
    }

    pub async fn react_to_post(&self, request: ReactToPostRequest) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .react_to_post(&request.post_id, &request.reaction)
            .await
    }

    pub async fn bookmark_post(&self, request: BookmarkPostRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .bookmark_post(&request.post_id, user_pubkey)
            .await
    }

    pub async fn unbookmark_post(&self, request: BookmarkPostRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .unbookmark_post(&request.post_id, user_pubkey)
            .await
    }
}