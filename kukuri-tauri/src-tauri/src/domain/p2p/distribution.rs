use serde::{Deserialize, Serialize};

/// イベント配信時の戦略を表すドメイン値。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DistributionStrategy {
    Broadcast,
    Gossip,
    Direct(String),
    Hybrid,
    Nostr,
    P2P,
}

/// 配信時のメトリクス記録を抽象化するトレイト。
pub trait DistributionMetrics: Send + Sync {
    fn record_attempt(&self, _strategy: &DistributionStrategy) {}
    fn record_success(&self, _strategy: &DistributionStrategy) {}
    fn record_failure(&self, _strategy: &DistributionStrategy) {}
}
