use crate::domain::entities::Post;
use async_trait::async_trait;

/// 投稿エンティティ用のキャッシュポート
#[async_trait]
pub trait PostCache: Send + Sync {
    /// 投稿をキャッシュに追加
    async fn add(&self, post: Post);

    /// ID でキャッシュを検索
    async fn get(&self, id: &str) -> Option<Post>;

    /// キャッシュから投稿を削除
    async fn remove(&self, id: &str) -> Option<Post>;
}
