use crate::domain::entities::{DomainEvent, ProfileMetadata};
use crate::domain::value_objects::{EventId, PublicKey, ReactionValue, TopicContent, TopicId};
use crate::shared::error::AppError;
use async_trait::async_trait;

/// EventManager 縺ｪ縺ｩ縺ｮ蜈ｷ菴灘ｮ溯｣・↓萓晏ｭ倥○縺壹、pplication 螻､縺九ｉ繧､繝吶Φ繝磯・菫｡繧呈桶縺・◆繧√・繝昴・繝医・///
/// 險ｭ險医ラ繧ｭ繝･繝｡繝ｳ繝・ `docs/01_project/activeContext/artefacts/phase5_event_gateway_design.md`
#[async_trait]
pub trait EventGateway: Send + Sync {
    /// P2P 繧・Gossip 縺ｪ縺ｩ螟夜Κ繧ｽ繝ｼ繧ｹ縺九ｉ蜿嶺ｿ｡縺励◆繧､繝吶Φ繝医ｒ蜃ｦ逅・☆繧九・
    async fn handle_incoming_event(&self, event: DomainEvent) -> Result<(), AppError>;

    /// 閾ｪ霄ｫ縺ｮ繝弱・繝峨→縺励※繝・く繧ｹ繝医ヮ繝ｼ繝医ｒ逋ｺ陦後☆繧九・
    async fn publish_text_note(&self, content: &str) -> Result<EventId, AppError>;

    /// 繝医ヴ繝・け縺ｫ邏舌▼縺乗兜遞ｿ繧貞・髢九☆繧九Ａreply_to` 縺ｫ繧医ｊ繧ｹ繝ｬ繝・ラ霑比ｿ｡繧呈欠遉ｺ縺ｧ縺阪ｋ縲・
    async fn publish_topic_post(
        &self,
        topic_id: &TopicId,
        content: &TopicContent,
        reply_to: Option<&EventId>,
        scope: Option<&str>,
        epoch: Option<i64>,
    ) -> Result<EventId, AppError>;

    /// 莉ｻ諢上う繝吶Φ繝医∈繝ｪ繧｢繧ｯ繧ｷ繝ｧ繝ｳ繧帝∽ｿ｡縺吶ｋ縲・
    async fn send_reaction(
        &self,
        target: &EventId,
        reaction: &ReactionValue,
    ) -> Result<EventId, AppError>;

    /// 繝励Ο繝輔ぅ繝ｼ繝ｫ繝｡繧ｿ繝・・繧ｿ繧呈峩譁ｰ縺励∵眠縺励＞繧､繝吶Φ繝・ID 繧定ｿ泌唆縺吶ｋ縲・
    async fn update_profile_metadata(
        &self,
        metadata: &ProfileMetadata,
    ) -> Result<EventId, AppError>;

    /// 謖・ｮ壹＆繧後◆繧､繝吶Φ繝育ｾ､繧貞炎髯､縺励∝炎髯､繧､繝吶Φ繝医・ ID 繧定ｿ泌唆縺吶ｋ縲・
    async fn delete_events(
        &self,
        targets: &[EventId],
        reason: Option<&str>,
    ) -> Result<EventId, AppError>;
    async fn publish_repost(&self, target: &EventId) -> Result<EventId, AppError>;

    /// 繝阪ャ繝医Ρ繝ｼ繧ｯ謗･邯壹ｒ蛻・妙縺吶ｋ縲・
    async fn disconnect(&self) -> Result<(), AppError>;

    /// 繝弱・繝峨′蛻ｩ逕ｨ縺吶ｋ蜈ｬ髢矩嵯繧貞叙蠕励☆繧九・
    async fn get_public_key(&self) -> Result<Option<PublicKey>, AppError>;

    /// 譌｢螳壹・雉ｼ隱ｭ繝医ヴ繝・け繧呈峩譁ｰ縺吶ｋ縲・
    async fn set_default_topics(&self, topics: &[TopicId]) -> Result<(), AppError>;

    /// 譌｢螳壹・雉ｼ隱ｭ繝医ヴ繝・け繧剃ｸ隕ｧ縺吶ｋ縲・
    async fn list_default_topics(&self) -> Result<Vec<TopicId>, AppError>;
}
