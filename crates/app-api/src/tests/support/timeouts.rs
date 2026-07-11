use super::super::*;

pub(crate) fn social_graph_propagation_timeout() -> Duration {
    kukuri_test_support::constrained_timeout(Duration::from_secs(10), Duration::from_secs(300))
}

pub(crate) fn p2p_replication_timeout() -> Duration {
    kukuri_test_support::constrained_timeout(Duration::from_secs(10), Duration::from_secs(60))
}

pub(crate) fn seeded_dht_publish_resolve_timeout() -> Duration {
    kukuri_test_support::constrained_timeout(Duration::from_secs(5), Duration::from_secs(15))
}

pub(crate) fn iroh_integration_test_lock() -> Arc<TokioMutex<()>> {
    static LOCK: OnceLock<Arc<TokioMutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(TokioMutex::new(()))).clone()
}
