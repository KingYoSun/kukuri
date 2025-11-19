[title] 作業中タスク（in_progress）

最終更新日: 2025年11月19日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク
### GitHub Actions ワークフロー失敗調査（担当: Codex）
- 状況: `gh act` でローカル再現しつつ、ワークフロー失敗要因を特定・修正する。
- メモ: 2025年11月19日着手。失敗ログの解析と修正内容は作業完了時に追記する。
- 進捗: `trending_metrics_job.rs` の未使用メソッドと `DistributorState::strategy` のテスト専用アクセサを整理し、`scripts/test-docker.ps1 lint` で `cargo clippy -D warnings` を再実行してエラーが消えたことを確認。`gh act --job format-check` と `gh act --job native-test-linux`（`NPM_CONFIG_PREFIX=/tmp/npm-global`, `--container-options "--user root"`）も完走し、Linux ネイティブ経路での Rust/TS/Lint すべて green。

### Direct Message ナイトリー/会話ページング整備（担当: Codex）
- 状況: `/profile/$userId` 導線の残タスクとして、DM 会話の50件超ページングと `nightly.direct-message` ジョブ未配備が指摘されている（`docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:21-24`）。現状の `list_direct_message_conversations` は LIMIT のみでカーソルに未対応（`kukuri-tauri/src-tauri/src/infrastructure/database/sqlite_repository/queries.rs:84-116`）。
- メモ: `scripts/test-docker.{ps1,sh} ts --scenario direct-message --no-build` で Kind4 IPC/既読同期のスモークは取得できるが、Nightly artefact や Runbookへの紐付けがない。UI 側も `DirectMessageInbox` がストア全件を前提にしているため、カーソル API 実装後の Infinite Query 化が必要。
- TODO:
  1. Rust の `list_direct_message_conversations` / DTO / `TauriApi` を cursor + `has_more` 対応へ拡張し、`DirectMessageInbox` に Infinite Query を導入する。
  2. `scripts/test-docker` に `direct-message` シナリオを正式追加し、`nightly.yml` へ `nightly.direct-message` ジョブ（ログ/JSON artefact含む）を追加する。
  3. Runbook Chapter5・`phase5_user_flow_inventory.md`・`phase5_ci_path_audit.md` に Nightly 手順とログパスを追記し、DM の多端末既読共有証跡を固定化する。

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
