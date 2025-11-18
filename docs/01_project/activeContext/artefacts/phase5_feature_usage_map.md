# Phase 5 機能使用状況マップ（アクティブ）

作成日: 2025年11月14日  
最終更新: 2025年11月18日

参照:
- `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md`（最終更新: 2025年11月10日）
- `docs/01_project/activeContext/artefacts/phase5_user_flow_summary.md`（最終更新: 2025年11月13日）

> 表中の `Inv x.x` は `phase5_user_flow_inventory.md` のセクション、`Sum 5.x` は `phase5_user_flow_summary.md` のサブセクションを指します。

## 1.1 オンボーディング & 認証

| 機能 | 呼び出し元 UI/ストア | フロント実装 | 主な Tauri/Rust 実装 |
| --- | --- | --- | --- |
| 新規アカウント作成 / 鍵生成 | `WelcomeScreen`（`/welcome`, Inv 1.1） | `kukuri-tauri/src/components/auth/WelcomeScreen.tsx`<br>`kukuri-tauri/src/routes/welcome.tsx`<br>`kukuri-tauri/src/stores/authStore.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/auth_commands.rs::generate_keypair`<br>`kukuri-tauri/src-tauri/src/presentation/commands/secure_storage_commands.rs::add_account` |
| nsec ログイン / セキュア保存 | `LoginForm`（`/login`, Inv 1.1）および起動時の `authStore.initialize` | `kukuri-tauri/src/components/auth/LoginForm.tsx`<br>`kukuri-tauri/src/routes/login.tsx`<br>`kukuri-tauri/src/stores/authStore.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/auth_commands.rs::{login, logout}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/secure_storage_commands.rs::{secure_login,list_accounts,get_current_account}` |
| プロフィール初期設定（Metadata / Avatar / Privacy） | `ProfileSetup`（`/profile-setup`, Inv 1.1, Sum 5.1） | `kukuri-tauri/src/components/auth/ProfileSetup.tsx`<br>`kukuri-tauri/src/hooks/useProfileAvatarSync.ts`<br>`kukuri-tauri/src/stores/privacySettingsStore.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::{update_nostr_metadata,upload_profile_avatar,fetch_profile_avatar,update_privacy_settings}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs::update_nostr_metadata` |
| アカウント切替 / ログアウト | `AccountSwitcher`（ヘッダー, Inv 1.2） | `kukuri-tauri/src/components/auth/AccountSwitcher.tsx`<br>`kukuri-tauri/src/components/layout/Header.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/secure_storage_commands.rs::{list_accounts,switch_account,remove_account}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/auth_commands.rs::logout` |

## 1.2 認証後メイン UI（タイムライン/フィード）

| 機能 | 呼び出し元 UI/ストア | フロント実装 | 主な Tauri/Rust 実装 |
| --- | --- | --- | --- |
| ホーム/参加トピックタイムライン表示 | `Home`（`/`, Inv 1.2）で `useTimelinePosts`/`usePostsByTopic` を切替 | `kukuri-tauri/src/pages/Home.tsx`<br>`kukuri-tauri/src/hooks/usePosts.ts`<br>`kukuri-tauri/src/stores/topicStore.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::get_posts`<br>`kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs::get_topic_stats` |
| 投稿作成・返信・グローバルコンポーザー | `PostComposer`（Home/Topic/Global Composer, Inv 1.2 & 1.6） | `kukuri-tauri/src/components/posts/PostComposer.tsx`<br>`kukuri-tauri/src/components/posts/GlobalComposer.tsx`<br>`kukuri-tauri/src/stores/{composerStore,draftStore}.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::create_post`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs::{publish_text_note,publish_topic_post}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/offline_commands.rs::{save_offline_action,sync_offline_actions}` |
| リアクション / ブースト / ブックマーク | `PostCard` アクション群（Inv 1.2, Sum 5.7） | `kukuri-tauri/src/components/posts/PostCard.tsx`<br>`kukuri-tauri/src/hooks/usePosts.ts`<br>`kukuri-tauri/src/stores/postStore.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::{like_post,boost_post,bookmark_post,unbookmark_post,get_bookmarked_post_ids}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs::send_reaction` |
| 投稿削除とオフライン再送 | `PostCard` メニュー + `SyncStatusIndicator`（Inv 5.10） | `kukuri-tauri/src/hooks/usePosts.ts`（`useDeletePost`）<br>`kukuri-tauri/src/components/posts/PostCard.tsx`<br>`kukuri-tauri/src/components/SyncStatusIndicator.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::delete_post`<br>`kukuri-tauri/src-tauri/src/presentation/commands/offline_commands.rs::{save_offline_action,add_to_sync_queue,update_sync_status}` |
| トレンドフィード | `routes/trending.tsx` + `TrendingSummaryPanel`（Inv 1.2, Sum 5.7） | `kukuri-tauri/src/routes/trending.tsx`<br>`kukuri-tauri/src/components/trending/TrendingSummaryPanel.tsx`<br>`kukuri-tauri/src/hooks/useTrendingFeeds.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs::list_trending_topics`<br>`kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::list_trending_posts` |
| フォロー中フィード | `routes/following.tsx` / `FollowingSummaryPanel`（Inv 1.2, Sum 5.7） | `kukuri-tauri/src/routes/following.tsx`<br>`kukuri-tauri/src/components/following/FollowingSummaryPanel.tsx`<br>`kukuri-tauri/src/hooks/useTrendingFeeds.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::list_following_feed`<br>`kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::{follow_user,unfollow_user}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs::subscribe_to_user` |

## 1.3 トピック管理

| 機能 | 呼び出し元 UI/ストア | フロント実装 | 主な Tauri/Rust 実装 |
| --- | --- | --- | --- |
| トピック一覧 / 作成 / 編集 | `TopicsPage`（`/topics`, Inv 1.3）とサイドバーの `TopicFormModal` | `kukuri-tauri/src/routes/topics.tsx`<br>`kukuri-tauri/src/components/topics/TopicFormModal.tsx`<br>`kukuri-tauri/src/components/topics/TopicCard.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs::{get_topics,get_topic_stats,create_topic,update_topic}` |
| トピック詳細 & P2P メッシュ（参加/離脱） | `TopicPage`（`/topics/$topicId`, Inv 1.3, Sum 5.9）と `TopicMeshVisualization` | `kukuri-tauri/src/routes/topics.$topicId.tsx`<br>`kukuri-tauri/src/components/TopicMeshVisualization.tsx`<br>`kukuri-tauri/src/hooks/useP2P.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs::{join_topic,leave_topic,delete_topic}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/p2p_commands.rs::{join_p2p_topic,leave_p2p_topic,get_p2p_metrics}` |

## 1.4 検索

| 機能 | 呼び出し元 UI/ストア | フロント実装 | 主な Tauri/Rust 実装 |
| --- | --- | --- | --- |
| 投稿/トピック検索タブ | `Search` ルート（`/search`, Inv 1.4）の posts/topics タブ | `kukuri-tauri/src/routes/search.tsx`<br>`kukuri-tauri/src/hooks/usePosts.ts`<br>`kukuri-tauri/src/stores/topicStore.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::get_posts`<br>`kukuri-tauri/src-tauri/src/presentation/commands/topic_commands.rs::get_topics` |
| ユーザー検索 / DM候補 | `Search` ルート users タブ + `DirectMessageInbox`（Inv 1.4, Sum 5.4） | `kukuri-tauri/src/components/search/UserSearchResults.tsx`<br>`kukuri-tauri/src/hooks/useUserSearchQuery.ts`<br>`kukuri-tauri/src/components/directMessages/DirectMessageInbox.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::search_users`<br>`kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::{follow_user,unfollow_user}` |

## 1.5 設定 & デバッグ

| 機能 | 呼び出し元 UI/ストア | フロント実装 | 主な Tauri/Rust 実装 |
| --- | --- | --- | --- |
| プロフィール編集 / プライバシー / アバター同期 | `Settings`（`/settings`, Inv 1.5, Sum 5.1）の `ProfileEditDialog` | `kukuri-tauri/src/components/settings/ProfileEditDialog.tsx`<br>`kukuri-tauri/src/components/auth/ProfileSetup.tsx`<br>`kukuri-tauri/src/hooks/useProfileAvatarSync.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::{update_nostr_metadata,update_privacy_settings,upload_profile_avatar,fetch_profile_avatar}` |
| P2P 接続管理 | `PeerConnectionPanel`（設定, Inv 1.5） | `kukuri-tauri/src/components/p2p/PeerConnectionPanel.tsx`<br>`kukuri-tauri/src/stores/p2pStore.ts`<br>`kukuri-tauri/src/lib/api/p2p.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/p2p_commands.rs::{initialize_p2p,get_node_address,get_p2p_status,connect_to_peer}` |
| Bootstrap 設定（n0/カスタム/CLI適用） | `BootstrapConfigPanel`（設定, Inv 1.5）と `RelayStatus` の適用ボタン | `kukuri-tauri/src/components/p2p/BootstrapConfigPanel.tsx`<br>`kukuri-tauri/src/components/RelayStatus.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/p2p_commands.rs::{get_bootstrap_config,set_bootstrap_nodes,clear_bootstrap_nodes,apply_cli_bootstrap_nodes}` |
| Nostr テストパネル | `NostrTestPanel`（開発者設定, Inv 1.5） | `kukuri-tauri/src/components/NostrTestPanel.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs::{publish_text_note,publish_topic_post,send_reaction,subscribe_to_topic,list_nostr_subscriptions}` |
| P2P デバッグパネル | `P2PDebugPanel`（開発者設定, Inv 1.5, Sum 5.6） | `kukuri-tauri/src/components/P2PDebugPanel.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/p2p_commands.rs::{broadcast_to_topic,join_p2p_topic,leave_p2p_topic,get_p2p_metrics}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs::list_nostr_subscriptions` |

## 1.6 その他グローバル要素

| 機能 | 呼び出し元 UI/ストア | フロント実装 | 主な Tauri/Rust 実装 |
| --- | --- | --- | --- |
| RelayStatus & CLI ブートストラップ適用 | サイドバー下部の `RelayStatus`（Inv 1.6, Sum 5.6） | `kukuri-tauri/src/components/RelayStatus.tsx`<br>`kukuri-tauri/src/components/layout/Sidebar.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/p2p_commands.rs::{get_relay_status,get_bootstrap_config,apply_cli_bootstrap_nodes}` |
| SyncStatusIndicator & OfflineIndicator | ヘッダー右上/画面下部（Inv 1.6, Sum 5.5/5.11） | `kukuri-tauri/src/components/SyncStatusIndicator.tsx`<br>`kukuri-tauri/src/components/OfflineIndicator.tsx`<br>`kukuri-tauri/src/hooks/useSyncManager.ts`<br>`kukuri-tauri/src/stores/offlineStore.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/offline_commands.rs::{get_cache_status,list_sync_queue_items,add_to_sync_queue,save_offline_action,sync_offline_actions,update_cache_metadata,update_sync_status,cleanup_expired_cache}` |
| RealtimeIndicator / useP2PEventListener | ヘッダー `RealtimeIndicator` とグローバル P2P イベント（Inv 1.6） | `kukuri-tauri/src/components/RealtimeIndicator.tsx`<br>`kukuri-tauri/src/hooks/useP2PEventListener.ts`<br>`kukuri-tauri/src/hooks/useDataSync.ts` | `kukuri-tauri/src/lib/api/p2p.ts` 経由で `kukuri-tauri/src-tauri/src/presentation/commands/p2p_commands.rs::{initialize_p2p,join_p2p_topic,leave_p2p_topic}` を呼び出し、イベント配信を購読 |

## 1.7 プロフィール / ソーシャル / メッセージ

| 機能 | 呼び出し元 UI/ストア | フロント実装 | 主な Tauri/Rust 実装 |
| --- | --- | --- | --- |
| プロフィール表示 / 投稿一覧 | `ProfilePage`（`/profile/$userId`, Inv 1.7） | `kukuri-tauri/src/routes/profile.$userId.tsx`<br>`kukuri-tauri/src/components/posts/PostCard.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::{get_user,get_user_by_pubkey}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/post_commands.rs::get_posts` |
| フォロー / フォロワー・フォロー中リスト | `ProfilePage` の `Follow` ボタンと `UserList`、`UserSearchResults`（Inv 1.7, Sum 5.7） | `kukuri-tauri/src/routes/profile.$userId.tsx`（`UserList`）<br>`kukuri-tauri/src/components/search/UserSearchResults.tsx` | `kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::{follow_user,unfollow_user,get_followers,get_following}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs::subscribe_to_user` |
| ダイレクトメッセージ（プロフィール/ヘッダー導線） | `MessageCircle`（`/profile/$userId`）とヘッダー `DirectMessageInbox`（Inv 1.7, Sum 5.4） | `kukuri-tauri/src/components/directMessages/DirectMessageDialog.tsx`<br>`kukuri-tauri/src/components/directMessages/DirectMessageInbox.tsx`<br>`kukuri-tauri/src/stores/directMessageStore.ts`<br>`kukuri-tauri/src/hooks/useDirectMessageBootstrap.ts` | `kukuri-tauri/src-tauri/src/presentation/commands/direct_message_commands.rs::{send_direct_message,list_direct_messages}`<br>`kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs::search_users`（宛先候補） |

## 2. 未使用機能（削除候補）

`docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md:168-196` で backlog 管理している未導線 API／dead code をコードベースと突合した結果を記録する。各項目について削除 or 保留の判断と次アクションを明記した。2025年11月18日時点での未使用機能は 0 件となり、UI/コマンド・バックエンド双方の backlog を消化済み。

### 2.1 UI/コマンド未導線

| 種別 | 機能 / コマンド | 実装箇所 | 未使用理由 | 判断 | 次アクション |
| --- | --- | --- | --- | --- | --- |
| Tauriコマンド | `add_relay` | （2025年11月14日撤去） | 外部リレーを無効化した Phase 5 方針に合わせ API ごと削除済み。 | 完了 | Phase 7 で外部リレーを再開する際に新仕様として再設計する。 |
| Tauriコマンド | `get_nostr_pubkey` | （2025年11月14日撤去） | `authStore` が pubkey/npub を保持しているため API を廃止。 | 完了 | multi-identity 再開時に SecureStorage からの再取得方式を再検討する。 |
| Tauriコマンド | `delete_events` | （2025年11月14日撤去） | 投稿削除フロー内で EventService が処理するため直接コマンドを廃止。 | 完了 | `delete_post` の統合テストで削除伝搬とキャッシュ整合性を保証する。 |
| Tauriコマンド | `join_topic_by_name` | （2025年11月14日撤去） | Global Composer fallback を再設計するまで API を廃止。 | 完了 | 次期仕様で名称解決ロジックと UI 導線をまとめて復活させる。 |
| Tauriコマンド | ~~`clear_all_accounts_for_test`~~ | （2025年11月18日撤去） | Dev 導線未接続のまま残存していたため UI/バックエンド双方から削除。 | 完了（2025年11月18日） | `SecureStorageApi` / `secure_storage_commands.rs` / `lib.rs` からコマンド登録を除去し、Keyring クリア処理も `DefaultSecureStorage` から削除済み。 |

### 2.2 バックエンド dead_code

| 種別 | 機能 | 実装箇所 | 未使用理由 | 判断 | 次アクション |
| --- | --- | --- | --- | --- | --- |
| Rust helper | ~~`TopicMesh::get_peers` / `get_recent_messages` / `clear_cache`~~ | （2025年11月18日撤去） | テスト専用の内部検査 API だったため、`TopicMesh` 本体から削除しユニットテストを `get_stats` / `subscribe` ベースに書き換えた。 | 完了（2025年11月18日） | `domain/p2p/topic_mesh.rs` と `domain/p2p/tests/topic_mesh_tests.rs`（および同ファイル内のテスト）でデバッグ API 依存を排除し、dead_code 扱いを解消。 |
| Rustサービス | ~~`AppState.encryption_service` / `DefaultEncryptionService`~~ | `kukuri-tauri/src-tauri/src/state.rs`（2025年11月14日時点で削除済） | DM 暗号化導線が未整備のまま放置されていたため、`AppState` から暗号サービス依存を撤去。暗号化ロジックを再導入する場合は `infrastructure::crypto` を再配線して利用する。 | 完了（2025年11月14日） | Phase6 以降で暗号導線を追加する際は、新ポート／サービス経由で必要な箇所に注入する。 |

## 3. 部分的に使用されている機能

`phase5_user_flow_inventory.md` と `phase5_user_flow_summary.md` で「一部の画面／モードからしか到達できない」と記録されている機能を洗い出し、現状使えている導線と欠落している導線をペアで整理する。



### 3.1 アクティブ導線トレース（2025年11月17日更新）

| 導線ID / Flow | UIイベント | Hook / Store | 呼び出す Tauri コマンド | テスト ID / Nightly artefact |
| --- | --- | --- | --- | --- |
| `Inv 1.3 / 5.9` トピック作成&参加導線 | `TopicsPage`・`Sidebar`・`PostComposer` から `TopicFormModal.onSubmit` を発火し、`Inv 1.3` で定義された作成/参加フローを辿る。 | `useTopicStore.queueTopicCreation` → `useOfflineStore.addPendingAction` + `useComposerStore.watchPendingTopic`（オフライン） / `useTopicStore.createTopic` → `joinTopic` → `useComposerStore.resolvePendingTopic`（オンライン）。 | `enqueue_topic_creation`, `create_topic`, `join_topic`, `mark_pending_topic_synced` / `mark_pending_topic_failed`。 | `ts-scenario topic-create`（`tmp/logs/topic_create_<ts>.log` + `test-results/topic-create/*.json` artefact）<br>`src/tests/unit/scenarios/topicCreateOffline.test.tsx`<br>`tests/integration/topic_create_join.rs` |
| `Inv 1.2 / 5.7` トレンド/フォロー Summary | `Sidebar` のトレンド CTA・`/trending` ルートのタブ切替・Summary Panel の DM CTA で `Inv 5.7` の指標を参照。 | `useTrendingTopicsQuery` / `useTrendingPostsQuery` / `useFollowingFeedQuery` / `prefetchTrendingCategory`、DM CTA は `useDirectMessageBadge` + `useDirectMessageStore.openInbox`。 | `list_trending_topics`, `list_trending_posts`, `list_following_feed`, `get_topic_stats`。 | `ts-scenario trending-feed`（`tmp/logs/trending-feed/<ts>.log` + `test-results/trending-feed/{reports,prometheus,metrics}` artefact）<br>`src/tests/unit/routes/{trending,following}.test.tsx`<br>`src/tests/unit/hooks/useTrendingFeeds.test.tsx` |
| `Inv 1.5 / 5.1` 鍵管理ダイアログ | `SettingsPage` の「鍵管理」ボタンで `KeyManagementDialog` を開き、Export/Import/Copy/Save/ファイル選択を実行。 | `KeyManagementDialog.handleExport` → `TauriApi.exportPrivateKey` → `useKeyManagementStore.recordAction`、`handleImport` → `useAuthStore.loginWithNsec(true)` → `SecureStorageApi.addAccount`。 | `export_private_key`, `login`, `secure_login`, `add_account`。 | `src/tests/unit/components/settings/KeyManagementDialog.test.tsx`<br>`src/tests/unit/stores/keyManagementStore.test.ts`<br>`tests/key_management.rs` + `./scripts/test-docker.ps1 rust -Test key_management`（artefact `key_management`） |
| `Inv 5.1 / 5.4` プライバシートグル | `SettingsPage` の `Switch#public-profile` / `Switch#show-online`、`ProfileSetup` / `ProfileEditDialog` Submit で `Inv 5.1` の公開範囲を更新。 | `usePrivacySettingsStore.setPublicProfile|setShowOnlineStatus` → `persistPrivacy`（`TauriApi.updatePrivacySettings` + `updateNostrMetadata`）→ `authStore.updateUser`、`useProfileAvatarSync.syncNow`。 | `update_privacy_settings`, `update_nostr_metadata`, `upload_profile_avatar`, `fetch_profile_avatar`。 | `scripts/test-docker.{sh,ps1} ts --scenario profile-avatar-sync --service-worker`（`profile-avatar-sync-logs` + `test-results/profile-avatar-sync/*` artefact）<br>`./scripts/test-docker.ps1 rust -Test profile_avatar_sync`<br>`src/tests/unit/routes/settings.test.tsx` |
| `Inv 1.5` DEV 検証パネル（Nostr/P2P） | `SettingsPage` DEV セクションで `NostrTestPanel`（投稿/購読/リアクション）と `P2PDebugPanel`（Join/Leave/Broadcast/metrics）を操作。 | `nostrApi.publishTextNote` / `.publishTopicPost` / `.sendReaction` / `.subscribeToTopic`、`useP2P`（`joinTopic` / `leaveTopic` / `broadcast`）、`useNostrSubscriptions`（`listNostrSubscriptions`） + `p2pApi.getMetrics`。 | `publish_text_note`, `publish_topic_post`, `send_reaction`, `subscribe_to_topic`, `join_p2p_topic`, `leave_p2p_topic`, `broadcast_to_topic`, `get_p2p_metrics`, `list_nostr_subscriptions`。 | `src/tests/unit/components/NostrTestPanel.test.tsx`<br>`src/tests/unit/components/P2PDebugPanel.test.tsx`<br>`src/tests/unit/routes/settings.test.tsx`（Nightly Frontend Unit Tests: `pnpm test:unit` artefact） |

> 2025年11月17日: `node scripts/check-tauri-commands.mjs` を実行し、Tauri コマンド 85 件すべてが上記導線いずれかから呼び出されることを確認。

### 3.2 ギャップとアクション

| 対象 | 使用されている導線 | 欠落している導線 / 利用文脈 | 主な実装 / コマンド | 推奨アクション |
| --- | --- | --- | --- | --- |
| トピック作成導線（`TopicFormModal` + `create_topic` / `join_topic`） | `/topics` (`TopicsPage`) から `TopicFormModal` を開く導線のみが有効で、ここから `create_topic` → `join_topic` を実行できる。`phase5_user_flow_inventory.md:43` | タイムライン系 UI には導線がなく、`GlobalComposer` やサイドバー、`TopicSelector` から直接作成できないため離脱が必須。`phase5_user_flow_inventory.md:94` `phase5_user_flow_inventory.md:495-505` | `TopicFormModal` / `topicStore.queueTopicCreation` / `TopicService::enqueue_topic_creation` でオフライン再送までは整備済み。`phase5_user_flow_inventory.md:505` `phase5_user_flow_inventory.md:525` | `TopicCreationDialog` をコンポーザーに組み込み、`TopicSelector` ショートカットとサイドバー「新規投稿」ボタンから作成→即投稿に戻れる経路を追加する。`phase5_user_flow_inventory.md:498-503` `phase5_user_flow_inventory.md:519-521` |
| トレンド/フォローカテゴリー（Sidebar） | `routes/trending.tsx` と `TrendingSummaryPanel` からはトレンド/フォロー情報を閲覧できる。`phase5_feature_usage_map.md:29` `phase5_user_flow_inventory.md:91` | Sidebar の「トレンド」「フォロー中」ボタンは `navigate` を無効化し「準備中」ツールチップのみで、カテゴリーショートカットからフィードを開けない。`phase5_user_flow_inventory.md:249-254` | `Sidebar`・`TrendingSummaryPanel` / `routes/trending.tsx` / `routes/following.tsx` / `useTrendingFeeds` によってデータ自体は取得済み。`phase5_feature_usage_map.md:29` | Sidebar カテゴリーのナビゲーションを有効化し、`phase5_ci_path_audit.md` と連携した `trending-feed` / `following-feed` テスト ID を追加して導線の回帰を監視する。`phase5_user_flow_inventory.md:249-257` |
| 鍵管理ボタン（`KeyManagementDialog` 構想 / `export_private_key`） | 設定 > 外観・アカウントに「鍵管理」ボタンが表示されている。`phase5_user_flow_inventory.md:58` | ボタンはダミーでダイアログが存在せず、バックアップ/復旧 UI が提供されていない。`phase5_user_flow_inventory.md:96` `phase5_user_flow_inventory.md:282-303` `phase5_user_flow_summary.md:125` | 既存コマンド `export_private_key`・`SecureStorageApi.addAccount`・`validate_nsec`（予定）が計画ドキュメントにまとまっている。`phase5_user_flow_inventory.md:282-303` | `KeyManagementDialog`（エクスポート/インポートタブ + 注意喚起 UI）を実装し、`pnpm vitest …KeyManagementDialog.test.tsx` や Runbook 手順を追加して MVP Exit のギャップを閉じる。`phase5_user_flow_inventory.md:284-295` `phase5_user_flow_summary.md:125` |
| プライバシー設定トグル | オンボーディングの `ProfileSetup` からは `update_privacy_settings` を呼び出して設定を同期できる。`phase5_feature_usage_map.md:18` | 設定画面の「プロフィール公開/オンライン表示」トグルはローカル永続のみでバックエンドに反映されない。`phase5_user_flow_inventory.md:59` `phase5_user_flow_inventory.md:97` `phase5_user_flow_summary.md:126` | `privacySettingsStore` と `user_commands.rs::update_privacy_settings` が既に存在し、UI/Service Worker からも利用できる。`phase5_feature_usage_map.md:18` | 設定トグルを `update_privacy_settings` へ接続し、`errorHandler` / Nightly テストで多端末同期と公開範囲の反映を検証する。`phase5_user_flow_summary.md:126` |
| DEV 専用テストパネル（`NostrTestPanel` / `P2PDebugPanel`） | `/settings` の DEV モードでのみ表示され、Nostr/P2P コマンドの直叩きができる。`phase5_user_flow_inventory.md:62-63` | 本番ビルドでは完全に非表示のため、これらの検証コマンドを UI から実行する導線やログ採取手段が無い。`phase5_user_flow_inventory.md:62-63` | `publish_text_note` / `send_reaction` / `join_p2p_topic` / `get_p2p_metrics` などの Tauri コマンド呼び出しを内包。`phase5_user_flow_inventory.md:62-63` | DEV パネル依存の検証を Runbook 手順 or ops 向け設定タブへ移植し、必要な操作だけを権限付き UI として提供する。 |
