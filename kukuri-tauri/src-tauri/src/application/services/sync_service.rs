use crate::infrastructure::p2p::NetworkService;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    post_service: Arc<super::PostService>,
    event_service: Arc<super::EventService>,
    status: Arc<RwLock<SyncStatus>>,
}

impl SyncService {
    pub fn new(
        network: Arc<dyn NetworkService>,
        post_service: Arc<super::PostService>,
        event_service: Arc<super::EventService>,
    ) -> Self {
        Self {
            network,
            post_service,
            event_service,
            status: Arc::new(RwLock::new(SyncStatus {
                is_syncing: false,
                pending_posts: 0,
                pending_events: 0,
                last_sync: None,
                sync_errors: 0,
            })),
        }
    }

    pub async fn start_sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut status = self.status.write().await;

        if status.is_syncing {
            return Ok(());
        }

        status.is_syncing = true;
        drop(status);

        // Ensure network is connected
        if !self.network.is_connected().await {
            self.network.connect().await?;
        }

        // Sync pending posts
        let synced_posts = self.post_service.sync_pending_posts().await?;

        // Sync pending events
        let synced_events = self.event_service.sync_pending_events().await?;

        // Update status
        let mut status = self.status.write().await;
        status.is_syncing = false;
        status.last_sync = Some(chrono::Utc::now().timestamp());
        status.pending_posts = status.pending_posts.saturating_sub(synced_posts);
        status.pending_events = status.pending_events.saturating_sub(synced_events);

        Ok(())
    }

    pub async fn stop_sync(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut status = self.status.write().await;
        status.is_syncing = false;
        Ok(())
    }

    pub async fn get_status(&self) -> SyncStatus {
        self.status.read().await.clone()
    }

    pub async fn reset_sync(&self) -> Result<(), Box<dyn std::error::Error>> {
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
            post_service: self.post_service.clone(),
            event_service: self.event_service.clone(),
            status: self.status.clone(),
        }
    }
}
