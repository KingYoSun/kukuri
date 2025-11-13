# Phase 5 機能使用状況マップ（アクティブ）

作成日: 2025年11月14日  
最終更新: 2025年11月14日

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

`docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md:168-196` で backlog 管理している未導線 API／dead code をコードベースと突合した結果を記録する。各項目について削除 or 保留の判断と次アクションを明記した。

### 2.1 UI/コマンド未導線

| 種別 | 機能 / コマンド | 実装箇所 | 未使用理由 | 判断 | 次アクション |
| --- | --- | --- | --- | --- | --- |
| Tauriコマンド | `add_relay` | `kukuri-tauri/src/lib/api/nostr.ts:54`<br>`docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md:181` | `rg "nostrApi.addRelay"` がユニットテストにしかヒットせず、UI からの導線が皆無。さらに `kukuri-tauri/src-tauri/src/lib.rs:123-148` にコマンド登録が存在せず、呼び出すとエラーになる。 | 削除（Phase7まで無効化） | ラッパー/モック/ドキュメントから `add_relay` を除去し、外部リレー運用を再開するタイミングで仕様ごと再設計する。 |
| Tauriコマンド | `get_nostr_pubkey` | `kukuri-tauri/src/lib/api/nostr.ts:123`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs:109` | `useAuthStore` がログイン時点で `currentUser.pubkey/npub` を保持しており（例: `kukuri-tauri/src/stores/authStore.ts:34-121`）、`rg "getNostrPubkey"` の利用箇所はテストのみ。 | 保留（multi-account設計待ち） | 複数アイデンティティ/バックアップ導線の仕様確定までは封印。要件が固まらない場合は API ごと削除、必要ならフロント導線＋テストを追加する。 |
| Tauriコマンド | `delete_events` | `kukuri-tauri/src/lib/api/nostr.ts:128`<br>`kukuri-tauri/src-tauri/src/presentation/commands/event_commands.rs:118` | `useDeletePost` は `delete_post` のみを呼び、`rg "deleteEvents"` もラッパーとテストのみ。`phase5_user_flow_inventory.md:183` でも未接続扱い。 | 保留（`delete_post`拡張待ち） | 投稿削除 UI から Nostr 側の `delete_events` まで連結し、キャッシュ無効化と一緒に E2E で検証する。 |
| Tauriコマンド | `join_topic_by_name` | `kukuri-tauri/src/lib/api/p2p.ts:112`<br>`kukuri-tauri/src-tauri/src/presentation/commands/p2p_commands.rs:103-126` | 名前ベース参加を行う UI が無く、`phase5_user_flow_inventory.md:184-192` でも優先度1の未接続コマンドとして列挙されている。 | 保留（最優先で導線追加） | `/topics` やグローバルコンポーザーからトピック名入力→参加できる fallback を設計し、`topicStore.joinTopic` から呼び出す。 |
| Tauriコマンド | `clear_all_accounts_for_test` | `kukuri-tauri/src/lib/api/secureStorage.ts:93`<br>`kukuri-tauri/src-tauri/src/presentation/commands/secure_storage_commands.rs:80-99` | デバッグ目的だが UI から呼べず、`rg "clearAllAccountsForTest"` の利用箇所も API 内のみ。`phase5_user_flow_inventory.md:185,195` で backlog 管理されている。 | 保留（DEVパネル組み込み前提） | Settings > DEV パネルに「Secure Storage リセット」ボタン＋確認ダイアログを追加し、`errorHandler` ログと合わせて安全に実行できるようにする。 |

### 2.2 バックエンド dead_code

| 種別 | 機能 | 実装箇所 | 未使用理由 | 判断 | 次アクション |
| --- | --- | --- | --- | --- | --- |
| Rust helper | `TopicMesh::get_peers` / `get_recent_messages` / `clear_cache` | `kukuri-tauri/src-tauri/src/domain/p2p/topic_mesh.rs:101-122` | 3 関数とも `#[allow(dead_code)]` 指定で、利用箇所は `domain/p2p/tests/topic_mesh_tests.rs` などテストのみ（本番コードからの `rg` ヒットなし）。メトリクスは `get_p2p_metrics` へ集約されたため UI から参照されない。 | 削除 | TopicMesh の内部構造を DEV 用に保持する必要が無いので削除し、必要なら将来のデバッグ用に独立した util を用意する。 |
| Rustサービス | `AppState.encryption_service` / `DefaultEncryptionService` | `kukuri-tauri/src-tauri/src/state.rs:92`<br>`kukuri-tauri/src-tauri/src/infrastructure/crypto/default_encryption_service.rs:12-112` | `AppState` が `Arc<dyn EncryptionService>` を保持しているが、`rg ".encrypt_symmetric"` で実サービスからの呼び出しはゼロ。DM 暗号化ロードマップの途中で停止している。 | 保留（暗号化方針待ち） | Phase6 で DM 暗号化を導入する場合は `DirectMessageService` 等へ配線し、見送りならフィールドごと削除する判断を行う。 |
