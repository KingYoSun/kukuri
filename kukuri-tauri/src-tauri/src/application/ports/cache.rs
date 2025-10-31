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

    /// トピック単位で投稿を取得（新しい順）
    async fn get_by_topic(&self, topic_id: &str, limit: usize) -> Vec<Post>;

    /// トピックの投稿キャッシュを丸ごと差し替え
    async fn set_topic_posts(&self, topic_id: &str, posts: Vec<Post>);

    /// トピックに紐づく投稿キャッシュを無効化
    async fn invalidate_topic(&self, topic_id: &str);
}
