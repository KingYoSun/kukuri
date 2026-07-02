use super::super::*;

pub(crate) fn social_graph_propagation_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(10)
    }
}

pub(crate) fn p2p_replication_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(10)
    }
}

pub(crate) fn seeded_dht_publish_resolve_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(5)
    }
}

pub(crate) fn iroh_integration_test_lock() -> Arc<TokioMutex<()>> {
    static LOCK: OnceLock<Arc<TokioMutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(TokioMutex::new(()))).clone()
}
