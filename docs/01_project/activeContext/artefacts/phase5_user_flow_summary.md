# Phase 5 ユーザー導線サマリー
作成日: 2025年11月03日  
最終更新: 2025年11月11日

## 概要
- Phase 5 時点でアプリ UI から到達できる体験を俯瞰し、欠落導線や改善ポイントを即座に把握できるようにする。
- 詳細なフロー・API 連携・設計メモは `phase5_user_flow_inventory.md` を参照し、本書では意思決定に必要なサマリーのみを掲載。
- 導線の状態は「稼働中」「改善中」「未実装」の 3 区分で整理し、次の対応タスクを明示する。

## MVP残タスクハイライト（2025年11月10日更新）
- **トレンド/フォロー**: Summary Panel と `/trending` `/following` UI は安定。2025年11月10日に `corepack enable pnpm` → `pnpm install --frozen-lockfile` を通し、`pnpm vitest run …` と Docker `./scripts/test-docker.sh ts --scenario trending-feed --no-build` をローカルで再実行（`tmp/logs/vitest_trending_topics_20251110020449.log` / `trending-feed_20251110020528.log`）。同日に `KUKURI_METRICS_PROMETHEUS_PORT` / `KUKURI_METRICS_EMIT_HISTOGRAM` を追加し、`curl http://localhost:<port>/metrics` で `trending_metrics_job_*` 指標を確認できるようになった。また `p2p_metrics_export --job trending`（`./scripts/metrics/export-p2p.sh --job trending --pretty --limit 50`）で `test-results/trending-feed/metrics/<timestamp>-trending-metrics.json` を生成できるようにし、`window_start_ms` / `lag_ms` / `score_weights` を Runbook・CI へ連携。Stage4 残は Docker `prometheus-trending` サービスと Nightly 成果物の保存先整理。
- **Direct Message / Inbox**: Kind4 IPC / `DirectMessageDialog` の無限スクロールは稼働中だが、`mark_direct_message_conversation_read` の多端末同期と会話検索・候補補完、`Header.test.tsx` / `DirectMessageDialog.test.tsx` / `FollowingSummaryPanel.test.tsx` の再実行ログが欠落している（Inventory 5.4 / 5.6.x）。
- **プロフィール/設定**: Stage3（Doc/Blob + privacy）を 2025年11月10日に完了し、`ProfileEditDialog` / `ProfileSetup` が `update_privacy_settings` → `upload_profile_avatar` → `useProfileAvatarSync` を直列実行。Service Worker (`profileAvatarSyncSW.ts`) と BroadcastChannel 連携を追加し、`useProfileAvatarSync` がワーカー経由で自動同期ジョブを処理できるようにした。`scripts/test-docker.{sh,ps1} ts -Scenario profile-avatar-sync` と `./scripts/test-docker.ps1 rust -Test profile_avatar_sync` のログ登録が残課題（Inventory 5.1, Sec.6）。
- **ユーザー検索**: `useUserSearchQuery` と `UserSearchResults` に cursor/sort／レートリミット UI／無限スクロールを実装し、2025年11月10日に `allow_incomplete` フォールバックと SearchBar 警告スタイル・補助検索ラベルを追加。同日に `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build`（PowerShell 版: `.\scripts\test-docker.ps1 ts -Scenario user-search-pagination -NoBuild`）を実装・実行し、`tmp/logs/user_search_pagination_20251110-142854.log` を採取。残課題は Nightly Frontend Unit Tests へのシナリオ組み込みと `test-results/user-search-pagination/*.json` の固定化。
- **Offline sync**: `SyncStatusIndicator` は `list_sync_queue_items` の履歴表示と `cache_types.metadata` 整形まで完了したが、Doc/Blob 対応の `cache_metadata` マイグレーションと conflict バナー / Service Worker (Phase4) の Runbook/CI 連携が未着手（Inventory 5.5 / 5.11）。
- **Mainline DHT / EventGateway**: Runbook Chapter10 / RelayStatus リンクに加え、2025年11月11日に `apply_cli_bootstrap_nodes` から `NetworkService::apply_bootstrap_nodes` を呼び出してランタイムでブートストラップリストを差し替える実装を追加。`kukuri-cli --export-path` → `RelayStatus` → 「最新リストを適用」でアプリ再起動なしに Mainline DHT 接続先を更新でき、`phase5_dependency_inventory_template.md` / `phase5_event_gateway_design.md` とも整合を取った。
- **Ops / CI**: `cmd.exe /c "corepack enable pnpm"` → `pnpm install --frozen-lockfile` を完走し、`tmp/logs/vitest_topic_create_20251110020423.log` / `post_delete_cache_20251110020439.log` を取得。2025年11月11日に GitHub Actions `test.yml` の `native-test-linux` / `format-check` / `build-test-windows` へ `kukuri-cli` の `cargo test` / `cargo fmt -- --check` / `cargo check` を追加し、CLI の品質ゲートをフロント/Rust本体と同列にした。同日 `parse_node_addr` / `export_bootstrap_list` / `load_bootstrap_peers_from_json` / `resolve_export_path` を検証する 6 件のユニットテストと、Docker Compose ベースの `docker_connectivity_direct_no_dht` / `docker_connectivity_with_mdns` シナリオを CLI テストに追加。`cargo test --workspace --all-features` でユニット + Docker シナリオが常時実行されるようにした。残課題は `post-delete-cache` Docker シナリオの整備、Stage4 Service Worker 追加時のログ（`sync_status_indicator_stage4_<timestamp>.log`）採取と Runbook 反映。

## MVP Exit Checklist（2025年11月10日版）

| カテゴリ | ゴール | 2025年11月10日時点のブロッカー | 次アクション | 参照 |
| --- | --- | --- | --- | --- |
| UX/体験導線 | `/trending` `/following` `/profile/$userId` `/direct-messages` `/search` を横断し、サイドバー/グローバルコンポーザーから同一導線を完走できる状態 | `DirectMessageInbox` の既読共有・会話検索、`UserSearchResults` の Docker `user-search-pagination` シナリオ、オフライン Topic 作成（`OfflineActionType::CREATE_TOPIC`）と Post 削除 `post-delete-cache` Docker シナリオが未完。Stage4 Service Worker（プロフィール/SyncStatus）実装と `trending_metrics_job` 監視も残る。 | Stage4 TODO（`phase5_user_flow_inventory.md` Sec.5.1/5.5/5.7/5.9/5.10/5.11）を順に実装。まずは `topic_create` / `post_delete_cache` のオフライン再送と `trending_metrics_job` Prometheus Export を着手し、完了後は `phase5_ci_path_audit.md` の該当行と Runbook を更新する。 | `phase5_user_flow_inventory.md` Sec.5.1/5.4/5.7/5.9/5.10/5.11, `tauri_app_implementation_plan.md` Phase3 |
| P2P & Discovery | EventGateway と P2PService Stack を抽象化し、Mainline DHT Runbook / `kukuri-cli` ブートストラップを UI から辿れる状態 | Runbook Chapter10 と RelayStatus からの遷移に加え、`kukuri-cli --export-path` → `RelayStatus` の「最新リストを適用」ボタンで CLI ブートストラップリストを切り替えられる PoC を実装。Gateway/mapper 差分は `phase5_event_gateway_design.md` に沿って順次反映中。 | Gateway 実装タスクを `refactoring_plan_2025-08-08_v3.md` Phase5 と `roadmap.md` KPI に再マッピングし、Runbook Chapter10 の CLI 手順と `phase5_dependency_inventory_template.md` P2P 行をメンテナンス対象に追加。 | `phase5_event_gateway_design.md`, `phase5_dependency_inventory_template.md`, `docs/03_implementation/p2p_mainline_runbook.md` |
| データ/同期 & メトリクス | sync_queue Stage3/4 と `trending_metrics_job` を Runbook / CI で再現できる状態 | Doc/Blob 対応の `cache_metadata` マイグレーションと conflict バナー UI（Phase4）が未実装。`scripts/test-docker.{sh,ps1} --scenario trending-feed` が環境制約で停止し、`phase5_ci_path_audit.md` に `profile-avatar-sync` / `trending-feed` / `SyncStatusIndicator` テスト ID が不足。ただし 2025年11月10日に `KUKURI_METRICS_PROMETHEUS_PORT` / `KUKURI_METRICS_EMIT_HISTOGRAM` を追加し、`curl http://localhost:<port>/metrics` で `trending_metrics_job_*` 指標を取得できるようになった。 | Inventory 5.5/5.11 を Stage3/4 用に更新し、CI 監査へ `scripts/test-docker.{sh,ps1} ts -Scenario profile-avatar-sync`・`rust -Test profile_avatar_sync`・`--scenario trending-feed` を登録、`tmp/logs/profile_avatar_sync_<timestamp>.log` へのリンクと `curl http://localhost:<port>/metrics` 採取手順を記載。 | `phase5_user_flow_inventory.md` 5.5/5.11, `tauri_app_implementation_plan.md` Phase4, `phase5_ci_path_audit.md` |
| Ops / CI | Nightly / GitHub Actions で MVP 導線を再現し、Runbook で復旧できる状態 | `pnpm` / `corepack` のセットアップ手順が欠落し TS ユニットテストと Docker `trending-feed` が失敗。`tmp/logs/*.log` への成果物リンクも未整備。 | `corepack enable pnpm` を onboarding に追加し、`phase5_ci_path_audit.md` 表へ新テスト ID とログパス（`tmp/logs/profile_avatar_sync_<timestamp>.log`, `tmp/logs/docker_rust_test_20251109.log` 等）を追記。Nightly `profile-avatar-sync` / `trending-feed` ジョブの成果物参照を `nightly.yml` と Runbookへ記録。 | `phase5_ci_path_audit.md`, `tasks/status/in_progress.md` (GitHub Actions), `scripts/test-docker.ps1` |

共通の検証手順
- `corepack enable pnpm && pnpm vitest run src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx`（同期導線）
- `corepack enable pnpm && pnpm vitest run src/tests/unit/components/topics/TopicSelector.test.tsx src/tests/unit/components/posts/PostCard.test.tsx src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx`（グローバルコンポーザー/投稿削除/トレンド導線）
- `./scripts/test-docker.{sh,ps1} ts -Scenario profile-avatar-sync` / `./scripts/test-docker.ps1 rust -Test profile_avatar_sync -NoBuild`（プロフィール Doc/Blob Stage3）
- `cargo test --package kukuri-cli -- test_bootstrap_runbook` / `./scripts/test-docker.ps1 rust -NoBuild`（Mainline DHT / Rust 側）

## 1. 画面別導線サマリー

### 1.1 オンボーディング & 認証
| 画面 | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| Welcome | `/welcome` | 新規アカウント作成、ログイン導線 | 稼働中 | `generate_keypair` で鍵を生成、SecureStorage 登録まで完了 |
| Login | `/login` | nsec ログイン、セキュア保存、リレー接続表示 | 稼働中 | `login`/`add_account`/`initialize_nostr` 連携、保存後の自動ログインあり |
| Profile Setup | `/profile-setup` | プロフィール入力、画像選択（ローカルファイル） | 稼働中 | Stage3（Doc/Blob + privacy）により `update_privacy_settings` → `upload_profile_avatar` → `useProfileAvatarSync` を直列実行し、`profile_avatar_sync` コマンドで Doc/Blob を常駐同期。プライバシー設定は `usePrivacySettingsStore` と `authStore.updateUser` へ即時反映。 |

### 1.2 認証後の主要導線
| セクション | パス/配置 | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| Home タイムライン | `/` | 投稿閲覧、いいね・ブースト・ブックマーク、グローバルコンポーザー | 稼働中 | `PostComposer` 下書き保存、`PostCard` アクション完備。投稿削除時は `useDeletePost` が React Query（`timeline`/`posts/*`/トレンド/フォロー中）と `topicStore.updateTopicPostCount` を即時更新（Inventory 5.10 完了）。 |
| サイドバー | 共通 | 参加トピック一覧、未読バッジ、「新規投稿」ボタン | 改善中 | カテゴリーは `useUIStore.activeSidebarCategory` で同期。参加トピックがゼロの場合は `TopicFormModal` (`mode=create-from-composer`) を先に開き、作成完了後に `openComposer({ topicId })` へ遷移する。`prefetchTrendingCategory`/`prefetchFollowingCategory` でレスポンスも最適化済み。 |
| ヘッダー | 共通 | `SyncStatusIndicator`、`RealtimeIndicator`、`AccountSwitcher`、DM 未読バッジ | 稼働中 | アカウント切替/追加/削除、同期状態表示、未読メッセージのバッジ表示と DM モーダル呼び出しを提供 |
| Global Composer | 共通（モーダル） | どの画面からでも投稿／トピック選択 | 稼働中 | `TopicSelector` に「新しいトピックを作成」ショートカットを追加。`TopicFormModal` (`create-from-composer`) + `useComposerStore.applyTopicAndResume` で作成直後に選択した状態のまま投稿を続行できる。 |
| トレンドフィード | `/trending` | トレンドスコア上位トピックのランキングカード、最新投稿プレビュー | 改善中 | `list_trending_topics`/`list_trending_posts`（limit=10/per_topic=3, staleTime=60s）。`generated_at` は `topic_metrics` の最新 `window_end`（`trending_metrics_job` が 5 分間隔で更新）を共有し、Summary Panel / Docker `trending-feed` シナリオでも同値となる。 |
| フォロー中フィード | `/following` | フォロー中ユーザーの専用タイムライン、無限スクロール | 改善中 | `list_following_feed`（limit=20, cursor=`{created_at}:{event_id}`）を `useInfiniteQuery` で表示。Summary Panel に DM 未読カードを追加し、Prefetch + Retry 導線と `routes/following.test.tsx` のカバレッジあり。 |
| プロフィール詳細 | `/profile/$userId` | プロフィール表示、フォロー/フォロー解除、投稿一覧、DM モーダル起動 | 改善中 | `DirectMessageDialog` は Kind4 IPC によるリアルタイム受信・未読管理・再送ボタンを提供。フォロワー/フォロー一覧にはソート（最新/古い/名前）、検索、件数表示を追加済み。既読同期の多端末共有とページング拡張が backlog。 |

### 1.3 トピック関連
| 画面 | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| Topics 一覧 | `/topics` | トピック検索、参加切替、新規作成 | 稼働中 | `TopicFormModal` の共通化完了。`mode='create-from-composer'` はグローバル導線から利用し、通常一覧では `mode='create'` を維持。統計は `get_topic_stats` を使用。 |
| トピック詳細 | `/topics/$topicId` | 投稿一覧、P2P メッシュ表示、参加/離脱 | 改善中 | 最終更新表示は修正済み。トピック削除・編集はモーダル導線あり |
| P2P Mesh | `/topics/$topicId` 内 | `TopicMeshVisualization` で Gossip/Mainline 状態を表示 | 改善中 | ステータス更新のリトライは今後の改善項目 |

### 1.4 検索
| タブ | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| 投稿 | `/search` (posts) | フロント側フィルタで投稿検索 | 稼働中 | 初回ロードで `get_posts` 呼び出し |
| トピック | `/search` (topics) | トピック名/説明で検索 | 稼働中 | `get_topics` 再利用 |
| ユーザー | `/search` (users) | `search_users` で実ユーザー検索、フォロー/解除ボタン | 改善中 | フォロー結果と Infinite Query は稼働中。2025年11月10日に関連度/最新順トグル・`allow_incomplete` フォールバック・SearchBar 警告スタイルを実装し、`npx pnpm vitest run src/tests/unit/hooks/useUserSearchQuery.test.tsx src/tests/unit/components/search/UserSearchResults.test.tsx`（`tmp/logs/vitest_user_search_allow_incomplete_20251110132951.log`）と Docker `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build`（`tmp/logs/user_search_pagination_20251110-142854.log`）で回帰を取得済み。残タスクは Nightly へのシナリオ組み込み。 |

### 1.5 設定 & デバッグ
| セクション | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| 外観 | `/settings` | テーマ切替（ライト/ダーク） | 稼働中 | `useUIStore` 経由で永続化 |
| アカウント | `/settings` | プロフィール編集モーダル、鍵管理プレースホルダー | 改善中 | プロフィール編集は稼働中。鍵管理ボタンは未配線 |
| プライバシー | `/settings` | 公開設定／オンライン表示トグル | 稼働中 | Stage3（Doc/Blob + privacy）完了。`ProfileEditDialog` / `ProfileSetup` が `update_privacy_settings` → `upload_profile_avatar` → `useProfileAvatarSync` を直列実行し、`scripts/test-docker.{sh,ps1} ts -Scenario profile-avatar-sync` / `rust -Test profile_avatar_sync` で検証。Stage4（Service Worker + バックオフ）は backlog。 |
| P2P 接続 | `/settings` | `PeerConnectionPanel` で手動接続/履歴管理 | 稼働中 | `connect_to_peer` コマンドに紐づく |
| Bootstrap 設定 | `/settings` | ブートストラップノード一覧の取得/登録/リセット | 稼働中 | `set_bootstrap_nodes` などと連携 |
| 開発者ツール (DEV) | `/settings`（開発モード） | `NostrTestPanel`, `P2PDebugPanel` | 改善中 | UI は Dev 限定。計測ログとテスト誘導の整理が backlog |

### 1.6 ダイレクトメッセージ
| セクション | パス/配置 | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| プロフィール起点 | `/profile/$userId` | `メッセージ` ボタン→`DirectMessageDialog` モーダル、履歴ロード・送信・再送ボタン | 改善中 | Kind4 IPC 連携済み。既読同期/多端末未読共有は backlog（Inventory 5.6.1） |
| グローバル導線 | ヘッダー右上 | `MessageCircle` ボタンで最新会話またはアクティブ会話を開き、隣の `Plus` アイコンから `DirectMessageInbox`（会話一覧 + 宛先入力）を起動 | 改善中 | `useDirectMessageBadge` が未読バッジを共有。Inbox はローカルストアの会話履歴のみを表示し、検索/候補は未実装 |
| Summary Panel | `/trending`, `/following` | `TrendingSummaryPanel` / `FollowingSummaryPanel` の DM カードに未読件数・最終受信時刻・`DM Inbox を開く` CTA を表示 | 改善中 | ヘッダーと同じ `useDirectMessageBadge` を共有し、CTA で `useDirectMessageStore.openInbox` を呼び出せる。未読の永続化と会話一覧 API は backlog |

## 2. グローバル要素
- **ステータスカード**: `RelayStatus` / `P2PStatus` が 30 秒間隔でステータス取得。フェイルオーバー時のバックオフと手動再試行を実装。
- **同期系 UI**: `SyncStatusIndicator`（Inventory 5.11）と `OfflineIndicator` が `offlineStore` / `syncEngine` の状態を共有し、オンライン復帰後 2 秒の自動同期・5 分ごとの定期同期・手動同期ボタン・競合解決ダイアログを提供。2025年11月07日: `get_cache_status` を 60 秒間隔＋手動操作で取得し、キャッシュ合計/ステール件数とタイプ別統計をポップオーバーに表示。ステールなタイプには「再送キュー」ボタンを表示し、`add_to_sync_queue`（`action_type=manual_sync_refresh`）で手動再送を登録できるようになった。2025年11月09日: `cache_types.metadata`（要求者/要求時刻/Queue ID/発行元）整形と OfflineIndicator の誘導コピーに加え、`list_sync_queue_items` を介した再送履歴（Queue ID フィルタ、最新 ID ハイライト、ステータス別バッジ、要求者/要求時刻/発行元/再試行回数、エラーメッセージ）をポップオーバーに追加。
- **リアルタイム更新**: `RealtimeIndicator` と `useP2PEventListener` で投稿受信を通知し、`topicStore` の未読管理を更新。
- **グローバルコンポーザー**: `useComposerStore` で Home/Sidebar/Topic から共通モーダルを制御し、投稿完了後にストアをリセット。2025年11月10日: `TopicSelector` に「新しいトピックを作成」ショートカット、`TopicFormModal(mode='create-from-composer')`、`useComposerStore.applyTopicAndResume` を実装し、作成直後に投稿継続できるようにした。`pnpm` 不足で `TopicSelector.test.tsx` / `PostCard.test.tsx` を再実行できていないため、`corepack enable pnpm` ＋ Docker ルートでの検証が必要（Inventory 5.9 / `phase5_ci_path_audit.md`）。
- **プロフィール導線**: `UserSearchResults` と `/profile/$userId` が連携し、フォロー操作後に React Query キャッシュを即時更新。`DirectMessageDialog` は React Query ベースの履歴ロード・未読リセット・無限スクロールまで接続済み。ヘッダー右上の `MessageCircle` ボタン／`Plus` ボタン、および `/trending` `/following` の Summary Panel CTA は `useDirectMessageBadge` と `useDirectMessageStore` を共有し、既存会話なら即座にモーダルを開き、未読が無い場合でも `DirectMessageInbox` から宛先入力＆会話一覧で新規 DM を開始できる。2025年11月08日以降は `list_direct_message_conversations` / `mark_direct_message_conversation_read` を通じて会話一覧と未読が SQLite に永続化され、ログイン直後の Inbox ハイドレートと Dialog からの既読更新が双方向に同期する。残課題は会話検索/補完・大量会話時の仮想スクロール・多端末既読共有で、Inventory 5.6.x/5.12 でフォロー。
- **ユーザー検索**: `UserSearchResults` の状態遷移（idle/typing/ready/loading/success/empty/rateLimited/error）と `SearchErrorState` ハンドリング、`query` バリデーション（2〜64文字、制御文字除去、連続スペース正規化）を Inventory 5.8 と `error_handling_guidelines.md` に記録。2025年11月10日: 関連度/最新順トグルを追加し、`useUserSearchQuery` が `sort` オプションをキャッシュキーと Tauri API に伝播するよう更新。`npx pnpm vitest …`（`tmp/logs/vitest_user_search_allow_incomplete_20251110132951.log`）に加え、Docker `user-search-pagination` シナリオ（`tmp/logs/user_search_pagination_20251110-142854.log`）でも回帰を取得。

## 3. 導線ギャップ Quick View
1. `/trending`・`/following` ルートは実装済み（Inventory 5.7 参照）。Summary Panel / DM 未読カード / QA は 2025年11月07日までに完了。現在は `corepack pnpm` 不足で `TrendingSummaryPanel.test.tsx` / Docker `--scenario trending-feed` を再実行できず、`trending_metrics_job` 常駐フックの改修が止まっている。`phase5_ci_path_audit.md` と `tauri_app_implementation_plan.md` Phase3 にギャップを記録済み。
2. `/profile/$userId` はフォロー導線とフォロワー/フォロー一覧（ソート・検索・件数表示）を備え、DirectMessageDialog も Kind4 IPC によるリアルタイム受信・未読管理・再送ボタンを提供。引き続き既読同期の多端末共有とページング拡張を Inventory 5.6.1/5.6.2 に沿って進める。
3. ヘッダーの `MessageCircle` + `Plus` ボタン、Summary Panel の CTA から `DirectMessageInbox` を開けるようになり、2025年11月08日時点で `direct_message_conversations` テーブル経由の会話一覧・未読永続化と既読 API が稼働。残課題は会話検索/補完、Limit 超過時のページング、仮想スクロールの最適化。
4. 投稿削除フローは 2025年11月03日に `delete_post` を UI に配線済み。Inventory 5.10 で React Query キャッシュ無効化・Docker シナリオ・統合テストのフォローアップを整理済み。
5. 設定 > 鍵管理ボタンがバックエンドと未接続。
6. プライバシー設定のローカル値をバックエンドへ同期する API が未提供。
7. ユーザー検索タブは `search_users` で動作するが、無限スクロール/状態遷移/エラーUIは未実装（Inventory 5.8 に状態機械・入力バリデーション・SearchErrorState 設計を追記済み、`error_handling_guidelines.md` にメッセージ鍵を登録済み）。
8. ホーム/サイドバーからのトピック作成導線は Inventory 5.9 で仕様化中。Global Composer の TopicSelector ショートカットと `createAndJoinTopic` 連携を整備する。
9. `SyncStatusIndicator` は `get_cache_status` / `add_to_sync_queue` / `list_sync_queue_items` を取り込み、キャッシュ統計・手動キュー登録・再送履歴可視化を提供済み。今後は `sync_engine` 側で処理完了イベントと Docker ログの参照先を `cache_metadata` に記録し、履歴カードから Runbook ／ログへ遷移できる導線を追加する。
10. 未接続 API は `join_topic_by_name`（Global Composer フォールバック）、`delete_events`（投稿削除 + Nostr 連携）、`add_relay`（鍵管理ダイアログと連動）、`get_nostr_pubkey`（プロフィール共有 UI 刷新時に再評価）、`clear_all_accounts_for_test`（Debug パネル）。Inventory 3.2/3.3 で優先度を整理し、Phase 5 backlog と同期した。

## 4. テストカバレッジ概要
- フロントエンド: `pnpm test:unit`（Home/Sidebar/RelayStatus/P2PStatus/Composer/Settings のユニットテストを含む）、`pnpm vitest run src/tests/integration/profileAvatarSync.test.ts`、`npx vitest run src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx src/tests/unit/components/layout/Header.test.tsx src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx`。
- Rust: `cargo test`（`kukuri-tauri/src-tauri` と `kukuri-cli`）で P2P ステータスおよびプロフィール同期を検証。
- Docker: `./scripts/test-docker.sh p2p`・`./scripts/test-docker.ps1 rust` で Gossip/Mainline スモークを再現。`./scripts/test-docker.sh ts --scenario trending-feed` / `.\scripts\test-docker.ps1 ts -Scenario trending-feed` でトレンド/フォロー導線の Vitest を Docker 上で実行（フィクスチャは `tests/fixtures/trending/default.json`）。

## 5. 関連資料
- `phase5_user_flow_inventory.md` — 詳細な導線/コマンド対応表・設計メモ。
- `tauri_app_implementation_plan.md` Phase 5 — 導線改善タスクとスケジュール。
- `phase5_ci_path_audit.md` — 関連テストと CI パスの依存関係。
- `refactoring_plan_2025-08-08_v3.md` 2.5 節 — 導線指標と未対応項目チェックリスト。
- `docs/03_implementation/trending_metrics_job.md` — トレンドメトリクス集計ジョブの設計案と監視手順ドラフト。

## 6. 未実装項目の優先度見直し（2025年11月05日）

| 優先度 | 項目 | 現状/課題 | ユーザー影響 | 次アクション |
| --- | --- | --- | --- | --- |
| A | 投稿削除 (`delete_post`) | 2025年11月03日: PostCard 削除メニューと `postStore.deletePostRemote` のオフライン対応を実装し、ユニットテストで検証済み。 | 楽観削除は機能するが、React Query キャッシュと Rust 統合テストが未整備。 | Inventory 5.10 に沿って React Query 側のキャッシュ無効化と `delete_post` コマンドの統合テスト追加、CI での回帰監視をフォローアップ。 |
| B | `/profile/$userId` ルート | `DirectMessageDialog` は Kind4 IPC・未読管理・再送ボタンまで実装済み。フォロワー/フォロー一覧のソート（最新/古い/名前）・検索・件数表示を実装済みで、既読同期の多端末共有とページング拡張が残課題。 | DM 履歴はモーダル表示で確認でき、フォロー一覧もソート/検索可能になったが、会話既読の多端末反映と 2 ページ目以降の自動補充が未対応。 | Inventory 5.6.1 で delivered/既読同期と Docker/contract テストを追加し、Inventory 5.6.2 でページング整合性とフォローアップテストを進める。 |
| B | 鍵管理ダイアログ | 設定>鍵管理ボタンがダミー。バックアップ・復旧手段が提供できていない。 | 端末故障時に復旧不能。運用リスク高。 | `KeyManagementDialog` 実装（エクスポート/インポート）、`export_private_key`/`SecureStorageApi.addAccount` 連携、注意喚起 UI とテスト追加。 |
| B | プライバシー設定のバックエンド連携 | トグルはローカル永続のみで、他クライアントへ反映されない。 | 公開範囲が端末ごとに不一致。誤公開や表示不整合の恐れ。 | `usePrivacySettingsStore` から Tauri コマンドを呼ぶ設計策定、Nostr/P2P への伝播API定義、同期テスト計画を追記。 |
| B | ユーザー検索導線改善 | `/search` (users) は `search_users` で実ユーザーを表示し、Infinite Query・レートリミット UI・関連度/最新順トグル・`allow_incomplete` フォールバック・SearchBar 警告スタイルまで実装済み（2025年11月10日）。Docker `user-search-pagination` シナリオも追加。 | ソートやページネーションは利用できるが、短い入力時の補助検索を Nightly で再現する自動化と成果物保存が未整備。 | Inventory 5.8 に沿って Nightly ジョブへシナリオを追加し、`tmp/logs/vitest_user_search_allow_incomplete_20251110132951.log` / `tmp/logs/user_search_pagination_20251110-142854.log` を Runbook/CI に登録する。 |
| B | `/trending` / `/following` フィード | Summary Panel で派生メトリクスと DM 未読カードを表示。Docker シナリオ・`trending_metrics_job` は未実装。 | フィード自体は閲覧できるものの、監視と再現性が不足。 | 5.7 節の順序 (Docker シナリオ → `trending_metrics_job`) に沿って実装し、各ステップ後にテスト/ドキュメント/CI を更新。 |

> 優先度A: 現行体験に致命的影響があるもの。<br>
> 優先度B: 早期に手当てしたいが依存タスクがあるもの。<br>
> 優先度C: 情報提供や暫定UIでの回避が可能なもの。
