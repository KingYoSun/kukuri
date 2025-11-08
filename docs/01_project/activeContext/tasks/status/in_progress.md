[title] 作業中タスク（in_progress）

最終更新日: 2025年11月08日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### GitHub Actions ワークフロー失敗調査

- [ ] `Test` ワークフローの失敗要因を切り分け、ローカルで再現・修正・再検証する
  - 2025年11月07日: `gh run view 19134966693 --log` で `native-test-linux` の TypeScript 型エラーと `format-check` の Rust フォーマッタ失敗を特定。`DirectMessageStore` 初期状態の Omit 漏れと `DirectMessageDialog` の `useInfiniteQuery` ジェネリクス設定ミスを修正し、`cargo fmt` で Rust 側の差分を整形。
  - 2025年11月07日: `npx vitest run` / `npx tsc --noEmit` / `npx eslint …` に加え、`cargo test`・`cargo clippy --all-features -- -D warnings`（`kukuri-tauri/src-tauri`・`kukuri-cli`）を再実行してローカル環境での回帰を確認。
  - 2025年11月07日: `gh act push -j native-test-linux -W .github/workflows/test.yml` を実行し、`Test/Native Test (Linux)` ジョブが `Job succeeded` となることをログ (`act-native-ps.log`) で確認。
  - 2025年11月07日: `profile.$userId.tsx` の `useInfiniteQuery` キー型／`sqlite_repository/users.rs` の `format!` 誘発箇所／`trending_metrics_job.rs` の `or_insert_with` を修正し、`pnpm test`・`pnpm type-check`・`pnpm lint`・`cargo clippy --workspace --all-features`（`kukuri-tauri/src-tauri` / `kukuri-cli`）・`cargo test --workspace --all-features`（`kukuri-cli`）・`scripts/test-docker.ps1 rust -NoBuild`・`gh act push -j native-test-linux -W .github/workflows/test.yml`（ログ: `gh-act-native.log`）で完走を再確認。
  - 2025年11月07日: `gh run view 19154875336 --job 54753280974 --log` で `kukuri-tauri/src-tauri/src/infrastructure/jobs/trending_metrics_job.rs` の `cargo fmt -- --check` 差分のみが残っていることを確認し、`cargo fmt` で再整形。`cargo test --workspace --all-features`（`kukuri-tauri/src-tauri`）/`cargo test --all-features`（`kukuri-cli`）/`pnpm format:check`/`python scripts/check_date_format.py` に加え、`gh act push -j format-check -W .github/workflows/test.yml`（ログ: `gh-act-format.log`）でフォーマットジョブ成功をローカル再現。
  - 2025年11月08日: `gh run view 19172338059 --job 54807981752 --log` を `tmp/logs/nightly_trending_feed_19172338059.log` に保存し、`ERR_PNPM_RECURSIVE_EXEC_FIRST_FAIL: Command "vitest" not found` で `Trending Feed (Docker)` が失敗していることを確認。`docker-compose.test.yml` の `ts-test`/`lint-check` からコードの bind mount を外し、`scripts/test-docker.sh` の `trending-feed` シナリオを `pnpm install --frozen-lockfile --ignore-workspace` のフォールバック＆`pnpm vitest run` 直呼びに変更（`--runInBand` は Vitest v3 では未サポートのため削除）。`./scripts/test-docker.sh ts --scenario trending-feed`・`--no-build`、および `gh act workflow_dispatch -W .github/workflows/nightly.yml -j trending-feed --bind` でローカル再現し、テスト実行は成功するが `act` 環境では `ACTIONS_RUNTIME_TOKEN` が無いためアーティファクトアップロードのみ失敗することを `tmp/logs/gh_act_nightly_trending_before_fix.log`／`tmp/logs/gh_act_nightly_trending_after_fix.log` に記録。さらに、Vitest の対象を `trending/following/routes` の 3 ファイルで常に走らせるため `scripts/test-docker.sh` 内でファイル単位に `pnpm vitest run` を分割し、抜け漏れがあった場合は即座に失敗するよう調整。`docker compose --project-name kukuri_tests build --no-cache ts-test` でローカルキャッシュを更新した上で `./scripts/test-docker.sh ts --scenario trending-feed (--no-build)` と `gh act workflow_dispatch -W .github/workflows/nightly.yml -j trending-feed --bind`（ログ: `tmp/logs/gh_act_nightly_trending_after_split.log`）を再実行し、3つの JSON レポートが毎回生成されることを確認（`act` では引き続きアーティファクトアップロードのみ権限不足）。
  - 2025年11月08日: `Test/Native Test (Linux)` の失敗原因となっていた `Header` 関連テストの待ち条件と `DirectMessageInbox` 周りのポータル描画ずれを是正。`DirectMessageInbox` に SR-only の告知テキストを追加しつつ、`Header.test.tsx` 側では `Dialog`/`DirectMessageInbox` を簡易コンポーネントにモック化し `openInbox` をスパイする形へ修正。さらに `SyncStatusIndicator` のキャッシュ統計表示で camelCase/snake_case のズレ、`AddToSyncQueueRequest` の型定義、`Button` サイズ指定、`directMessageStore` の初期型漏れを修正し Prettier も実行。`pnpm test`/`pnpm lint`/`cargo test`（Docker 経由）/`cargo test`（kukuri-cli）と `gh act push -j native-test-linux -W .github/workflows/test.yml` を実行して緑化ログ（`tmp/logs/pnpm-test.log`, `tmp/logs/test-docker-rust-20251108.log`, `tmp/logs/gh-act-native-20251108.log`）を保存済み。
  - 2025年11月08日: `gh run view 19188900203 --job 54860361705 --log` を `tmp/logs/gha_format_19188900203.log` に保存し、`Format Check / Check TypeScript formatting` が `Header.test.tsx` の Prettier 差分警告で失敗していることを特定。`pnpm prettier --write src/tests/unit/components/layout/Header.test.tsx` で整形してから `pnpm format:check` と `gh act push -j format-check -W .github/workflows/test.yml`（ログ: `tmp/logs/gh_act_test_run_20251108.log`）を実行し、ローカルでフォーマットジョブ成功を再現。

### Clippy 警告ゼロ体制の復帰

- [x] `domain/entities/event/validation/nip19.rs` の `format!` 文字列を埋め込み式へ置換し、Clippy (`uninlined_format_args`) を解消
- [x] `infrastructure/p2p/dht_integration.rs` の `format!` 文字列を修正し、`AppError::DeserializationError` 周辺のログ表現を統一
- [x] `cargo clippy --all-features -- -D warnings` を `kukuri-tauri/src-tauri` で再実行し、警告ゼロを確認（ログ取得・`phase5_ci_path_audit.md` へ反映）
- [x] `kukuri-cli` 側でも `cargo clippy --all-features -- -D warnings` を実行し、警告ゼロ継続を確認
- [x] Clippy 対応後に `refactoring_plan_2025-08-08_v3.md` の成功指標欄を更新し、再発防止タスクを記録

### ユーザー導線ドキュメント整備

- [ ] UI から到達可能な機能一覧を棚卸しし、`docs/01_project/activeContext/artefacts/` 配下にサマリードキュメントを作成
  - 2025年11月01日: 主要画面（Welcome/Home/Topics/Search/Settings/Debugパネル）とサイドバー導線を確認。未リンク状態の要素（Sidebar「トレンド」「フォロー中」、UserSearchResultsの`/profile/$userId`リンク）を記録済み。
  - 2025年11月01日: `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` を起票し、画面別導線と改善候補を整理。
  - 2025年11月02日: グローバル要素セクションにサイドバー未読バッジ/最終活動更新ロジック、PostComposer の下書き運用、offlineSyncService の同期フローを追記し、TopicPage の最終更新表示（秒→ミリ秒換算）の対処内容を整理。
  - 2025年11月02日: `RootRoute` / `MainLayout` の認証ガードと設定画面のプライバシートグル未連携を追記して導線ギャップを更新。
  - 2025年11月02日: セクション5を追加し、設定画面のプライバシー/プロフィール導線とサイドバー新規投稿導線の具体的な実装案・テスト計画を整理。
  - 2025年11月02日: `usePrivacySettingsStore` と `ProfileEditDialog` を実装し、設定画面からプロフィール編集モーダルを起動できるようにした。
  - 2025年11月02日: `useComposerStore` / `GlobalComposer` を追加し、Home/Sidebar の「投稿する」「新規投稿」ボタンからグローバルコンポーザーを開閉できるように接続。
  - 2025年11月02日: 次ステップ候補を整理 — (1) プライバシー設定のバックエンド連携/API 仕様検討、(2) グローバルコンポーザーのUI/UX改善（トピック選択・ショートカット）の要件定義。
  - 2025年11月03日: プロフィール画像アップロード導線をリモート同期必須に更新し、iroh-blobs 0.96.0 / iroh-docs 0.94.0 を用いた設計案を `phase5_user_flow_inventory.md` に追記。鍵管理モーダルとの連携方針を整理し、ギャップ欄に画像アップロード実装待ちを登録。
  - 2025年11月03日: 共有スコープの Capability 分岐と `StreamEncryptor` 採用、外部 URL フォールバック廃止を決定し、データ構造・処理フロー・セキュリティ節へ反映。
  - 2025年11月03日: サイドバーのステータスカードと同期インジケーター群の挙動を再チェックし、`phase5_user_flow_inventory.md` セクション1.6/5.5にポーリング条件・エラー表示ギャップ・テスト計画を追記。`tauri_app_implementation_plan.md` Phase 5 記録を更新し、`phase5_ci_path_audit.md` のテスト項目とリンク付け。
  - 2025年11月03日: `RelayStatus` / `P2PStatus` ユニットテストをフェイクタイマー対応で刷新し、`npx vitest run src/tests/unit/components/RelayStatus.test.tsx src/tests/unit/components/P2PStatus.test.tsx` を実行。5.5節に `get_relay_status` エラー UI / リトライ設計を追加し、`p2p_mainline_runbook.md` へ `get_p2p_status` 拡張準備メモ（connection_status/peers）を追記。
  - 2025年11月03日: バックオフ実装後の検証として `npx vitest run … components/stores/hooks/lib/api` と `cargo test`（`kukuri-tauri/src-tauri`・`kukuri-cli`）を実施し、Runbook と CI パス監査に反映。`phase5_user_flow_inventory.md` 5.5節を実装結果ベースの記述へ更新。
  - 2025年11月03日: UI 導線の状態を一覧できるサマリードキュメント `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md` を作成。稼働中/改善中/未実装の別と主要ギャップを整理し、`phase5_ci_path_audit.md`・`tauri_app_implementation_plan.md`・`refactoring_plan_2025-08-08_v3.md` へ参照リンクを追加。
  - 2025年11月03日: ユーザー検索導線と `/profile/$userId` の現状を反映するため、`phase5_user_flow_inventory.md` 2章/3章/4章を更新し、未使用コマンドに `get_followers` / `get_following` を追記。あわせて `phase5_user_flow_summary.md` の Quick View と優先度表を最新化。
  - 2025年11月03日: `/profile/$userId` ルートをプレースホルダーから差し替え、`get_user` / `get_posts(author_pubkey)` 連携でプロフィール基本情報と投稿一覧を表示。フォロー・フォロワー導線は backlog として整理。
  - 2025年11月03日: サマリーを用いて未実装導線の優先度を再評価し、`phase5_user_flow_summary.md` に優先度テーブルを追記。`tauri_app_implementation_plan.md` の Phase 5 優先度リストを更新し、`/profile/$userId` ルート実装・投稿削除導線・鍵管理ダイアログを最優先に再配置。
  - 2025年11月04日: `phase5_user_flow_inventory.md` に 1.7「プロフィール詳細」と 5.6「フォロー体験」節を追加し、1.4 のユーザー検索行を現行実装（`search_users`/フォロー操作）に更新。`phase5_user_flow_summary.md` に `/profile/$userId` の導線行とグローバル要素「プロフィール導線」を追記し、優先度表の `/profile` 行を最新化。
  - 2025年11月04日: DirectMessageDialog のスケルトンを基にコマンド/ストアの挙動を確認し、ユニットテスト `src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx` を整備。`pnpm test:unit` を実行して DM ダイアログ・プロフィール導線の回帰を確認し、`phase5_user_flow_summary.md` に DM 導線のテスト状況を追記。
  - 2025年11月04日: `phase5_user_flow_inventory.md` にダイレクトメッセージ節を追加し、`send_direct_message` が UI 接続済みながら Tauri で `NotImplemented` を返している点と `list_direct_messages` 未配線を明記。併せて残課題・次アクションを更新し、`phase5_user_flow_summary.md` のプロフィール行・グローバル要素・優先度表を最新の状況に差し替え。
  - 2025年11月04日: Inventory 5.6.1/5.6.2 に `send_direct_message` / `list_direct_messages` Tauri 実装計画（サービス/ポート/リポジトリ/マイグレーション/テスト）とフォロワー一覧ソート・ページネーション設計を詳細化。サマリードキュメントの Quick View / 優先度表を同計画と紐付け、React Query・Rust・Docker テストのフォローアップ項目を明文化。
  - 2025年11月05日: `/trending`・`/following` 用に `list_trending_topics` / `list_trending_posts` / `list_following_feed` Tauri コマンドと React Query フック（`useTrendingTopicsQuery` / `useTrendingPostsQuery` / `useFollowingFeedQuery`）を実装。`pnpm vitest run src/tests/unit/hooks/useTrendingFeeds.test.tsx`・`cargo test`（`kukuri-tauri/src-tauri` / `kukuri-cli`）・`docker compose -f docker-compose.test.yml up --build --abort-on-container-exit --exit-code-from test-runner test-runner` を完走し、トレンド/フォロー系のユニット・統合テスト基盤を整備。
  - 2025年11月05日: `/trending`・`/following` ページコンポーネントを実装し、UI テスト `src/tests/unit/routes/trending.test.tsx`・`src/tests/unit/routes/following.test.tsx` を追加。`node_modules/.bin/vitest run src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx` でローディング/エラー/空状態の表示を検証し、Sidebar カテゴリーから新ルートへ遷移できるように接続。
  - 2025年11月05日: Direct Message 導線の進捗を反映し、`phase5_user_flow_inventory.md` 5.6.1 を実装状況ベースに更新。`send_direct_message` / `list_direct_messages` の Tauri サービス内容とテスト結果を追記し、`phase5_user_flow_summary.md` のプロフィール行・Quick View・優先度表も同内容に同期。
  - 2025年11月05日: `DirectMessageDialog` を `useInfiniteQuery` と `list_direct_messages` で接続し、IntersectionObserver 無限スクロール・既読リセット・エラー/再試行 UI を実装。`pnpm vitest run src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx` を再実行して回帰を確認し、Inventory 5.6 / Summary 1.2・2・3・6 節に React Query 連携と残課題（Kind4 IPC・会話リスト未読・フォロワーソート）を反映。
  - 2025年11月06日: `phase5_user_flow_inventory.md` 5.7節にトレンド/フォロー中のデータ要件（limit/per_topic/cursor/キャッシュ方針）とテスト計画を追記し、`generated_at` ミリ秒化・Summary Panel・`trending_metrics_job`・Docker シナリオをフォロー項目として明記。`phase5_user_flow_summary.md` と `tauri_app_implementation_plan.md` を同内容に更新し、Quick View / 優先度表 / テストカバレッジを同期。
  - [x] ユーザー検索タブのページネーション／エラー UI／入力バリデーション方針を「ユーザー検索導線改善」節として整理し、`phase5_user_flow_summary.md` と `tauri_app_implementation_plan.md`、`docs/03_implementation/error_handling_guidelines.md` に反映する。
    - 2025年11月06日: Inventory 5.8 に状態遷移図と入力ガード（2〜64文字・制御文字除去・`#`/`@` 補助検索）を追記し、Summary・Phase5計画・エラーハンドリングガイドラインへ `SearchErrorState`/React Query デバウンス方針を同期。`tauri_app_implementation_plan.md` の優先度項目とエラーキー記載を更新。
  - [x] ホーム/サイドバーからのトピック作成導線（グローバルコンポーザー連携案）の仕様・依存コマンド・テスト計画をまとめ、Inventory と Summary に追記する。
    - 2025年11月06日: `phase5_user_flow_inventory.md` 5.9 節を追加し、TopicSelector ショートカット／`createAndJoinTopic` ヘルパー／`OfflineActionType::CREATE_TOPIC`／テスト計画を整理。`phase5_user_flow_summary.md`・`tauri_app_implementation_plan.md`・`phase5_ci_path_audit.md` に同内容を連携。
  - [x] 投稿削除後の React Query キャッシュ無効化と `delete_post` 統合テスト整備をフォローアップする文書（Inventory・Summary・`phase5_ci_path_audit.md`）を作成し、進捗と整合性を確認する。
    - 2025年11月06日: Inventory 5.10 に `useDeletePost` ミューテーション／`invalidatePostCaches` 方針／Docker シナリオとテスト計画を追記し、Summary・実装計画・CI 監査の各ドキュメントを更新。`phase5_ci_path_audit.md` に `post-delete-cache` テスト ID を登録。
  - [x] DM/フォロワー/プロフィール画像導線のフォローアップ項目に合わせ、`docs/03_implementation/error_handling_guidelines.md`・`phase5_dependency_inventory_template.md`・`tauri_app_implementation_plan.md` の該当節を更新する。
    - 2025年11月06日: エラーハンドリングガイドラインへ `Topic.*` / `Post.*` / `DirectMessage.*` / `ProfileAvatar.upload_failed` キーを追加し、依存棚卸しに `useDeletePost`・`GlobalComposer TopicCreation`・`DirectMessageService`・`ProfileAvatarSync` 行を新設。実装計画 Phase 5 の優先項目へ新タスクを追加。
  - [x] `phase5_user_flow_summary.md` と `phase5_user_flow_inventory.md` の最終更新日・Quick View・優先度表・ギャップ一覧が上記残タスクと整合するよう再確認し、必要な差分を反映する。
    - 2025年11月06日: `/trending`・`/following` で利用している `generated_at` / `server_time` が `timestamp_millis` で返却されていることをコードベース（`topic_handler.rs` / `post_handler.rs`）で確認し、Inventory 5.7 および Summary の記述を更新。Quick View と優先度表からミリ秒化フォローアップを削除し、未対応項目（Summary Panel・Docker シナリオ・DM 未読ハイライト）のみに絞り込んだ。
  - [x] Summary Panel / DM 未読ハイライト / Docker シナリオ / `trending_metrics_job` の着手順序とタスク粒度を整理し、5.7 節・Summary・実装計画に反映する。
    - 2025年11月06日: 5.7 節に「Summary Panel → DM 未読ハイライト → Docker シナリオ → `trending_metrics_job`」の順序と各ステップの前提・テスト計画を追記。Summary Quick View と優先度表に同順序を連動させ、タスク着手時の参照位置を明確化した。
  - 2025年11月06日: `TrendingSummaryPanel` / `FollowingSummaryPanel` を実装し、派生メトリクス（件数・平均スコア・最終更新・残ページ）を表示。`pnpm vitest run src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx` を実行して新UIの挙動を確認。
  - 2025年11月07日: `/profile/$userId` のメッセージ導線が稼働していることを確認し、`phase5_user_flow_inventory.md` のサマリー表・ギャップ欄・5.6.1「残課題」を更新。`phase5_user_flow_summary.md` の最終更新日も同期。
  - 2025年11月07日: フォロワー/フォロー一覧にソート（最新/古い/名前）、検索、`totalCount` 表示を追加。Rust 側は `FollowListSort` と新カーソル形式を導入し、`pnpm vitest run src/tests/unit/routes/profile.$userId.test.tsx`・`cargo test`（`kukuri-tauri/src-tauri` は `STATUS_ENTRYPOINT_NOT_FOUND` で異常終了、`kukuri-cli` は成功）で検証。ドキュメント（Inventory 5.6.2 / Summary / in_progress）を更新。
  - 2025年11月07日: `/trending` `/following` 導線の手動 QA を実施し、`phase5_user_flow_inventory.md` 2章・5.7節、`phase5_user_flow_summary.md` Quick View、`phase5_ci_path_audit.md` のテスト項目を更新。Summary Panel 指標と Nightly テストコマンドの整合性を確認しつつ、Docker シナリオ `trending-feed` と `trending_metrics_job` を backlog として明記。
  - 2025年11月07日: Inventory 3.2/3.3 を再編し、`get_cache_status` / `add_to_sync_queue` / `update_cache_metadata` / `update_sync_status` を「連携済み・監視対象」へ移行。未接続 API を `join_topic_by_name` / `delete_events` / `add_relay` / `get_nostr_pubkey` / `clear_all_accounts_for_test` に絞り、`phase5_user_flow_summary.md` Quick View 項目9 と Phase 5 backlog の優先度を同期。
  - 2025年11月07日: Docker シナリオ `trending-feed` の要件を整理し、`phase5_user_flow_inventory.md` 5.7節・`phase5_ci_path_audit.md`・`phase5_user_flow_summary.md`・`docs/03_implementation/docker_test_environment.md`・`windows_test_docker_runbook.md` へ反映。`scripts/test-docker.{sh,ps1}` の `--scenario/-Scenario` 追加と Nightly `Trending Feed (Docker)` ジョブの組み込み方針を明文化。
  - 2025年11月07日: `docs/03_implementation/trending_metrics_job.md` を起草し、集計ジョブの要件・アーキテクチャ・監視指針・テスト計画を整理。`phase5_dependency_inventory_template.md` に `TrendingMetricsJob` 行を追加し、実装と CI 連携の依存を記録。
  - 2025年11月07日: `scripts/test-docker.{sh,ps1}` に `--scenario/-Scenario trending-feed` 実装と成果物出力処理を追加し、Nightly `Trending Feed (Docker)` ジョブ（`nightly.yml`）を稼働化。`topic_metrics` 用 migration（`20251107094500_*`）と `sqlx prepare` を実行し、`TopicMetricsRepository`・`TrendingMetricsJob`（ステップ1〜3）を Rust 層へ追加。
- [x] 投稿削除導線（delete_post）実装準備
  - 2025年11月03日: `phase5_user_flow_inventory.md` セクション5.6に UX / バックエンド / テスト仕様を整理。`delete_post` コマンドを活用した削除フローと楽観更新、オフライン再送の方針を定義。
  - 2025年11月03日: `phase5_user_flow_summary.md` 優先度表を更新し、本タスクを Priority A として再掲。`PostCard` に削除メニューと確認ダイアログを追加し、`postStore.deletePostRemote` をオフライン対応込みで接続。関連ユニットテスト（PostCard / postStore）を更新。
  - 2025年11月03日: `pnpm vitest run src/tests/unit/components/posts/PostCard.test.tsx src/tests/unit/stores/postStore.test.ts` を再実行し、削除メニューの回帰とオフライン時に `TauriApi.deletePost` が呼ばれないことを確認。`phase5_ci_path_audit.md` / `tauri_app_implementation_plan.md` に結果を反映済み。
- [x] Tauri コマンド呼び出し状況（フロントエンド `invoke` 探索結果）と未使用 API の整理結果をドキュメントへ反映
  - 2025年11月01日: `TauriApi`・`SecureStorageApi`・`p2pApi`・`nostr`ユーティリティの `invoke` 使用箇所を洗い出し、未使用コマンド候補（例: `add_relay`, `subscribe_to_user`, `join_topic_by_name` など）を整理中。
  - 2025年11月01日: `offlineApi` 系コマンドと `syncEngine` の直接 `invoke`（`get_post_metadata` など）を棚卸しし、`phase5_user_flow_inventory.md` の 3.1/3.2 に追記。
  - 2025年11月02日: `invokeCommand` / `invoke` 呼び出しをスクリプトで抽出し、統合テスト専用コマンド群（`import_key` ほか）と未使用 API を `phase5_user_flow_inventory.md` 3.2/3.3 に反映、併せて 1.6/3.1 の補足内容を更新。
- [ ] `refactoring_plan_2025-08-08_v3.md` のユーザー導線指標チェックボックスを更新し、未達項目のフォロータスクを連携
  - 2025年11月01日: 「UIから到達可能な全機能の文書化完了」を達成済みに更新し、参照ドキュメントと更新日を記録。
  - 2025年11月02日: 指標欄に最新ドキュメント更新（統合テスト専用コマンド整理）と未導線APIの整理状況を追記、Phase2.5セクションへ `TopicPage` 最終更新バグの改善候補を登録。
  - 2025年11月03日: 2.5節を棚卸し結果サマリーへ差し替え、Relay/P2Pステータスカードとプロフィール編集導線の進捗メモを追記。
  - 2025年11月04日: 2.5節に `/profile/$userId` 導線の現状（フォロー/フォロワー表示）と残課題を反映し、ユーザー導線指標欄へ 1.4/1.7/5.6 更新（ユーザー検索・プロフィール導線・フォロー体験）を記録。
  - 2025年11月04日: Rust 側で `direct_message_service` / `messaging_gateway` / `direct_message_repository` を実装し、`send_direct_message` と `list_direct_messages` コマンドを配線。kind4 DM の暗号化送信・SQLite 永続化・カーソルページング・復号レスポンスまで通し、`cargo sqlx prepare` → `cargo test`（`kukuri-tauri/src-tauri -q` / `kukuri-cli`）を実行して新規ユニットテストを含めて確認。
  - 2025年11月06日: 指標欄に Inventory 5.7-5.10 / Summary 1.2・2・3 の追記事項と未使用 API・Nightly テスト更新を反映し、未接続コマンドの残課題を Phase 5 backlog と同期。
  - 2025年11月07日: `refactoring_plan_2025-08-08_v3.md` のユーザー導線指標を最新化し、Inventory 5.6.1/5.6.2 と 5.11、`phase5_user_flow_summary.md` Quick View、`phase5_ci_path_audit.md` のテスト行を引用して SyncStatusIndicator / プロフィール導線 / フォロー体験の進捗記録と Tauri コマンド使用状況を追記。最終更新日も 2025年11月07日へ更新。
  - 2025年11月08日: `phase5_user_flow_inventory.md` を 2025年11月08日付に更新し、5.12 節でヘッダー `MessageCircle` ボタン／`useDirectMessageBadge`／`TrendingSummaryPanel`・`FollowingSummaryPanel` の DM 連携と課題（新規会話不可・Summary Panel に CTA/テスト無し・未読カウンタ未永続化）を整理し、Hook/IPC/UI それぞれのフォローアップを追記。
- [ ] 作成した資料を `phase5_ci_path_audit.md` / `tauri_app_implementation_plan.md` へリンクし、タスク完了後に in_progress.md を更新予定
  - 2025年11月01日: `phase5_ci_path_audit.md` に関連ドキュメントリンクを追加し、`tauri_app_implementation_plan.md` Phase 5 セクションから参照を追記。
  - 2025年11月02日: 上記 2 ドキュメントを最新内容に合わせて再更新し、最終更新日と追記内容にユーザー導線棚卸しの差分を反映。
  - 2025年11月02日: Phase 5 backlog の優先順位を再定義（投稿導線統一→プロフィール編集→プライバシー設定→トレンド/フォロー中→テスト整備）し、`tauri_app_implementation_plan.md` に反映。
  - 2025年11月03日: `phase5_ci_path_audit.md` に Relay/P2P ステータスカードのユニットテスト計画と `profileAvatarSync` 統合テスト計画を追記。`tauri_app_implementation_plan.md` へステータスカード検証・鍵管理モーダル・プロフィール画像リモート同期を Phase 5 優先度として追加。
  - 2025年11月04日: `phase5_ci_path_audit.md` の最終更新日と関連ドキュメント欄を更新し、ユーザー検索/プロフィール導線の差分反映と整合性を確認。追加で「追加予定のテスト」節を起票し、ProfilePage フォロー導線・DirectMessageDialog・フォロワー無限スクロールのテスト計画を記録。
  - 2025年11月05日: `Sidebar` のカテゴリー状態管理とクエリプリフェッチを実装し、`phase5_user_flow_inventory.md`（5.7節）と `phase5_user_flow_summary.md` のトレンド/フォロー導線を更新。`npx vitest run src/tests/unit/components/layout/Sidebar.test.tsx src/tests/unit/stores/uiStore.test.ts src/tests/unit/hooks/useTrendingFeeds.test.tsx` を実行してカテゴリ同期とプリフェッチの回帰を確認。
  - 2025年11月06日: Kind4 IPC で DM 受信→永続化→通知まで一貫処理を追加し、`useDirectMessageEvents` / `useDirectMessageBadge` を導入。ヘッダーと Trending/Following Summary Panel に未読バッジを表示、`DirectMessageDialog` に送信失敗時の再送ボタンを実装。`npx vitest run src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx src/tests/unit/components/layout/Header.test.tsx` と `cargo test`（`kukuri-tauri/src-tauri`・`kukuri-cli`）を実行し、導線ドキュメント（inventory/summary）を更新。
  - 2025年11月06日: ダイレクトメッセージ履歴導線の現状を `phase5_user_flow_inventory.md` / `phase5_user_flow_summary.md` に反映し、`list_direct_messages` を UI 接続済みとして整理。未使用コマンド表を更新し、Quick View/次アクションを DM 未読バッジ・送信再試行・IPC 連携のフォローアップへ書き換え。
  - 2025年11月06日: `phase5_ci_path_audit.md` に test:unit 更新と関連資料リンクを追加し、`tauri_app_implementation_plan.md` の Phase 5 メモへ DM 未読バッジ・Summary Panel 反映の記録を追記。`refactoring_plan_2025-08-08_v3.md` の指標更新と整合を確認。
  - 2025年11月06日: `useOfflineStore.refreshCacheMetadata` / `useSyncManager.persistSyncStatuses` を実装し、同期完了時に `update_cache_metadata` / `update_sync_status` を自動呼び出し。`pnpm vitest run src/tests/unit/stores/offlineStore.test.ts` でメタデータ更新のユニットテストを追加し、`cargo test`（`kukuri-tauri/src-tauri`）は既知の `STATUS_ENTRYPOINT_NOT_FOUND` で停止することを確認（再実行は保留）。
  - 2025年11月06日: Inventory 3.2/3.3 の未接続コマンドを再評価し、`update_cache_metadata` → `update_sync_status` → `get_cache_status` → `add_to_sync_queue` を最優先グループとする対応順を決定。続いて `join_topic_by_name` / `delete_events` / `add_relay` / `get_nostr_pubkey` / `clear_all_accounts_for_test` の順に Phase 5 backlog を進める方針を記録。
  - 2025年11月07日: `phase5_user_flow_inventory.md` に 5.11「SyncStatusIndicator とオフライン同期導線」を追加し、`useSyncManager` / `offlineStore` / `offlineApi.update_cache_metadata` / `update_sync_status` の流れとギャップ・テスト計画を整理。あわせて `phase5_user_flow_summary.md` のグローバル要素/Quick View を同期導線の最新状態に更新し、`SyncStatusIndicator` と `OfflineIndicator` の役割分担＆API未連携課題を反映。
  - 2025年11月07日: `get_cache_status` / `add_to_sync_queue` を `useSyncManager` / `SyncStatusIndicator` に組み込み、キャッシュ統計表示・再送キュー追加ボタン・手動更新ボタンを実装。`useSyncManager` に `cacheStatus` ステートと `enqueueSyncRequest` を追加し、`npx vitest run src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx` でキャッシュ周りのユニットテストを整備。ドキュメント（Inventory 5.11 / Summary Quick View）を最新化。
  - 2025年11月07日: `phase5_ci_path_audit.md` に SyncStatusIndicator/useSyncManager テスト行と `phase5_user_flow_summary.md` 2025年11月07日版へのリンクを追加し、`tauri_app_implementation_plan.md` Phase 5 セクションへ Inventory 5.6.1/5.6.2・5.11 / Summary Quick View の参照と Nightly テスト更新ログを追記。両ファイルの最終更新日を 2025年11月07日へ揃え、導線タスクとの紐付けを明示。
  - 2025年11月07日: GH Actions Run ID `19172338059`（Nightly Frontend Unit Tests）の `Frontend Unit Tests` ジョブログから `src/tests/unit/hooks/useSyncManager.test.tsx` / `src/tests/unit/components/SyncStatusIndicator.test.tsx` 実行を確認し、`phase5_ci_path_audit.md` の SyncStatus 行へ証跡を追記。Trending Feed (Docker) ジョブ失敗により artefact が得られなかった点は別タスクでフォローする。
  - 2025年11月08日: `phase5_user_flow_summary.md` を 2025年11月08日付に更新し、1.6「ダイレクトメッセージ」節とヘッダー/ Summary Panel のギャップ、Quick View 番号の見直し、グローバル要素の DM 表示仕様を追記。`refactoring_plan_2025-08-08_v3.md` 2.5節の参照日付とギャップ一覧を同内容で更新し、ヘッダー DM ボタン/CTA 不足/未接続 API を同期。
  - 2025年11月08日: `phase5_ci_path_audit.md` の `test:unit` 行に `Header.test.tsx` / `useDirectMessageBadge.test.tsx` / `TrendingSummaryPanel.test.tsx` / `FollowingSummaryPanel.test.tsx` を追記し、Nightly Frontend Unit Tests で DM Inbox / Summary CTA 回帰を監視できるようにした。
  - 2025年11月08日: `direct_message_conversations` テーブルを追加し、`direct_message_service` に会話一覧 (`list_direct_message_conversations`) と既読更新 (`mark_conversation_as_read`) を実装。Tauri 側に `list_direct_message_conversations` / `mark_direct_message_conversation_read` コマンドを追加し、`DirectMessageDialog`・`useDirectMessageEvents`・`useDirectMessageBootstrap`・`directMessageStore` を連携して Inbox の初期表示と未読永続化を実現。`pnpm vitest run src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx src/tests/unit/components/layout/Header.test.tsx`、`cargo test`（`kukuri-tauri/src-tauri` は既知の `STATUS_ENTRYPOINT_NOT_FOUND` で停止）、`./scripts/test-docker.ps1 rust`、`cargo test`（`kukuri-cli`）を実行し、Docker 経由では Rust テストが完走することを確認。
  - 2025年11月08日: Inventory 1.2 のヘッダー行を `MessageCircle`/`Plus` ボタン + `DirectMessageInbox` の導線に差し替え、Summary 2章の「プロフィール導線」を Inbox/CTA 対応へ更新。`refactoring_plan_2025-08-08_v3.md` と `tauri_app_implementation_plan.md` に 11月8日分の記録を追加し、会話リスト永続化・宛先検索/補完・未読共有 API を Phase 5 backlog に追記。

### プロフィールアバター UI 連携

- [x] フロントの `ProfileForm` など既存 UI から新 `upload_profile_avatar` / `fetch_profile_avatar` API を呼び出す配線と UX チューニングを実施
  - 2025年11月02日: 現行フォームの画像アップロード導線・プレビュー処理を調査し、新コマンドに合わせたストア更新とエラーハンドリング改善の洗い出しを開始。
  - 2025年11月02日: `ProfileForm` に Tauri ダイアログ経由の画像選択・プレビュー・バリデーションを実装し、`ProfileSetup` / `ProfileEditDialog` から `upload_profile_avatar` / `fetch_profile_avatar` を呼び出すよう接続。アップロード後は `authStore` にメタデータ（`avatar`）とデータURLを反映し、Vitest の関連ユニットテストを更新済み。
  - 2025年11月02日: `authStore` がログイン／初期化／アカウント切替時に `fetch_profile_avatar` を呼び出して `currentUser.avatar` と `picture` を同期し、`AccountSwitcher` / `ReplyForm` / `QuoteForm` でデフォルトアバターにフォールバックするようリファレンスを更新。ユニットテストにリモート取得ケースを追加し、全体テストを実行済み。
  - 2025年11月03日: `ProfileEditDialog` のユニットテストを追加し、アップロード・フェッチ・エラー処理パスを検証。Vitest から `@tauri-apps/plugin-dialog` / `fs` を解決できるよう専用モックと `vitest.config.ts` のエイリアスを追加した上でテストを実行。
- [x] 全体の UI で新アバターメタデータを参照している箇所を洗い、必要に応じて default_avatar フォールバックを適用しきれているか確認
  - 2025年11月02日: `resolveUserAvatarSrc` ユーティリティを追加し、ReplyForm・QuoteForm・PostCard・AccountSwitcher・UserSearchResults のアバター参照を共通化。UserSearchResults 用のフォールバック検証テストを新設し、既存フォーム/ポスト系ユニットテストと併せて `pnpm vitest run ...` で成功を確認。
