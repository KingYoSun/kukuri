use crate::domain::entities::{DomainEvent, ProfileMetadata};
use crate::domain::value_objects::{EventId, PublicKey, ReactionValue, TopicContent, TopicId};
use crate::shared::error::AppError;
use async_trait::async_trait;

/// EventManager などの具体実装に依存せず、Application 層からイベント配信を扱うためのポート。
///
/// 設計ドキュメント: `docs/01_project/activeContext/artefacts/phase5_event_gateway_design.md`
#[async_trait]
pub trait EventGateway: Send + Sync {
    /// P2P や Gossip など外部ソースから受信したイベントを処理する。
    async fn handle_incoming_event(&self, event: DomainEvent) -> Result<(), AppError>;

    /// 自身のノードとしてテキストノートを発行する。
    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError>;

    /// トピックに紐づく投稿を公開する。`reply_to` によりスレッド返信を指示できる。
    async fn publish_topic_post(
        &self,
        topic_id: &TopicId,
        content: &TopicContent,
        reply_to: Option<&EventId>,
    ) -> Result<EventId, AppError>;

    /// 任意イベントへリアクションを送信する。
    async fn send_reaction(
        &self,
        target: &EventId,
        reaction: &ReactionValue,
    ) -> Result<EventId, AppError>;

    /// プロフィールメタデータを更新し、新しいイベント ID を返却する。
    async fn update_profile_metadata(
        &self,
        metadata: &ProfileMetadata,
    ) -> Result<EventId, AppError>;

    /// 指定されたイベント群を削除し、削除イベントの ID を返却する。
    async fn delete_events(
        &self,
        targets: &[EventId],
        reason: Option<&str>,
    ) -> Result<EventId, AppError>;

    /// ネットワーク接続を切断する。
    async fn disconnect(&self) -> Result<(), AppError>;

    /// ノードが利用する公開鍵を取得する。
    async fn get_public_key(&self) -> Result<Option<PublicKey>, AppError>;

    /// 既定の購読トピックを更新する。
    async fn set_default_topics(&self, topics: &[TopicId]) -> Result<(), AppError>;

    /// 既定の購読トピックを一覧する。
    async fn list_default_topics(&self) -> Result<Vec<TopicId>, AppError>;
}
