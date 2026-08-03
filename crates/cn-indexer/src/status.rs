//! 観測状態の HTTP 公開（#613 T3）。
//!
//! `COMMUNITY_NODE_INDEXER_STATUS_ADDR` が設定されたときだけ待ち受ける最小のエンドポイント。
//! 起動完了判定（#612）や運用監視が機械的に読む:
//! - `GET /healthz` — プロセス生存確認（常に `ok`）。
//! - `GET /v1/status` — [`IndexerStateSnapshot`] の JSON。
//!
//! 運用者がローカルホストや内部ネットワークに割り当てる前提で認証は持たない。

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{Json, Router, extract::State, routing::get};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::state::{IndexerRuntimeState, IndexerStateSnapshot};

/// 状態エンドポイントの停止用の持ち手。
pub struct StatusServerHandle {
    local_addr: SocketAddr,
    stop_tx: watch::Sender<bool>,
    join: JoinHandle<()>,
}

impl StatusServerHandle {
    /// 実際に待ち受けているアドレス（ポート 0 指定時の解決結果を含む）。
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// サーバを止め、終了を待つ。
    pub async fn shutdown(self) {
        let _ = self.stop_tx.send(true);
        if let Err(error) = tokio::time::timeout(std::time::Duration::from_secs(5), self.join).await
        {
            warn!(%error, "status server did not stop within 5s");
        }
    }
}

async fn healthz() -> &'static str {
    "ok"
}

async fn status(State(state): State<Arc<IndexerRuntimeState>>) -> Json<IndexerStateSnapshot> {
    Json(state.snapshot())
}

/// 状態エンドポイントを起動する。bind できなければ起動失敗（fail-closed）。
pub async fn spawn_status_server(
    addr: SocketAddr,
    state: Arc<IndexerRuntimeState>,
) -> Result<StatusServerHandle> {
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/status", get(status))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind the indexer status endpoint at {addr}"))?;
    let local_addr = listener
        .local_addr()
        .context("failed to resolve the indexer status endpoint address")?;
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let join = tokio::spawn(async move {
        let shutdown = async move {
            // 送信側 drop でも確実に止まるよう、変更通知エラーは停止として扱う。
            while stop_rx.changed().await.is_ok() {
                if *stop_rx.borrow() {
                    break;
                }
            }
        };
        if let Err(error) = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await
        {
            warn!(%error, "indexer status server exited with an error");
        }
    });
    info!(addr = %local_addr, "indexer status endpoint listening");
    Ok(StatusServerHandle {
        local_addr,
        stop_tx,
        join,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn status_endpoint_serves_snapshot_and_healthz() {
        let state = Arc::new(IndexerRuntimeState::default());
        state.set_ingest_enabled(true);
        let server = spawn_status_server("127.0.0.1:0".parse().expect("addr"), state.clone())
            .await
            .expect("spawn status server");
        let base = format!("http://{}", server.local_addr());

        let health = reqwest::get(format!("{base}/healthz"))
            .await
            .expect("healthz request");
        assert!(health.status().is_success());
        assert_eq!(health.text().await.expect("healthz body"), "ok");

        let status = reqwest::get(format!("{base}/v1/status"))
            .await
            .expect("status request");
        assert!(status.status().is_success());
        let snapshot: serde_json::Value = status.json().await.expect("status json");
        assert_eq!(snapshot["ingest_enabled"], serde_json::json!(true));
        assert_eq!(snapshot["worker_running"], serde_json::json!(false));

        server.shutdown().await;
    }
}
