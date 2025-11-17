# Nightly テストID / Artefact マッピング
最終更新日: 2025年11月17日

Nightly ワークフローで収集している導線テストは、ID ごとに artefact 名称やログ配置がばらつき始めていた。  
本ドキュメントでは `nightly.profile-avatar-sync` などのテスト ID から GitHub Actions ジョブ・実行コマンド・artefact 名・ログ／JSON の所在を一気に辿れるよう整理する。Runbook 詳細は既存ドキュメント（`docs/03_implementation/p2p_mainline_runbook.md` など）に委譲し、ここでは参照パスと目的を紐付ける。

## クイックリファレンス
| テストID | GitHub Actions ジョブ | Docker/Vitest エントリ | Artefact 名 |
| --- | --- | --- | --- |
| `nightly.desktop-e2e` | `.github/workflows/nightly.yml` `desktop-e2e` | `./scripts/test-docker.sh e2e` | `nightly.desktop-e2e-logs`, `nightly.desktop-e2e-reports` |
| `nightly.trending-feed` | `trending-feed` | `./scripts/test-docker.sh ts --scenario trending-feed` | `nightly.trending-feed-logs`, `nightly.trending-feed-reports`, `nightly.trending-feed-metrics` |
| `nightly.profile-avatar-sync` | `profile-avatar-sync` | `./scripts/test-docker.sh ts --scenario profile-avatar-sync --service-worker` | `profile-avatar-sync-logs` |
| `nightly.sync-status-indicator` | `sync-status-indicator` | `./scripts/test-docker.sh ts --scenario offline-sync` | `sync-status-indicator-logs`（JSON: `test-results/offline-sync/`） |
| `nightly.user-search-pagination` | `user-search-pagination` | `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build` | `user-search-pagination-logs`, `user-search-pagination-log-archive`, `user-search-pagination-reports` |
| `nightly.topic-create` | `topic-create` | `./scripts/test-docker.sh ts --scenario topic-create` | `topic-create-logs`, `topic-create-reports` |
| `nightly.post-delete-cache` | `post-delete-cache` | `./scripts/test-docker.sh ts --scenario post-delete-cache` | `post-delete-cache-logs`, `post-delete-cache-reports` |
| `nightly.direct-message` | （`direct-message` 追加予定） | `./scripts/test-docker.sh ts --scenario direct-message --no-build` | `direct-message-logs`, `direct-message-reports` |

## 個別メモ

### `nightly.desktop-e2e`
- 参照: [`docs/01_project/activeContext/build_e2e_test.md`](../activeContext/build_e2e_test.md)
- 目的: Key インポート→フィード読込→投稿作成までを WebDriverIO + `tauri-driver` で自動化し、Windows/Ubuntu どちらでも `./scripts/test-docker.{ps1,sh} e2e` から同じシナリオを起動する。
- artefact:
  - `nightly.desktop-e2e-logs`: `tmp/logs/desktop-e2e/<timestamp>.log`（PowerShell 版は `Tee-Object` で採取）。
  - `nightly.desktop-e2e-reports`: `test-results/desktop-e2e/`（WDIO JSON、スクリーンショット、`tests/e2e/output/*`）。
- 調査時は `tmp/logs/desktop-e2e/*.log` と `test-results/desktop-e2e/screenshots/` を突き合わせ、`build_e2e_test.md` 3.5 節のフローに沿ってドライバー初期化を再実行する。

### `nightly.trending-feed`
- 参照: [`./nightly.trending-feed.md`](./nightly.trending-feed.md), [`docs/03_implementation/trending_metrics_job.md`](../../03_implementation/trending_metrics_job.md)
- 目的: `/trending` `/following` の Vitest + `trending_metrics_job` 監視を Docker で再現し、Prometheus/JWT 設定ずれを即発見する。
- artefact:
  - `nightly.trending-feed-logs`: `tmp/logs/trending-feed/*.log`, `tmp/logs/trending_metrics_job_stage4_<timestamp>.log`, `test-results/trending-feed/prometheus/*.log`
  - `nightly.trending-feed-reports`: `test-results/trending-feed/reports/<timestamp>-*.json`
  - `nightly.trending-feed-metrics`: `test-results/trending-feed/metrics/<timestamp>-trending-metrics.json`
- `gh act` 実行時は `--bind --container-options "--privileged"` を必須とし、ログ一覧のみを出力する `List trending feed outputs (act)` ステップで成果物を確認してから Runbook のトリアージ手順に沿って解析する。

### `nightly.profile-avatar-sync`
- 参照: [`docs/03_implementation/p2p_mainline_runbook.md` Chapter4](../../03_implementation/p2p_mainline_runbook.md)、[`docs/01_project/progressReports/gh-act_ci-runbook.md`](./gh-act_ci-runbook.md)
- 目的: Service Worker 経由での avatar 再送・`cache_metadata` 更新を Docker で再現し、Stage4 仕様（TTL 30 分 / BroadcastChannel / `offlineApi.addToSyncQueue` ログ）が壊れていないかを追跡する。
- artefact:
  - `profile-avatar-sync-logs`: `tmp/logs/profile_avatar_sync_stage4_<timestamp>.log`（Vitest + Worker 実行結果を同一ファイルに集約）。
- 参考コマンド: `./scripts/test-docker.ps1 ts -Scenario profile-avatar-sync -ServiceWorker -NoBuild`（Windows）、`./scripts/test-docker.sh ts --scenario profile-avatar-sync --service-worker`（Bash）。Rust 側の `./scripts/test-docker.ps1 rust -Test profile_avatar_sync` も Runbook 4.2 で合わせて参照する。

### `nightly.sync-status-indicator`
- 参照: [`docs/03_implementation/p2p_mainline_runbook.md` Chapter5](../../03_implementation/p2p_mainline_runbook.md)
- 目的: `SyncStatusIndicator` / `OfflineIndicator` / `useSyncManager` の Stage4/Stage5 仕様（再送履歴 / `retryMetrics`）を Docker `offline-sync` シナリオで検証する。
- artefact:
  - `sync-status-indicator-logs`: `tmp/logs/sync_status_indicator_stage4_<timestamp>.log`
  - JSON レポートは `test-results/offline-sync/<timestamp>-*.json` に保存される（現状はワークスペース内に出力。必要に応じて `sync-status-indicator-logs` と一緒に圧縮してアップロードする）。
- 参考コマンド: `./scripts/test-docker.ps1 ts -Scenario offline-sync -NoBuild`。`phase5_ci_path_audit.md` の該当行にログ採取例がまとまっている。

### `nightly.user-search-pagination`
- 参照: [`docs/03_implementation/p2p_mainline_runbook.md` Chapter6](../../03_implementation/p2p_mainline_runbook.md), [`docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md`](../activeContext/artefacts/phase5_ci_path_audit.md)
- 目的: `/search` (users) の cursor/sort/429 handling と `allow_incomplete` fallback を Docker で再現し、レート制限 UI の退避ログを保存する。
- artefact:
  - `user-search-pagination-logs`: `tmp/logs/user_search_pagination_<timestamp>.log`
  - `user-search-pagination-log-archive`: `test-results/user-search-pagination/logs/*.log`（長期保存用）
  - `user-search-pagination-reports`: `test-results/user-search-pagination/reports/*.json`
- `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build`（PowerShell 版あり）で再現可能。Vitest は `npx pnpm vitest run ... | tee tmp/logs/user_search_pagination_<timestamp>.log` の形式に統一されている。

### `nightly.topic-create`
- 参照: [`docs/03_implementation/p2p_mainline_runbook.md` Chapter5](../../03_implementation/p2p_mainline_runbook.md)
- 目的: Topic 作成ショートカット + Offline queue (`OfflineActionType::CREATE_TOPIC`) をホスト/Vitest/Docker 双方で再現し、`watchPendingTopic` までの導線を検証する。
- artefact:
  - `topic-create-logs`: `tmp/logs/topic_create_<timestamp>.log`（Docker）、`tmp/logs/topic_create_host_<timestamp>.log`（ホスト実行分も同梱）
  - `topic-create-reports`: `test-results/topic-create/<timestamp>-*.json`（TopicSelector / PostComposer / Sidebar / Scenario の4ファイル）
- 再現コマンド: `./scripts/test-docker.sh ts --scenario topic-create [-NoBuild]`、PowerShell 版は `./scripts/test-docker.ps1 ts -Scenario topic-create`。

### `nightly.post-delete-cache`
- 参照: [`docs/03_implementation/p2p_mainline_runbook.md` Chapter5](../../03_implementation/p2p_mainline_runbook.md)
- 目的: 投稿削除時の React Query キャッシュ無効化・Offline queue 処理を Vitest + Docker で再現し、`useDeletePost` / `postStore` / `PostCard` の整合性を担保する。
- artefact:
  - `post-delete-cache-logs`: `tmp/logs/post_delete_cache_<timestamp>.log`（ホスト／Docker双方のログ）
  - `post-delete-cache-reports`: `test-results/post-delete-cache/<timestamp>-*.json`
- `./scripts/test-docker.ps1 ts -Scenario post-delete-cache` で Windows からも Docker シナリオを取得できる。`phase5_ci_path_audit.md` の 2025年11月13日ログを参照すると、ホストと Docker それぞれの採取例が確認できる。

### `nightly.direct-message`
- 参照: [`docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md`](../activeContext/artefacts/phase5_ci_path_audit.md), [`docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md`](../activeContext/artefacts/phase5_user_flow_summary.md)
- 目的: `Header` / `DirectMessageInbox` / `DirectMessageDialog` / `useDirectMessageBadge` に跨る DM 既読共有導線を Docker で再現し、仮想スクロールや宛先検索のリグレッションを即検知する。
- artefact（`nightly.yml` へのジョブ追加を準備中）:
  - `direct-message-logs`: `tmp/logs/vitest_direct_message_<timestamp>.log`
  - `direct-message-reports`: `test-results/direct-message/<timestamp>-*.json`
- 追加予定のコマンドは `./scripts/test-docker.sh ts --scenario direct-message --no-build`（PowerShell: `./scripts/test-docker.ps1 ts -Scenario direct-message -NoBuild`）。Rust contract (`tests/contract/direct_messages.rs`) のログは `./scripts/test-docker.ps1 rust` で別途取得し、`phase5_user_flow_summary.md` の DM 行で共有している。
