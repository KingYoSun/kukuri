use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use tokio::time::{sleep, timeout};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PollState<T> {
    Pending,
    Ready(T),
}

#[derive(Debug, Error)]
pub enum PollError<E> {
    #[error("poll timed out")]
    Timeout,
    #[error("poll operation failed: {0}")]
    Operation(E),
}

pub async fn poll_until<F, Fut, T, E>(
    deadline: Duration,
    interval: Duration,
    stable_ready_polls: usize,
    mut operation: F,
) -> Result<T, PollError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<PollState<T>, E>>,
{
    let required = stable_ready_polls.max(1);
    timeout(deadline, async {
        let mut ready_polls = 0usize;
        loop {
            match operation().await.map_err(PollError::Operation)? {
                PollState::Ready(value) => {
                    ready_polls += 1;
                    if ready_polls >= required {
                        return Ok(value);
                    }
                }
                PollState::Pending => ready_polls = 0,
            }
            sleep(interval).await;
        }
    })
    .await
    .map_err(|_| PollError::Timeout)?
}

pub fn constrained_timeout(local: Duration, constrained: Duration) -> Duration {
    if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        constrained
    } else {
        local
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TopicSyncSnapshot {
    pub topic: String,
    pub peer_count: usize,
    pub connected_peers: Vec<String>,
    pub docs_assist_peer_ids: Vec<String>,
    pub configured_peer_ids: Vec<String>,
    pub missing_peer_ids: Option<Vec<String>>,
    pub delivery_state: String,
    pub status_detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncSnapshot {
    pub connected: bool,
    pub peer_count: usize,
    pub status_detail: String,
    pub last_error: Option<String>,
    pub discovery_connected_peers: Vec<String>,
    pub topics: Vec<TopicSyncSnapshot>,
}

#[async_trait]
pub trait SyncStatusSource {
    type Error: Debug + Send;

    async fn sync_snapshot(&self) -> Result<SyncSnapshot, Self::Error>;
}

pub fn format_sync_snapshot(snapshot: &SyncSnapshot, topic: &str) -> String {
    let topic_status = snapshot
        .topics
        .iter()
        .find(|entry| entry.topic == topic)
        .map(|entry| {
            let missing = entry
                .missing_peer_ids
                .as_ref()
                .map(|ids| format!(", missing_peer_ids={ids:?}"))
                .unwrap_or_default();
            format!(
                "topic_peers={}, connected_peers={:?}, docs_assist_peer_ids={:?}, configured_peer_ids={:?}{missing}, delivery_state={}, status_detail={}",
                entry.peer_count,
                entry.connected_peers,
                entry.docs_assist_peer_ids,
                entry.configured_peer_ids,
                entry.delivery_state,
                entry.status_detail
            )
        })
        .unwrap_or_else(|| "topic_status=missing".to_string());
    format!(
        "connected={}, peer_count={}, status_detail={}, last_error={:?}, discovery_connected_peers={:?}, {}",
        snapshot.connected,
        snapshot.peer_count,
        snapshot.status_detail,
        snapshot.last_error,
        snapshot.discovery_connected_peers,
        topic_status
    )
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;

    #[tokio::test]
    async fn poll_requires_consecutive_ready_results() {
        let states = [true, true, false, true, true, true];
        let mut index = 0usize;
        let value = poll_until(Duration::from_secs(1), Duration::ZERO, 3, || {
            let ready = states[index];
            index += 1;
            async move {
                Ok::<_, Infallible>(if ready {
                    PollState::Ready(index)
                } else {
                    PollState::Pending
                })
            }
        })
        .await
        .expect("poll result");
        assert_eq!(value, 6);
    }

    #[tokio::test]
    async fn poll_reports_timeout() {
        let result = poll_until(
            Duration::from_millis(5),
            Duration::from_millis(1),
            1,
            || async { Ok::<_, Infallible>(PollState::<()>::Pending) },
        )
        .await;
        assert!(matches!(result, Err(PollError::Timeout)));
    }

    #[test]
    fn snapshot_format_keeps_diagnostics() {
        let snapshot = SyncSnapshot {
            connected: true,
            peer_count: 1,
            status_detail: "ready".into(),
            topics: vec![TopicSyncSnapshot {
                topic: "kukuri:test".into(),
                peer_count: 1,
                delivery_state: "Live".into(),
                status_detail: "joined".into(),
                ..TopicSyncSnapshot::default()
            }],
            ..SyncSnapshot::default()
        };
        assert!(format_sync_snapshot(&snapshot, "kukuri:test").contains("delivery_state=Live"));
    }
}
