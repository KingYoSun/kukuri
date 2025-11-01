use serde::{Deserialize, Serialize};

use crate::infrastructure::p2p::metrics::GossipMetricsSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMetricsSummary {
    pub joins: u64,
    pub leaves: u64,
    pub broadcasts_sent: u64,
    pub messages_received: u64,
}

impl GossipMetricsSummary {
    pub fn from_snapshot(snapshot: &GossipMetricsSnapshot) -> Self {
        Self {
            joins: snapshot.joins,
            leaves: snapshot.leaves,
            broadcasts_sent: snapshot.broadcasts_sent,
            messages_received: snapshot.messages_received,
        }
    }
}
