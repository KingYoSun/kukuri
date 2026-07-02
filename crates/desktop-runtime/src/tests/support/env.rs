use super::super::*;

pub(crate) fn social_graph_propagation_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(30)
    }
}

pub(crate) fn seeded_dht_runtime_ready_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(120)
    } else {
        Duration::from_secs(20)
    }
}

pub(crate) fn runtime_replication_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(30)
    }
}

pub(crate) fn runtime_shutdown_timeout() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(15)
    }
}

pub(crate) async fn acquire_async_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().await
}
pub(crate) fn public_connectivity_reapply_interval() -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        Duration::from_secs(20)
    } else {
        Duration::from_secs(10)
    }
}
pub(crate) fn delete_sqlite_artifacts(db_path: &Path) {
    for path in [
        db_path.to_path_buf(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
    ] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("delete sqlite artifact {}: {error}", path.display()),
        }
    }
}
