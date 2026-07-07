//! topic rendezvous の wire 型(cn endpoint contract の一部)。
//! 保存側(redis store)は cn-core に残り、ここは request / response の形のみ。

use serde::{Deserialize, Serialize};

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
