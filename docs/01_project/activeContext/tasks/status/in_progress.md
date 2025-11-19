[title] 作業中タスク（in_progress）

最終更新日: 2025年11月19日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### Offline Sync Nightly拡張とエラーハンドリング指標（担当: Codex）
- 状況: Stage4 で `SyncStatusIndicator` と Doc/Blob 用 Service Worker は完成したが、Topic/Post/Follow など他の OfflineAction を Nightly に含める計画や `errorHandler` の `SyncStatus.*` 拡張は backlog と整理されている（`docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:24-25`）。`useSyncManager` では再送メトリクスを `offlineApi.recordOfflineRetryOutcome` へ送る実装があるものの（`kukuri-tauri/src/hooks/useSyncManager.ts:360-416`）、Runbook への掲載と artefact 化が未完。
- メモ: `scripts/test-docker.{ps1,sh} ts --scenario offline-sync --no-build` は Doc/Blob ケースのみ。Nightly に Topic/Post/Follow の queued action を追加し、`offline-sync` artefact をテストごとに分岐させる必要がある。
- TODO:
  1. `offline-sync` シナリオを Topic/Post/Follow/DM 操作で再構成し、`sync_queue` / `offline_actions` テーブルの再送ログを `tmp/logs/sync_status_indicator_stage4_*.log` に追記する。
  2. `nightly.yml` へ追加シナリオをブランチし、artefact を `sync-status-indicator-topic`, `...-post` などに分割、Runbook Chapter5 と `phase5_ci_path_audit.md` に反映する。
  3. `errorHandler` へ `SyncStatus.*` カテゴリを追加し、`SyncStatusIndicator` の UI からオフライン再送メトリクスを記録、`docs/01_project/progressReports/nightly.partial-feature-usage.md#syncstatusindicator` にログ取得手順を追記する。
