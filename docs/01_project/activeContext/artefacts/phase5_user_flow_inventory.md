# Phase 5 ユーザー導線棚卸し
作成日: 2025年11月01日  
最終更新: 2025年11月03日

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
| サイドバー | 共通 | 参加トピック一覧（P2P最終活動時刻でソート）、未読バッジ、`新規投稿`ボタンでグローバルコンポーザーを起動、カテゴリー（`トピック一覧`/`検索`/`トレンド`/`フォロー中`） | `join_topic`/`leave_topic`（`TopicCard` 経由、`subscribe_to_topic` と連動）、`useComposerStore.openComposer`。※「トレンド」「フォロー中」はパス未割り当て |
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
| ユーザー検索 | `/search` (Tab: users) | `mockUsers` を用いたダミー結果、`/profile/$userId` リンクは未実装ルート | 将来の API 連携が未整備 |

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

## 2. 確認できた導線ギャップ
- サイドバーの「トレンド」「フォロー中」は routing 未実装のプレースホルダー。
- `UserSearchResults` が `/profile/$userId` へ遷移させるが、該当ルート/画面が未定義のため 404 となる。
- `TopicsPage` 以外にはトピック作成導線が存在せず、タイムラインから直接作成できない。
- 投稿の削除導線（`delete_post`）が UI から利用できず、`postStore.deletePostRemote` は未接続。
- 設定画面の「鍵管理」ボタンは依然として UI 表示のみで実装が無い。
- 設定画面の「プライバシー」トグル（プロフィール公開/オンライン表示）は 2025年11月02日時点で `usePrivacySettingsStore` によるローカル永続化まで対応済み。バックエンド連携と反映タイミングは未実装。
- プロフィール編集モーダル内の「画像をアップロード」ボタンはプレースホルダーで、ファイル選択導線が未実装。

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

### 3.2 未使用・要確認コマンド
| コマンド | ラッパー | 想定用途 | 備考 |
| --- | --- | --- | --- |
| `delete_post` | `TauriApi.deletePost` | 投稿削除 | `postStore.deletePostRemote` のみ参照。UI導線未実装。 |
| `add_relay` | `nostrApi.addRelay` / `NostrAPI.addRelay` | リレー追加 | 現状テスト専用。UIからの追加導線なし。 |
| `subscribe_to_user` | `nostrApi.subscribeToUser` / `NostrAPI.subscribeToUser` | ユーザー購読 | UI未接続。 |
| `get_nostr_pubkey` | `nostrApi.getNostrPubkey` / `NostrAPI.getNostrPubkey` | 現在の公開鍵取得 | 呼び出し箇所なし。 |
| `delete_events` | `nostrApi.deleteEvents` / `NostrAPI.deleteEvents` | Nostrイベント削除 | UI/ストア未接続。 |
| `join_topic_by_name` | `p2pApi.joinTopicByName` | 名前ベース参加 | テストのみで、UI導線なし。 |
| `clear_all_accounts_for_test` | `SecureStorageApi.clearAllAccountsForTest` | テスト用リセット | デバッグ UI 未接続。 |
| `get_cache_status` | `offlineApi.getCacheStatus` | キャッシュ診断 | 取得結果の表示先が未決定で、ストアからは未呼び出し。 |
| `add_to_sync_queue` | `offlineApi.addToSyncQueue` | 手動キュー投入 | 既存フローから未使用。今後の再索引拡張候補。 |
| `update_cache_metadata` | `offlineApi.updateCacheMetadata` | キャッシュ更新メタデータ反映 | 呼び出し先がなく、要否検討。 |
| `update_sync_status` | `offlineApi.updateSyncStatus` | 同期状態トラッキング | 現状は同期エンジンが内製で管理。Tauri 連携は保留。 |

統合テストでは以下のコマンドを直接 `invoke` し、バックエンド API の状態確認やスモーク検証を実施している（UI 導線なし）。
- 認証 E2E: `import_key`, `get_public_key`
- リレー接続: `connect_relay`, `disconnect_relay`, `get_relay_status`
- 投稿/トピック状態検証: `create_post`, `create_topic`, `list_posts`, `list_topics`

## 4. 次のアクション候補
1. グローバルコンポーザーの初期トピック選択と投稿後のリフレッシュを最適化し、各画面からの動線を検証する。
2. 「トレンド」「フォロー中」カテゴリー用のルーティング／一覧画面を定義するか、未実装である旨を UI 上に表示する。
3. `UserSearchResults` のリンク先 `/profile/$userId` を実装するか、リンクを無効化する。
4. 投稿削除フローを設計し、`delete_post` コマンドを UI から使用できるようにする。
5. 将来的に利用予定の未使用コマンド（例: `add_relay`, `subscribe_to_user`）について、要否を `refactoring_plan_2025-08-08_v3.md` に整理する。
6. 設定画面の「鍵管理」ボタンについて、バックアップ/インポート導線とコマンド連携を定義する。

## 5. 優先実装メモ（2025年11月02日更新）

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
