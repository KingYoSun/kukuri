use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex, Weak};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Default)]
pub(crate) struct SessionLocks(StdMutex<HashMap<String, Weak<Mutex<()>>>>);

impl SessionLocks {
    pub(crate) async fn lock(&self, base_url: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.0.lock().expect("session locks poisoned");
            locks.retain(|_, lock| lock.strong_count() > 0);
            match locks.get(base_url).and_then(Weak::upgrade) {
                Some(lock) => lock,
                None => {
                    let lock = Arc::new(Mutex::new(()));
                    locks.insert(base_url.to_owned(), Arc::downgrade(&lock));
                    lock
                }
            }
        };
        lock.lock_owned().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;

    #[tokio::test]
    async fn stalled_node_does_not_hold_another_nodes_session_lock() {
        let locks = SessionLocks::default();
        let first = locks.lock("https://a.invalid").await;
        assert!(locks.lock("https://a.invalid").now_or_never().is_none());
        let other = locks.lock("https://b.invalid").now_or_never();
        assert!(other.is_some(), "node A must not block node B");
        drop(first);
        assert!(locks.lock("https://a.invalid").now_or_never().is_some());
    }
}
