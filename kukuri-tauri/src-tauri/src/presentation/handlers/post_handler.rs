use crate::{
    application::services::{AuthService, PostService, TopicService, TopicTimelineEntry},
    domain::entities::Post,
    presentation::dto::{
        Validate,
        post_dto::{
            BookmarkPostRequest, CreatePostRequest, DeletePostRequest, FollowingFeedPageResponse,
            GetPostsRequest, GetThreadPostsRequest, GetTopicTimelineRequest,
            ListFollowingFeedRequest, ListTrendingPostsRequest, ListTrendingPostsResponse,
            PostResponse, ReactToPostRequest, TopicTimelineEntryResponse,
            TrendingTopicPostsResponse,
        },
    },
    shared::error::AppError,
};
use chrono::Utc;
use futures::future::join_all;
use std::sync::Arc;

pub struct PostHandler {
    post_service: Arc<PostService>,
    auth_service: Arc<AuthService>,
    topic_service: Arc<TopicService>,
}

impl PostHandler {
    async fn map_post(post: Post) -> PostResponse {
        let author_pubkey = post.author.pubkey.clone();
        let npub = tokio::task::spawn_blocking({
            let pubkey = author_pubkey.clone();
            move || {
                use nostr_sdk::prelude::*;
                PublicKey::from_hex(&pubkey)
                    .ok()
                    .and_then(|pk| pk.to_bech32().ok())
                    .unwrap_or(pubkey)
            }
        })
        .await
        .unwrap_or(author_pubkey.clone());

        PostResponse {
            id: post.id.to_string(),
            content: post.content,
            author_pubkey: author_pubkey.clone(),
            author_npub: npub,
            topic_id: post.topic_id,
            thread_namespace: post.thread_namespace,
            thread_uuid: post.thread_uuid,
            thread_root_event_id: post.thread_root_event_id,
            thread_parent_event_id: post.thread_parent_event_id,
            scope: post.scope,
            epoch: post.epoch,
            is_encrypted: post.is_encrypted,
            created_at: post.created_at.timestamp(),
            likes: post.likes,
            boosts: post.boosts,
            replies: post.replies.len() as u32,
            is_synced: post.is_synced,
        }
    }

    async fn map_posts(posts: Vec<Post>) -> Vec<PostResponse> {
        let futures = posts.into_iter().map(Self::map_post);
        join_all(futures).await
    }

    async fn map_timeline_entry(entry: TopicTimelineEntry) -> TopicTimelineEntryResponse {
        let parent_post = Self::map_post(entry.parent_post).await;
        let first_reply = match entry.first_reply {
            Some(post) => Some(Self::map_post(post).await),
            None => None,
        };

        TopicTimelineEntryResponse {
            thread_uuid: entry.thread_uuid,
            parent_post,
            first_reply,
            reply_count: entry.reply_count,
            last_activity_at: entry.last_activity_at,
        }
    }

    pub fn new(
        post_service: Arc<PostService>,
        auth_service: Arc<AuthService>,
        topic_service: Arc<TopicService>,
    ) -> Self {
        Self {
            post_service,
            auth_service,
            topic_service,
        }
    }

    pub async fn create_post(&self, request: CreatePostRequest) -> Result<PostResponse, AppError> {
        // 入力検証
        request.validate().map_err(AppError::InvalidInput)?;

        // 現在のユーザーを取得
        let current_user =
            self.auth_service.get_current_user().await?.ok_or_else(|| {
                AppError::Unauthorized("ユーザーが認証されていません".to_string())
            })?;

        // サービス層を呼び出し
        let post = self
            .post_service
            .create_post(
                request.content,
                current_user,
                request.topic_id,
                request.thread_uuid,
                request.reply_to,
                request.scope,
            )
            .await?;

        // DTOに変換
        Ok(Self::map_post(post).await)
    }

    pub async fn get_posts(&self, request: GetPostsRequest) -> Result<Vec<PostResponse>, AppError> {
        let pagination = request.pagination.unwrap_or_default();

        let posts = if let Some(topic_id) = request.topic_id {
            self.post_service
                .get_posts_by_topic(&topic_id, pagination.limit.unwrap_or(50) as usize)
                .await?
        } else if let Some(author) = request.author_pubkey {
            self.post_service
                .get_posts_by_author(&author, pagination.limit.unwrap_or(50) as usize)
                .await?
        } else {
            self.post_service
                .get_recent_posts(pagination.limit.unwrap_or(50) as usize)
                .await?
        };

        let results = Self::map_posts(posts).await;
        Ok(results)
    }

    pub async fn get_thread_posts(
        &self,
        request: GetThreadPostsRequest,
    ) -> Result<Vec<PostResponse>, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let pagination = request.pagination.unwrap_or_default();
        let posts = self
            .post_service
            .get_thread_posts(
                &request.topic_id,
                &request.thread_uuid,
                pagination.limit.unwrap_or(200) as usize,
            )
            .await?;

        Ok(Self::map_posts(posts).await)
    }

    pub async fn get_topic_timeline(
        &self,
        request: GetTopicTimelineRequest,
    ) -> Result<Vec<TopicTimelineEntryResponse>, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let pagination = request.pagination.unwrap_or_default();
        let limit = pagination.limit.unwrap_or(50).clamp(1, 100) as usize;
        let entries = self
            .post_service
            .get_topic_timeline(&request.topic_id, limit)
            .await?;

        let futures = entries.into_iter().map(Self::map_timeline_entry);
        Ok(join_all(futures).await)
    }

    pub async fn delete_post(&self, request: DeletePostRequest) -> Result<(), AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        self.post_service.delete_post(&request.post_id).await?;
        Ok(())
    }

    pub async fn react_to_post(&self, request: ReactToPostRequest) -> Result<(), AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        self.post_service
            .react_to_post(&request.post_id, &request.reaction)
            .await?;
        Ok(())
    }

    pub async fn bookmark_post(
        &self,
        request: BookmarkPostRequest,
        user_pubkey: &str,
    ) -> Result<(), AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        self.post_service
            .bookmark_post(&request.post_id, user_pubkey)
            .await?;
        Ok(())
    }

    pub async fn unbookmark_post(
        &self,
        request: BookmarkPostRequest,
        user_pubkey: &str,
    ) -> Result<(), AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        self.post_service
            .unbookmark_post(&request.post_id, user_pubkey)
            .await?;
        Ok(())
    }

    /// ユーザーのブックマーク済み投稿IDを取得
    pub async fn get_bookmarked_post_ids(
        &self,
        user_pubkey: &str,
    ) -> Result<Vec<String>, AppError> {
        let post_ids = self
            .post_service
            .get_bookmarked_post_ids(user_pubkey)
            .await?;
        Ok(post_ids)
    }

    pub async fn list_trending_posts(
        &self,
        request: ListTrendingPostsRequest,
    ) -> Result<ListTrendingPostsResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let per_topic = request.per_topic.unwrap_or(3).clamp(1, 20) as usize;
        let mut topics = Vec::new();

        for (index, topic_id) in request.topic_ids.iter().enumerate() {
            if let Some(topic) = self.topic_service.get_topic(topic_id).await? {
                let posts = self
                    .post_service
                    .get_posts_by_topic(topic_id, per_topic)
                    .await?;
                let responses = Self::map_posts(posts).await;
                topics.push(TrendingTopicPostsResponse {
                    topic_id: topic.id.clone(),
                    topic_name: topic.name.clone(),
                    relative_rank: (index + 1) as u32,
                    posts: responses,
                });
            }
        }

        let generated_at = self
            .topic_service
            .latest_metrics_generated_at()
            .await?
            .unwrap_or_else(|| Utc::now().timestamp_millis());

        Ok(ListTrendingPostsResponse {
            generated_at,
            topics,
        })
    }

    pub async fn list_following_feed(
        &self,
        follower_pubkey: &str,
        request: ListFollowingFeedRequest,
    ) -> Result<FollowingFeedPageResponse, AppError> {
        request.validate().map_err(AppError::InvalidInput)?;

        let limit = request.limit.unwrap_or(20).clamp(1, 100) as usize;
        let _include_reactions = request.include_reactions.unwrap_or(false);
        let feed = self
            .post_service
            .list_following_feed(follower_pubkey, request.cursor.as_deref(), limit)
            .await?;
        let items = Self::map_posts(feed.items).await;

        Ok(FollowingFeedPageResponse {
            items,
            next_cursor: feed.next_cursor,
            has_more: feed.has_more,
            server_time: feed.server_time,
        })
    }
}
