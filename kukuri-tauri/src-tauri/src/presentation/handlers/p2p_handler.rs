use crate::application::services::p2p_service::P2PServiceTrait;
use crate::presentation::dto::Validate;
use crate::presentation::dto::p2p::{
    BroadcastRequest, GossipMetricsSummaryResponse, JoinTopicRequest, LeaveTopicRequest,
    NodeAddressResponse, P2PStatusResponse, TopicStatus,
};
use crate::shared::error::AppError;
use serde_json::json;
use std::sync::Arc;

pub struct P2PHandler {
    p2p_service: Arc<dyn P2PServiceTrait>,
}

impl P2PHandler {
    pub fn new(p2p_service: Arc<dyn P2PServiceTrait>) -> Self {
        Self { p2p_service }
    }

    /// P2Pネットワークを初期化
    pub async fn initialize_p2p(&self) -> Result<serde_json::Value, AppError> {
        self.p2p_service.initialize().await?;
        Ok(json!({ "success": true }))
    }

    /// P2Pトピックに参加
    pub async fn join_topic(
        &self,
        request: JoinTopicRequest,
    ) -> Result<serde_json::Value, AppError> {
        request.validate()?;

        self.p2p_service
            .join_topic(&request.topic_id, request.initial_peers)
            .await?;

        Ok(json!({ "success": true }))
    }

    /// P2Pトピックから離脱
    pub async fn leave_topic(
        &self,
        request: LeaveTopicRequest,
    ) -> Result<serde_json::Value, AppError> {
        request.validate()?;

        self.p2p_service.leave_topic(&request.topic_id).await?;

        Ok(json!({ "success": true }))
    }

    /// トピックにメッセージをブロードキャスト
    pub async fn broadcast_to_topic(
        &self,
        request: BroadcastRequest,
    ) -> Result<serde_json::Value, AppError> {
        request.validate()?;

        self.p2p_service
            .broadcast_message(&request.topic_id, &request.content)
            .await?;

        Ok(json!({ "success": true }))
    }

    /// P2Pステータスを取得
    pub async fn get_p2p_status(&self) -> Result<P2PStatusResponse, AppError> {
        let status = self.p2p_service.get_status().await?;
        let crate::application::services::p2p_service::P2PStatus {
            connected,
            endpoint_id,
            active_topics,
            peer_count,
            metrics_summary,
        } = status;

        // サービスから取得したステータスをDTOに変換
        let topic_statuses: Vec<TopicStatus> = active_topics
            .into_iter()
            .map(|topic| TopicStatus {
                topic_id: topic.id,
                peer_count: topic.peer_count,
                message_count: topic.message_count,
                last_activity: topic.last_activity,
            })
            .collect();

        Ok(P2PStatusResponse {
            connected,
            endpoint_id,
            active_topics: topic_statuses,
            peer_count,
            metrics_summary: GossipMetricsSummaryResponse {
                joins: metrics_summary.joins,
                leaves: metrics_summary.leaves,
                broadcasts_sent: metrics_summary.broadcasts_sent,
                messages_received: metrics_summary.messages_received,
            },
        })
    }

    /// ノードアドレスを取得
    pub async fn get_node_address(&self) -> Result<NodeAddressResponse, AppError> {
        let addresses = self.p2p_service.get_node_addresses().await?;

        Ok(NodeAddressResponse { addresses })
    }

    /// トピック名で参加
    pub async fn join_topic_by_name(
        &self,
        topic_name: String,
        initial_peers: Vec<String>,
    ) -> Result<serde_json::Value, AppError> {
        if topic_name.is_empty() {
            return Err(AppError::ValidationError(
                "Topic name is required".to_string(),
            ));
        }

        // トピック名からIDを生成してjoin_topicを呼び出す
        let topic_id = self.p2p_service.generate_topic_id(&topic_name);

        self.p2p_service
            .join_topic(&topic_id, initial_peers)
            .await?;

        Ok(json!({
            "success": true,
            "topic_id": topic_id
        }))
    }
}
