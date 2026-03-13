use crate::shared::error::AppError;
use async_trait::async_trait;
use nostr_sdk::Timestamp;

/// 購読復元や加入リクエストを実行するためのポート。
///
/// EventService など Application 層はこの trait を通じて購読処理を発行し、
/// 具体的な実装（EventManager など）は Infrastructure 層に閉じ込める。
#[async_trait]
pub trait SubscriptionInvoker: Send + Sync {
    /// 指定トピックに対する購読を開始する。
    async fn subscribe_topic(
        &self,
        topic_id: &str,
        since: Option<Timestamp>,
    ) -> Result<(), AppError>;

    /// 指定ユーザー（公開鍵）に対する購読を開始する。
    async fn subscribe_user(&self, pubkey: &str, since: Option<Timestamp>) -> Result<(), AppError>;
}
