use crate::shared::error::AppError;
use async_trait::async_trait;

/// EventManager が参照するイベントとトピックの対応情報を
/// アプリケーション層に閉じ込めるためのポート。
#[async_trait]
pub trait EventTopicStore: Send + Sync {
    /// イベントとトピックの関連を保存する（冪等）
    async fn add_event_topic(&self, event_id: &str, topic_id: &str) -> Result<(), AppError>;

    /// イベントが属するトピック一覧を取得する
    async fn get_event_topics(&self, event_id: &str) -> Result<Vec<String>, AppError>;
}
