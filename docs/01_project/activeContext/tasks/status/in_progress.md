[title] 作業中タスク（in_progress）

最終更新日: 2025年11月07日

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

### プロフィールアバター UI 連携

- [x] フロントの `ProfileForm` など既存 UI から新 `upload_profile_avatar` / `fetch_profile_avatar` API を呼び出す配線と UX チューニングを実施
  - 2025年11月02日: 現行フォームの画像アップロード導線・プレビュー処理を調査し、新コマンドに合わせたストア更新とエラーハンドリング改善の洗い出しを開始。
  - 2025年11月02日: `ProfileForm` に Tauri ダイアログ経由の画像選択・プレビュー・バリデーションを実装し、`ProfileSetup` / `ProfileEditDialog` から `upload_profile_avatar` / `fetch_profile_avatar` を呼び出すよう接続。アップロード後は `authStore` にメタデータ（`avatar`）とデータURLを反映し、Vitest の関連ユニットテストを更新済み。
  - 2025年11月02日: `authStore` がログイン／初期化／アカウント切替時に `fetch_profile_avatar` を呼び出して `currentUser.avatar` と `picture` を同期し、`AccountSwitcher` / `ReplyForm` / `QuoteForm` でデフォルトアバターにフォールバックするようリファレンスを更新。ユニットテストにリモート取得ケースを追加し、全体テストを実行済み。
  - 2025年11月03日: `ProfileEditDialog` のユニットテストを追加し、アップロード・フェッチ・エラー処理パスを検証。Vitest から `@tauri-apps/plugin-dialog` / `fs` を解決できるよう専用モックと `vitest.config.ts` のエイリアスを追加した上でテストを実行。
- [x] 全体の UI で新アバターメタデータを参照している箇所を洗い、必要に応じて default_avatar フォールバックを適用しきれているか確認
  - 2025年11月02日: `resolveUserAvatarSrc` ユーティリティを追加し、ReplyForm・QuoteForm・PostCard・AccountSwitcher・UserSearchResults のアバター参照を共通化。UserSearchResults 用のフォールバック検証テストを新設し、既存フォーム/ポスト系ユニットテストと併せて `pnpm vitest run ...` で成功を確認。
