use std::sync::{Arc, Weak};
use std::time::Duration;

use tokio::time::MissedTickBehavior;
use tracing::{info, warn};

use super::*;

const SYNC_STATUS_OBSERVER_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Default)]
struct SyncStatusEventSnapshot {
    sync_status: Option<SyncStatus>,
    community_node_statuses: Option<Vec<CommunityNodeNodeStatus>>,
    sync_error: Option<String>,
    community_node_error: Option<String>,
}

impl SyncStatusEventSnapshot {
    fn update_sync_status(&mut self, result: Result<SyncStatus>) -> Option<SyncStatus> {
        match result {
            Ok(next) => {
                if self.sync_error.take().is_some() {
                    info!("sync status observer recovered");
                }
                if self.sync_status.as_ref() == Some(&next) {
                    return None;
                }
                self.sync_status = Some(next.clone());
                Some(next)
            }
            Err(error) => {
                let message = format!("{error:#}");
                if self.sync_error.as_deref() != Some(message.as_str()) {
                    warn!(error = %message, "sync status observer failed to load sync status");
                    self.sync_error = Some(message);
                }
                None
            }
        }
    }

    fn update_community_node_statuses(
        &mut self,
        result: Result<Vec<CommunityNodeNodeStatus>>,
    ) -> Option<Vec<CommunityNodeNodeStatus>> {
        match result {
            Ok(next) => {
                if self.community_node_error.take().is_some() {
                    info!("sync status observer recovered community-node status");
                }
                if self.community_node_statuses.as_ref() == Some(&next) {
                    return None;
                }
                self.community_node_statuses = Some(next.clone());
                Some(next)
            }
            Err(error) => {
                let message = format!("{error:#}");
                if self.community_node_error.as_deref() != Some(message.as_str()) {
                    warn!(error = %message, "sync status observer failed to load community-node status");
                    self.community_node_error = Some(message);
                }
                None
            }
        }
    }
}

impl DesktopRuntime {
    async fn observe_sync_status_once(&self, snapshot: &mut SyncStatusEventSnapshot) {
        let (sync_status, community_node_statuses) =
            tokio::join!(self.get_sync_status(), self.get_community_node_statuses());
        let sync_status = snapshot.update_sync_status(sync_status);
        let community_node_statuses =
            snapshot.update_community_node_statuses(community_node_statuses);
        if sync_status.is_some() || community_node_statuses.is_some() {
            self.emit_event(RuntimeEvent::SyncStatusChanged {
                sync_status: sync_status.map(Box::new),
                community_node_statuses,
            });
        }
    }

    pub async fn start_sync_status_observer(self: &Arc<Self>) {
        self.start_sync_status_observer_with_interval(SYNC_STATUS_OBSERVER_INTERVAL)
            .await;
    }

    pub(crate) async fn start_sync_status_observer_with_interval(self: &Arc<Self>, tick: Duration) {
        let mut task = self.sync_status_observer_task.lock().await;
        if task.as_ref().is_some_and(|handle| !handle.is_finished()) {
            return;
        }

        let weak: Weak<Self> = Arc::downgrade(self);
        *task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut snapshot = SyncStatusEventSnapshot::default();
            loop {
                interval.tick().await;
                let Some(runtime) = weak.upgrade() else {
                    return;
                };
                runtime.observe_sync_status_once(&mut snapshot).await;
                drop(runtime);
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_updates_each_status_source_independently() {
        let mut snapshot = SyncStatusEventSnapshot::default();

        assert!(
            snapshot
                .update_sync_status(Err(anyhow!("sync unavailable")))
                .is_none()
        );
        assert_eq!(
            snapshot.update_community_node_statuses(Ok(Vec::new())),
            Some(Vec::new())
        );
        assert!(
            snapshot
                .update_community_node_statuses(Ok(Vec::new()))
                .is_none(),
            "an unchanged successful source must be suppressed"
        );
    }
}
