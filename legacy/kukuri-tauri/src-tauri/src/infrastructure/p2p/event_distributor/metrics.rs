use crate::domain::p2p::distribution::{DistributionMetrics, DistributionStrategy};
use crate::infrastructure::p2p::metrics;

pub struct P2PDistributionMetrics;

impl DistributionMetrics for P2PDistributionMetrics {
    fn record_success(&self, strategy: &DistributionStrategy) {
        match strategy {
            DistributionStrategy::Nostr => metrics::record_broadcast_success(),
            DistributionStrategy::P2P
            | DistributionStrategy::Broadcast
            | DistributionStrategy::Gossip
            | DistributionStrategy::Direct(_)
            | DistributionStrategy::Hybrid => metrics::record_broadcast_success(),
        }
    }

    fn record_failure(&self, strategy: &DistributionStrategy) {
        match strategy {
            DistributionStrategy::Nostr => metrics::record_broadcast_failure(),
            DistributionStrategy::P2P
            | DistributionStrategy::Broadcast
            | DistributionStrategy::Gossip
            | DistributionStrategy::Direct(_)
            | DistributionStrategy::Hybrid => metrics::record_broadcast_failure(),
        }
    }
}
