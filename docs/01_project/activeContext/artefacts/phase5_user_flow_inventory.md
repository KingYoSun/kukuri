# Phase 5 ユーザー導線棚卸し
作成日: 2025年11月01日  
最終更新: 2025年11月08日

## 目的
- Phase 5 で想定しているデスクトップアプリ体験のうち、現状 UI から到達できる機能と欠落導線を把握する。
- フロントエンドから発行している Tauri コマンド (`invoke`) を棚卸しし、未使用 API と連携している画面を明確化する。
- 今後の改善タスク（導線不足／未接続 API）を `refactoring_plan_2025-08-08_v3.md` へ反映するためのインプットを提供する。

## 1. 画面別導線サマリー

### 1.1 オンボーディング & 認証
| 画面 | パス | 主なコンポーネント/機能 | 主な操作に紐づくコマンド |
| --- | --- | --- | --- |
| Welcome | `/welcome` | `WelcomeScreen` – アプリ紹介、`新規アカウント作成`、`既存アカウントでログイン` | `generate_keypair`（新規作成→`authStore.generateNewKeypair` 経由で SecureStorage 登録） |
| Login | `/login` | `LoginForm` – nsec入力、表示切替、セキュア保存チェック、警告表示 | `login`, `add_account`（保存時）、`initialize_nostr`, `get_relay_status` |
| Profile Setup | `/profile-setup` | `ProfileSetup` – 名前/表示名/自己紹介/画像/NIP-05、スキップ/保存 | `update_nostr_metadata` |

### 1.2 認証後メイン UI（タイムライン/サイドバー/ヘッダー）
| 要素 | パス/配置 | 主な機能 | 関連コマンド/ストア |
| --- | --- | --- | --- |
| Home タイムライン | `/` | 参加中トピックがあればフィルタリング、`PostComposer` で投稿/下書き/Markdown、`PostCard` でいいね・ブースト・返信・引用・ブックマーク | `get_posts`, `create_post`, `like_post`, `boost_post`, `bookmark_post`, `unbookmark_post`, `get_bookmarked_post_ids`, `send_reaction` |
| サイドバー | 共通 | 参加トピック一覧（P2P最終活動時刻でソート）、未読バッジ、`新規投稿`ボタンでグローバルコンポーザーを起動、カテゴリー（`トピック一覧`/`検索`/`トレンド`/`フォロー中`） | `join_topic`/`leave_topic`（`TopicCard` 経由、`subscribe_to_topic` と連動）、`useComposerStore.openComposer`、`useUIStore`（`activeSidebarCategory` でボタンをハイライト）、`prefetchTrendingCategory` / `prefetchFollowingCategory` でクエリを事前取得 |
| トレンドフィード | `/trending` (`routes/trending.tsx`) | トレンドスコア上位トピックのランキングカード表示、最新投稿プレビュー、更新時刻表示、参加/ブックマーク導線 | `list_trending_topics`, `list_trending_posts`, `get_topic_stats`, `join_topic`, `bookmark_post` |
| フォロー中フィード | `/following` (`routes/following.tsx`) | フォロー中ユーザーの最新投稿タイムライン、無限スクロール、再試行ボタン、プロフィール導線 | `list_following_feed`（`include_reactions` 対応）, `get_posts`, `follow_user`/`unfollow_user`, `subscribe_to_user`, `list_direct_messages` |
| ヘッダー | 共通 | `RealtimeIndicator`, `SyncStatusIndicator`, 通知アイコン（ダミー）、`AccountSwitcher`（アカウント切替/追加/削除/ログアウト） | `switch_account`, `list_accounts`, `remove_account`, `logout`, `disconnect_nostr`, `secure_login`（自動ログイン時） |
| グローバル同期 | 共通 | `SyncStatusIndicator` でオフライン同期進捗/競合対応、`useSyncManager` によるローカル→Tauri リクエスト | `create_post`, `like_post`, `join_topic`, `leave_topic`（未同期操作の再送） |

### 1.3 トピック管理
| 画面/コンポーネント | パス | 主な機能 | 主なコマンド |
| --- | --- | --- | --- |
| Topics 一覧 | `/topics` (`TopicsPage`) | トピック検索、`TopicCard` で詳細/参加切替、`TopicFormModal` で新規作成 | `get_topics`, `get_topic_stats`, `create_topic`, `join_topic`, `leave_topic`, `subscribe_to_topic` |
| トピック詳細 | `/topics/$topicId` (`TopicPage`) | トピック概要、`TopicMeshVisualization` で P2P 状態、`PostComposer`、投稿一覧、メニューから編集・削除 | `get_posts`, `create_post`, `join_topic`, `leave_topic`, `update_topic`, `delete_topic`, `broadcast_to_topic`, `get_p2p_metrics`（間接的）, `join_p2p_topic` |
| トピック編集 | モーダル | 既存トピックの名前/説明編集（`TopicFormModal` `mode=edit`） | `update_topic` |
| トピック削除 | ダイアログ | `TopicDeleteDialog` で削除確認 | `leave_topic`, `delete_topic` |

### 1.4 検索
| タブ | パス | 実装状況 | 備考 |
| --- | --- | --- | --- |
| 投稿検索 | `/search` (Tab: posts) | `usePosts` 全件からクライアントフィルタ | Tauri 呼び出し：初回ロード時の `get_posts` |
| トピック検索 | `/search` (Tab: topics) | `useTopics` データからクライアントフィルタ | `get_topics` を再利用 |
| ユーザー検索 | `/search` (Tab: users) | `search_users` で実ユーザーを取得し、フォロー/フォロー解除ボタンと `/profile/$userId` へのリンクを表示 | フォロー状態は React Query で即時更新。ページネーションとエラーUI・入力バリデーションは未整備。 |

### 1.5 設定 & デバッグ
| セクション | パス | 主な機能 | 主なコマンド |
| --- | --- | --- | --- |
| 外観・アカウント | `/settings` | テーマ切替、プロフィール編集モーダル、鍵管理ボタン（未実装） | `useUIStore.setTheme`、`update_nostr_metadata`、`authStore.updateUser`（ProfileEditDialog） |
| プライバシー設定 | `/settings` | プロフィール公開/オンライン表示トグル（ローカル永続化） | `usePrivacySettingsStore.setPublicProfile` / `.setShowOnlineStatus`（Tauri 連携待ち） |
| P2P 接続状況 | `/settings` | `PeerConnectionPanel` – ノード初期化、手動接続、履歴管理 | `initialize_p2p`, `get_node_address`, `get_p2p_status`, `connect_to_peer` |
| Bootstrap 設定 | `/settings` | `BootstrapConfigPanel` – ノード一覧取得/保存/リセット | `get_bootstrap_config`, `set_bootstrap_nodes`, `clear_bootstrap_nodes` |
| Nostr テスト (DEVのみ) | `/settings` `import.meta.env.DEV` 条件 | `NostrTestPanel` – テキストノート送信、トピック投稿、購読、リアクション送信、イベント受信ログ | `publish_text_note`, `publish_topic_post`, `send_reaction`, `subscribe_to_topic` |
| P2P デバッグ (DEVのみ) | `/settings` `import.meta.env.DEV` 条件 | `P2PDebugPanel` – Gossip/Mainline メトリクス取得、トピック参加、ブロードキャスト、サブスクリプション一覧 | `get_p2p_metrics`, `join_p2p_topic`, `leave_p2p_topic`, `broadcast_to_topic`, `list_nostr_subscriptions` |

### 1.6 その他グローバル要素
- サイドバー参加中トピックリスト: `topicStore` の `topicUnreadCounts` と `handleIncomingTopicMessage` で未読数と最終活動時刻を更新し、P2Pメッセージのタイムスタンプを秒換算して降順表示。
- `PostComposer` / `DraftManager`: シンプル/Markdown 切替と 2 秒デバウンスの自動保存で下書きを保持し、一覧から再開・削除が可能。
- `RelayStatus`（サイドバー下部）: `get_relay_status` を 30 秒ごとにポーリングし接続状態を表示。
- `P2PStatus`（サイドバー下部）: `useP2P` からの接続状態・メトリクス要約を表示し、接続時のみ 30 秒間隔で `refreshStatus` を実行。手動更新ボタンで `get_p2p_metrics` を再取得し、参加トピックとピア数を可視化。
- `SyncStatusIndicator`: `useSyncManager` の `syncStatus`/`pendingActionsCount` を参照し、Popover 内で同期進捗・競合・手動同期ボタンを表示。手動同期は `triggerManualSync` を通じて `syncEngine` の再実行を要求する。
- `RealtimeIndicator`: ブラウザの `online`/`offline` イベントと `realtime-update` カスタムイベント（`useP2PEventListener` が投稿受信時に発火）を監視し、最後の更新からの経過時間をバッジ表示する。
- `OfflineIndicator`: `useOfflineStore` の `isOnline`/`lastSyncedAt`/`pendingActions` を購読し、オフライン時は画面上部バナー、未同期アクションがあれば右下フローティングボタンで件数と同期状態を通知する。
- `GlobalComposer`: `MainLayout` 末尾にモーダルを配置し、`useComposerStore` で任意ページから `PostComposer` を開閉（投稿成功時は `complete` コールバックでストアをリセット）。
- `ProfileEditDialog`: 設定>アカウントからモーダルを開き、`update_nostr_metadata` → `authStore.updateUser` でプロフィールを即時更新。`ProfileForm` を再利用しつつトースト通知と `errorHandler` ログ出力を実装。
- `useP2PEventListener` / `useDataSync`: P2Pイベントを購読して投稿/トピックの React Query キャッシュを無効化し、5 分ごとの再フェッチとオンライン復帰時の全体再同期を実施。
- `offlineSyncService` と `offlineStore` / `syncEngine`: ネットワークイベントを監視し 30 秒間隔で同期、失敗時は指数バックオフで再試行しつつ `save_offline_action` / `sync_offline_actions` / `save_optimistic_update` などを通じて再送・競合解消を制御。
- `RootRoute` / `MainLayout`: 起動時に `authStore.initialize` と `useTopics` を待機し、未認証時は `/welcome` へ強制遷移、認証後はヘッダー・サイドバー付きレイアウトへ切り替える。
- `TopicPage` ヘッダーの最終更新表示: `topic.lastActive` を秒→ミリ秒換算して日付を描画（2025年11月02日修正適用）。

### 1.7 プロフィール詳細
| 要素 | パス/コンポーネント | 主な機能 | 備考 |
| --- | --- | --- | --- |
| プロフィール取得 | `/profile/$userId` (`ProfilePage`) | `getUserProfile` / `getUserProfileByPubkey` を順に呼び、存在するユーザー情報を `mapUserProfileToUser` で整形して表示。 | `npub` / `pubkey` の双方に対応。存在しない場合は空表示を返し、トーストで通知。 |
| 投稿一覧 | `/profile/$userId` (`ProfilePage`) | `getPosts({ author_pubkey, pagination: { limit: 50 } })` で個人投稿を取得し、`PostCard` を並べて表示。 | 50件固定でページネーションは未実装。読み込み中はスピナーを表示し、投稿ゼロ時はプレースホルダーを出す。 |
| フォロー操作 | `/profile/$userId`, `UserSearchResults` | `follow_user` / `unfollow_user` を呼び出し、成功時は React Query キャッシュで `['social','following']` と `['profile',npub,'followers']` を更新。`subscribe_to_user` を併用し購読を開始。 | 未ログイン時や自身への操作はブロック。処理中はボタンを無効化し、トーストで成功/失敗を通知。 |
| フォロワー/フォロー中リスト | `/profile/$userId` (`UserList`) | `get_followers` / `get_following` の結果をカード内で 2 カラム表示。 | 2025年11月07日: ソート（最新/古い/名前）とキーワード検索を実装。React Query の `totalCount` を利用し、表示件数と合計を同期。取得失敗時は `errorHandler` を通じてログとトーストを表示。 |
| メッセージ導線 | `/profile/$userId` (`ProfilePage`) | `MessageCircle` ボタンで `DirectMessageDialog` を開き、Kind4 IPC 経由のリアルタイム受信と未読バッジを連動 | `TauriApi.sendDirectMessage` / `.listDirectMessages` と `useDirectMessageStore` を接続済み。再送・未読リセット対応。既読の多端末同期は backlog。 |

## 2. 確認できた導線ギャップ
- `/trending`・`/following` ルートは 2025年11月07日時点で UI/API ともに稼働中。ただし集計ジョブ（`trending_metrics_job`）と Docker シナリオ（`trending-feed`）が未着手のため、データ鮮度と CI 自動検証が backlog（詳細は 5.7 節）。
- ユーザー検索は実ユーザーを返すが、ページネーション・検索エラーUI・入力バリデーションが未整備（改善計画は 5.8 節を参照）。
- `/profile/$userId` はフォロー導線と DM モーダル、フォロワー/フォロー中リストのソート・検索を備えたが、既読ステータスの多端末同期とページング拡張（2ページ目以降の自動補充/差分同期）が未実装。
- `TopicsPage` 以外にはトピック作成導線が存在せず、タイムラインから直接作成できない。
- 投稿削除は UI から利用可能になったが、React Query のキャッシュ無効化と `delete_post` コマンド統合テスト整備が未完了。
- 設定画面の「鍵管理」ボタンは依然として UI 表示のみで実装が無い。
- 設定画面の「プライバシー」トグル（プロフィール公開/オンライン表示）は 2025年11月02日時点で `usePrivacySettingsStore` によるローカル永続化まで対応済み。バックエンド連携と反映タイミングは未実装。

## 3. Tauri コマンド呼び出しマップ

### 3.1 利用中のコマンド
#### 認証・アカウント
| コマンド | ラッパー | 呼び出し元 | UI導線 |
| --- | --- | --- | --- |
| `generate_keypair` | `TauriApi.generateKeypair` | `authStore.generateNewKeypair` | Welcome「新規アカウント作成」 |
| `login` | `TauriApi.login` | `authStore.loginWithNsec` | Login 画面で nsec ログイン |
| `logout` | `TauriApi.logout` | `authStore.logout` | AccountSwitcher「ログアウト」 |
| `add_account` / `list_accounts` / `switch_account` / `remove_account` / `get_current_account` / `secure_login` | `SecureStorageApi.*` | `authStore`（ログイン/自動ログイン/アカウント切替/削除）、`AccountSwitcher` | Welcome/ Login / AccountSwitcher 導線、起動時の自動ログイン |

#### トピック・投稿
| コマンド | ラッパー | 呼び出し元 | UI導線 |
| --- | --- | --- | --- |
| `get_topics` / `get_topic_stats` | `TauriApi.getTopics`, `.getTopicStats` | `useTopicStore.fetchTopics`, `useTopics` | Topics 一覧、ナビゲーション |
| `create_topic` / `update_topic` / `delete_topic` | `TauriApi.*` | `TopicFormModal`, `TopicDeleteDialog` | Topics 一覧/詳細モーダル |
| `join_topic` / `leave_topic` | `TauriApi.*` | `topicStore.joinTopic`, `.leaveTopic`, `TopicMeshVisualization` | TopicCard「参加/参加中」、Topic Mesh「P2P参加/切断」 |
| `get_posts` | `TauriApi.getPosts` | `usePosts`, `postStore.fetchPosts` | タイムライン/トピック投稿一覧 |
| `create_post` | `TauriApi.createPost` | `PostComposer`, `ReplyForm`, `QuoteForm`, `syncEngine` | 投稿作成/返信/引用/オフライン同期 |
| `like_post` / `boost_post` | `TauriApi.*` | `PostCard` アクション, `syncEngine` | いいね/ブーストボタン |
| `bookmark_post` / `unbookmark_post` / `get_bookmarked_post_ids` | `TauriApi.*` | `bookmarkStore`, `PostCard` | ブックマーク操作と初期ロード |
| `delete_post` | `TauriApi.deletePost` | `postStore.deletePostRemote`, `PostCard` | 投稿メニュー（自分の投稿のみ）から削除。オフライン時は待機アクションとして保存 |

#### プロフィール・ユーザー
| コマンド | ラッパー | 呼び出し元 | UI導線 |
| --- | --- | --- | --- |
| `get_user` / `get_user_by_pubkey` | `TauriApi.getUserProfile`, `.getUserProfileByPubkey` | `/profile/$userId` ルート（`ProfilePage`） | ユーザー検索・直接アクセスからプロフィール表示 |
| `search_users` | `TauriApi.searchUsers` | `UserSearchResults` | `/search` (users) タブでプロフィール候補を取得 |
| `follow_user` / `unfollow_user` | `TauriApi.followUser`, `.unfollowUser` | `UserSearchResults`, `/profile/$userId` | 検索/プロフィール双方で同一ミューテーションを共有し、成功時に `subscribe_to_user` を呼び出す |
| `get_followers` / `get_following` | `TauriApi.getFollowers`, `.getFollowing` | `/profile/$userId` | フォロワー/フォロー中カードを React Query の無限スクロールで表示（ソート切替は未実装） |
| `upload_profile_avatar` / `fetch_profile_avatar` | `TauriApi.*` | `ProfileForm`（オンボーディング/設定モーダル）、`ProfileEditDialog`, `authStore.initialize` | プロフィール画像のアップロードと同期済みアバターの取得 |

#### ダイレクトメッセージ
| コマンド | ラッパー | 呼び出し元 | UI導線 |
| --- | --- | --- | --- |
| `send_direct_message` | `TauriApi.sendDirectMessage` | `DirectMessageDialog`, `useDirectMessageStore` | `/profile/$userId`「メッセージ」ボタン→モーダル。2025年11月04日: `DirectMessageService` / `NostrMessagingGateway` / SQLite リポジトリを実装し、kind4 を暗号化送信できるようになった。UI は Optimistic Update＋トースト通知で成功/失敗を反映し、`queued` フラグで未配信状態も扱う。 |
| `list_direct_messages` | `TauriApi.listDirectMessages` | `DirectMessageDialog`, `useDirectMessageStore` | `/profile/$userId` モーダルで履歴ロード・無限スクロールを実装（2025年11月05日）。`{created_at}:{event_id}` カーソルと `direction='backward'` を利用し、`dedupeMessages` でストアと統合。2025年11月06日: Kind4 IPC 経由でリアルタイム受信→未読バッジ更新→ヘッダー/サマリーパネルへの反映を実装し、失敗メッセージの再送 UI を追加。 |

#### Nostr 関連
| コマンド | ラッパー | 呼び出し元 | UI導線 |
| --- | --- | --- | --- |
| `initialize_nostr` / `disconnect_nostr` | `initializeNostr`, `disconnectNostr` | `authStore` ログイン/ログアウト処理 | Welcome/Login/AccountSwitcher |
| `update_nostr_metadata` | `updateNostrMetadata` | `ProfileSetup` | プロフィール保存 |
| `subscribe_to_topic` | `subscribeToTopic` | `topicStore.joinTopic`, `NostrTestPanel` | トピック参加、DEVテスト |
| `send_reaction` | `NostrAPI.sendReaction` | `ReactionPicker` | PostCard リアクション |
| `publish_text_note` / `publish_topic_post` | `nostrApi.*` | `NostrTestPanel` (DEV) | 設定>開発者ツール |
| `get_relay_status` | `getRelayStatus` | `authStore.updateRelayStatus`, `RelayStatus` | サイドバーのリレー表示 |
| `list_nostr_subscriptions` | `listNostrSubscriptions` | `useNostrSubscriptions` → `P2PDebugPanel` | DEV デバッグ画面 |
| `pubkey_to_npub` / `npub_to_pubkey` | `nostr.utils` | `postStore`, `useP2PEventListener` | 投稿・P2Pイベント正規化 |

#### オフライン同期
| コマンド | ラッパー | 呼び出し元 | UI導線 |
| --- | --- | --- | --- |
| `save_offline_action` / `get_offline_actions` / `sync_offline_actions` | `offlineApi.*` | `offlineStore.saveOfflineAction` / `.syncPendingActions` / `.loadPendingActions` | 投稿・トピック操作失敗時の再送（PostComposer、TopicFormModal など） |
| `cleanup_expired_cache` | `offlineApi.cleanupExpiredCache` | `offlineStore.cleanupExpiredCache`（1時間ごと） | バックグラウンドで古いオフラインアクションを整理 |
| `save_optimistic_update` / `confirm_optimistic_update` / `rollback_optimistic_update` | `offlineApi.*` | `offlineStore.applyOptimisticUpdate` / `.confirmUpdate` / `.rollbackUpdate` | PostCard のリアクション・ブックマークなど楽観的更新の確定 |

`syncEngine.getEntityLastModified` は `@tauri-apps/api/core` を動的 import し、`get_post_metadata` / `get_topic_metadata` / `get_user_metadata` / `get_reaction_metadata` を直接 `invoke` している（TypeScript ラッパー未整備）。

#### P2P 関連
| コマンド | ラッパー | 呼び出し元 | UI導線 |
| --- | --- | --- | --- |
| `initialize_p2p` / `get_node_address` / `get_p2p_status` | `p2pApi.*` | `p2pStore.initialize`, `useP2P` | アプリ起動時、サイドバー/ステータス表示 |
| `join_p2p_topic` / `leave_p2p_topic` | `p2pApi.joinTopic`, `.leaveTopic` | `useP2P`, `P2PDebugPanel`, `TopicMeshVisualization` | トピック参加操作、DEVデバッグ |
| `broadcast_to_topic` | `p2pApi.broadcast` | `P2PDebugPanel` | DEV デバッグ送信 |
| `get_p2p_metrics` | `p2pApi.getMetrics` | `P2PDebugPanel`, `TopicMeshVisualization`（統計表示） | DEV デバッグ/トピック詳細 |
| `connect_to_peer` | `p2pApi.connectToPeer` | `PeerConnectionPanel` | 設定>ピア接続 |
| `get_bootstrap_config` / `set_bootstrap_nodes` / `clear_bootstrap_nodes` | `p2pApi.*` | `BootstrapConfigPanel` | 設定>Bootstrap 設定 |

### 3.2 未使用・要確認コマンド（2025年11月07日更新）

#### 3.2.1 連携済み・監視対象
| コマンド | ラッパー | 状態 | 参照箇所 |
| --- | --- | --- | --- |
| `get_cache_status` | `offlineApi.getCacheStatus` | `useSyncManager` が `SyncStatusIndicator` / `OfflineIndicator` へキャッシュ統計を反映。UI からの手動リフレッシュボタンを提供済み。 | Inventory 5.11, Summary Quick View, `phase5_ci_path_audit.md`（SyncStatus テスト） |
| `add_to_sync_queue` | `offlineApi.addToSyncQueue` | `SyncStatusIndicator` の「再送をキューに追加」ボタンから呼び出し、未送信操作を再送キューへ登録。 | Inventory 5.11（UI フロー/テスト計画） |
| `update_cache_metadata` | `offlineApi.updateCacheMetadata` | `useOfflineStore.refreshCacheMetadata` が同期完了時に呼び出し、`get_cache_status` が参照する統計を蓄積。 | Inventory 5.5 / 5.11、`phase5_ci_path_audit.md` |
| `update_sync_status` | `offlineApi.updateSyncStatus` | `useSyncManager.persistSyncStatuses` で同期失敗・競合情報を Tauri 側へ記録し、次回ロード時に UI へ表示。 | Inventory 5.5 / 5.11、Summary グローバル要素 |

#### 3.2.2 未接続コマンド
| コマンド | ラッパー | 想定用途 | 備考 |
| --- | --- | --- | --- |
| `add_relay` | `nostrApi.addRelay` / `NostrAPI.addRelay` | リレー追加 | 現状テスト専用。UIからの追加導線なし。 |
| `get_nostr_pubkey` | `nostrApi.getNostrPubkey` / `NostrAPI.getNostrPubkey` | 現在の公開鍵取得 | 呼び出し箇所なし。 |
| `delete_events` | `nostrApi.deleteEvents` / `NostrAPI.deleteEvents` | Nostrイベント削除 | UI/ストア未接続。`delete_post` のバックエンド拡張フェーズで利用予定。 |
| `join_topic_by_name` | `p2pApi.joinTopicByName` | 名前ベース参加 | テストのみで、UI導線なし。Global Composer からのフォールバック用に待機。 |
| `clear_all_accounts_for_test` | `SecureStorageApi.clearAllAccountsForTest` | テスト用リセット | デバッグ UI 未接続。 |

### 3.3 未接続コマンドの対応優先度（2025年11月07日更新）

`follow_user` / `unfollow_user` 経由で `subscribe_to_user` を利用開始済み。SyncStatus 系の 4 コマンドは 2025年11月07日に UI 配線とテストを完了し、監視対象へ移行した。残コマンドの Phase 5 backlog 優先度は以下のとおり。

1. **`join_topic_by_name`** — Inventory 5.9 のトピック作成ショートカット／Global Composer 連携で ID 未確定の参加フローを扱うために最優先で整備。P2P 側の名前解決ロジックとセットで実装する。
2. **`delete_events`** — Inventory 5.10 の投稿削除キャッシュ整合性と連動し、Nostr イベントを確実に削除するためのバックエンド API。`delete_post` の統合テスト拡張と同じマイルストーンで着手する。
3. **`add_relay`** — 2025年09月15日の方針どおり外部リレー非接続だが、鍵管理モーダル（Phase 5 優先度 #9）と併せて再開可否を評価する。現状は開発者ツールの backlog。
4. **`get_nostr_pubkey`** — `authStore` で pubkey を保持しているため優先度は低い。プロフィール共有 UI の刷新時に再評価し、`SecureStorage` からの再取得や multi-identity 表示に備える。
5. **`clear_all_accounts_for_test`** — Debug パネルの「テスト用リセット」導線に組み込む計画。誤操作防止の確認ダイアログとログ記録を実装した後、開発者向け機能として公開する。

統合テストでは以下のコマンドを直接 `invoke` し、バックエンド API の状態確認やスモーク検証を実施している（UI 導線なし）。
- 認証 E2E: `import_key`, `get_public_key`
- リレー接続: `connect_relay`, `disconnect_relay`, `get_relay_status`
- 投稿/トピック状態検証: `create_post`, `create_topic`, `list_posts`, `list_topics`

- 2025年11月06日: `useOfflineStore.refreshCacheMetadata` と `useSyncManager` に `update_cache_metadata` / `update_sync_status` を組み込み、同期処理完了時に Tauri 側へ未同期件数・競合情報を反映するパイプラインを実装。`SyncStatusIndicator` の `lastSyncTime` はバックエンド更新に追従できるようになった。

## 4. 次のアクション候補
1. グローバルコンポーザーの初期トピック選択と投稿後のリフレッシュを最適化し、各画面からの動線を検証する。
2. 「トレンド」「フォロー中」カテゴリー用のルーティング／一覧画面を定義するか、未実装である旨を UI 上に表示する。
3. ユーザー検索のページネーション、検索エラーUI、入力バリデーションを整備し、`search_users` のレート制御を決定する。
4. `/profile/$userId` のメッセージ導線で既読同期の多端末反映と Docker/contract テストを整備し、フォロワー/フォロー中リストのソート／フィルタリング／ページングを含めてブラッシュアップする。
5. 投稿削除後の React Query キャッシュ無効化と `delete_post` コマンド統合テストを整備する。
6. 設定画面のプライバシートグルをバックエンドへ同期する API 設計・実装を行う。
7. 設定画面の「鍵管理」ボタンについて、バックアップ/インポート導線とコマンド連携を定義する。

## 5. 優先実装メモ（2025年11月04日更新）

### 5.1 設定画面のプライバシー設定・プロフィール編集導線
- **目的**: 設定画面から即時にユーザー情報と公開状態を更新できるようにし、オンボーディング後も同一フォームでプロフィールを保守できるようにする。
- **UI 実装案**
  - `settings.tsx` の「プロフィール編集」ボタン押下でモーダルを開き、`ProfileSetup` フォームを再利用する。入力部分を共通コンポーネント `ProfileForm` に切り出し、起動モードに応じて `navigate` の代わりにコールバックを受け取れるようにする。
  - プライバシーセクションは `Switch` から `usePrivacySettingsStore`（新規）を更新するようにし、状態を UI にバインドする。永続化には既存の `withPersist` + `createMapAwareStorage` を使用し、キー名は `privacyPreferences` を想定。
  - 保存ボタンをモーダルに追加し、`updateNostrMetadata` / `authStore.updateUser` を呼び出す。結果はトーストで通知し、`errorHandler` を利用する。
- 実装状況: 2025年11月02日に `ProfileForm` 抽出と設定画面モーダル導線のプロトタイプを実装済み（Stage1 完了、Stage2 はバックエンド連携待ち）。
- **バックエンド連携**
  - プライバシー設定（例: プロフィール公開/オンライン表示）は現状 API が無いため、Stage1 ではローカルストアの値をフロントで参照するのみとする。Stage2 で `nostr` / `p2p` へ伝播するコマンドを追加予定として `tauri_app_implementation_plan.md` にフォローアップタスクを記録する。
- **テスト計画**
  - `SettingsPage` の単体テストでモーダルの開閉とストア更新を検証。
  - `usePrivacySettingsStore` のストアテストで初期値・永続化を確認。
  - 既存 `ProfileSetup` のテストは共通化後も成功することを確認し、モーダルモード用のケースを追加。

### 5.2 サイドバー「新規投稿」ボタンと未導線機能
- **目的**: タイムライン以外の画面からも投稿を開始できるようにし、未結線の UI 要素（トレンド/フォロー中）を段階的に解消する。
- **UI 実装案**
  - `Home` ページのローカル状態 `showComposer` を `useComposerStore`（新規）へ移し、`Sidebar` のボタンから `openComposer({ topicId })` を呼び出す。モーダルは現在のページに関係なく描画できるよう、`MainLayout` に `PostComposerContainer`（ポータル）を追加する。
  - 未実装カテゴリー（トレンド/フォロー中）は一旦 `navigate` を無効化し、`tooltip` で「準備中」と表示するか、バックログで実装優先度を下げる旨を UI 上で明示する。
- 実装状況: 2025年11月02日に `useComposerStore` とグローバルコンポーザー・モーダルを実装し、Sidebar / Home / MainLayout からの導線をプロトタイプ化済み。
- **バックログ調整案**
  - フェーズ 5 の優先度を「投稿導線統一」「プロフィール編集再利用」「プライバシー設定反映」「トレンド/フォロー中の導線定義」の順に再編し、`tauri_app_implementation_plan.md` に反映する。
- **テスト計画**
  - `Sidebar` のテストにコンポーザートリガーのケースを追加。
  - `Home` の統合テストでストア経由の `openComposer` 呼び出しを検証。

### 5.3 プロフィール画像アップロード導線（リモート同期必須）
- **目的**: オンボーディングと設定モーダルの双方から同一フォームでプロフィール画像を差し替え、iroh-blobs 0.96.0 / iroh-docs 0.94.0 を用いたリモート同期を必須要件とする。
- **UI 実装案**
  - `ProfileForm` の「画像をアップロード」ボタン押下で `@tauri-apps/plugin-dialog.open` を呼び出し、`filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }]` を指定して単一選択に制限する。選択結果が無い場合は既存 URL 入力欄を維持。
  - 選択パスを `@tauri-apps/plugin-fs.readBinaryFile` で読み込んだ上で即時プレビューを `URL.createObjectURL` で差し替え、保存操作まではメモリ上に保持する（ローカルの恒久保存は禁止）。
  - 保存確定時は Tauri コマンド `upload_profile_avatar`（新設）を呼び出し、フロントからファイルバイトとメタデータ（拡張子/サイズ/MIME）を渡す。ローカルディスクへの直接書き込みはこのコマンド内部に限定する。
- **バックエンド連携（必須フロー）**
  1. `upload_profile_avatar` で一時ファイルへ保存後、`StreamEncryptor` で暗号化（セッションキー生成）し、暗号化済みバイト列と Capability（`access_level` / 復号キー）を準備する。
  2. 暗号化済みファイルを `iroh_blobs::client::quic::Client`（0.96.0）の `Client::blobs().add_path` に渡し、戻り値の `HashAndFormat` を取得。
  3. `client.share(hash)` で Capability 付き `BlobTicket` を生成し、`profile_avatars` Doc へ `Doc::set`（0.94.0）で `blob_hash` / `format` / `size_bytes` / `access_level` / `share_ticket` / `encrypted_key` を書き込む（`Doc::author().sign_change` を利用）。
  4. 他クライアントは `SyncSubscription` で Doc 更新を受信し、Capability 検証後に `Client::blobs().fetch(hash)` で暗号化 Blob を取得。復号キーは Capability から復号し、ストリーム復号して `appDataDir/profile_avatars/{hash}` へ保存する。
  5. Nostr プロフィール (`update_nostr_metadata`) には Blob ハッシュと Doc バージョンを含む URL 形式（例: `iroh+avatar://{doc_id}?hash={hash}`）を保存し、他ノードが解決可能にする。
- **バリデーション**
  - アップロード前にファイルサイズ上限（例: 2MB）と MIME 判定（`image/` プレフィックス + 拡張子一致）を実施し、Zstd 圧縮後も 2MB を超える場合は拒否。
  - Blob 登録時に `Client::blobs().stat(hash)` でサイズ確認を行い、Doc 更新には `size_bytes` と `content_sha256`（暗号化前に計算）を含めて改ざん検出を可能にする。
- **テスト計画**
  - `ProfileForm` のユニットテストでアップロード成功/キャンセル/サイズ超過/読み込み失敗をモックし、プレビュー更新と `upload_profile_avatar` 呼び出し条件を確認。
  - Tauri コマンドの結合テストで `upload_profile_avatar` → `iroh_blobs::client` 呼び出し → `iroh_docs::Doc` 更新までのハッピーパス／エラーパス（Blob 登録失敗・Doc 競合）を検証。
  - リモートノード同期テストとして `iroh_docs` の複数ノードシナリオを Docker で再現し、Doc 更新から Blob ダウンロードまでを `phase5_ci_path_audit.md` に記録する。

### 5.4 鍵管理ボタンの実装方針（検討中）
- **目的**: 秘密鍵のバックアップ・復旧をアプリ内で完結させ、複数デバイス運用時の手順とリスクをユーザーに提示する。
- **UI 実装案**
  - 設定 > アカウントの「鍵管理」ボタンから `KeyManagementDialog`（新規）を開き、「エクスポート」「インポート」のタブを提供する。
  - エクスポートタブ: `export_private_key` で取得した nsec を `dialog.save` + `fs.writeTextFile` により `.nsec` ファイルとして保存し、必要に応じてマスク表示→コピー (`navigator.clipboard.writeText`) も提供する。操作前に注意文と確認ダイアログを表示。
  - インポートタブ: `dialog.open` で `.nsec` ファイルを読み込み、`SecureStorageApi.addAccount` / `authStore.loginWithNsec` を再利用。既存アカウントと重複する場合は確認ダイアログを挟み、キャンセル時は状態を変更しない。
- **バックエンド連携**
  - エクスポート: 既存の Tauri コマンド `export_private_key` を TypeScript ラッパー（例: `TauriApi.exportPrivateKey`）として公開し、取得した秘密鍵はフロント側でのみ保持する。ファイル保存前に `withPersist` へログを追加して操作痕跡を残す。
  - インポート: 追加の Tauri コマンドが不要な場合は `login_with_nsec` / `SecureStorageApi.addAccount` で完結。今後エラーバリデーションを強化するために Rust 側へ `validate_nsec` コマンドを追加する案を backlog に記録する。
- **セキュリティ**
  - エクスポート結果をクリップボードへコピーした場合は 30 秒後に空文字列を書き込むオプションを設定。ログには秘密鍵を含めず、`errorHandler.info` で操作種別のみ記録。
  - エクスポート/インポートどちらも実行後に `toast` でフィードバックを表示し、エラー時は `errorHandler.log('KeyManagementDialog.export', error)` などコンテキスト付きで出力する。
- **テスト計画**
  - `KeyManagementDialog` のユニットテストでエクスポート成功/失敗・保存キャンセル・クリップボードコピーのパスを検証。`export_private_key` が 1 回のみ呼ばれることとローディング表示を確認。
  - `authStore` 統合テストに `.addAccount` を通じたインポートケースを追加し、重複アカウント時にエラーが表示されることを確認。

- **構成更新メモ**: 2025年11月03日、下記の通り実装とテストを完了。

### 5.5 Relay/P2P ステータスカードと監視タスク（2025年11月03日更新）
- **目的**: サイドバー下部の `RelayStatus` / `P2PStatus` カードでネットワーク状態とメトリクスを可視化し、Phase 5 の接続系リグレッション検出を支援する。
- **実装状況**
  - `RelayStatus` は `useAuthStore` に追加した `relayStatusBackoffMs` / `lastRelayStatusFetchedAt` / `relayStatusError` を参照し、初回取得後は指数バックオフ制御付き `setTimeout` で `get_relay_status` を再実行する。エラー発生時もカードを維持し、ヘッダーに「最終更新」「次回再取得」の表示と `再試行` ボタンを提供。
  - `P2PStatus` は `get_p2p_status` が返却する `connection_status` / `peers` を反映し、ヘッダーの `再取得` ボタンで手動更新・バックオフ情報を表示。`statusError` が存在する場合はエラーバナーと `再取得` ボタンを提示し、ネットワーク未接続時でもバックオフ制御で自動再取得を継続する。
  - Rust 側 `P2PStatus` 構造体に `connection_status`（`connected`/`connecting`/`disconnected`/`error`）と `peers`（`node_id`/`address`/`connected_at`/`last_seen`）を追加し、TypeScript の `p2pApi` / `p2pStore` が新フィールドを取り込むよう更新。`useP2P` は `setTimeout` ベースのポーリングと `isRefreshingStatus` を用いた重複リクエスト防止を実装した。
- **現時点のギャップ**
  - `SyncStatusIndicator` からリレー再取得を呼び出す導線は未接続で、Phase 5 backlog にフォローアップ済み。
  - `RelayStatus` の失敗回数を URL 単位で表示する UI は未実装。今後 `relayStatusError` の履歴と組み合わせて可視化する。
  - `PeerStatus` にはトピック参加情報が含まれていないため、将来的に backend 側で topics を付与し、UI にツールチップ表示する余地がある。
- **テスト / フォローアップ**
  - 2025年11月03日: `src/tests/unit/components/RelayStatus.test.tsx` / `src/tests/unit/components/P2PStatus.test.tsx` を更新し、バックオフ・手動リトライ・エラー表示をフェイクタイマーで検証。`npx vitest run src/tests/unit/components/RelayStatus.test.tsx src/tests/unit/components/P2PStatus.test.tsx` を実行し成功。
  - 同日、`src/tests/unit/stores/authStore.test.ts` / `src/tests/unit/stores/p2pStore.test.ts` / `src/tests/unit/hooks/useP2P.test.tsx` を拡張し、バックオフ遷移・エラー保持・`isRefreshingStatus` 排他制御を検証。
  - Rust 側では `cargo test`（`kukuri-tauri/src-tauri` / `kukuri-cli`）を実行し、`application::services::p2p_service::tests` における `connection_status` / `peers` の復帰とフォールバック動作を確認。Runbook 9章に新フィールドと検証手順を追記済み。

### 5.6 プロフィール詳細導線とフォロー体験（2025年11月05日更新）
- **目的**: `/profile/$userId` を起点にプロフィール閲覧・フォロー操作・投稿参照を一貫した導線として提供し、検索結果や他画面からの遷移後も同等の体験を維持する。
- **実装状況**
  - 2025年11月03日: プレースホルダールートを差し替え、`getUserProfile` / `getUserProfileByPubkey` / `getPosts({ author_pubkey })` を用いた実データ取得と、フォロー/フォロー解除ボタンを実装。
  - `follow_user` / `unfollow_user` 成功時に `React Query` の `['social','following']` / `['profile',npub,'followers']` キャッシュを即時更新し、`subscribe_to_user` でイベント購読を開始する。
  - `UserSearchResults` からのフォロー操作も同一ミューテーションを共有し、検索結果→プロフィール詳細間の導線差異を解消。
  - 2025年11月04日: `DirectMessageDialog` と `useDirectMessageStore` を追加し、プロフィール画面の「メッセージ」ボタンからモーダルを開閉できるよう接続。`DirectMessageDialog` 単体テストで楽観的更新・失敗時の `toast` 表示を検証。
  - 同日: Rust 側で `direct_message_service` / `messaging_gateway` / SQLite リポジトリを実装し、`TauriApi.sendDirectMessage` から暗号化送信→永続化まで通るよう更新。
  - 2025年11月05日: `DirectMessageDialog` を `useInfiniteQuery(['direct-messages', npub])` と `TauriApi.listDirectMessages` で接続し、初期履歴ロード・IntersectionObserver ベースの無限スクロール・`markConversationAsRead` による未読リセットを実装。`Load more` ボタンとローディング/エラー UI を追加し、ストアの既存会話と React Query の結果を `dedupeMessages` で統合。
- **残課題**
  - Kind4 既読状態を他端末と同期する仕組み（delivered/ack 更新・contract テスト）と Docker シナリオを整備する。
  - プロフィール投稿一覧は 50 件固定で pagination 未対応。スクロールロードや日付ソートなどの UX 改善が必要。
  - フォロワー/フォロー中リストに検索・ソートが無く、件数が多い場合の利用性が下がる。
  - 送信失敗後の自動バックオフやレート制御は未整備。現状は手動の「再送」ボタンのみのため、再送間隔と失敗履歴のコントロールを追加する。
  - Tauri 経由のエラーハンドリングはトースト表示に偏っているため、`errorHandler` のメタデータ拡充とリトライ導線を検討。
- **対応計画（2025年11月06日更新）**
  - Direct Message は 5.6.1 の実装状況を参照。Kind4 IPC 実装済みのため、既読同期・Docker シナリオ・contract テストを追加しつつレート制御/バックオフの設計を進める。
  - フォロワー一覧のソート/ページネーションは 5.6.2 に実装計画を記載。API 拡張・フロント実装・テストカバレッジを網羅。

#### 5.6.1 DirectMessage Tauri 実装状況（2025年11月05日更新）
- **実装済みコンポーネント**
  - `application/services/direct_message_service.rs` が `send_direct_message` / `list_direct_messages` を提供。空メッセージは `ValidationFailureKind::Generic` で検証し、暗号化と配送は `MessagingGateway` に委譲。
  - `infrastructure/messaging/nostr_gateway.rs` が kind 4 の生成と配信を担当し、`KeyManager.export_private_key` から秘密鍵を取得して `nip04` で暗号化・復号。
  - `infrastructure/database/sqlite_repository/direct_messages.rs` が SQLite 永続化とカーソルページング（"{created_at}:{event_id}"）・方向指定（Backward/Forward）を実装。
  - `presentation/commands/direct_message_commands.rs` が Tauri コマンド `send_direct_message` / `list_direct_messages` を公開し、`ensure_authenticated` で owner npub を決定した上で `ApiResponse` を返却。
- **UI 連携**
  - `DirectMessageDialog` は `useInfiniteQuery(['direct-messages', npub])` で `list_direct_messages` を呼び出し、IntersectionObserver と `Load more` ボタンで無限スクロール・再取得を制御。取得したページは `dedupeMessages` でストアの会話履歴に統合し、読み込み成功時に `markConversationAsRead` で未読カウントをリセットする。
  - `DirectMessageDialog` からの送信は従来どおり楽観更新を行い、`resolveOptimisticMessage` / `failOptimisticMessage` で状態同期。sonner toast で成功/失敗を通知し、`queued` フラグは `status: 'pending'` 表示に対応。
  - `useDirectMessageStore` が既読カウントと会話ログを保持し、`dedupeMessages` で `eventId` / `clientMessageId` をキーに重複排除。
- **テスト / 検証**
  - Rust: `cargo sqlx prepare` → `cargo test`（`kukuri-tauri/src-tauri` と `kukuri-cli`）で Direct Message サービスとリポジトリのユニットテストを実行済み。
  - 2025年11月05日: `pnpm vitest run src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx` を実行し、履歴ロード・送信フローが回帰しないことを確認。
  - TypeScript: `DirectMessageDialog.test.tsx` で Optimistic Update・エラーハンドリング・トースト表示・初期履歴の描画を検証し、Vitest 結果を記録。
- **残課題**
  - 既読ステータスの多端末同期（`markConversationAsRead` → IPC → SQLite 反映）と Docker / contract テストの追加が必要。
  - 会話リスト（サイドバー想定）に履歴の最新メッセージを反映する仕組みは未整備。React Query のキャッシュ共有も含めた一覧更新方式を検討する。
  - 送信レート制御・暗号化鍵キャッシュ・失敗時のバックオフは運用シナリオでの検証が必要。
#### 5.6.2 フォロワー一覧ソート/検索実装状況（2025年11月07日更新）
  - **実装内容**
    - `get_followers` / `get_following` リクエストに `sort`（`recent` / `oldest` / `name_asc` / `name_desc`）と `search` を追加し、レスポンスへ `total_count` を含めるよう更新。既存呼び出しとの後方互換は維持。
    - SQLite リポジトリでソート種別ごとのカーソル式（`{sort}|{base64(primary)}|{pubkey}`）を導入し、`LIKE` フィルターと件数取得を同条件で構築。`QueryBuilder` でバインド順を統一。
    - `ProfilePage` の `UserList` に `Select`（ソート）と `Input`（検索）を追加。`useInfiniteQuery` のキーへソート/検索を含め、ヘッダーに「表示中 X / totalCount 件」を表示。
    - フォロー/フォロー解除時に現在のソート・検索条件へ一致するデータを楽観更新し、それ以外の条件は `invalidateQueries(['profile', npub, 'followers'])` で再取得させる。
  - **テスト / 検証**
    - `pnpm vitest run src/tests/unit/routes/profile.$userId.test.tsx`
    - `cargo fmt`
    - `cargo test`（`kukuri-tauri/src-tauri` は Windows 環境で `STATUS_ENTRYPOINT_NOT_FOUND` により実行時エラー、`kukuri-cli` は成功）
  - **残課題**
    - Windows 環境での `cargo test` 実行時エラー（`STATUS_ENTRYPOINT_NOT_FOUND`）の原因調査と解消。
    - 2 ページ目以降を自動補充する際のキャッシュ整合性（`FOLLOW_PAGE_SIZE` 超過時の繰り上げ）と E2E カバレッジの整備。
    - フォロワー非公開（403）ケースや多端末既読同期など、残タスクのシナリオテストを Rust / Vitest 側に追加。

### 5.7 トレンド/フォロー中導線実装計画（2025年11月04日追加）
- **目的**: サイドバーカテゴリー「トレンド」「フォロー中」からアクセスできる発見導線とマイフィード導線を整備し、Home タイムラインとの差別化と優先度の可視化を実現する。
- **進捗（2025年11月07日更新）**
  - `Sidebar` のカテゴリーは `useUIStore.activeSidebarCategory` でハイライトを同期し、`prefetchTrendingCategory` / `prefetchFollowingCategory` によりクリック時に関連クエリを事前取得できるようにした。
  - `useTrendingFeeds.ts` をリファクタリングし、`trendingTopicsQueryKey` などの共有ロジックとプリフェッチ API を整備。`routes/trending.tsx` / `routes/following.tsx` は新ヘルパーを利用してロード/エラー/空状態をハンドリング済み。
  - テスト実行: `npx vitest run src/tests/unit/components/layout/Sidebar.test.tsx src/tests/unit/stores/uiStore.test.ts src/tests/unit/hooks/useTrendingFeeds.test.tsx`（2025年11月05日）。カテゴリ状態の同期・プリフェッチ分岐・クエリマッピングをユニットテストで検証。
  - 2025年11月06日: `list_trending_topics` / `list_trending_posts` / `list_following_feed` のデータ仕様と UI/ST テスト要件を整理し、本節ならびに Summary・実装計画へ反映。`topic_handler.rs` / `post_handler.rs` で `Utc::now().timestamp_millis()` を採用していることを確認し、Query キャッシュ境界条件も記録。
  - 2025年11月06日: `TrendingSummaryPanel` / `FollowingSummaryPanel` を追加し、派生メトリクス（トピック数・プレビュー件数・平均スコア・最終更新・ユニーク投稿者・残ページ）を表示。`pnpm vitest run src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx` で新UIと集計値のテストを実施。
  - 2025年11月07日: `/trending` `/following` の手動 QA を実施し、`formatDistanceToNow` へのミリ秒入力・無限スクロール境界（空ページ/`hasNextPage=false`）・DM 未読バッジ連携を確認。`phase5_user_flow_summary.md` と `phase5_ci_path_audit.md` の参照リンクを更新し、Summary Panel の派生メトリクスが最新データと一致することを検証。
- **未実装（2025年11月07日 要件定義完了）**
  1. Docker シナリオ `trending-feed`: `scripts/test-docker.{sh,ps1}` に `--scenario/-Scenario` オプションを追加し、`ts` コマンド経由で `pnpm vitest run src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx src/tests/unit/hooks/useTrendingFeeds.test.tsx` を実行。フィクスチャは `kukuri-tauri/tests/fixtures/trending/default.json` を既定とし、`VITE_TRENDING_FIXTURE_PATH` で差し替え。Nightly ワークフローに「Trending Feed (Docker)」ジョブを追加し、成果物 `test-results/trending-feed/latest.log` をアップロードする。
  2. 集計ジョブ `trending_metrics_job`: `docs/03_implementation/trending_metrics_job.md` のドラフトに沿って、24h/6h ウィンドウ集計・バックフィル・失敗時リトライ・Prometheus エクスポート・Runbook 更新を実装。完了後は Summary / CI パス監査の backlog から除外する。
- **データ要件（2025年11月06日更新）**
  - `list_trending_topics` は `TopicService::list_trending_topics`（`topic_service.rs`）が `topics` テーブルの `member_count` と `post_count` を基に `trending_score = post_count * 0.6 + member_count * 0.4` を計算し、`TrendingTopicDto { topic_id, name, description, member_count, post_count, trending_score, rank, score_change }` を `limit` 件返却する。UI 側は `limit=10` をデフォルトとし、`staleTime=60秒` / `refetchInterval=120秒` でキャッシュするため、レスポンスの `generated_at` は **ミリ秒エポック**（`topic_handler.rs` で `Utc::now().timestamp_millis()` を返却済み）となる。フォローアップでは集計ジョブ導入後の値の安定性を監視する。
  - `list_trending_posts` は `ListTrendingPostsRequest { topic_ids, per_topic }` を受け取り、`per_topic` を `1..=20` にクランプ（デフォルト 3）。`TrendingTopicPostsResponse` には `topic_id`・`topic_name`・`relative_rank` と `PostResponse` 配列（`id`/`content`/`author_pubkey`/`author_npub`/`topic_id`/`created_at`(秒)/`likes`/`boosts`/`replies`/`is_synced`）が含まれる。フロントは `mapPostResponseToDomain` で `created_at` を秒→`Date` に変換しつつ Markdown を表示する。
  - `list_following_feed` は認証必須。`ListFollowingFeedRequest` の `limit` は `1..=100`、デフォルト 20。`cursor` には `"{created_at}:{event_id}"` 形式、`include_reactions` は現状プレースホルダだが true 時にリアクション数を同梱する設計を維持。レスポンスは `FollowingFeedPageResponse { items, next_cursor, has_more, server_time }` で `server_time` はミリ秒。UI は `useInfiniteQuery` で `cursor` を繋ぎ、フォールバックボタンを併用する。
  - 例外時は各 DTO の `Validate` 実装により `AppError::InvalidInput`（HTTP 400）が返る。UI 側では `errorHandler.log('TrendingTopics.fetchFailed'|...)` / `errorHandler.log('Sidebar.prefetchFailed', …)` を使用し、ログキー単位で通知文面を切り替える。
  - Prefetch ロジックは `prefetchTrendingCategory` が `trendingTopicsQueryKey(limit)` → `trendingPostsQueryKey(topicIds, perTopic)` を順に取得、`prefetchFollowingCategory` は `prefetchInfiniteQuery` で初回ページをキャッシュする。`QueryClient` のキー、`staleTime`、`enabled` 条件をドキュメント化し、キャッシュミス時の遅延を許容する。
  - Docker シナリオでは `VITE_TRENDING_FIXTURE_PATH`（既定: `tests/fixtures/trending/default.json`）を inject して Vitest 実行中のフェイク API 応答を固定する。Nightly で差し替える場合は `tests/fixtures/trending/<scenario>.json` を追加し、`scripts/test-docker.{sh,ps1}` から `--fixture` オプションとして受け渡す。
- **UI 実装案**
  - ✅ `routes/trending.tsx` でランキングカードと投稿プレビューを実装済み。更新タイムスタンプとスコア差分、再試行導線を画面ヘッダーに配置。
  - ✅ `routes/following.tsx` で無限スクロール版タイムラインを実装。フォロー解除やプロフィール遷移の導線は引き続き拡張予定。
  - ✅ `TrendingSummaryPanel` / `FollowingSummaryPanel` を追加し、派生メトリクスをまとめて表示。
  - ✅ サイドバーでカテゴリーごとにボタン強調を行い、別画面遷移後に `activeSidebarCategory` をリセット。
  - Skeleton / `ErrorStateCard` / `EmptyStateCard` は両ルートで共通利用。文言・サポートリンクは `errorHandler` のキーに合わせて整理済み。
- **バックエンド/コマンド設計**
  - `list_trending_topics`: 2025年11月05日時点では `TopicRepository.get_all_topics` → `TopicService::list_trending_topics` のシンプル実装で稼働。今後 `topic_metrics` テーブルと `trending_metrics_job` を導入して 24h ウィンドウ集計へ移行する（本節のデータ要件に沿って仕様を明記）。移行後は DTO の互換性を保ったまま `trend_score` の内訳を取得できるようにする。
  - `list_trending_posts`: `PostService::get_posts_by_topic` を並行実行し、取得できなかったトピックはスキップ。`per_topic` 超過時のエラーハンドリングは DTO 側で吸収。将来的に `topic_metrics` の `posts_24h` を用いてプレフィルタリングする案を検討する。
  - `list_following_feed`: `PostRepository::list_following_feed` が `PostFeedCursor` を解釈してページング。空配列時は `has_more=false` / `next_cursor=null` を返す。`include_reactions` は `post_service.list_following_feed` 内で確保されているが、現状は拡張フラグとして保持していることをドキュメント化。
  - メトリクス集計ワーカー `trending_metrics_job` は backlog。導入時は `topic_metrics(window_start)` の TTL 設計と、`docs/03_implementation/p2p_mainline_runbook.md` への監視手順追記が必要。
- **状態管理・ストア**
  - ✅ `useTrendingTopicsQuery` / `useTrendingPostsQuery` をヘルパー化し、`fetchTrendingTopics` などの共通ロジックを導入。`QueryClient.prefetchQuery` からも再利用可能にした。
  - ✅ `useFollowingFeedQuery` は `prefetchFollowingCategory` からも呼び出せるよう拡張。`keepPreviousData` と `includeReactions` オプションを統一。
  - ✅ `useUIStore` に `activeSidebarCategory` とリセット関数を追加。`Sidebar` ではセレクタで購読し、余計なレンダーを避けつつ状態を同期。
- **テスト計画**
  - TypeScript（既存）: `Sidebar.test.tsx`（カテゴリー遷移/プリフェッチ）、`useTrendingFeeds.test.tsx`（引数検証・prefetch・cursor）、`uiStore.test.ts`（状態遷移）を維持。
  - TypeScript（追加）: `routes/trending.test.tsx` / `routes/following.test.tsx` で Loading/Error/Empty/Success・`fetchNextPage` をカバー済み。今後は `prefetchTrendingCategory` のクエリキャッシュ検証と `formatDistanceToNow` の時刻表示（generated_at ミリ秒値）をスナップショット化する。
  - Rust: `topic_handler::list_trending_topics` / `post_handler::list_trending_posts` / `post_handler::list_following_feed` の単体テストを追加し、(1) limit / per_topic / cursor の境界値、(2) `AppError::InvalidInput` の伝播、(3) `server_time` がミリ秒で返ること、(4) Topic 未検出時にスキップされる挙動を確認する。`PostFeedCursor` の parse/recompose テストも追加する。
  - Docker / Nightly: `docker-compose.test.yml` に `trending-feed` シナリオを追加し、Windows 向け `./scripts/test-docker.ps1 ts -Scenario trending-feed` を案内。Nightly では Trending/Follower ルートの Vitest をジョブに追加し、`phase5_ci_path_audit.md` にテスト ID を記録する。
- **次の着手順序（2025年11月06日更新）**
  1. ✅ Summary Panel 実装（2025年11月06日完了）  
     - `TrendingSummaryPanel` / `FollowingSummaryPanel` で派生メトリクスを表示し、Vitest で検証済み。  
  2. ✅ DM 未読ハイライト & Kind4 IPC 対応（2025年11月06日完了）  
     - `direct_message_service` が Kind4 受信時に `direct-message:received` を emit し、`DirectMessageService::ingest_incoming_message` で暗号化ペイロードを復号→永続化→通知まで一貫処理。  
     - `DirectMessageDialog` に未読管理・失敗メッセージの再送 UI を追加し、`useDirectMessageEvents` / `useDirectMessageBadge` フックでヘッダーと Trending/Following Summary Panel のバッジ表示を同期。  
     - Vitest（Dialog/Trending/Following/Header）と `cargo test` で動作を検証。  
  3. **Docker シナリオ `trending-feed` 整備**  
     - 目的: CI / ローカル検証でトレンド・フォロー導線の UI テストを Docker 内で再現し、バックエンド API 仕様変更時のリグレッションを早期検知する。  
     - 具体: `docker-compose.test.yml` の `test-runner` に `pnpm vitest run src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx src/tests/unit/hooks/useTrendingFeeds.test.tsx` を呼ぶシナリオを追加。フィクスチャは `tests/fixtures/trending/default.json`（`VITE_TRENDING_FIXTURE_PATH`）で差替可能とし、結果ログを `test-results/trending-feed/latest.log` として保存。  
     - 付随: Windows 向け `./scripts/test-docker.ps1 ts -Scenario trending-feed` / Linux 向け `./scripts/test-docker.sh ts --scenario trending-feed` を追加し、`docs/03_implementation/docker_test_environment.md` と README のテスト手順に追記。Nightly ワークフローへ「Trending Feed (Docker)」ジョブを追加し、失敗時はアーティファクトと `phase5_ci_path_audit.md` を参照する運用とする。  
  4. **`trending_metrics_job` 導入**  
     - 目的: Summary Panel とトレンド表示の精度を高めるため、24h 集計ジョブで `topic_metrics` を更新し、トレンドスコアや参加者数の時間ベース推移を取得可能にする。  
     - バックエンド: 新規ジョブ `trending_metrics_job` を `tokio::task` で起動し、`topics` / `posts` テーブルから統計値を集計して `topic_metrics` テーブルへ反映。`TopicService::list_trending_topics` をメトリクスを活用する実装にリファクタ。  
     - テスト: Rust で集計ジョブの単体テスト + 統合テストを追加。Docker シナリオ内でジョブ実行を確認し、Summary Panel の表示値が集計結果と一致するかを検証。  
     - ドキュメント: `docs/03_implementation/p2p_mainline_runbook.md` に監視・障害対応手順を追記し、`phase5_ci_path_audit.md` にジョブ用テストケース ID を登録。
- **フォローアップ**
  - `phase5_user_flow_summary.md`（1.2節 / 3節 / 6節）と `tauri_app_implementation_plan.md` Phase 5 優先度に本計画をリンク済み。
  - `docs/03_implementation/p2p_mainline_runbook.md` にトレンドメトリクス監視手順としきい値、アラート対応を追記予定。
  - CI: `phase5_ci_path_audit.md` に `TrendingRoute`/`FollowingRoute` のユニット・統合テスト ID を追加し、Nightly テストでの実行対象に含める。

### 5.8 ユーザー検索導線改善計画（2025年11月04日追加）
- **目的**: `/search` (users) タブで安定した検索体験（ページネーション・エラー復旧・レート制御）を提供し、フォロー導線とプロフィール遷移を促進する。
- **UI 実装案**
  - 検索入力は `query.trim().length >= 2` を必須条件とし、それ未満の場合はリクエストを発行せず空状態カードを表示。「2文字以上入力してください」とガイダンスを提示。
  - `UserSearchResults` を `useInfiniteQuery` に切り替え、カーソルによる追加取得・`Load more` ボタン・`IntersectionObserver` を併用。`keepPreviousData` を有効化し、再検索時にフラッシュを抑制。
  - エラー表示は `SearchErrorState`（新規）で `errorHandler` のキーを解釈し、`再試行` ボタン・サポートリンク・レートリミット残り時間表示を提供。無結果時は `EmptyStateCard` を表示。
  - `UserSearchResults` の状態遷移は `idle`（入力なし）→`typing`（2文字未満）→`ready`（バリデーション通過）→`loading`（リクエスト中）→`success`/`empty`/`rateLimited`/`error` を明示し、`rateLimited` 到達時は `retryAfter` カウントダウン完了後に自動で `ready` に戻す。React Query の `status` とローカルステートを組み合わせ、UI レベルで分岐を管理する。
  - 入力欄下部に検索時間・ヒット件数を表示し、結果差分が発生した場合は `diff` ハイライト（CSS アニメーション）で通知。フォロー操作成功時は該当行で楽観的更新し、エラー時は `errorHandler` でロールバック。
- **入力バリデーション方針**
  - 入力欄では `query` を `trim` し、全角半角スペース・改行・タブを除去。長さは 2〜64 文字に制限し、上限超過時は自動でスライス（UI は「64文字まで」のヒントを表示）。
  - 制御文字と `[\u0000-\u001F\u007F]` を除外し、違反した場合は `invalid_query` を発火させてフィールド下にバリデーションエラーを表示。Nostr キー（npub/hex）・表示名・Bio 断片を入力できるよう、英数/記号/日本語を許可する。
  - 連続スペースを 1 つに正規化し、`query` の前後に `#` や `@` がある場合は補助検索（タグ/npub）と認識。UI では補助検索ラベルを表示し、結果が空でも「部分一致で検索中」のトーストを表示する。
  - リクエスト発行は 300ms デバウンス＋最新の `AbortController` を用いてキャンセル。`allow_incomplete=true` の場合のみ、直前のレスポンスを保持したままスピナーを表示する。
- **バックエンド/コマンド**
  - `search_users` コマンドを `SearchUsersRequest { query: String, cursor: Option<String>, limit: u16, sort: Option<SearchSort>, allow_incomplete: bool }` へ拡張。
    - `cursor` は `"{last_seen_at}:{pubkey}"` 形式。`sort` は `relevance`（デフォルト）/`recency`。`allow_incomplete` はフォールバック（キャッシュ結果のみ返す）を許可するフラグ。
    - クエリ長が 2 未満の場合は `AppError::InvalidInput`（コード: `USER_SEARCH_QUERY_TOO_SHORT`）を返却。
    - `limit` はデフォルト 20、最大 50。上限を超えるリクエストは 50 にクランプし、`AppError::InvalidInput` の `details` に `requested_limit` を格納する。
  - `UserSearchService`（新規）を追加し、Nostr インデックスから取得したプロフィールとローカルキャッシュを統合。`rank = text_score * 0.7 + mutual_follow * 0.2 + recent_activity * 0.1` を計算し、`relevance` ソートに利用。
    - `allow_incomplete=true` の場合はキャッシュヒットのみを返却しつつ `has_more=false` を設定。Nostr リレーへ接続不可でも UX を保つ。
  - レートリミットはユーザー単位で 10 秒間に 30 リクエストまで。超過時は `AppError::RateLimited { retry_after_seconds }` を返し、UI がカウントダウンを表示できるようにする。
- **エラーハンドリング**
  - `errorHandler` に `UserSearch.fetch_failed`, `UserSearch.invalid_query`, `UserSearch.rate_limited` を追加（詳細は `docs/03_implementation/error_handling_guidelines.md`）。
  - `SearchErrorState` は `invalid_query` の場合に入力欄へ警告スタイルを適用し、レートリミットの場合は再試行ボタンを無効化してクールダウンタイマーを表示。
  - バックエンドは `AppError::RateLimited` を 429 としてラップし、`retry_after_seconds` の値をレスポンス JSON に含める。
- **テスト計画**
  - TypeScript: `UserSearchResults.test.tsx` の拡張で (1) クエリ長 < 2 ではリクエストが送信されない、(2) 正常系で `fetchNextPage` が cursor を渡す、(3) レートリミット→カウントダウン→自動再取得、(4) エラー再試行時に既存データを保持する、の各ケースを検証。
  - TypeScript: `useUserSearchQuery.test.ts`（新規）でデバウンス・キャンセル・クリーンアップをテスト（`vi.useFakeTimers()` 使用）。
  - Rust: `user_search_service` ユニットテストで短いクエリ・レートリミット・ソート順・カーソル境界を網羅。`AppError` 変換のテストを追加。
  - Docker: `docker-compose.test.yml` に `user-search-pagination` シナリオを追加し、Nostr リレー未接続時でもキャッシュのみで検索可能か検証。Windows 用には `./scripts/test-docker.ps1 ts -Scenario user-search-pagination` を案内。
- **フォローアップ**
  - `phase5_user_flow_summary.md` と `tauri_app_implementation_plan.md` Phase 5 優先度表へ本節をリンク。
  - `docs/03_implementation/error_handling_guidelines.md` に新しいキーとユーザー向けトースト文言を追記。
- CI では Nightly Frontend Unit Tests に `UserSearchResults` / `useUserSearchQuery` テストの実行ログを追加し、`phase5_ci_path_audit.md` にテスト ID を記録。

### 5.9 ホーム/サイドバーからのトピック作成導線（2025年11月06日追加）
- **目的**: タイムラインやサイドバーから離脱せずに新しいトピックを作成し、そのまま投稿作成へ移行できる導線を提供する。
- **現状**: トピック作成は `/topics` ルートの `TopicFormModal` に限定され、`GlobalComposer` やサイドバーからはアクセスできない。`TopicSelector` も参加済みトピックのみ表示するため、新規ユーザーは投稿開始前に必ず一覧ページへ遷移する必要がある。
- **UI 実装案**
  - `GlobalComposer` 内のトピック行に「新しいトピックを作成」アクションを追加し、押下時に `TopicFormModal` を再利用した `TopicCreationDialog`（mode=`create-from-composer`）を表示する。作成完了後は `useComposerStore` に新しい `applyTopicAndResume(topicId)` を実装して投稿モードへ復帰させる。
  - `TopicSelector` にショートカット項目（`CommandItem` + `data-testid="create-topic-shortcut"`）を追加し、検索結果が 0 件の場合も同アクションを提示する。キーボード操作（`Ctrl+Enter` / `⌘+Enter`）で作成モーダルを起動できるようアクセラレーターを設定する。
  - サイドバーの「新規投稿」ボタンは参加トピックが 0 件の場合に作成モーダルを優先表示し、完了後 `openComposer({ topicId: createdTopic.id })` を呼び出す。参加済みの場合は従来どおり投稿モーダルを開く。
  - トピック作成モーダルに公開設定トグル（公開/非公開）とカテゴリタグ入力を追加し、将来的なフィルタリング要件を見越したフォーム構造へ拡張する。
- **バックエンド / コマンド**
  - `TauriApi.createTopic` の成功時に `join_topic` を連続実行する `createAndJoinTopic` ヘルパーを TypeScript 側へ追加し、UI からの二重呼び出しを防ぐ。Rust 側でも `TopicService::create_topic` 内で作成者の自動参加を保証する。
  - オフライン時に備えて `OfflineActionType::CREATE_TOPIC` を新設し、`TopicFormModal` で楽観的にトピックをストアへ追加→`syncEngine` がオンライン復帰後に `create_topic` / `join_topic` を再送するフローを定義する。
- **エラーハンドリング / UX**
  - `errorHandler` に `Topic.create_failed` / `Topic.join_failed` キーを追加し、モーダル内にインラインエラーと再試行ボタンを表示する。成功時は `toast` で「トピックを作成しました」を通知し、直後にコンポーザー本文へフォーカスを戻す。
  - 作成途中でキャンセルした場合は `TopicFormModal` の入力値をドラフトとして保持し、再度開いた際に復元する。オフライン登録時は「接続後に自動作成されます」とガイダンスを表示する。
- **テスト計画**
  - TypeScript: `GlobalComposer.test.tsx` にトピック作成ショートカット → モーダル → 作成完了 → コンポーザー再開のフローを追加。
  - TypeScript: `TopicSelector.test.tsx` へショートカット項目の描画、検索 0 件時の表示、ショートカットキーのハンドリングを検証するケースを追加。
  - TypeScript: `Sidebar.test.tsx` / `Home.test.tsx` で参加トピックが 0 件の際に `createAndJoinTopic` が呼ばれることを確認する。
  - Rust: `tests/integration/topic_create_join.rs`（新規）で `create_topic` → `join_topic` → `list_topics` が一連で成功し、`OfflineActionType::CREATE_TOPIC` の再送が反映されることを検証する。
- **フォローアップ**
  - `phase5_user_flow_summary.md` の 1.2 / 1.3 節と Quick View に新規導線を追記。
  - `tauri_app_implementation_plan.md` Phase 5 優先度へ「Global Composer からのトピック作成」タスクを追加。
  - `phase5_ci_path_audit.md` に `GlobalComposer.topic-create` / `TopicSelector.create-shortcut` テスト ID を登録し、Nightly Frontend Unit Tests の対象に含める。

### 5.10 投稿削除後の React Query キャッシュ整合性（2025年11月06日追加）
- **目的**: 投稿削除操作後に全てのフィードで即時に結果を反映し、Zustand ストアと React Query キャッシュの不整合を解消する。
- **現状**: `postStore.deletePostRemote` は `posts` / `postsByTopic` を更新するが、`useTimelinePosts` / `usePostsByTopic` / `useTrendingPostsQuery` / `useFollowingFeedQuery` のキャッシュを無効化しておらず、削除済み投稿が再表示される。オフライン削除キュー登録時も React Query へ通知されない。
- **改善案**
  - `usePosts.ts` に `useDeletePost` ミューテーションを追加し、成功時に `invalidateQueries`（`['timeline']`, `['posts', 'all']`, `['posts', topicId]`）とトピックメトリクスの再取得をトリガーする。`prefetchTrendingCategory` / `prefetchFollowingCategory` が用いるキーもまとめて無効化する。
  - `useTrendingFeeds.ts` へ `removePostFromTrendingCache` / `removePostFromFollowingCache` ヘルパーを実装し、`QueryClient.setQueryData` で `InfiniteData` から対象投稿を除去する。`PostCard` から呼び出すユーティリティ `invalidatePostCaches(queryClient, post)` を作成する。
  - オフライン時に `OfflineActionType::DELETE_POST` を保存した直後、`queryClient.invalidateQueries` を呼び出してローカルキャッシュを stale とマークし、同期完了後に `syncEngine` が再度無効化する。`useTopicStore.updateTopicPostCount(post.topicId, -1)` を即時反映してサイドバー統計とトレンドスコアを更新する。
- **バックエンド / コマンド**
  - `PostService::delete_post` で `PostCache::remove` を呼び出し、フロントからの再フェッチが削除済み投稿を返さないようにする。
  - `tests/integration/post_delete_flow.rs`（新規）で `create_post` → `delete_post` → `list_following_feed` / `list_trending_posts` が削除済み投稿を含まないことを検証する。Docker シナリオ `post-delete-cache` を追加し、CI で `pnpm vitest run src/tests/unit/hooks/useDeletePost.test.ts` と連動させる。
- **エラーハンドリング**
  - `errorHandler` に `Post.delete_failed` / `Post.delete_offline_enqueued` を追加し、失敗時は「投稿の削除に失敗しました」、オフライン時は「削除は接続後に自動で反映されます」と案内する。
  - `PostCard` の削除メニュー内で再試行ボタンとバックオフ状態を表示し、エラー詳細は `metadata`（`postId`, `topicId`）に記録する。
- **テスト計画**
  - TypeScript: `useDeletePost.test.ts`（新規）でミューテーション成功時の `invalidateQueries` / `setQueryData` 呼び出しとオフライン経路を検証する。
  - TypeScript: `PostCard.test.tsx` に `useDeletePost` フローとオフラインキュー UI を追加し、`topicStore.updateTopicPostCount` 呼び出しを確認する。
  - Rust: `tests/integration/post_delete_flow.rs` と `application/tests/post_service_delete.rs` でキャッシュ削除とイベント発行をユニット/統合テストする。
- **フォローアップ**
  - `phase5_user_flow_summary.md` のタイムライン行および優先度表へキャッシュ整合性改善計画を追記する。
  - `phase5_ci_path_audit.md` に `useDeletePost` / `post_delete_flow` テスト ID を追加し、Nightly テストのカバレッジに含める。
  - `tauri_app_implementation_plan.md` Phase 5 の優先タスクへ「投稿削除キャッシュ整合性」を追加する。

### 5.11 SyncStatusIndicator とオフライン同期導線（2025年11月07日追加）
- **目的**: オフライン操作や差分同期の状態を一元的に可視化し、「いつ同期されるのか」「失敗/競合時にどう対処するのか」を UI 上で完結させる。Relay/P2P インジケーターとは別に、投稿/トピック/フォローなど全エンティティの再送を追跡できるようにする。
- **UI 実装状況**
  - `SyncStatusIndicator`（`src/components/SyncStatusIndicator.tsx`）はヘッダー右側のゴーストボタン＋ポップオーバーで構成。アイコンは `isOnline` / `isSyncing` / `pendingActionsCount` / `conflicts` / `error` を見て `WifiOff`・`RefreshCw`・`AlertTriangle`・`AlertCircle`・`CheckCircle` を切り替える。
  - ポップオーバーには (1) 接続状態、(2) 同期進捗バー（同期中のみ）、(3) 未同期アクション件数、(4) 上位 3 件までの競合カード、(5) エラーメッセージ、(6) 最終同期からの経過時間を表示。`今すぐ同期` ボタンはオンラインかつ未同期アクションが存在する場合のみ有効化される。
  - 競合カードをクリックすると `AlertDialog` で `resolveConflict('local'|'remote'|'merge')` を選択でき、`selectedConflict` をローカルステートで保持する。`SyncConflict` の `localAction.createdAt` を `toLocaleString('ja-JP')` で表示。
  - `PendingActions` が 0 件でもアイコンとテキストで「同期済み」を表示し、バッジは描画しない。`pendingActionsCount > 0` の場合のみ `Badge` に件数を表示。
  - 2025年11月07日: `get_cache_status` の結果を 60 秒間隔（＋ `pendingActions` 変化時）で取得し、キャッシュ合計/ステール件数と `cache_types` をカードで表示。ステールなタイプには「再送キュー」ボタンを表示し、押下時は `add_to_sync_queue` で `action_type='manual_sync_refresh'`・`payload={ cacheType, source: 'sync_status_indicator', requestedAt }` を登録する。`Refresh` ボタンで手動更新し、取得エラー (`cacheStatusError`) は赤字で表示する。
- **同期エンジン / ストア連携**
  - `useSyncManager`（`src/hooks/useSyncManager.ts`）が `syncEngine.performDifferentialSync` を呼び出し、`SyncResult` を解析して `setSyncError`・`clearSyncError`・`syncPendingActions`（`useOfflineStore`）を更新。オンライン復帰後 2 秒で自動同期、さらに 5 分間隔の定期同期を行う。
  - `persistSyncStatuses` は同期結果ごとに `offlineApi.updateSyncStatus(entityType, entityId, status)` を実行し、`fully_synced` / `failed` / `conflict` を Tauri DB に記録。`extractEntityContext` は `OfflineActionType` から `entityType` / `entityId` を推定し、未定義の場合は JSON payload から拾う。
  - `offlineStore.refreshCacheMetadata` が `offlineApi.updateCacheMetadata` を呼び出し、`pendingCount`・`syncErrorCount`・`isSyncing`・`lastSyncedAt` を 1 時間 TTL で記録。`addPendingAction` / `removePendingAction` / `setSyncError` / `clearSyncError` / `syncPendingActions` など全ての経路で `refreshMetadata()` を非同期実行する。
  - `offlineStore` はブラウザの `online/offline` イベントを監視し、オンライン化時に `localStorage.currentUserPubkey` を元に `syncPendingActions` を即時起動。Tauri 側の `offline://reindex_complete` イベントも購読し、再索引完了後に `loadPendingActions` と `updateLastSyncedAt` を呼び出す。
  - `useSyncManager.resolveConflict` は `syncEngine['applyAction']` を直接呼んでローカル/リモート/マージ結果を適用し、成功時は `toast` で通知。解決済みの競合は `setSyncStatus(...conflicts.filter(...))` で除外。
- **バックエンド / コマンド**
  - `offlineApi.saveOfflineAction` / `.syncOfflineActions` / `.getOfflineActions` / `.cleanupExpiredCache` / `.saveOptimisticUpdate` / `.confirmOptimisticUpdate` といった Tauri コマンドを `offlineStore` が直接利用。`saveOfflineAction` 成功時は `OfflineActionType` に応じて `OfflineAction` を `pendingActions` へ登録し、オンラインなら即座に `syncPendingActions` を再実行する。
  - `update_cache_metadata` と `update_sync_status` は 2025年11月06日に導入済みで、`SyncStatusIndicator` のポップオーバー表示とバックエンド統計を一致させるための前提 API。2025年11月07日: `get_cache_status` を `useSyncManager.refreshCacheStatus` から 60 秒間隔＋手動同期後に呼び出し、`cacheStatus` state として UI へ供給。`add_to_sync_queue` は「再送キュー」ボタン経由で `manual_sync_refresh` アクションを生成し、バックエンドの `sync_queue` に JSON payload（`cacheType`/`requestedAt`/`source`/`userPubkey`）を保存する。
  - 今後は `cache_types.metadata` の詳細（失敗理由・最終同期アカウント）を API 側で拡張し、UI へ付加情報を表示する。キュー投入後の進捗を `sync_queue` テーブルと連携し、`SyncStatusIndicator` でステータスをフィードバックする導線を backlog に残す。
- **ギャップ / 今後の導線強化**
  - `SyncStatusIndicator` と `OfflineIndicator` が別コンポーネントのため、画面右下バナーとの重複表示がある。Phase 5 では `OfflineIndicator` を簡易版（接続状態と件数のみ）に絞り、詳細は `SyncStatusIndicator` へ誘導する計画を追加（`tauri_app_implementation_plan.md` にフォローアップを記録予定）。
  - 競合解決ダイアログは `merge` オプションこそ UI に出ているが、`syncEngine['applyAction']` へ渡す `mergedData` を UI 側で生成していないため、実際には `local` / `remote` の 2 択となっている。Conflict preview へ差分表示・マージ入力を追加する必要がある。
  - `errorHandler` は `useSyncManager` / `offlineStore` から `log` / `info` / `warn` を呼び出しているが、UI 側でのユーザー向け文言は `SyncStatusIndicator` のポップオーバーに限定されている。`error_handling_guidelines.md` へ `SyncStatus.*` キーを追加し、トースト文言とメタデータを整理する。
- **テスト計画**
  - 既存: `src/tests/unit/components/SyncStatusIndicator.test.tsx` で `pendingActionsCount`・競合ボタン表示・手動同期ボタン活性・最終同期時刻フォーマットに加え、2025年11月07日からキャッシュステータス表示/更新ボタン/再送キュー操作をカバー。`src/tests/unit/hooks/useSyncManager.test.tsx` も `triggerManualSync` ガード・`persistSyncStatuses`・競合検出に加え、`get_cache_status` の取得タイミングと `enqueueSyncRequest` による `add_to_sync_queue` 呼び出しを検証。`src/tests/unit/stores/offlineStore.test.ts` は `refreshCacheMetadata` / `saveOfflineAction` / `syncPendingActions` の副作用をテスト。
  - 追加予定: (1) `useSyncManager` の 5 分タイマー／オンライン復帰 2 秒同期のフェイクタイマー検証、(2) `offlineStore` の `offline://reindex_complete` リスナー E2E（Vitest の `vi.mock('@tauri-apps/api/event')` によるイベントエミュレーション）、(3) Docker シナリオ `offline-sync` を `docker-compose.test.yml` へ追加し、`npx vitest run src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/hooks/useSyncManager.test.tsx` を Linux/Windows で反復実行。
  - CI: `phase5_ci_path_audit.md` に `SyncStatusIndicator.ui` / `useSyncManager.logic` / `offlineStore.cache-metadata` のパスを追加し、Nightly でのカバレッジ可視化を行う。

### 5.12 ヘッダーDMボタンと Summary Panel 連携（2025年11月08日更新）
- **現状**
  - `src/components/layout/Header.tsx` に `DirectMessageInbox`（`src/components/directMessages/DirectMessageInbox.tsx`）を常時マウントし、メッセージアイコンは既存会話（`activeConversationNpub` → `latestConversationNpub`）を優先して開き、それ以外の場合は Inbox ダイアログを開く。隣に追加した `Plus` ボタン（`data-testid="open-dm-inbox-button"`）から常に Inbox を開けるため、ヘッダー単体で新規 DM を開始できる。
  - `DirectMessageInbox` は会話一覧（`conversations` の末尾メッセージと未読件数をソート）と新規宛先入力（npub / ユーザーID）を提供し、入力バリデーション・最新会話ショートカットを備える。会話を選択すると `useDirectMessageStore.openDialog` を呼び出し、Inbox は自動的に閉じる。
  - Summary Panel の DM カードは `SummaryMetricCard` の `action` プロップを利用して CTA ボタン（`DM Inbox を開く`）を表示し、`useDirectMessageStore.openInbox` を共有導線として呼び出す。ヘッダー/Trending/Following が同じ `DirectMessageInbox` を開くため、どの画面からでも追加クリック無しで DM モーダルへ遷移できるようになった。
  - `useDirectMessageBadge` は `useDirectMessageStore` の `unreadCounts` と `conversations` を集計し、最新メッセージと合計未読をヘッダーおよび Summary Panel へ供給する。`useDirectMessageEvents`（kind4 IPC）による `receiveIncomingMessage` 更新で数値がリアルタイムに反映される。
- **ギャップ / 課題**
  - Inbox は一時的なストアのみで会話リストを保持しており、アプリ再起動や別端末では履歴が表示されない。`direct_message_service` 側に「既読未読／会話一覧」を供給するクエリが無く、SQLite 上の `list_conversations` API を追加する必要がある。
  - 宛先入力は npub/ID の手動入力のみで、ユーザー検索や候補補完が無い。`search_users` 連携や QR コード読み取りなどのフォローアップが必要。
  - Inbox のリストは messages の最終メッセージを用いた簡易ソートのため、大量会話時の仮想スクロールやフィルタリングが未実装。未読カウンタの永続化（`list_direct_messages` で初期値を復元）も backlog。
- **テスト / フォローアップ**
  - TypeScript: `Header.test.tsx` に Inbox CTA・未読バッジ・会話あり/なしの分岐を追加。`useDirectMessageBadge.test.tsx` を新設し、未読集計と最新メッセージ判定を検証。
  - TypeScript: `components/trending/TrendingSummaryPanel.test.tsx` / `components/following/FollowingSummaryPanel.test.tsx` を追加し、DM カードの Helper 表示と CTA で `openInbox` が呼ばれることを確認。`phase5_ci_path_audit.md` の test:unit 行へ追記し、Nightly Frontend Unit Tests で監視。
  - Rust / IPC: 既読カウンタ永続化と会話一覧 API（`list_direct_message_threads` 仮称）を `direct_message_service` に追加し、Inbox の初期表示に反映する。`direct-message:received` イベント payload へ `increment_amount` を含め、他端末での未読同期を検討する。

## 6. プロフィール画像リモート同期設計（iroh-blobs 0.96.0 / iroh-docs 0.94.0）

### 6.1 要件
- プロフィール画像はローカル保存のみを禁止し、必ず iroh ノードを介した Blob 共有と Doc 追跡を行う。
- 画像更新は即時に `profile_avatars` Doc へ反映され、他クライアントは Doc のレプリケーションを通じて最新版を取得する。
- Blob ハッシュと Doc バージョンは Nostr メタデータおよびローカルキャッシュキーとして利用できるようにする。

### 6.2 コンポーネントと役割
| レイヤー | 役割 | 主なAPI/モジュール |
| --- | --- | --- |
| フロント（React/TS） | ファイル選択、プレビュー、Tauri コマンド呼び出し、Doc 更新イベントの監視 | `ProfileForm`, `ProfileEditDialog`, `useProfileAvatarSync`（新規フック） |
| Tauri (Rust) | ファイルの一時保管、Blob 登録、Doc 更新、Blob チケット配布、キャッシュ管理 | `upload_profile_avatar`（新コマンド）、`fetch_profile_avatar`（取得）、`iroh_blobs::client::quic::Client`, `iroh_docs::Doc` |
| iroh-blobs 0.96.0 | 画像バイナリの保管・ハッシュ計算・チケット生成 | `Client::builder`, `Client::blobs().add_path`, `BaoHash`, `BlobTicket` |
| iroh-docs 0.94.0 | プロフィール画像メタデータの CRDT 管理とバージョニング | `Doc::set`, `Author`, `DocTicket`, `Replicator::subscribe` |
| リモートピア | Blob/Doc のレプリケーション、キャッシュ更新 | `profile_avatar_sync` ワーカー（新規サービス） |

### 6.3 データ構造
```json
{
  "doc_name": "profile_avatars",
  "entry_key": "<npub hex>",
  "value": {
    "blob_hash": "bao1h...",
    "format": "image/png",
    "size_bytes": 123456,
    "updated_at": "2025-11-03T12:34:56Z",
    "share_ticket": "iroh-blobs://ticket/...",
    "access_level": "contacts_only",
    "doc_version": 42,
    "uploader_node": "iroh-node-id",
    "signature": "ed25519 signature",
    "encrypted_key": "base64(ciphertext)"
  }
}
```
- `doc_version`: `Doc::clock()` から取得したローカルカウンタ。競合時は新しい `LamportTimestamp` を自動採用。
- `signature`: `Author::sign_change` を流用し、Doc の CRDT と一貫性を保つ。
- `share_ticket`: Blob 取得に必要な Capability Token を encode した文字列。`access_level` に応じて Capability（公開/フォロワー限定/プライベート）を切り替える。
- `encrypted_key`: `StreamEncryptor` で使用したセッションキーを Capability 受領者のみ復号できるよう暗号化したデータ。

### 6.4 処理フロー
1. **アップロード**
  1. フロントが `upload_profile_avatar` を呼び出し、ファイルバイトと `format`・`size_bytes`・希望 `access_level` を送信。
  2. Tauri 側で一時ディレクトリ（`profile_avatars/tmp/{uuid}`）へ書き出し、`StreamEncryptor` で暗号化したバイト列を生成（セッションキーを Capability に封入）。
  3. 暗号化済みファイルを `Client::blobs().add_path` で登録し、戻り値のハッシュを取得。
  4. `client.share(hash)` で Capability 付きの共有チケットを生成し、Doc Value に `share_ticket` / `access_level` / `encrypted_key` を含めて `Doc::set(entry_key, value)` を実行。
  5. `Doc::share()` で Doc チケットを更新し、Mainline DHT 経由でピアへ通知。
2. **ダウンロード**
   1. 他クライアントは `Replicator::subscribe(doc_id)` で Doc 更新を監視。
   2. 新しい `blob_hash` を検出したら `Client::blobs().fetch(hash)` を実行し、成功後 `appDataDir/profile_avatars/{hash}` に保存。
   3. 保存完了時に `authStore.updateUser` を通じてフロントへ反映し、`ProfileForm` の初期値にローカルキャッシュを適用。
3. **削除/ローテーション**
   - 旧 Blob の参照は Doc 更新で上書きする。物理削除は `Client::blobs().delete(hash)` を別ジョブで実行。

### 6.5 セキュリティ・プライバシー
- Blob チケットは Capability に `access_level` を含め、受領者が復号キーを取得できる場合のみ Blob をダウンロード可能とする。
- Doc への書き込みは `Author` 秘密鍵で署名し、別ユーザーが上書きできないようにする（`Doc::set_author` によるアクセス制御）。
- リモートへ送る前に画像を `image` クレートでリサイズ（最大 512x512）しつつ `StreamEncryptor` で暗号化、非権限者への漏えいを防ぐ。

### 6.6 決定事項とフォローアップ
- 共有スコープは `share_ticket` の Capability に埋め込むアクセスレベル（`public` / `contacts_only` / `private`) で分岐し、Doc 参加者はチケット検証によって権限を判断する。設計詳細を `phase5_dependency_inventory_template.md` に反映する。
- Blob の End-to-end 暗号化には `iroh_blobs::crypto::StreamEncryptor` を採用し、アップロード前にクライアント側で暗号化→Blob 登録を行う。鍵管理は Doc 内のメタデータに暗号化された形で保持し、共有先は Capability から復号キーを取得する。
- 既存の外部 URL フォールバックは廃止し、リモート同期が失敗した場合は Tauri アプリ内に同梱したデフォルトアバター（`assets/profile/default_avatar.png`）を表示する。Doc/Blob 未取得時はこのローカル画像を使用し、同期完了後に差し替える。


