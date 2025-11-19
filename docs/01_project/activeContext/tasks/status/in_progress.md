[title] 作業中タスク（in_progress）

最終更新日: 2025年11月19日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### `/search` 補助検索と SearchErrorState artefact 自動化（担当: Codex）
- 状況: `/search` 行は cursor/レートリミット UI まで実装済みだが、2文字未満補助検索や `SearchErrorState` の artefact 化が未着手と整理されている（`docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:21-25`）。Nightly では `user-search-pagination` job があるものの、短文キーワードや `retryAfter` 解除パスを検証していない。
- メモ: `search.tsx` / `useUserSearchQuery.ts` では `allow_incomplete` と `retryAfterSeconds` を扱っている（`kukuri-tauri/src/routes/search.tsx:17-164`、`kukuri-tauri/src/hooks/useUserSearchQuery.ts:1-200`）ため、テレメトリを Nightly に取り込めば Exit 条件を満たせる。
- TODO:
  1. `scripts/test-docker.ts --scenario user-search-pagination` を拡張し、2文字未満入力→補助検索→`retryAfter` 自動解除→`SearchErrorState` 表示までのケースを JSON/ログに保存する。
  2. `nightly.yml` の `user-search-pagination` job で `SearchErrorState` キャプチャを artefact (`user-search-pagination-search-error`) に追加し、`docs/01_project/progressReports/nightly.partial-feature-usage.md#search` にリンクする。
  3. Runbook と `phase5_ci_path_audit.md` の `/search` セクションを更新し、短文補助検索の再現コマンドと artefact 位置を記録する。

### Offline Sync Nightly拡張とエラーハンドリング指標（担当: Codex）
- 状況: Stage4 で `SyncStatusIndicator` と Doc/Blob 用 Service Worker は完成したが、Topic/Post/Follow など他の OfflineAction を Nightly に含める計画や `errorHandler` の `SyncStatus.*` 拡張は backlog と整理されている（`docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:24-25`）。`useSyncManager` では再送メトリクスを `offlineApi.recordOfflineRetryOutcome` へ送る実装があるものの（`kukuri-tauri/src/hooks/useSyncManager.ts:360-416`）、Runbook への掲載と artefact 化が未完。
- メモ: `scripts/test-docker.{ps1,sh} ts --scenario offline-sync --no-build` は Doc/Blob ケースのみ。Nightly に Topic/Post/Follow の queued action を追加し、`offline-sync` artefact をテストごとに分岐させる必要がある。
- TODO:
  1. `offline-sync` シナリオを Topic/Post/Follow/DM 操作で再構成し、`sync_queue` / `offline_actions` テーブルの再送ログを `tmp/logs/sync_status_indicator_stage4_*.log` に追記する。
  2. `nightly.yml` へ追加シナリオをブランチし、artefact を `sync-status-indicator-topic`, `...-post` などに分割、Runbook Chapter5 と `phase5_ci_path_audit.md` に反映する。
  3. `errorHandler` へ `SyncStatus.*` カテゴリを追加し、`SyncStatusIndicator` の UI からオフライン再送メトリクスを記録、`docs/01_project/progressReports/nightly.partial-feature-usage.md#syncstatusindicator` にログ取得手順を追記する。
