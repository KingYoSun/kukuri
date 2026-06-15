use super::*;

pub(crate) fn gossip_disabled_channel_key(topic_id: &str, channel_id: &str) -> String {
    format!("{topic_id}::{channel_id}")
}

impl AppService {
    pub(crate) async fn is_topic_gossip_disabled(&self, topic_id: &str) -> bool {
        self.gossip_disabled_topics.lock().await.contains(topic_id)
    }

    pub(crate) async fn is_channel_gossip_disabled(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> bool {
        if self.is_topic_gossip_disabled(topic_id).await {
            return true;
        }
        self.gossip_disabled_channels
            .lock()
            .await
            .contains(&gossip_disabled_channel_key(topic_id, channel_id))
    }

    pub async fn list_gossip_disabled_topics(&self) -> Vec<String> {
        let mut topics = self
            .gossip_disabled_topics
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        topics.sort();
        topics
    }

    pub async fn list_gossip_disabled_channels(&self) -> Vec<String> {
        let mut channels = self
            .gossip_disabled_channels
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        channels.sort();
        channels
    }

    pub async fn restore_gossip_disabled_state(
        &self,
        disabled_topics: Vec<String>,
        disabled_channels: Vec<String>,
    ) {
        {
            let mut topics = self.gossip_disabled_topics.lock().await;
            topics.clear();
            topics.extend(disabled_topics);
        }
        {
            let mut channels = self.gossip_disabled_channels.lock().await;
            channels.clear();
            channels.extend(disabled_channels);
        }
    }

    /// Tear down the gossip subscription for a single private channel without
    /// leaving the channel. Mirrors the abort path of
    /// [`restart_private_channel_subscription`].
    pub(crate) async fn unsubscribe_private_channel(
        &self,
        topic_id: &str,
        channel_id: &str,
    ) -> Result<()> {
        let prefix = joined_private_channel_subscription_prefix(topic_id, channel_id);
        let keys = self
            .private_channel_subscriptions
            .lock()
            .await
            .keys()
            .filter(|key| key.starts_with(prefix.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            if let Some(handle) = self
                .private_channel_subscriptions
                .lock()
                .await
                .remove(key.as_str())
            {
                handle.abort();
            }
        }
        self.hint_transport
            .unsubscribe_hints(&private_channel_hint_topic(channel_id))
            .await
    }
}
