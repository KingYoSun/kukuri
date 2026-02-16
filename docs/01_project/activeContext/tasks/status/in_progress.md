[title] 作業中タスク（in_progress）

最終更新日: 2026年02月16日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク

### 2025年11月20日 MVP動作確認シナリオ整理

- 目的: Phase 5 Exit Criteria 全項目（`docs/01_project/design_doc.md` / `phase5_user_flow_summary.md`）が実際のアプリ体験として再現できることを確認する。
- 状態: 着手（チェックリスト化のみ完了。次は各項目を `scripts/test-docker.ps1` / Nightly artefactで順次実施）。
- 確認する挙動（想定導線と参照 artefact をすべて列挙）:
  1. **オンボーディング/アカウント/キー管理**
     - `/welcome`→`/login`→`/profile-setup` で `generate_keypair` / `login` / `add_account` / `initialize_nostr` / `update_nostr_metadata` が一連で動作し、複数アカウントを `authStore` から切り替えられる。
     - `ProfileSetup` で入力したプロフィールが Settings > Profile でも同期され、オンボーディング後の再編集時にもドラフト復元が効く。
     - `KeyManagementDialog` から `.nsec` をエクスポート/復旧し、`authStore.loginWithNsec(..., true)` で Secure Storage へ再登録できる（`scripts/test-docker.ps1 ts --filter KeyManagementDialog` / `rust -Test key_management` ログを確認）。
     - ✅ `tests/e2e/specs/onboarding.key-management.spec.ts` でオンボーディング〜キー管理〜複数アカウント切替までを自動検証（msedgedriver 未導入のためローカルWDIO実行は driver 不足で失敗するが、シナリオ自体は自動化済み）
  2. **プロフィール/プライバシー/アバター同期**
     - `ProfileEditDialog` / `SettingsPage` でのプライバシーフラグ更新が `update_privacy_settings`→`authStore.updateUser` に即時反映される。
     - `upload_profile_avatar` → `ProfileAvatarService` → Doc/Blob 保存 → `useProfileAvatarSync.syncNow` の連携でヘッダー画像と `OfflineIndicator` が即更新される。
     - `profileAvatarSyncSW.ts` がバックグラウンド更新・指数バックオフ・`tmp/logs/profile_avatar_sync_*.log` 出力を実施し、`scripts/test-docker.ps1 ts --scenario profile-avatar-sync` / `rust -Test profile_avatar_sync` artefact が取得できる。
  3. **ホーム/トピック/投稿操作**
     - `/` で `PostComposer` のドラフト保存/Markdownプレビュー/添付（必要な場合）が正常動作し、`TopicSelector` のショートカット・最近使用トピックが反映される。
     - `PostCard` からのいいね/ブースト/返信/引用/ブックマーク/ブックマーク解除が `get_posts` / `like_post` / `boost_post` / `bookmark_post` 系 Tauri コマンドと同期して UI が即座に更新される。
     - トピック作成・削除（`nightly.topic-create` / `nightly.post-delete-cache`）が Offline Queue を通じて成功し、`tmp/logs/topic_create_*` / `post_delete_cache_*` / `test-results/topic-*` が Runbook 手順通り採取できる。
  4. **トレンド/フォロー/サマリーパネル**
     - `/trending` で `TrendingSummaryPanel.generated_at` のラグ表示と DM カードが表示され、クリックで対象トピック/DMに遷移する。
     - `/following` でフォロー済みトピックのみが表示され、Follow/Unfollow 操作がサイドバー/TopicSelector/ホームフィードに即時反映される。
     - `trending_metrics_job` → `test-results/trending-feed/{reports,prometheus,metrics}` → UI 表示までのラウンドトリップが `scripts/test-docker.ps1 ts --scenario trending-feed` / Nightly artefactで再現できる。
  5. **Direct Messages（/direct-messages + Profile 経由）**
     - `DirectMessageInbox` の TanStack Virtualizer による会話一覧・検索・未読数が `useDirectMessageBootstrap`（RelayState依存）と同期する。
     - `DirectMessageDialog` で送信/再送/ドラフト保存が Offline Queue を経由して動作し、Kind4 既読共有が複数インスタンス間で同期される（contract: `kukuri-tauri/src-tauri/tests/contract/direct_messages.rs`）。
     - `nightly.direct-message` の `tmp/logs/vitest_direct_message_*.log` / `test-results/direct-message/*.json` が更新され、Runbook Chapter5の DM 手順と突合できる。
  6. **ユーザー検索（/search + 全画面共有コンポーネント）**
     - `useUserSearchQuery` の state machine（allowIncomplete/cooldown/retryAfter）が RateLimit UI と連動して正しく遷移し、補助検索→本検索が自動で切り替わる。
     - 並び替え（関連度/最新順）、無限スクロール、検索エラー時の `SearchErrorState` コンポーネントが `/search` / Sidebar / DM などすべての呼び出し元で共通に機能する。
     - `scripts/test-docker.ps1 ts --scenario user-search-pagination --no-build` の `test-results/user-search-pagination/{reports,logs,search-error}` を Nightly artefact として保存できる。
  7. **SyncStatusIndicator / Offline 同期**
     - `list_sync_queue_items` 60 秒ポーリングと手動再読込が働き、失敗アクションを選択→再試行→成功クリアまで UI で追跡できる。
     - Conflict バナーと `offlineApi.addToSyncQueue` による Doc/Blob（プロフィール・投稿・フォロー・DM）の楽観更新が Stage4 仕様に沿って動作し、`test-results/offline-sync/{topic,post,follow,dm}` と `tmp/logs/sync_status_indicator_stage4_*` が取得できる。
     - `SyncStatusIndicator` が Online/Syncing/Attention Required/Offline の各状態を通知し、Runbook Chapter5 のチェックリストと一致する。
  8. **P2P / RelayStatus / CLI ブートストラップ**
     - RelayStatus カードで `get_relay_status` により Mainline DHT ノード/Peer 数/Bootstrap 情報が表示され、Runbook Chapter10 への導線が機能する。
     - `apply_cli_bootstrap_nodes` で `cn-cli --export-path` のリストを UI から適用し、`P2PStack` が `ENABLE_P2P_INTEGRATION=1` で再初期化される。
     - `scripts/test-docker.ps1 integration -NoBuild`（または `./scripts/test-docker.ps1 rust` 内の `p2p_mainline_smoke.rs`）で Mainline DHT 接続ヘルスチェックが再現できる。
  9. **バックグラウンドジョブ / Runbook 連携**
     - `trending_metrics_job` / `nightly.topic-create` / `nightly.post-delete-cache` / `nightly.profile-avatar-sync` / `nightly.sync-status-indicator` の artefact が `.act-artifacts/` と `tmp/logs/*` に揃い、Runbook Chapter4/5/10 の参照先が欠落していない。
     - `docs/03_implementation/p2p_mainline_runbook.md` 記載の採取コマンドで必要ログを収集し、`phase5_ci_path_audit.md` に載っているテスト ID と対応付けられる。
  10. **Ops/CI ガード**
      - `gh act --workflows .github/workflows/test.yml --job format-check` / `--job native-test-linux` が成功し、`.act-artifacts/` に最新ログが保存される。
      - Community Node テストは OS を問わずコンテナ経路を既定とし、`docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch` + `docker compose -f docker-compose.test.yml build test-runner` + `docker run --rm --network kukuri_community-node-network ... kukuri-test-runner ... cargo test --workspace --all-features` を実行して `test-results/` を更新する。Windows の Tauri 側検証は従来どおり `./scripts/test-docker.ps1 ts|rust|all`（必要に応じて `--scenario trending-feed` など）を利用する。
      - 2026年01月28日: desktop-e2e の Meilisearch 認証ヘッダ修正。`./scripts/test-docker.ps1 e2e-community-node` 通過、`gh act` の format-check / native-test-linux 実行済み。
      - 2026年01月29日: desktop-e2e（community node）で `onboarding.key-management.spec.ts` のアカウント切替がタイムアウトするため、DOM 直接クリック＋bridge フォールバックへ調整。`./scripts/test-docker.ps1 e2e-community-node` と `gh act --job format-check` / `--job native-test-linux` を完走。
      - 2026年02月02日: `authStore.bootstrapTopics` の public topic join を非同期化（オンボーディング遅延対策）。`./scripts/test-docker.ps1 e2e` を完走（14 specs pass, 13分37秒、`tmp/logs/desktop-e2e/20260202-115045.log`）。`gh act --job format-check` / `--job native-test-linux` を完走。
      - 2026年02月02日: `generateNewKeypair` の後段初期化（nostr/relay/accounts/topic/avatar）を defer 可能にし、`WelcomeScreen` からは defer で実行して `profile-setup` の表示を先に出す調整。`./scripts/test-docker.ps1 ts` / `./scripts/test-docker.ps1 rust` を完走（act/useRouter の警告は既知）。`gh act --job format-check` / `--job native-test-linux` を完走。
      - 2026年02月03日: community node の labels/trust 署名検証で pubkey ミスマッチを回避する修正を反映。`./scripts/test-docker.ps1 e2e-community-node` と `gh act --job format-check` / `--job native-test-linux` を完走（ログは `tmp/logs/gh-act-*.log`）。
      - 2026年02月15日: PR #26（Issue #22 Task4）CI fix loop 対応として、`cn-core` の環境変数テストを非 `set_var/remove_var` 化し、`cn-admin-api` 契約テストの trigger 初期化を advisory lock で直列化。`gh act --job format-check` / `--job native-test-linux` / `--job community-node-tests` を完走（ログ: `tmp/logs/gh-act-*-issue22-pr26-fix-loop.log`）。
      - 2026年02月15日: Issue #22 最終再監査（PR #23/#24/#25/#26 マージ後）で strict gate を再評価し、Gate1-6 すべて PASS、追加実装タスク 0 を確認。Issue #22 はクローズ可能。

### 2026年02月16日 Issue #27 検索PG移行計画の初期監査（完了）

- 目的: `docs/01_project/activeContext/search_pg_migration/` の PR-01..PR-07 を現行実装へ突合し、実装前ギャップを明確化する。
- 状態: 完了（`docs/01_project/activeContext/tasks/completed/2026-02-16.md` に移管）。
- 実施メモ:
  - 監査結果: `docs/01_project/activeContext/search_pg_migration/issue27_initial_audit_2026-02-16.md`
  - 未着手タスク起票: `docs/01_project/activeContext/tasks/priority/search_pg_migration_roadmap.md`
  - 進捗レポート: `docs/01_project/progressReports/2026-02-16_issue27_search_pg_migration_audit.md`
  - 2026年02月16日: PR-01（拡張導入とランタイムフラグ基盤）完了に伴い、`tasks/completed/2026-02-16.md` へ追記して本ファイルから作業中エントリを削除。
  - 2026年02月16日: PR #28 Community Node Tests fix loop対応（`cn-admin-api` 契約テスト直列化）を完了し、`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr28_community_node_tests_fix_loop.md` へ反映。`gh act` の `format-check` / `native-test-linux` / `community-node-tests` も pass。
  - 2026年02月16日: PR-02（投稿検索ドキュメント + PGroonga read/write 切替）を完了し、`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr02_post_search_pgroonga.md` へ反映。
  - 2026年02月16日: PR #29 fix loop（`post_search_documents` の primary key 競合）対応として、`(post_id, topic_id)` キー化・`ON CONFLICT` 修正・multi-topic 回帰テストを実装。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr29_post_search_topic_key_fix_loop.md` に反映。
  - 2026年02月16日: PR #29 fix loop second pass（Run `22050188054`）として、`default_signature_service` の署名時刻同期と `cn-user-api` 検索契約テストの runtime flag 競合直列化を実施。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr29_second_pass_ci_fix_loop.md` に反映。
  - 2026年02月16日: PR-03（コミュニティ候補生成: `pg_trgm` + prefix）を完了し、`cn_search.community_search_terms` migration、`/v1/communities/suggest`（pg/legacy切替 + fallback）、回帰テストを追加。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr03_community_candidates_pg_trgm.md` に反映。
  - 2026年02月16日: PR #30 fix loop（migration alias backfill の `kukuri:<64hex>` ノイズ候補）対応として、m9 backfill 条件へ hashed tail 除外を追加し、`cn-user-api` 契約テスト `community_search_alias_backfill_skips_kukuri_hashed_tail_topics` を実装。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr30_alias_backfill_hashed_tail_fix_loop.md` に反映。
  - 2026年02月16日: PR #30 fix loop second pass（Run `22052290440` / Job `63712785423`）として、OpenAPI Artifacts Check の `user-api.json` ドリフト（`/v1/communities/suggest` 欠落）を再生成で解消。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr30_second_pass_openapi_artifacts_fix_loop.md` に反映。
  - 2026年02月16日: PR-04（AGE グラフ同期）を完了し、`cn_search.graph_sync_offsets` / `cn_search.user_community_affinity` migration、`cn-index` の outbox 差分同期 + checkpoint 再開 + affinity 再計算ワーカー、統合テスト 2件を追加。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr04_age_graph_sync.md` に反映。
  - 2026年02月16日: PR #31 fix loop（`VIEWED_COMMUNITY` stale edge）を完了し、delete/非アクティブ upsert 時に閲覧エッジを現行イベント集合から再計算して stale edge を削除する修正と回帰テストを追加。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr31_viewed_community_delete_fix_loop.md` に反映。
  - 2026年02月16日: PR-05（2段階サジェスト rerank）を完了し、`/v1/communities/suggest` の Stage-A/Stage-B 接続、`suggest_rerank_mode` + relation weights ランタイムフラグ、block/mute/visibility 最終フィルタ、shadow/enabled 回帰テスト、observability メトリクス追加を実施。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr05_two_stage_suggest_rerank.md` に反映。
  - 2026年02月16日: PR-06（dual-write/backfill/shadow-read）を完了し、`cn_search.backfill_jobs` / `cn_search.backfill_checkpoints` / `cn_search.shadow_read_logs` migration、`cn-index` 片系失敗再送 + backfill worker、`cn-user-api` shadow-read 比較保存を実装。`tasks/completed/2026-02-16.md` と `progressReports/2026-02-16_issue27_pr06_dual_write_backfill_shadow.md` に反映。
