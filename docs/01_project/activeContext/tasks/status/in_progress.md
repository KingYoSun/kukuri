[title] 作業中タスク（in_progress）

最終更新日: 2025年11月14日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### リファクタリング計画 2025-08-08 v3 未完了タスク

1. **機能使用状況マップ: 部分的に使用される機能の可視化**  
   - 背景: `refactoring_plan_2025-08-08_v3.md:219` のチェックが未完。  
   - やること: 一部画面からのみ到達できるコンポーネント/コマンドを列挙し、使用箇所と未使用箇所を明記。  
   - 完了条件: すべての部分利用機能に対して使用/未使用の両方の文脈が記録される。

2. **すべてのRustテストをグリーンに戻す**  
   - 背景: `modules::offline::tests` 系で 8 件失敗が続いており（`refactoring_plan_2025-08-08_v3.md:81-84`）、成功指標 `すべてのRustテスト成功`（同:424）が未達。  
   - やること: SQLite パーミッション問題の再現・修正、`cargo test`（`kukuri-tauri/src-tauri` と `kukuri-cli`）の連続実行で確認。  
   - 完了条件: 両ディレクトリで `cargo test` が成功し、ドキュメントのチェックを完了できる。

3. **コード重複率30%削減の達成**  
   - 背景: 成功指標（`refactoring_plan_2025-08-08_v3.md:425`）が未達で、Phase4 DRY 施策の実行が途中。  
   - やること: 4.1 共通化方針（共通モック／ストア基盤）を具現化し、重複計測（lint/coverage ツール）で 30% 減を証明。  
   - 完了条件: 重複率レポートを添えて 30% 以上削減したことを記録し、チェックを更新。

4. **未使用APIエンドポイントをゼロにする**  
   - 背景: 成功指標（`refactoring_plan_2025-08-08_v3.md:436`）未達。残タスクは `add_relay` / `join_topic_by_name` / `delete_events` / `get_nostr_pubkey` 等（同:437-440）。  
   - やること: 各コマンドを UI に接続するか、不要であれば削除案＋テスト更新を提示。  
   - 完了条件: 未使用 API が 0 件であることをドキュメント化し、チェックを完了。

5. **孤立コンポーネントをゼロにする**  
   - 背景: 成功指標（`refactoring_plan_2025-08-08_v3.md:441-443`）未達。鍵管理ダイアログ等が導線未接続のまま。  
   - やること: Sidebar/Header など既存導線に統合するか、不要な UI は廃止。テストケースも更新。  
   - 完了条件: 全コンポーネントの導線が明示され、孤立要素が解消されている。

6. **dead_code の 80%以上を削除または活用**  
   - 背景: `refactoring_plan_2025-08-08_v3.md:444-446` による残タスク。`hybrid_distributor` / `event_sync` / `offline_api` 周辺に候補が集中。  
   - やること: dead_code 一覧を精査し、使用開始 or 削除のいずれかに分類。削除時は SQLx / Rust テストで回帰確認。  
   - 完了条件: dead_code 削減率が 80% 以上になり、成果を artefact として記録。

7. **すべての Tauri コマンドをフロントエンド導線へ接続**  
   - 背景: 成功指標（`refactoring_plan_2025-08-08_v3.md:447-449`）が未完。`add_relay` `join_topic_by_name` 等が UI から未呼び出し。  
   - やること: コマンド単位で呼び出し経路を追加し、`phase5_user_flow_summary.md` / `phase5_ci_path_audit.md` のリンク更新。  
   - 完了条件: すべてのコマンドに UI からの呼び出しパスが存在し、成功指標チェックを閉じられる。

### MVP Exit タスク

10. **トレンド/フォロー導線: メトリクスとテレメトリの仕上げ**  
    - 背景: `docs/01_project/roadmap.md:14-20` と `docs/01_project/activeContext/tauri_app_implementation_plan.md:13` で、`trending_metrics_job` の 24h 集計・`list_trending_*` の `generated_at` ミリ秒保証・`scripts/test-docker.{sh,ps1} ts --scenario trending-feed` / `scripts/metrics/export-p2p --job trending` の artefact 固定が未完了と明記されている。  
    - やること: (1) `trending_metrics_job` の 24h スライディングウィンドウと `score_weights` を確定し、`list_trending_topics/posts` がミリ秒精度の `generated_at` を返すよう Rust 側を更新。(2) `scripts/test-docker.{sh,ps1} ts --scenario trending-feed` に `prometheus-trending` サービス起動・`curl http://127.0.0.1:9898/metrics` 採取・`scripts/metrics/export-p2p --job trending` 実行を組み込み、`tmp/logs/trending_metrics_job_stage4_<timestamp>.log` と `test-results/trending-feed/{reports,prometheus}` を Nightly artefact 化。(3) `TrendingSummaryPanel` / `FollowingSummaryPanel` のテレメトリ（DM 未読カード含む）を更新し、Runbook・`phase5_ci_path_audit.md` に手順とログパスを追記。  
    - 完了条件: `nightly.trending-feed` が安定して緑化し、Prometheus/JSON artefact と UI テレメトリが同期している状態を `roadmap.md` / Runbook 双方で確認できる。

11. **Direct Message Inbox: 多端末既読共有と contract テストの完了**  
    - 背景: `docs/01_project/activeContext/tauri_app_implementation_plan.md:12` と `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:124` で、DM Inbox の仮想スクロール・宛先補完・`mark_direct_message_conversation_read` の多端末同期・ユニット/contract テストが残課題とされている。  
    - やること: (1) SQLite `direct_message_conversations` と `mark_direct_message_conversation_read` を拡張し、マルチデバイスの既読伝搬と `useDirectMessageBootstrap` の 30 秒再同期を契約テストで保証。(2) `DirectMessageInbox` / `DirectMessageDialog` / `Header` の仮想スクロール・宛先検索テレメトリを整理し、`pnpm vitest ...Header.test.tsx DirectMessageDialog.test.tsx` と Docker `direct-message` シナリオを再取得。(3) `phase5_user_flow_inventory.md` 5.4・`phase5_ci_path_audit.md` にテスト ID / `tmp/logs/vitest_direct_message_<timestamp>.log` / `test-results/direct-message/*.json` を追記。  
    - 完了条件: DM Inbox が全端末で同じ未読数を表示し、Nightly artefact で contract テストとログが参照できる。

12. **ユーザー検索導線: レートリミット UI と Nightly artefact の整備**  
    - 背景: `docs/01_project/activeContext/tauri_app_implementation_plan.md:14` と `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:127` で、`user-search-pagination` シナリオの Nightly 組み込みと成果物固定、レートリミット UI の検証が未完と整理されている。  
    - やること: (1) `nightly.yml` に `nightly.user-search-pagination` を追加し、`./scripts/test-docker.sh ts --scenario user-search-pagination --no-build`（PowerShell 版含む）を呼び出して `tmp/logs/user_search_pagination_<timestamp>.log` / `test-results/user-search-pagination/{logs,reports}` を artefact 化。(2) SearchBar のレートリミット / `allow_incomplete` フォールバック / エラー状態を `errorHandler` とテレメトリに接続し、`useUserSearchQuery` / `UserSearchResults` テストで UI 遷移を再検証。(3) `phase5_user_flow_inventory.md` 5.8 と Runbook 6.4 に Nightly 手順とログ参照先を追記。  
    - 完了条件: `/search` 導線が Nightly で自動再現され、レートリミット UI と成果物が Runbook から参照できる。

13. **Offline sync_engine: 再送メトリクスと Runbook/CI 連携**  
    - 背景: `docs/01_project/roadmap.md:19` および `docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md:17` で、Stage4 完了後も `sync_engine` の再送ログ・メトリクス・`nightly.sync-status-indicator` 連動が未整備とされている。  
    - やること: (1) `sync_engine` / `offline_actions` / Service Worker に再送メトリクス（成功/失敗・retry_count・backoff）を追加し、`metrics::record_outcome` と `SyncStatusIndicator` に露出。(2) `scripts/test-docker.{sh,ps1} ts --scenario offline-sync` と `nightly.sync-status-indicator` artefact を更新し、`tmp/logs/sync_status_indicator_stage4_<timestamp>.log` / `test-results/offline-sync/*.json` に再送情報を保存。(3) Runbook Chapter5 と `phase5_ci_path_audit.md` に新しいメトリクス項目・ログパス・トリアージ手順を追記。  
    - 完了条件: Offline 操作の再送状況が UI / Nightly artefact / Runbook で一貫して観測できる。

14. **グローバルコンポーザー & 投稿削除: キャッシュ整合とテスト更新**  
    - 背景: `docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md:19` と `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:123,128` で、`TopicSelector` の create-from-composer モードと `useDeletePost` の React Query 無効化テスト、`post-delete-cache` Docker シナリオの整備が未完と整理されている。  
    - やること: (1) `corepack enable pnpm` 前提で `pnpm vitest run src/tests/unit/components/topics/TopicSelector.test.tsx src/tests/unit/components/posts/PostCard.test.tsx src/tests/unit/routes/{trending,following}.test.tsx` を再実行し、グローバルコンポーザー導線と Summary Panel を検証。(2) `useDeletePost` / `postStore` に `invalidatePostCaches` を実装し、トレンド/フォロー/トピック/プロフィール各キャッシュと `offlineStore` の整合を確認、Docker `./scripts/test-docker.{sh,ps1} ts --scenario post-delete-cache` の artefact を更新。(3) 結果を `phase5_user_flow_inventory.md` 5.7/5.9/5.10 と Runbook に反映。  
    - 完了条件: グローバルコンポーザー経由のトピック作成と投稿削除がキャッシュ不整合なく動作し、Vitest/Docker/Nightly のログが揃っている。

15. **鍵管理ダイアログ: 鍵バックアップ/復旧フローの提供**  
    - 背景: `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md:125` で、設定 > 鍵管理ボタンが未配線でバックアップ手段が無いことが MVP ブロッカーとして挙げられている。  
    - やること: (1) `KeyManagementDialog` を実装し、`export_private_key` / `SecureStorageApi.addAccount` / `add_relay` 連動と注意喚起 UI を整備。(2) エクスポート/インポート操作を `errorHandler` に記録、`withPersist` へ操作履歴を残す。(3) `pnpm vitest`（UI）と `./scripts/test-docker.ps1 rust -Test key_management`（仮）でバックアップ/復旧の契約テストを追加し、Runbook・`phase5_user_flow_inventory.md` 5.1/5.6 に掲載。  
    - 完了条件: ユーザーが UI から鍵を安全にバックアップ/復旧でき、テストとドキュメントで手順が保証される。

16. **Ops/CI: Nightly & GitHub Actions で MVP 導線を安定再現**  
    - 背景: `docs/01_project/roadmap.md:20` と `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` の「追加予定のテスト/artefact」節で、GitHub Actions の `trending-feed` Docker 失敗・Nightly artefact 権限・`scripts/test-docker.ps1 all` の安定化・`docs/01_project/progressReports/` への Runbook リンク不足が指摘されている。  
    - やること: (1) GitHub Actions `trending-feed` ジョブで発生している Docker 権限問題と artefact 不足を切り分け、`nightly.yml` の `*-logs` / `*-reports` 命名を固定。(2) `cmd.exe /c "corepack enable pnpm"` → `pnpm install --frozen-lockfile` を `docs/01_project/setup_guide.md` / Runbook に追記し、`scripts/test-docker.ps1 all` で同前提を明文化。(3) `docs/01_project/progressReports/` へ Nightly テスト ID（`nightly.profile-avatar-sync`, `nightly.trending-feed`, `nightly.user-search-pagination`, ほか）と対応するログ/artefact リンクを整理。  
    - 完了条件: GitHub Actions / Nightly がすべての MVP 導線を再現し、failure 時に参照すべき artefact ・ Runbook リンクが一元化されている。
