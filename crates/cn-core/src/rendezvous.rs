use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::config::TOPIC_RENDEZVOUS_TTL_SECONDS;
use crate::models::CommunityNodeSeedPeer;

#[derive(Clone, Debug)]
pub struct TopicRendezvousStore {
    client: redis::Client,
    key_prefix: String,
    ttl_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRendezvousHeartbeat {
    pub endpoint_id: String,
    pub addr_hint: Option<String>,
    pub joins: Vec<String>,
    pub refreshes: Vec<String>,
    pub leaves: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRendezvousCandidate {
    pub endpoint_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr_hint: Option<String>,
    #[serde(default)]
    pub relay_urls: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRendezvousTopicResponse {
    pub topic_key: String,
    pub peers: Vec<TopicRendezvousCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRendezvousHeartbeatResponse {
    pub expires_in_seconds: u64,
    pub topics: Vec<TopicRendezvousTopicResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredRendezvousPeer {
    endpoint_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    addr_hint: Option<String>,
}

impl TopicRendezvousStore {
    pub fn new(redis_url: &str, key_prefix: impl Into<String>) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let key_prefix = normalize_key_prefix(key_prefix.into().as_str())?;
        Ok(Self {
            client,
            key_prefix,
            ttl_seconds: TOPIC_RENDEZVOUS_TTL_SECONDS,
        })
    }

    pub async fn heartbeat(
        &self,
        heartbeat: TopicRendezvousHeartbeat,
        relay_urls: &[String],
    ) -> Result<TopicRendezvousHeartbeatResponse> {
        let endpoint = CommunityNodeSeedPeer::new(heartbeat.endpoint_id, heartbeat.addr_hint)?;
        let joins = normalize_topic_keys(heartbeat.joins)?;
        let refreshes = normalize_topic_keys(heartbeat.refreshes)?;
        let leaves = normalize_topic_keys(heartbeat.leaves)?;
        let mut active_topics = BTreeSet::new();
        active_topics.extend(joins.iter().cloned());
        active_topics.extend(refreshes.iter().cloned());

        let mut connection = self.client.get_multiplexed_async_connection().await?;
        let stored_peer = serde_json::to_string(&StoredRendezvousPeer {
            endpoint_id: endpoint.endpoint_id.clone(),
            addr_hint: endpoint.addr_hint.clone(),
        })?;

        if !active_topics.is_empty() {
            let _: () = connection
                .set_ex(
                    self.peer_key(endpoint.endpoint_id.as_str()),
                    stored_peer,
                    self.ttl_seconds,
                )
                .await?;
        }

        for topic_key in &active_topics {
            let key = self.topic_key(topic_key);
            let _: usize = connection
                .sadd(key.as_str(), endpoint.endpoint_id.as_str())
                .await?;
            let _: bool = connection
                .expire(key.as_str(), self.ttl_seconds as i64)
                .await?;
        }

        for topic_key in &leaves {
            let key = self.topic_key(topic_key);
            let _: usize = connection
                .srem(key.as_str(), endpoint.endpoint_id.as_str())
                .await?;
        }

        let mut topics = Vec::with_capacity(active_topics.len());
        for topic_key in active_topics {
            let key = self.topic_key(topic_key.as_str());
            let mut endpoint_ids: Vec<String> = connection.smembers(key.as_str()).await?;
            endpoint_ids.sort();
            endpoint_ids.dedup();

            let mut peers = Vec::new();
            for endpoint_id in endpoint_ids {
                if endpoint_id == endpoint.endpoint_id {
                    continue;
                }
                let peer_json: Option<String> =
                    connection.get(self.peer_key(endpoint_id.as_str())).await?;
                let Some(peer_json) = peer_json else {
                    let _: usize = connection.srem(key.as_str(), endpoint_id.as_str()).await?;
                    continue;
                };
                let peer: StoredRendezvousPeer = serde_json::from_str(peer_json.as_str())?;
                peers.push(TopicRendezvousCandidate {
                    endpoint_id: peer.endpoint_id,
                    addr_hint: peer.addr_hint,
                    relay_urls: relay_urls.to_vec(),
                });
            }
            peers.sort_by(|left, right| left.endpoint_id.cmp(&right.endpoint_id));
            topics.push(TopicRendezvousTopicResponse { topic_key, peers });
        }

        Ok(TopicRendezvousHeartbeatResponse {
            expires_in_seconds: self.ttl_seconds,
            topics,
        })
    }

    fn topic_key(&self, topic_key: &str) -> String {
        format!("{}:topic:{topic_key}", self.key_prefix)
    }

    fn peer_key(&self, endpoint_id: &str) -> String {
        format!("{}:peer:{endpoint_id}", self.key_prefix)
    }
}

fn normalize_key_prefix(value: &str) -> Result<String> {
    let trimmed = value.trim().trim_end_matches(':');
    if trimmed.is_empty() {
        bail!("topic rendezvous key prefix must not be empty");
    }
    Ok(trimmed.to_string())
}

fn normalize_topic_keys(values: Vec<String>) -> Result<Vec<String>> {
    let mut deduped = BTreeMap::new();
    for value in values {
        let normalized = normalize_topic_key(value.as_str())?;
        deduped.insert(normalized.clone(), normalized);
    }
    Ok(deduped.into_values().collect())
}

fn normalize_topic_key(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.len() != 64 || !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("topic rendezvous key must be a 64-character opaque hex value");
    }
    Ok(trimmed.to_ascii_lowercase())
}
