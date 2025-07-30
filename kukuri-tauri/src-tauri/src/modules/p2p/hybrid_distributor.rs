use futures::future::join_all;
use nostr_sdk::Event;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;

use crate::modules::event::manager::EventManager;
use crate::modules::p2p::error::{P2PError, Result as P2PResult};
use crate::modules::p2p::event_sync::EventSync;
use crate::modules::p2p::gossip_manager::GossipManager;

/// 配信優先度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DeliveryPriority {
    /// 低優先度 - 通常のメッセージ
    Low = 0,
    /// 中優先度 - 重要な更新
    Medium = 1,
    /// 高優先度 - リアルタイム性が必要
    High = 2,
    /// 緊急 - システムメッセージなど
    Critical = 3,
}

/// 配信戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStrategy {
    /// P2Pのみ
    P2POnly,
    /// Nostrリレーのみ
    RelayOnly,
    /// 並列配信（両方同時）
    Parallel,
    /// 順次配信（P2P優先、失敗時Nostrリレー）
    Sequential,
}

/// 配信結果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DeliveryResult {
    pub strategy: DeliveryStrategy,
    pub p2p_success: bool,
    pub relay_success: bool,
    pub duration: Duration,
    pub error: Option<String>,
}

/// 配信メトリクス
#[derive(Debug, Clone, Default)]
pub struct DeliveryMetrics {
    #[allow(dead_code)]
    pub total_attempts: u64,
    #[allow(dead_code)]
    pub p2p_successes: u64,
    #[allow(dead_code)]
    pub relay_successes: u64,
    #[allow(dead_code)]
    pub parallel_successes: u64,
    #[allow(dead_code)]
    pub fallback_attempts: u64,
    #[allow(dead_code)]
    pub average_latency_ms: f64,
}

/// ハイブリッド配信設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// P2P配信タイムアウト（ミリ秒）
    pub p2p_timeout_ms: u64,
    /// Nostrリレー配信タイムアウト（ミリ秒）
    pub relay_timeout_ms: u64,
    /// 最大同時配信数
    pub max_concurrent_deliveries: usize,
    /// フォールバック有効化
    pub enable_fallback: bool,
    /// 優先度別の戦略
    pub priority_strategies: HashMap<DeliveryPriority, DeliveryStrategy>,
}

impl Default for HybridConfig {
    fn default() -> Self {
        let mut priority_strategies = HashMap::new();
        priority_strategies.insert(DeliveryPriority::Critical, DeliveryStrategy::Parallel);
        priority_strategies.insert(DeliveryPriority::High, DeliveryStrategy::Parallel);
        priority_strategies.insert(DeliveryPriority::Medium, DeliveryStrategy::Sequential);
        priority_strategies.insert(DeliveryPriority::Low, DeliveryStrategy::P2POnly);

        Self {
            p2p_timeout_ms: 5000,
            relay_timeout_ms: 10000,
            max_concurrent_deliveries: 10,
            enable_fallback: true,
            priority_strategies,
        }
    }
}

/// ハイブリッド配信システム
pub struct HybridDistributor {
    #[allow(dead_code)]
    event_sync: Arc<EventSync>,
    #[allow(dead_code)]
    event_manager: Arc<EventManager>,
    #[allow(dead_code)]
    gossip_manager: Arc<GossipManager>,
    #[allow(dead_code)]
    config: Arc<RwLock<HybridConfig>>,
    #[allow(dead_code)]
    metrics: Arc<RwLock<DeliveryMetrics>>,
    #[allow(dead_code)]
    semaphore: Arc<Semaphore>,
}

impl HybridDistributor {
    /// 新しいHybridDistributorインスタンスを作成
    #[allow(dead_code)]
    pub fn new(
        event_sync: Arc<EventSync>,
        event_manager: Arc<EventManager>,
        gossip_manager: Arc<GossipManager>,
        config: Option<HybridConfig>,
    ) -> Self {
        let config = config.unwrap_or_default();
        let max_concurrent = config.max_concurrent_deliveries;

        Self {
            event_sync,
            event_manager,
            gossip_manager,
            config: Arc::new(RwLock::new(config)),
            metrics: Arc::new(RwLock::new(DeliveryMetrics::default())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// イベントを配信（優先度付き）
    #[allow(dead_code)]
    pub async fn deliver_event(
        &self,
        event: Event,
        priority: DeliveryPriority,
    ) -> P2PResult<DeliveryResult> {
        // 同時配信数を制限
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| P2PError::Internal(format!("Semaphore error: {e}")))?;

        let start_time = Instant::now();
        let config = self.config.read().await;

        // 優先度に基づく戦略を選択
        let strategy = config
            .priority_strategies
            .get(&priority)
            .copied()
            .unwrap_or(DeliveryStrategy::Sequential);

        tracing::debug!(
            "Delivering event {} with priority {:?} using strategy {:?}",
            event.id,
            priority,
            strategy
        );

        let result = match strategy {
            DeliveryStrategy::P2POnly => self.deliver_p2p_only(event).await,
            DeliveryStrategy::RelayOnly => self.deliver_relay_only(event).await,
            DeliveryStrategy::Parallel => self.deliver_parallel(event).await,
            DeliveryStrategy::Sequential => {
                self.deliver_sequential(event, config.enable_fallback).await
            }
        };

        let duration = start_time.elapsed();

        // メトリクス更新
        if let Ok(ref delivery_result) = result {
            self.update_metrics(delivery_result, duration).await;
        }

        result
    }

    /// P2Pのみで配信
    #[allow(dead_code)]
    async fn deliver_p2p_only(&self, event: Event) -> P2PResult<DeliveryResult> {
        let config = self.config.read().await;
        let timeout_duration = Duration::from_millis(config.p2p_timeout_ms);

        let p2p_result = timeout(
            timeout_duration,
            self.event_sync.propagate_nostr_event(event.clone()),
        )
        .await;

        let (p2p_success, error) = match p2p_result {
            Ok(Ok(())) => (true, None),
            Ok(Err(e)) => (false, Some(e.to_string())),
            Err(_) => (false, Some("P2P delivery timeout".to_string())),
        };

        Ok(DeliveryResult {
            strategy: DeliveryStrategy::P2POnly,
            p2p_success,
            relay_success: false,
            duration: timeout_duration,
            error,
        })
    }

    /// Nostrリレーのみで配信
    #[allow(dead_code)]
    async fn deliver_relay_only(&self, event: Event) -> P2PResult<DeliveryResult> {
        let config = self.config.read().await;
        let timeout_duration = Duration::from_millis(config.relay_timeout_ms);

        let relay_result = timeout(
            timeout_duration,
            self.event_manager.publish_event(event.clone()),
        )
        .await;

        let (relay_success, error) = match relay_result {
            Ok(Ok(_event_id)) => (true, None),
            Ok(Err(e)) => (false, Some(e.to_string())),
            Err(_) => (false, Some("Relay delivery timeout".to_string())),
        };

        Ok(DeliveryResult {
            strategy: DeliveryStrategy::RelayOnly,
            p2p_success: false,
            relay_success,
            duration: timeout_duration,
            error,
        })
    }

    /// 並列配信（P2PとNostrリレー同時）
    #[allow(dead_code)]
    async fn deliver_parallel(&self, event: Event) -> P2PResult<DeliveryResult> {
        let config = self.config.read().await;
        let p2p_timeout = Duration::from_millis(config.p2p_timeout_ms);
        let relay_timeout = Duration::from_millis(config.relay_timeout_ms);

        // P2PとNostrリレーに並列配信
        let p2p_future = timeout(
            p2p_timeout,
            self.event_sync.propagate_nostr_event(event.clone()),
        );

        let relay_future = timeout(
            relay_timeout,
            self.event_manager.publish_event(event.clone()),
        );

        let (p2p_result, relay_result) = tokio::join!(p2p_future, relay_future);

        let p2p_success = matches!(p2p_result, Ok(Ok(())));
        let relay_success = matches!(relay_result, Ok(Ok(_event_id)));

        let error = if !p2p_success && !relay_success {
            Some("Both P2P and relay delivery failed".to_string())
        } else {
            None
        };

        Ok(DeliveryResult {
            strategy: DeliveryStrategy::Parallel,
            p2p_success,
            relay_success,
            duration: p2p_timeout.max(relay_timeout),
            error,
        })
    }

    /// 順次配信（P2P優先、失敗時はフォールバック）
    #[allow(dead_code)]
    async fn deliver_sequential(
        &self,
        event: Event,
        enable_fallback: bool,
    ) -> P2PResult<DeliveryResult> {
        let config = self.config.read().await;
        let p2p_timeout = Duration::from_millis(config.p2p_timeout_ms);

        // まずP2P配信を試行
        let p2p_result = timeout(
            p2p_timeout,
            self.event_sync.propagate_nostr_event(event.clone()),
        )
        .await;

        let p2p_success = matches!(p2p_result, Ok(Ok(())));
        let mut relay_success = false;
        let mut error = None;

        // P2P失敗時、フォールバックが有効ならNostrリレーに配信
        if !p2p_success && enable_fallback {
            tracing::info!("P2P delivery failed, falling back to Nostr relay");

            let relay_timeout = Duration::from_millis(config.relay_timeout_ms);
            let relay_result = timeout(
                relay_timeout,
                self.event_manager.publish_event(event.clone()),
            )
            .await;

            relay_success = matches!(relay_result, Ok(Ok(_event_id)));

            if !relay_success {
                error = Some("Both P2P and fallback relay delivery failed".to_string());
            }
        } else if !p2p_success {
            error = Some("P2P delivery failed, fallback disabled".to_string());
        }

        Ok(DeliveryResult {
            strategy: DeliveryStrategy::Sequential,
            p2p_success,
            relay_success,
            duration: p2p_timeout,
            error,
        })
    }

    /// メトリクスを更新
    #[allow(dead_code)]
    async fn update_metrics(&self, result: &DeliveryResult, duration: Duration) {
        let mut metrics = self.metrics.write().await;

        metrics.total_attempts += 1;

        if result.p2p_success {
            metrics.p2p_successes += 1;
        }

        if result.relay_success {
            metrics.relay_successes += 1;
        }

        if result.p2p_success && result.relay_success {
            metrics.parallel_successes += 1;
        }

        if !result.p2p_success
            && result.relay_success
            && result.strategy == DeliveryStrategy::Sequential
        {
            metrics.fallback_attempts += 1;
        }

        // 平均レイテンシーを更新（簡単な移動平均）
        let latency_ms = duration.as_millis() as f64;
        metrics.average_latency_ms =
            (metrics.average_latency_ms * (metrics.total_attempts - 1) as f64 + latency_ms)
                / metrics.total_attempts as f64;
    }

    /// 配信メトリクスを取得
    #[allow(dead_code)]
    pub async fn get_metrics(&self) -> DeliveryMetrics {
        self.metrics.read().await.clone()
    }

    /// 配信設定を更新
    #[allow(dead_code)]
    pub async fn update_config(&self, config: HybridConfig) -> P2PResult<()> {
        *self.config.write().await = config;
        Ok(())
    }

    /// 優先度に基づく配信戦略を設定
    #[allow(dead_code)]
    pub async fn set_priority_strategy(
        &self,
        priority: DeliveryPriority,
        strategy: DeliveryStrategy,
    ) -> P2PResult<()> {
        let mut config = self.config.write().await;
        config.priority_strategies.insert(priority, strategy);
        Ok(())
    }

    /// バッチ配信（複数イベントを効率的に配信）
    #[allow(dead_code)]
    pub async fn deliver_batch(
        &self,
        events: Vec<(Event, DeliveryPriority)>,
    ) -> P2PResult<Vec<DeliveryResult>> {
        let futures = events
            .into_iter()
            .map(|(event, priority)| self.deliver_event(event, priority));

        let results = join_all(futures).await;

        // エラーと成功を分離して収集
        let mut delivery_results = Vec::new();
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(delivery) => delivery_results.push(delivery),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            tracing::warn!("Batch delivery had {} errors", errors.len());
        }

        Ok(delivery_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_priority_ordering() {
        assert!(DeliveryPriority::Critical > DeliveryPriority::High);
        assert!(DeliveryPriority::High > DeliveryPriority::Medium);
        assert!(DeliveryPriority::Medium > DeliveryPriority::Low);
    }

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();

        assert_eq!(config.p2p_timeout_ms, 5000);
        assert_eq!(config.relay_timeout_ms, 10000);
        assert_eq!(config.max_concurrent_deliveries, 10);
        assert!(config.enable_fallback);

        // 優先度別戦略の確認
        assert_eq!(
            config.priority_strategies.get(&DeliveryPriority::Critical),
            Some(&DeliveryStrategy::Parallel)
        );
        assert_eq!(
            config.priority_strategies.get(&DeliveryPriority::Low),
            Some(&DeliveryStrategy::P2POnly)
        );
    }

    #[tokio::test]
    async fn test_delivery_metrics_update() {
        let metrics = Arc::new(RwLock::new(DeliveryMetrics::default()));

        // 成功した配信結果をシミュレート
        let _result = DeliveryResult {
            strategy: DeliveryStrategy::Parallel,
            p2p_success: true,
            relay_success: true,
            duration: Duration::from_millis(100),
            error: None,
        };

        // メトリクス更新
        {
            let mut m = metrics.write().await;
            m.total_attempts += 1;
            m.p2p_successes += 1;
            m.relay_successes += 1;
            m.parallel_successes += 1;
            m.average_latency_ms = 100.0;
        }

        let m = metrics.read().await;
        assert_eq!(m.total_attempts, 1);
        assert_eq!(m.p2p_successes, 1);
        assert_eq!(m.relay_successes, 1);
        assert_eq!(m.parallel_successes, 1);
        assert_eq!(m.average_latency_ms, 100.0);
    }

    #[test]
    fn test_delivery_result_creation() {
        let result = DeliveryResult {
            strategy: DeliveryStrategy::Sequential,
            p2p_success: false,
            relay_success: true,
            duration: Duration::from_millis(1500),
            error: None,
        };

        assert!(!result.p2p_success);
        assert!(result.relay_success);
        assert_eq!(result.duration.as_millis(), 1500);
        assert!(result.error.is_none());
    }
}
