use crate::{
    application::services::{PostService, AuthService},
    presentation::dto::{
        post_dto::{
            BatchBookmarkRequest, BatchGetPostsRequest, BatchReactRequest, BookmarkAction,
            BookmarkPostRequest, CreatePostRequest, DeletePostRequest, GetPostsRequest,
            PostResponse, ReactToPostRequest,
        },
        ApiResponse, Validate,
    },
    shared::error::AppError,
};
use futures::future::join_all;
use std::sync::Arc;

pub struct PostHandler {
    post_service: Arc<PostService>,
    auth_service: Arc<AuthService>,
}

impl PostHandler {
    pub fn new(post_service: Arc<PostService>) -> Self {
        // TODO: AuthServiceを適切に注入する必要がある
        // 一時的にパニックを避けるため、コンパイルエラーのままにする
        unimplemented!("PostHandler needs AuthService injection")
    }
    
    pub fn with_auth(post_service: Arc<PostService>, auth_service: Arc<AuthService>) -> Self {
        Self { 
            post_service,
            auth_service,
        }
    }

    pub async fn create_post(&self, request: CreatePostRequest) -> Result<PostResponse, AppError> {
        // 入力検証
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        // 現在のユーザーを取得
        let current_user = self.auth_service
            .get_current_user()
            .await?
            .ok_or_else(|| AppError::Unauthorized("ユーザーが認証されていません".to_string()))?;

        // サービス層を呼び出し
        let post = self
            .post_service
            .create_post(request.content, current_user, request.topic_id)
            .await?;

        // DTOに変換
        Ok(PostResponse {
            id: post.id.to_string(),
            content: post.content,
            author_pubkey: post.author.pubkey.clone(),
            author_npub: {
                use nostr_sdk::prelude::*;
                PublicKey::from_hex(&post.author.pubkey)
                    .ok()
                    .and_then(|pk| pk.to_bech32().ok())
                    .unwrap_or_else(|| post.author.pubkey.clone())
            },
            topic_id: post.topic_id,
            created_at: post.created_at.timestamp(),
            likes: post.likes,
            boosts: post.boosts,
            replies: post.replies.len() as u32,
            is_synced: post.is_synced,
        })
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

        // 並行処理でnpub変換を行う
        let futures = posts.into_iter().map(|post| {
            async move {
                // npub変換をブロッキングタスクで並行実行
                let npub = tokio::task::spawn_blocking({
                    let pubkey = post.author.pubkey.clone();
                    move || {
                        use nostr_sdk::prelude::*;
                        PublicKey::from_hex(&pubkey)
                            .ok()
                            .and_then(|pk| pk.to_bech32().ok())
                            .unwrap_or(pubkey)
                    }
                }).await.unwrap_or_else(|_| post.author.pubkey.clone());

                PostResponse {
                    id: post.id.to_string(),
                    content: post.content,
                    author_pubkey: post.author.pubkey.clone(),
                    author_npub: npub,
                    topic_id: post.topic_id,
                    created_at: post.created_at.timestamp(),
                    likes: post.likes,
                    boosts: post.boosts,
                    replies: post.replies.len() as u32,
                    is_synced: post.is_synced,
                }
            }
        });

        // すべての変換を並行実行
        let results = join_all(futures).await;
        Ok(results)
    }

    pub async fn delete_post(&self, request: DeletePostRequest) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .delete_post(&request.post_id)
            .await?;
        Ok(())
    }

    pub async fn react_to_post(&self, request: ReactToPostRequest) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .react_to_post(&request.post_id, &request.reaction)
            .await?;
        Ok(())
    }

    pub async fn bookmark_post(&self, request: BookmarkPostRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .bookmark_post(&request.post_id, user_pubkey)
            .await?;
        Ok(())
    }

    pub async fn unbookmark_post(&self, request: BookmarkPostRequest, user_pubkey: &str) -> Result<(), AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        self.post_service
            .unbookmark_post(&request.post_id, user_pubkey)
            .await?;
        Ok(())
    }

    /// ユーザーのブックマーク済み投稿IDを取得
    pub async fn get_bookmarked_post_ids(&self, user_pubkey: &str) -> Result<Vec<String>, AppError> {
        let post_ids = self.post_service
            .get_bookmarked_post_ids(user_pubkey)
            .await?;
        Ok(post_ids)
    }

    // バッチ処理メソッド
    pub async fn batch_get_posts(&self, request: BatchGetPostsRequest) -> Result<Vec<PostResponse>, AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        // 並行して複数の投稿を取得
        let futures = request.post_ids.iter().map(|post_id| {
            let service = self.post_service.clone();
            let id = post_id.clone();
            async move {
                service.get_post(&id).await
            }
        });

        let results = join_all(futures).await;
        
        let mut posts = Vec::new();
        for result in results {
            if let Ok(Some(post)) = result {
                // npub変換を並行処理
                let npub = tokio::task::spawn_blocking({
                    let pubkey = post.author.pubkey.clone();
                    move || {
                        use nostr_sdk::prelude::*;
                        PublicKey::from_hex(&pubkey)
                            .ok()
                            .and_then(|pk| pk.to_bech32().ok())
                            .unwrap_or(pubkey)
                    }
                }).await.unwrap_or_else(|_| post.author.pubkey.clone());

                posts.push(PostResponse {
                    id: post.id.to_string(),
                    content: post.content,
                    author_pubkey: post.author.pubkey.clone(),
                    author_npub: npub,
                    topic_id: post.topic_id,
                    created_at: post.created_at.timestamp(),
                    likes: post.likes,
                    boosts: post.boosts,
                    replies: post.replies.len() as u32,
                    is_synced: post.is_synced,
                });
            }
        }

        Ok(posts)
    }

    pub async fn batch_react(&self, request: BatchReactRequest) -> Result<Vec<Result<(), String>>, AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        // 並行して複数のリアクションを処理
        let futures = request.reactions.iter().map(|reaction| {
            let service = self.post_service.clone();
            let req = reaction.clone();
            async move {
                service.react_to_post(&req.post_id, &req.reaction)
                    .await
                    .map_err(|e| e.to_string())
            }
        });

        let results = join_all(futures).await;
        Ok(results)
    }

    pub async fn batch_bookmark(&self, request: BatchBookmarkRequest, user_pubkey: &str) -> Result<Vec<Result<(), String>>, AppError> {
        request.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        // 並行して複数のブックマークを処理
        let futures = request.post_ids.iter().map(|post_id| {
            let service = self.post_service.clone();
            let id = post_id.clone();
            let pubkey = user_pubkey.to_string();
            let action = request.action.clone();
            
            async move {
                match action {
                    BookmarkAction::Add => {
                        service.bookmark_post(&id, &pubkey)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    BookmarkAction::Remove => {
                        service.unbookmark_post(&id, &pubkey)
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
            }
        });

        let results = join_all(futures).await;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::dto::Pagination;

    #[test]
    fn test_create_post_request_validation() {
        // 空のコンテンツはエラー
        let invalid_request = CreatePostRequest {
            content: "".to_string(),
            topic_id: Some("public".to_string()),
        };
        assert!(invalid_request.validate().is_err());

        // 有効なリクエスト
        let valid_request = CreatePostRequest {
            content: "Test post content".to_string(),
            topic_id: Some("public".to_string()),
        };
        assert!(valid_request.validate().is_ok());
    }

    #[test]
    fn test_get_posts_request_default_pagination() {
        let request = GetPostsRequest {
            topic_id: Some("public".to_string()),
            author_pubkey: None,
            pagination: None,
        };

        // デフォルトのpaginationが適用されることを確認
        let pagination = request.pagination.unwrap_or_default();
        assert_eq!(pagination.limit, Some(50));
        assert_eq!(pagination.offset, Some(0));
    }

    #[test]
    fn test_batch_bookmark_request_validation() {
        // 空のpost_idsはエラー
        let invalid_request = BatchBookmarkRequest {
            post_ids: vec![],
            action: BookmarkAction::Add,
        };
        assert!(invalid_request.validate().is_err());

        // 有効なリクエスト
        let valid_request = BatchBookmarkRequest {
            post_ids: vec!["post1".to_string(), "post2".to_string()],
            action: BookmarkAction::Add,
        };
        assert!(valid_request.validate().is_ok());
    }

    #[test]
    fn test_batch_react_request_validation() {
        // 空のreactionsはエラー
        let invalid_request = BatchReactRequest {
            reactions: vec![],
        };
        assert!(invalid_request.validate().is_err());

        // 有効なリクエスト
        let valid_request = BatchReactRequest {
            reactions: vec![
                ReactToPostRequest {
                    post_id: "post1".to_string(),
                    reaction: "+".to_string(),
                },
            ],
        };
        assert!(valid_request.validate().is_ok());
    }
}