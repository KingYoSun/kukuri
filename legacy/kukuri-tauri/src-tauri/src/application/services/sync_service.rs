use crate::infrastructure::p2p::NetworkService;
use crate::shared::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait SyncParticipant: Send + Sync {
    async fn sync_pending(&self) -> Result<u32, AppError>;
}

#[async_trait]
pub trait SyncServiceTrait: Send + Sync {
    async fn start_sync(&self) -> Result<(), AppError>;
    async fn stop_sync(&self) -> Result<(), AppError>;
    async fn get_status(&self) -> SyncStatus;
    async fn reset_sync(&self) -> Result<(), AppError>;
    async fn schedule_sync(&self, interval_secs: u64);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncStatus {
    pub is_syncing: bool,
    pub pending_posts: u32,
    pub pending_events: u32,
    pub last_sync: Option<i64>,
    pub sync_errors: u32,
}

pub struct SyncService {
    network: Arc<dyn NetworkService>,
    post_participant: Arc<dyn SyncParticipant>,
    event_participant: Arc<dyn SyncParticipant>,
    status: Arc<RwLock<SyncStatus>>,
}

impl SyncService {
    pub fn new(
        network: Arc<dyn NetworkService>,
        post_participant: Arc<dyn SyncParticipant>,
        event_participant: Arc<dyn SyncParticipant>,
    ) -> Self {
        Self {
            network,
            post_participant,
            event_participant,
            status: Arc::new(RwLock::new(SyncStatus {
                is_syncing: false,
                pending_posts: 0,
                pending_events: 0,
                last_sync: None,
                sync_errors: 0,
            })),
        }
    }

    pub async fn start_sync(&self) -> Result<(), AppError> {
        {
            let mut status = self.status.write().await;

            if status.is_syncing {
                return Ok(());
            }

            status.is_syncing = true;
        }

        let result = async {
            if !self.network.is_connected().await {
                self.network.connect().await?;
            }

            let synced_posts = self.post_participant.sync_pending().await?;
            let synced_events = self.event_participant.sync_pending().await?;
            Ok((synced_posts, synced_events))
        }
        .await;

        let mut status = self.status.write().await;
        status.is_syncing = false;

        match result {
            Ok((synced_posts, synced_events)) => {
                status.last_sync = Some(chrono::Utc::now().timestamp());
                status.pending_posts = status.pending_posts.saturating_sub(synced_posts);
                status.pending_events = status.pending_events.saturating_sub(synced_events);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub async fn stop_sync(&self) -> Result<(), AppError> {
        let mut status = self.status.write().await;
        status.is_syncing = false;
        Ok(())
    }

    pub async fn get_status(&self) -> SyncStatus {
        self.status.read().await.clone()
    }

    pub async fn reset_sync(&self) -> Result<(), AppError> {
        let mut status = self.status.write().await;
        status.pending_posts = 0;
        status.pending_events = 0;
        status.sync_errors = 0;
        Ok(())
    }

    pub async fn schedule_sync(&self, interval_secs: u64) {
        let service = Arc::new(self.clone());
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                if let Err(e) = service.start_sync().await {
                    tracing::error!("Sync error: {}", e);
                    let mut status = service.status.write().await;
                    status.sync_errors += 1;
                }
            }
        });
    }
}

impl Clone for SyncService {
    fn clone(&self) -> Self {
        Self {
            network: self.network.clone(),
            post_participant: self.post_participant.clone(),
            event_participant: self.event_participant.clone(),
            status: self.status.clone(),
        }
    }
}

#[async_trait]
impl SyncServiceTrait for SyncService {
    async fn start_sync(&self) -> Result<(), AppError> {
        SyncService::start_sync(self).await
    }

    async fn stop_sync(&self) -> Result<(), AppError> {
        SyncService::stop_sync(self).await
    }

    async fn get_status(&self) -> SyncStatus {
        SyncService::get_status(self).await
    }

    async fn reset_sync(&self) -> Result<(), AppError> {
        SyncService::reset_sync(self).await
    }

    async fn schedule_sync(&self, interval_secs: u64) {
        SyncService::schedule_sync(self, interval_secs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::p2p::{NetworkStats, Peer};
    use crate::shared::config::BootstrapSource;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Mutex;

    struct StubNetworkService {
        connected: bool,
        connect_calls: AtomicUsize,
    }

    impl StubNetworkService {
        fn new(connected: bool) -> Self {
            Self {
                connected,
                connect_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl NetworkService for StubNetworkService {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        async fn connect(&self) -> Result<(), AppError> {
            self.connect_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn disconnect(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn get_peers(&self) -> Result<Vec<Peer>, AppError> {
            Ok(Vec::new())
        }

        async fn add_peer(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }

        async fn remove_peer(&self, _: &str) -> Result<(), AppError> {
            Ok(())
        }

        async fn get_stats(&self) -> Result<NetworkStats, AppError> {
            Ok(NetworkStats {
                connected_peers: 0,
                total_messages_sent: 0,
                total_messages_received: 0,
                bandwidth_up: 0,
                bandwidth_down: 0,
            })
        }

        async fn is_connected(&self) -> bool {
            self.connected
        }

        async fn get_node_id(&self) -> Result<String, AppError> {
            Ok("node".to_string())
        }

        async fn get_addresses(&self) -> Result<Vec<String>, AppError> {
            Ok(Vec::new())
        }

        async fn apply_bootstrap_nodes(
            &self,
            _: Vec<String>,
            _: BootstrapSource,
        ) -> Result<(), AppError> {
            Ok(())
        }
    }

    struct SequenceParticipant {
        results: Mutex<Vec<Result<u32, AppError>>>,
        calls: AtomicUsize,
    }

    impl SequenceParticipant {
        fn new(results: Vec<Result<u32, AppError>>) -> Self {
            Self {
                results: Mutex::new(results),
                calls: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl SyncParticipant for SequenceParticipant {
        async fn sync_pending(&self) -> Result<u32, AppError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.results.lock().await.remove(0)
        }
    }

    #[tokio::test]
    async fn start_sync_resets_is_syncing_after_failure() {
        let network = Arc::new(StubNetworkService::new(true));
        let post_participant = Arc::new(SequenceParticipant::new(vec![
            Err(AppError::Internal("boom".to_string())),
            Ok(1),
        ]));
        let event_participant = Arc::new(SequenceParticipant::new(vec![Ok(0)]));
        let service = SyncService::new(network, post_participant.clone(), event_participant);

        let first = service.start_sync().await;
        assert!(first.is_err());
        assert!(!service.get_status().await.is_syncing);

        service
            .start_sync()
            .await
            .expect("second sync should retry");
        let status = service.get_status().await;
        assert!(!status.is_syncing);
        assert!(status.last_sync.is_some());
        assert_eq!(post_participant.call_count(), 2);
    }
}
