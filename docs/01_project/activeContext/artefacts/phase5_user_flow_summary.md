# Phase 5 ユーザー導線サマリー
作成日: 2025年11月03日  
最終更新: 2025年11月13日

## 概要
- Phase 5 時点でアプリ UI から到達できる体験を俯瞰し、欠落導線や改善ポイントを即座に把握できるようにする。
- 詳細なフロー・API 連携・設計メモは `phase5_user_flow_inventory.md` を参照し、本書では意思決定に必要なサマリーのみを掲載。
- 導線の状態は「稼働中」「改善中」「未実装」の 3 区分で整理し、次の対応タスクを明示する。

## MVP残タスクハイライト（2025年11月12日更新）
- **トレンド/フォロー**: Summary Panel と `/trending` `/following` UI は安定。2025年11月10日に `corepack enable pnpm` → `pnpm install --frozen-lockfile` を通し、`pnpm vitest run …` と Docker `./scripts/test-docker.sh ts --scenario trending-feed --no-build` をローカルで再実行（`tmp/logs/vitest_trending_topics_20251110020449.log` / `trending-feed_20251110020528.log`）。同日に `KUKURI_METRICS_PROMETHEUS_PORT` / `KUKURI_METRICS_EMIT_HISTOGRAM` を追加し、`curl http://localhost:<port>/metrics` で `trending_metrics_job_*` 指標を確認できるようになった。2025年11月11日には `scripts/test-docker.{sh,ps1} ts --scenario trending-feed` へ `prometheus-trending` サービスの自動起動と `curl http://127.0.0.1:9898/metrics` のスナップショット採取を追加し、`tmp/logs/trending_metrics_job_stage4_20251111-191137.log` に `curl` 出力とコンテナログを記録できるようにした。2025年11月12日: Nightly `trending-feed` ジョブで `tmp/logs/trending_metrics_job_stage4_*.log` を artefact 化し、Runbook/CI 双方で metrics ログをトレースできる状態にした。`p2p_metrics_export --job trending` の JSON と併せ、Stage4 backlog（Prometheus 監視 + artefact 固定）はクローズ済み。
  さらに同ログは `test-results/trending-feed/prometheus/` にも複製され、Nightly artefact `trending-metrics-prometheus` として Runbook から直接参照可能となった。
- **Direct Message / Inbox**: Kind4 IPC / `DirectMessageDialog` の無限スクロールは稼働中。2025年11月12日に `useDirectMessageBootstrap` へ 30 秒間隔の再取得・フォーカス復帰・`DirectMessageInbox`/`DirectMessageDialog` オープン時の即時同期を追加し、多端末既読共有を SQLite `direct_message_conversations` の未読数と連動させた。`DirectMessageInbox` の宛先検索成功時には `errorHandler.info` でクエリ長/ヒット件数を記録し、`tmp/logs/vitest_direct_message_20251112-124608.log` で `Header` / `DirectMessageDialog` / Inbox / Summary Panel のユニットテストを再取得済み。Nightly/Runbook 両方の DM 行へ同ログと `test-results/direct-message/*.json` を紐付け（Inventory 5.4 / 5.6.x 更新）。
- **プロフィール/設定**: Stage3（Doc/Blob + privacy）を 2025年11月10日に完了し、`ProfileEditDialog` / `ProfileSetup` が `update_privacy_settings` → `upload_profile_avatar` → `useProfileAvatarSync` を直列実行。2025年11月12日: Stage4（Service Worker + Offline ログ）も完了し、`profileAvatarSyncSW.ts` の指数バックオフ／`offlineApi.addToSyncQueue` ログ／`profile_avatar_sync` の `cache_metadata` TTL 30 分を Runbook / CI artefact（`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log`）に統合。Nightly は `scripts/test-docker.{sh,ps1} ts --scenario profile-avatar-sync --service-worker` と `./scripts/test-docker.ps1 rust -Test profile_avatar_sync` をセットで実行し、`phase5_ci_path_audit.md` / Runbook Chapter4 に Test ID（`nightly.profile-avatar-sync`）と採取パスを明記。
- **ユーザー検索**: `useUserSearchQuery` と `UserSearchResults` に cursor/sort／レートリミット UI／無限スクロールを実装し、2025年11月10日に `allow_incomplete` フォールバックと SearchBar 警告スタイル・補助検索ラベルを追加。同日 `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build` を実行し、`tmp/logs/user_search_pagination_20251110-142854.log` を採取。2025年11月12日: `nightly.yml` に `user-search-pagination` ジョブを追加し、`test-results/user-search-pagination/*.json` と `tmp/logs/user_search_pagination_<timestamp>.log` を artefact 化。ローカルでも `npx pnpm vitest … | tee tmp/logs/user_search_pagination_20251112-125208.log` を取得済み。
- **Offline sync**: 2025年11月11日に Stage4（Doc/Blob 対応 `cache_metadata` + 競合バナー + Service Worker + Docker `offline-sync`）を完了。`cache_types` に `doc_version` / `blob_hash` / `payload_bytes` を返すよう Rust 側を拡張し、`SyncStatusIndicator` へ Doc/Blob サマリーと競合バナーを追加。`npx vitest run src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx` および `./scripts/test-docker.sh ts --scenario offline-sync --no-build`（PowerShell 版あり）を実行し、`tmp/logs/sync_status_indicator_stage4_<timestamp>.log` を Runbook Chapter5 に保存。2025年11月12日: Topic/Post 系 OfflineAction の Docker シナリオとして `ts --scenario topic-create` / `post-delete-cache` を追加し、`tmp/logs/topic_create_20251112-125226.log` / `post_delete_cache_20251112-125301.log` と `test-results/topic-create/*.json` を Nightly artefact に登録。2025年11月13日: `post_delete_cache` Stage4 を完了し、`tmp/logs/post_delete_cache_20251113-085756.log`・`tmp/logs/post-delete-cache_docker_20251113-002140.log`・`test-results/post-delete-cache/20251113-002140.json` を Nightly/Runbook へ反映。
- **Topic create（Stage4）**: 2025年11月12日に `TopicService::enqueue_topic_creation` / `topics_pending` / `PendingTopicRepository` を追加し、オフライン作成を `OfflineActionType::CREATE_TOPIC` として保存→同期後に `mark_pending_topic_synced|failed` を呼び出せるようにした。`topicStore.pendingTopics` と `TopicSelector` の「保留中のトピック」表示、`TopicFormModal` のオフライン経路（`watchPendingTopic` → `resolvePendingTopic`）を整備し、`Input` を `forwardRef` 化して Radix ref 警告を解消。`npx pnpm vitest run … | Tee-Object -FilePath ../tmp/logs/topic_create_host_20251112-231141.log` と `./scripts/test-docker.ps1 ts -Scenario topic-create`（`tmp/logs/topic_create_20251112-231334.log`, `test-results/topic-create/20251112-231334-*.json`）で TopicSelector/PostComposer/Sidebar/Scenario の 47 ケースを再取得し、Runbook Chapter5 / `phase5_ci_path_audit.md` に採取パスを登録した。
- **Mainline DHT / EventGateway**: Runbook Chapter10 / RelayStatus リンクに加え、2025年11月11日に `apply_cli_bootstrap_nodes` から `NetworkService::apply_bootstrap_nodes` を呼び出してランタイムでブートストラップリストを差し替える実装を追加。`kukuri-cli --export-path` → `RelayStatus` → 「最新リストを適用」でアプリ再起動なしに Mainline DHT 接続先を更新でき、2025年11月12日には PoC ログ（`tmp/logs/relay_status_cli_bootstrap_20251112-094500.log`）を Runbook 10.3/10.4・`phase5_ci_path_audit.md` と同期。同日、`RelayStatus` のバックオフ更新/再試行/CLI 適用を一つの `refreshRelaySnapshot`（`src/components/RelayStatus.tsx`）へまとめ、`p2pApi.getBootstrapConfig` を毎回再取得することで CLI リストの差分を即座に UI へ反映。`pnpm vitest run src/tests/unit/components/RelayStatus.test.tsx` と `cargo test`（`kukuri-tauri/src-tauri`, `kukuri-cli`）で回帰し、`phase5_dependency_inventory_template.md` / `phase5_event_gateway_design.md` も更新済み。
- **Ops / CI**: `docs/01_project/setup_guide.md`（Ops / CI Onboarding 節）に `cmd.exe /c "corepack enable pnpm"`／`corepack pnpm --version` を追加し、Windows/WSL/Unix いずれでも Corepack 経由で `pnpm install --frozen-lockfile` を行う手順を固定。`nightly.yml` も Corepack で pnpm を有効化し、新たに `sync-status-indicator` ジョブ（`./scripts/test-docker.sh ts --scenario offline-sync`）を追加して `tmp/logs/sync_status_indicator_stage4_<timestamp>.log` artefact を採取するよう統一した。`profile-avatar-sync` / `trending-feed` / `user-search-pagination` / `post-delete-cache` 各ジョブは artefact 名（`*-logs`, `*-reports`, `trending-metrics-*`）と `tmp/logs/<scenario>_<timestamp>.log` を揃え、Runbook 6.4 と `phase5_ci_path_audit.md` にテスト ID（`nightly.<job>`）＋ログパスを登録。MVP Exit Checklist Ops/CI 行は Onboarding/artefact ギャップが解消済みのため、残タスクは CLI/Runbook 変更時の同期確認のみ。

## MVP Exit Checklist（2025年11月12日版）

| カテゴリ | ゴール | 2025年11月10日時点のブロッカー | 次アクション | 参照 |
| --- | --- | --- | --- | --- |
| UX/体験導線 | `/trending` `/following` `/profile/$userId` `/direct-messages` `/search` を横断し、サイドバー/グローバルコンポーザーから同一導線を完走できる状態 | `DirectMessageInbox` の既読共有・会話検索、`UserSearchResults` の Docker `user-search-pagination` シナリオが未完。`topic_create` Stage4 は 2025年11月12日に完了し、`tmp/logs/topic_create_host_20251112-231141.log` / `tmp/logs/topic_create_20251112-231334.log` と `test-results/topic-create/20251112-231334-*.json` を Nightly artefact に追加済み。Post 削除 Stage4 は 2025年11月13日に完了し、`tmp/logs/post_delete_cache_20251113-085756.log` / `tmp/logs/post-delete-cache_docker_20251113-002140.log` と `test-results/post-delete-cache/20251113-002140.json` を Runbook/CI へ登録。プロフィール Stage4（Service Worker + Offline ログ）は `tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` / `profile-avatar-sync-logs` artefact で追跡中。 | 次は DM/Search/Post backlog（`phase5_user_flow_inventory.md` Sec.5.4/5.6/5.7/5.10/5.11）を解消。`user-search-pagination` を Nightly 常設シナリオに昇格させたので、DM 既読共有の contract テストと Runbook 更新を進める。 | `phase5_user_flow_inventory.md` Sec.5.1/5.4/5.7/5.9/5.10/5.11, `tauri_app_implementation_plan.md` Phase3 |
| P2P & Discovery | EventGateway と P2PService Stack を抽象化し、Mainline DHT Runbook / `kukuri-cli` ブートストラップを UI から辿れる状態 | Runbook Chapter10 と RelayStatus からの遷移に加え、`kukuri-cli --export-path` → `RelayStatus` の「最新リストを適用」ボタンで CLI ブートストラップリストを切り替えられる PoC を実装。2025年11月12日: `refreshRelaySnapshot` によりバックオフ更新・再試行・CLI 適用後の再取得が同じコードパスとなり、`p2pApi.getBootstrapConfig` が毎回再読込されるため、CLI の JSON を更新するだけで UI の「CLI 提供」表示が同期される。Gateway/mapper 差分は `phase5_event_gateway_design.md` に沿って順次反映中。 | Gateway 実装タスクを `refactoring_plan_2025-08-08_v3.md` Phase5 と `roadmap.md` KPI に再マッピングし、Runbook Chapter10 の CLI 手順と `phase5_dependency_inventory_template.md` P2P 行をメンテナンス対象に追加。 | `phase5_event_gateway_design.md`, `phase5_dependency_inventory_template.md`, `docs/03_implementation/p2p_mainline_runbook.md` |
| データ/同期 & メトリクス | sync_queue Stage3/4 と `trending_metrics_job` を Runbook / CI で再現できる状態 | プロフィール Doc/Blob Stage4 は `cache_metadata` TTL 30 分・`profile-avatar-sync-logs` への採取まで完了。残課題は SyncStatus conflict バナー UI（Phase4）と `scripts/test-docker.{sh,ps1} --scenario trending-feed` の安定化（Prometheus Export / artefact `trending-metrics-logs` 更新）、`SyncStatusIndicator` / `post-delete-cache` / `offline-sync` ジョブの artefact 整備。 | Inventory 5.5/5.11 を Stage4 実装版へ更新し、CI 監査に `scripts/test-docker.{sh,ps1} ts --scenario profile-avatar-sync --service-worker`・`rust -Test profile_avatar_sync`・`--scenario trending-feed` を登録。`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` と `curl http://localhost:<port>/metrics` 採取手順を Runbook / `phase5_ci_path_audit.md` へ記載。 | `phase5_user_flow_inventory.md` 5.5/5.11, `tauri_app_implementation_plan.md` Phase4, `phase5_ci_path_audit.md` |
| Ops / CI | Nightly / GitHub Actions で MVP 導線を再現し、Runbook で復旧できる状態 | ✅ 2025年11月12日: `docs/01_project/setup_guide.md` Onboarding に `corepack enable pnpm` / `corepack pnpm --version` を明記し、`nightly.yml` でも Corepack を有効化。`profile-avatar-sync` / `trending-feed` / `sync-status-indicator` / `user-search-pagination` / `post-delete-cache` の各ジョブへ artefact 名と `tmp/logs/<scenario>_<timestamp>.log` を登録し、Runbook 6.4 / `phase5_ci_path_audit.md` にテスト ID（`nightly.<job>`）＋ログパスを追記。 | CLI/Runbook 更新や新シナリオ追加時は `nightly.<job>` テスト ID・`tmp/logs` パス・artefact 名をセットで更新する運用を継続。現状の Nightly artefact（`*-logs`, `*-reports`, `trending-metrics-*`）が欠損した場合は該当ジョブを rerun し、`phase5_ci_path_audit.md` の行に調査ログをリンクする。 | `phase5_ci_path_audit.md`, `.github/workflows/nightly.yml`, `docs/03_implementation/p2p_mainline_runbook.md` 6.4, `docs/01_project/setup_guide.md` |

共通の検証手順
- `corepack enable pnpm && pnpm vitest run src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx`（同期導線）
- `corepack enable pnpm && pnpm vitest run src/tests/unit/components/topics/TopicSelector.test.tsx src/tests/unit/components/posts/PostCard.test.tsx src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx`（グローバルコンポーザー/投稿削除/トレンド導線）
- `./scripts/test-docker.{sh,ps1} ts --scenario profile-avatar-sync --service-worker` / `./scripts/test-docker.ps1 rust -Test profile_avatar_sync -NoBuild`（プロフィール Doc/Blob Stage4 + Service Worker）
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
| ユーザー | `/search` (users) | `search_users` で実ユーザー検索、フォロー/解除ボタン | 稼働中 | フォロー結果と Infinite Query は稼働中。2025年11月10日に関連度/最新順トグル・`allow_incomplete` フォールバック・SearchBar 警告スタイルを実装し、`npx pnpm vitest run src/tests/unit/hooks/useUserSearchQuery.test.tsx src/tests/unit/components/search/UserSearchResults.test.tsx`（`tmp/logs/vitest_user_search_allow_incomplete_20251110132951.log`）と Docker `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build`（`tmp/logs/user_search_pagination_20251110-142854.log`）で回帰を取得済み。2025年11月12日に `nightly.yml` へ `nightly.user-search-pagination` ジョブを追加し、`tmp/logs/user_search_pagination_<timestamp>.log` を `user-search-pagination-logs`、`test-results/user-search-pagination/*.json` を `user-search-pagination-reports` artefact として保存する運用を確立（`phase5_ci_path_audit.md` / Runbook 6.4 に登録）。 |

### 1.5 設定 & デバッグ
| セクション | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| 外観 | `/settings` | テーマ切替（ライト/ダーク） | 稼働中 | `useUIStore` 経由で永続化 |
| アカウント | `/settings` | プロフィール編集モーダル、鍵管理プレースホルダー | 改善中 | プロフィール編集は稼働中。鍵管理ボタンは未配線 |
| プライバシー | `/settings` | 公開設定／オンライン表示トグル | 稼働中 | Stage4（2025年11月12日）で Service Worker / BroadcastChannel / `cache_metadata` TTL 30 分 / `offlineApi.addToSyncQueue` ログを実装し、`scripts/test-docker.{sh,ps1} ts --scenario profile-avatar-sync --service-worker` / `./scripts/test-docker.ps1 rust -Test profile_avatar_sync` / `pnpm vitest run ...ProfileAvatarSyncWorker.test.ts` を再実行。`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` は `nightly.profile-avatar-sync` artefact として Runbook Chapter4 / `phase5_ci_path_audit.md` に連携済み。 |
| P2P 接続 | `/settings` | `PeerConnectionPanel` で手動接続/履歴管理 | 稼働中 | `connect_to_peer` コマンドに紐づく |
| Bootstrap 設定 | `/settings` | ブートストラップノード一覧の取得/登録/リセット | 稼働中 | `set_bootstrap_nodes` などと連携 |
| 開発者ツール (DEV) | `/settings`（開発モード） | `NostrTestPanel`, `P2PDebugPanel` | 改善中 | UI は Dev 限定。計測ログとテスト誘導の整理が backlog |

### 1.6 ダイレクトメッセージ
| セクション | パス/配置 | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| プロフィール起点 | `/profile/$userId` | `メッセージ` ボタン→`DirectMessageDialog` モーダル、履歴ロード・送信・再送ボタン | 稼働中 | Kind4 IPC に加えて多端末既読同期を実装済み。`tmp/logs/direct_message_inbox_20251113-140827.log`（Vitest）と `tmp/logs/rust_docker_20251113-141846.log`（Docker `./scripts/test-docker.ps1 rust`）で Dialog/Store/contract テストを再実行し、`mark_direct_message_conversation_read` のレプリケーションを確認。 |
| グローバル導線 | ヘッダー右上 | `MessageCircle` ボタンで最新会話またはアクティブ会話を開き、隣の `Plus` アイコンから `DirectMessageInbox`（会話一覧 + 宛先入力）を起動 | 稼働中 | `useDirectMessageBadge` と共有する Inbox が会話検索/補完・Enter での自動起動・`@tanstack/react-virtual` の実測スクロール・未読/同期バッジを備え、`tmp/logs/direct_message_inbox_20251113-140827.log` で UI 回帰を監視。 |
| Summary Panel | `/trending`, `/following` | `TrendingSummaryPanel` / `FollowingSummaryPanel` の DM カードに未読件数・最終受信時刻・`DM Inbox を開く` CTA を表示 | 稼働中 | CTA から開いた Inbox が SQLite の会話サマリ/既読タイムスタンプを即座に反映し、ヘッダーと同じストアで多端末共有・検索・仮想スクロールが完結する。Docker Rust ログ `tmp/logs/rust_docker_20251113-141846.log` で `tests/contract/direct_messages.rs` を含む一連の DM 契約テストを確認。 |

## 2. グローバル要素
- **ステータスカード**: `RelayStatus` / `P2PStatus` が 30 秒間隔でステータス取得。フェイルオーバー時のバックオフと手動再試行を実装。
- **同期系 UI**: `SyncStatusIndicator`（Inventory 5.11）と `OfflineIndicator` が `offlineStore` / `syncEngine` の状態を共有し、オンライン復帰後 2 秒の自動同期・5 分ごとの定期同期・手動同期ボタン・競合解決ダイアログを提供。2025年11月07日: `get_cache_status` を 60 秒間隔＋手動操作で取得し、キャッシュ合計/ステール件数とタイプ別統計をポップオーバーに表示。ステールなタイプには「再送キュー」ボタンを表示し、`add_to_sync_queue`（`action_type=manual_sync_refresh`）で手動再送を登録できるようになった。2025年11月09日: `cache_types.metadata`（要求者/要求時刻/Queue ID/発行元）整形と OfflineIndicator の誘導コピーに加え、`list_sync_queue_items` を介した再送履歴（Queue ID フィルタ、最新 ID ハイライト、ステータス別バッジ、要求者/要求時刻/発行元/再試行回数、エラーメッセージ）をポップオーバーに追加。
- **リアルタイム更新**: `RealtimeIndicator` と `useP2PEventListener` で投稿受信を通知し、`topicStore` の未読管理を更新。
- **グローバルコンポーザー**: `useComposerStore` で Home/Sidebar/Topic から共通モーダルを制御し、投稿完了後にストアをリセット。2025年11月10日: `TopicSelector` に「新しいトピックを作成」ショートカット、`TopicFormModal(mode='create-from-composer')`、`useComposerStore.applyTopicAndResume` を実装し、作成直後に投稿継続できるようにした。`pnpm` 不足で `TopicSelector.test.tsx` / `PostCard.test.tsx` を再実行できていないため、`corepack enable pnpm` ＋ Docker ルートでの検証が必要（Inventory 5.9 / `phase5_ci_path_audit.md`）。
- **プロフィール導線**: `UserSearchResults` と `/profile/$userId` が連携し、フォロー操作後に React Query キャッシュを即時更新。`DirectMessageDialog` は React Query ベースの履歴ロード・未読リセット・無限スクロールに加え、`mark_direct_message_conversation_read` で同期された `lastReadAt` を即時にストアへ反映する。ヘッダー右上の `MessageCircle` ボタン／`Plus` ボタン、および `/trending` `/following` の Summary Panel CTA は `useDirectMessageBadge` と `useDirectMessageStore` を共有し、既存会話なら即座にモーダルを開き、未読が無い場合でも `DirectMessageInbox` から宛先入力・会話検索・Enter 補完で新規 DM を開始できる。2025年11月13日: `direct_message_conversations` ハイドレートに `lastReadAt` を含め、`DirectMessageInbox` へ仮想スクロール最適化/多端末既読バッジ/検索ログ（`tmp/logs/direct_message_inbox_20251113-140827.log`）を追加。Rust 側は `tests/contract/direct_messages.rs` を Docker 実行（`tmp/logs/rust_docker_20251113-141846.log`）で監視し、バックログは送信レート制御と再送バックオフに絞られた。
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
10. 2025年11月14日: `add_relay` / `join_topic_by_name` / `delete_events` / `get_nostr_pubkey` を Tauri + フロントエンド双方から撤去し、未接続 API リストを 0 件に更新。以降はデバッグ専用の `clear_all_accounts_for_test` のみ残存し、通常フローに属さないため除外扱いとする（Inventory 3.2/3.3 へ記録済み）。

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
