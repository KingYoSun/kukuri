# Nightly 部分利用導線トリアージガイド
最終更新日: 2025年11月18日

## 背景
- `/profile/$userId`・`/search`・SyncStatusIndicator など、UI 上は稼働しているが導線の一部が backlog に残っている機能について、Nightly artefact と Runbook の突合が分散していた。
- `phase5_feature_usage_map.md` 3章の内容を `phase5_user_flow_summary.md`「部分利用導線マトリクス」と `phase5_user_flow_inventory.md` 5.6 / 5.8 / 5.11 のセクションで追跡できるよう、テスト ID とログ採取手順を本ガイドに集約する。
- 監査時は本ファイル → `nightly.index.md` → `phase5_ci_path_audit.md`（ログ一覧）→ Inventory / Runbook の順で辿り、artefact の欠落や Nightly failure を即座に切り分ける。

## クイックマップ
| Flow / Route | テストID（Nightly） | Docker / Vitest エントリ | 主な artefact / ログ | 関連ドキュメント |
| --- | --- | --- | --- | --- |
| `/profile/$userId`（プロフィール詳細 + DM） | `nightly.profile-avatar-sync`（Service Worker / Doc）、`nightly.direct-message`、Rust: `direct_messages` 契約テスト | TypeScript: `./scripts/test-docker.{ps1,sh} ts --scenario direct-message --no-build`<br>Rust: `./scripts/test-docker.ps1 rust -Test direct_messages`<br>Service Worker: `./scripts/test-docker.{ps1,sh} ts --scenario profile-avatar-sync --service-worker` | `nightly.direct-message-logs`（`tmp/logs/vitest_direct_message_<timestamp>.log`）、`nightly.direct-message-reports`（`test-results/direct-message/<timestamp>-*.json`）、`profile-avatar-sync-logs`、`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` | `phase5_user_flow_inventory.md` 5.6 / 5.6.1 / 5.6.2 / 5.6.3、`phase5_user_flow_summary.md`「部分利用導線マトリクス」 `/profile/$userId` 行 |
| `/search` (users) | `nightly.user-search-pagination` | `./scripts/test-docker.{ps1,sh} ts --scenario user-search-pagination --no-build` | `user-search-pagination-logs`（`tmp/logs/user_search_pagination_<timestamp>.log`）、`user-search-pagination-log-archive`、`user-search-pagination-reports`、`user-search-pagination-search-error`（2文字未満→補助検索→`retryAfter` 自動解除→`SearchErrorState` JSON） | `phase5_user_flow_inventory.md` 5.8 / 5.8.1、`phase5_user_flow_summary.md`「部分利用導線マトリクス」 `/search` 行 |
| SyncStatusIndicator / Offline Sync | `nightly.sync-status-indicator.{topic,post,follow,dm}`、Doc/Blob 連携: `nightly.profile-avatar-sync` | `./scripts/test-docker.{ps1,sh} ts --scenario offline-sync --offline-category <cat> --no-build`（Topic/Post/Follow/DM の OfflineAction）＋ `offlineSyncTelemetry.test.tsx`、`./scripts/test-docker.{ps1,sh} ts --scenario profile-avatar-sync --service-worker` | `sync-status-indicator-{topic,post,follow,dm}`（`tmp/logs/sync_status_indicator_stage4_{cat}_<timestamp>.log` と `test-results/offline-sync/{cat}/${timestamp}-*.json`）、`profile-avatar-sync-logs` | `phase5_user_flow_inventory.md` 5.11 / 5.11.1、`phase5_user_flow_summary.md`「部分利用導線マトリクス」 SyncStatus 行 |

---

## 1. `/profile/$userId` ルート

### 1.1 監視対象
- プロフィール詳細 → `DirectMessageDialog` への導線（Header / Summary / Profile の CTA）が共有ストアで同期されているか。
- `direct_message_conversations` の limit/pagination と Kind4 既読同期のログが `tmp/logs/vitest_direct_message_<timestamp>.log` に記録されているか。
- `nightly.profile-avatar-sync` で Service Worker が `offlineApi.addToSyncQueue` / `cache_metadata` を更新し、プロフィール編集ルートへ影響がないか。

### 1.2 トリアージ手順
1. `./scripts/test-docker.ps1 ts -Scenario direct-message -NoBuild`（または `.sh` 版）を実行し、`tmp/logs/vitest_direct_message_<timestamp>.log` と `test-results/direct-message/<timestamp>-*.json`（Dialog / Inbox / Header / Badge）を採取。`phase5_ci_path_audit.md` の direct-message 行へタイムスタンプを追記する。
2. `./scripts/test-docker.ps1 rust -Test direct_messages` を実行し、`tests/contract/direct_messages.rs::direct_message_read_receipts_sync_across_devices` で Kind4 既読同期を確認。ログは `tmp/logs/rust_docker_<timestamp>.log` に記録する。
3. プロフィール編集導線の Service Worker / Offline 同期は `./scripts/test-docker.ps1 ts -Scenario profile-avatar-sync -ServiceWorker` → `profile-avatar-sync-logs` / `tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` で確認する。
4. `phase5_user_flow_inventory.md` 5.6.3 の未解決項目（50件超ページング、Nightly artefact 追加）を見ながら、`docs/01_project/activeContext/artefacts/phase5_feature_usage_map.md` 3.2 の `/profile` 行と整合しているか確認する。

### 1.3 Artefact / 参照
- `tmp/logs/vitest_direct_message_<timestamp>.log`
- `test-results/direct-message/<timestamp>-{dialog,inbox,header,useDirectMessageBadge}.json`
- `profile-avatar-sync-logs`、`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log`
- `tmp/logs/rust_docker_<timestamp>.log`（`direct_messages` 契約テスト）
- `phase5_ci_path_audit.md` direct-message / profile-avatar-sync 行

---

## 2. `/search` (users)

### 2.1 監視対象
- `useUserSearchQuery` の cursor / sort / rate limit UI が `user-search-pagination` シナリオで再現できるか。
- 2文字未満の補助検索で `allow_incomplete` が有効化され、`errorHandler.info('UserSearch.allow_incomplete_enabled', …)` と SearchBar 警告がログに残っているか。
- `retryAfterSeconds` のカウントダウンが `SearchErrorState` で表示され、artefact JSON と UI の表示が一致しているか。

### 2.2 トリアージ手順
1. `./scripts/test-docker.ps1 ts -Scenario user-search-pagination -NoBuild` を実行し、`tmp/logs/user_search_pagination_<timestamp>.log` を `user-search-pagination-logs` artefact へアップロード。`test-results/user-search-pagination/reports/<timestamp>.json` とログの `retryAfter` 値を突き合わせる。
2. `user-search-pagination-search-error` artefact (`test-results/user-search-pagination/search-error/<timestamp>-search-error-state.json`) を開き、`helperSearch.term` / `steps[].retryAfterSeconds` / `searchErrorState.buttonLabelBefore/After` がログと一致しているか確認。補助検索ワードが 2 文字未満になっていることを合わせて確認する。
3. `phase5_ci_path_audit.md` の user-search 行と `phase5_user_flow_summary.md`「部分利用導線マトリクス」 `/search` 行で最新タイムスタンプが一致しているか、Nightly failure 時は JSON を添付して Runbook 6.4 に記録する。

### 2.3 Artefact / 参照
- `user-search-pagination-logs`
- `user-search-pagination-log-archive`
- `user-search-pagination-reports`
- `user-search-pagination-search-error`
- `phase5_ci_path_audit.md` user-search 行

---

## 3. SyncStatusIndicator / Offline Sync

### 3.1 監視対象
- Doc/Blob 対応 `cache_metadata` と Stage4 UI（Doc/Blob 競合カード / Offline CTA）が Docker `offline-sync` シナリオで再現できるか。
- `pendingActionSummary` のカテゴリ（Topic/Post/Follow/DM）が UI に表示され、`errorHandler.info('SyncStatus.queue_snapshot' | '...pending_actions_snapshot' | '...retry_metrics_snapshot')` が `tmp/logs/sync_status_indicator_stage4_{cat}_<timestamp>.log` に出力されているか。
- `profile-avatar-sync` の Service Worker ログと各カテゴリの `offline-sync` メトリクスが矛盾していないか（Doc/Blob + Topic/Post/Follow/DM の Nightly artefact が揃っているか）。

### 3.2 トリアージ手順
1. 各カテゴリで Docker シナリオを実行  
   ```powershell
   ./scripts/test-docker.ps1 ts -Scenario offline-sync -OfflineCategory topic [-NoBuild]
   ./scripts/test-docker.ps1 ts -Scenario offline-sync -OfflineCategory post -NoBuild
   ./scripts/test-docker.ps1 ts -Scenario offline-sync -OfflineCategory follow -NoBuild
   ./scripts/test-docker.ps1 ts -Scenario offline-sync -OfflineCategory dm -NoBuild
   ```  
   生成された `tmp/logs/sync_status_indicator_stage4_{cat}_<timestamp>.log` に `queue_snapshot` / `pending_actions_snapshot` / `retry_metrics_snapshot` の JSON が揃っているか確認し、JSON レポートは `test-results/offline-sync/{cat}/${timestamp}-*.json` に保管する。
2. `./scripts/test-docker.ps1 ts -Scenario profile-avatar-sync -ServiceWorker` を同じターミナルで実行し、Doc/Blob 側の `offlineApi.addToSyncQueue` ログと TTL が維持されているか確認する。
3. `phase5_user_flow_inventory.md` 5.11.1 と `phase5_user_flow_summary.md` SyncStatus 行を参照し、カテゴリごとの Nightly artefact（Topic/Post/Follow/DM）が揃っているか、`phase5_ci_path_audit.md` の同期ログにタイムスタンプを追記する。

### 3.3 Artefact / 参照
- `sync-status-indicator-topic` / `sync-status-indicator-post` / `sync-status-indicator-follow` / `sync-status-indicator-dm`
- `test-results/offline-sync/{topic,post,follow,dm}/${timestamp}-*.json`
- `profile-avatar-sync-logs`
- `phase5_ci_path_audit.md` sync-status / profile-avatar-sync 行

---

## 4. 更新ルール
- 監視対象の導線に追加タスクが発生した場合は、先に `phase5_feature_usage_map.md` 3.2 と `phase5_user_flow_summary.md`「部分利用導線マトリクス」を更新してから、本ファイルと Inventory 5.6 / 5.8 / 5.11 に反映する。
- Nightly artefact の名称や格納先に変更が入った場合は、`nightly.index.md` → 本ファイル → `phase5_ci_path_audit.md` の順番で必ず更新し、`tasks/status/in_progress.md` の該当タスクへリンクを貼る。
