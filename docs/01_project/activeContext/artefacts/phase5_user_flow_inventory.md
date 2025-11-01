# Phase 5 ユーザー導線棚卸し
作成日: 2025年11月01日  
最終更新: 2025年11月01日

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
| サイドバー | 共通 | 参加トピック一覧、未読バッジ、`新規投稿`ボタン（未実装）、カテゴリー（`トピック一覧`/`検索`/`トレンド`/`フォロー中`） | `join_topic`/`leave_topic`（`TopicCard` 経由）、`join_topic` で `subscribe_to_topic` 連動。※「トレンド」「フォロー中」はパス未割り当て、`新規投稿`ボタンは onClick 未実装 |
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
| 外観・アカウント | `/settings` | テーマ切替、プロフィール編集/鍵管理ボタン（押下時の処理未実装） | なし（UIスタブ） |
| P2P 接続状況 | `/settings` | `PeerConnectionPanel` – ノード初期化、手動接続、履歴管理 | `initialize_p2p`, `get_node_address`, `get_p2p_status`, `connect_to_peer` |
| Bootstrap 設定 | `/settings` | `BootstrapConfigPanel` – ノード一覧取得/保存/リセット | `get_bootstrap_config`, `set_bootstrap_nodes`, `clear_bootstrap_nodes` |
| Nostr テスト (DEVのみ) | `/settings` `import.meta.env.DEV` 条件 | `NostrTestPanel` – テキストノート送信、トピック投稿、購読、リアクション送信、イベント受信ログ | `publish_text_note`, `publish_topic_post`, `send_reaction`, `subscribe_to_topic` |
| P2P デバッグ (DEVのみ) | `/settings` `import.meta.env.DEV` 条件 | `P2PDebugPanel` – Gossip/Mainline メトリクス取得、トピック参加、ブロードキャスト、サブスクリプション一覧 | `get_p2p_metrics`, `join_p2p_topic`, `leave_p2p_topic`, `broadcast_to_topic`, `list_nostr_subscriptions` |

### 1.6 その他グローバル要素
- `RelayStatus`（サイドバー下部）: `get_relay_status` を 30 秒ごとにポーリングしステータス表示。
- `P2PStatus`: `useP2PStore` の状態を表示（接続ピア数、メッセージ数）。
- `useP2PEventListener` / `useDataSync`: アプリ起動時にバックグラウンド処理を登録、`pubkey_to_npub` などのコマンドを使用。

## 2. 確認できた導線ギャップ
- サイドバー上部の「新規投稿」ボタンに onClick が未設定で、クリックしても `PostComposer` が開かない。
- サイドバーの「トレンド」「フォロー中」は routing 未実装のプレースホルダー。
- `UserSearchResults` が `/profile/$userId` へ遷移させるが、該当ルート/画面が未定義のため 404 となる。
- `TopicsPage` 以外にはトピック作成導線が存在せず、タイムラインから直接作成できない。
- 投稿の削除導線（`delete_post`）が UI から利用できず、`postStore.deletePostRemote` は未接続。
- 設定画面の「プロフィール編集」「鍵管理」ボタンは UI 表示のみで実装が無い。

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

## 4. 次のアクション候補
1. `Sidebar` の「新規投稿」ボタンを `PostComposer` と連携させ、どの画面からでも投稿できるようにする。
2. 「トレンド」「フォロー中」カテゴリー用のルーティング／一覧画面を定義するか、未実装である旨を UI 上に表示する。
3. `UserSearchResults` のリンク先 `/profile/$userId` を実装するか、リンクを無効化する。
4. 投稿削除フローを設計し、`delete_post` コマンドを UI から使用できるようにする。
5. 将来的に利用予定の未使用コマンド（例: `add_relay`, `subscribe_to_user`）について、要否を `refactoring_plan_2025-08-08_v3.md` に整理する。
6. 設定画面の「プロフィール編集」「鍵管理」ボタンに対し、実際の編集/バックアップ導線を定義する。
