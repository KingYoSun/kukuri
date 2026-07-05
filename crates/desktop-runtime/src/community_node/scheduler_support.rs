use std::sync::{Arc, Weak};
use std::time::Duration;

use super::*;

impl DesktopRuntime {
    /// CN セッション維持の 1 tick。設定済みの全 node へ registration refresh(if_due)を回し、
    /// 続けて connectivity self-heal 判定を実行する。従来 `get_sync_status` の副作用として
    /// UI ポーリング経由でのみ駆動されていた内容を、ポーリング非依存で駆動する(WP-C1)。
    /// refresh は deadline ゲート済みの冪等設計のため、短い間隔で繰り返し呼んでも安全。
    pub(crate) async fn run_community_node_session_maintenance_once(&self) {
        let config = self.community_node_config.lock().await.clone();
        if config.nodes.is_empty() {
            return;
        }
        for node in config.nodes {
            if let Err(error) = self
                .refresh_community_node_registration_if_due(node.base_url.as_str())
                .await
            {
                warn!(
                    base_url = %node.base_url,
                    error = %error,
                    "failed to refresh community-node registration from session scheduler"
                );
            }
        }
        match self.app_service.get_sync_status().await {
            Ok(status) => {
                self.maybe_self_heal_community_node_connectivity(&status)
                    .await;
            }
            Err(error) => {
                warn!(
                    error = %error,
                    "failed to load sync status for community-node self-heal from session scheduler"
                );
            }
        }
    }

    /// セッション維持スケジューラを既定 tick(15 秒)で起動する。
    /// トレイ常駐中(フロントのポーリング停止時)も CN 側の bootstrap 登録(TTL 90 秒)と
    /// トークン失効前再認証を維持するための常駐タスク。起動は opt-in(コンストラクタでは
    /// 起動しない)で、production では src-tauri の状態構築時に 1 回だけ呼ぶ。
    /// 停止は `DesktopRuntime::shutdown` が行う。
    pub async fn start_community_node_session_scheduler(self: &Arc<Self>) {
        self.start_community_node_session_scheduler_with_interval(Duration::from_secs(
            COMMUNITY_NODE_SESSION_SCHEDULER_TICK_SECONDS,
        ))
        .await;
    }

    /// tick 間隔を注入できる起動口(テスト用)。既に起動済みなら no-op。
    pub(crate) async fn start_community_node_session_scheduler_with_interval(
        self: &Arc<Self>,
        tick: Duration,
    ) {
        let mut task = self.community_node_scheduler_task.lock().await;
        if task.as_ref().is_some_and(|handle| !handle.is_finished()) {
            return;
        }
        // Weak 参照で runtime の所有権を持たない: 最後の Arc が drop されたら tick 側から
        // 自然終了する(shutdown 忘れでもリークしない)。upgrade した Arc は tick 実行中のみ保持。
        let weak: Weak<Self> = Arc::downgrade(self);
        *task = Some(tokio::spawn(async move {
            loop {
                let Some(runtime) = weak.upgrade() else {
                    return;
                };
                runtime.run_community_node_session_maintenance_once().await;
                drop(runtime);
                tokio::time::sleep(tick).await;
            }
        }));
    }
}
