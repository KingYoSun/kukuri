# Phase 5 ユーザー導線サマリー
作成日: 2025年11月03日  
最終更新: 2025年11月04日

## 概要
- Phase 5 時点でアプリ UI から到達できる体験を俯瞰し、欠落導線や改善ポイントを即座に把握できるようにする。
- 詳細なフロー・API 連携・設計メモは `phase5_user_flow_inventory.md` を参照し、本書では意思決定に必要なサマリーのみを掲載。
- 導線の状態は「稼働中」「改善中」「未実装」の 3 区分で整理し、次の対応タスクを明示する。

## 1. 画面別導線サマリー

### 1.1 オンボーディング & 認証
| 画面 | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| Welcome | `/welcome` | 新規アカウント作成、ログイン導線 | 稼働中 | `generate_keypair` で鍵を生成、SecureStorage 登録まで完了 |
| Login | `/login` | nsec ログイン、セキュア保存、リレー接続表示 | 稼働中 | `login`/`add_account`/`initialize_nostr` 連携、保存後の自動ログインあり |
| Profile Setup | `/profile-setup` | プロフィール入力、画像選択（ローカルファイル） | 改善中 | `upload_profile_avatar` / `fetch_profile_avatar` でリモート同期。`update_nostr_metadata` と連動し、アクセスレベルは `contacts_only` 固定 |

### 1.2 認証後の主要導線
| セクション | パス/配置 | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| Home タイムライン | `/` | 投稿閲覧、いいね・ブースト・ブックマーク、グローバルコンポーザー | 稼働中 | `PostComposer` 下書き保存、`PostCard` アクション完備 |
| サイドバー | 共通 | 参加トピック一覧、未読バッジ、「新規投稿」ボタン | 改善中 | 「トレンド」「フォロー中」は `/trending`・`/following` ルート実装待ち（Inventory 5.7 参照）。新規投稿は `useComposerStore` でモーダル起動 |
| ヘッダー | 共通 | `SyncStatusIndicator`、`RealtimeIndicator`、`AccountSwitcher` | 稼働中 | アカウント切替/追加/削除、同期状態表示、オフライン通知を提供 |
| Global Composer | 共通（モーダル） | どの画面からでも投稿／トピック選択 | 改善中 | 基本導線は実装済み。トピック初期選択とショートカット改善が backlog |
| トレンドフィード | `/trending`（新設予定） | トレンドスコア上位トピックのランキングカード、最新投稿プレビュー | 未実装（計画済み） | ランキング/UI/テスト仕様は Inventory 5.7 と Phase5 Implementation Plan を参照 |
| フォロー中フィード | `/following`（新設予定） | フォロー中ユーザーの専用タイムライン、未読境界・フォロー解除ショートカット | 未実装（計画済み） | 無限スクロール/API 設計は Inventory 5.7 と Phase5 Implementation Plan を参照 |
| プロフィール詳細 | `/profile/$userId` | プロフィール表示、フォロー/フォロー解除、投稿一覧、DM モーダル起動 | 改善中 | `DirectMessageDialog` は実装済みだが `send_direct_message` (Tauri) が未実装で送信失敗。フォロワー無限スクロールは導入済み、ソート/ページネーションは未対応。 |

### 1.3 トピック関連
| 画面 | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| Topics 一覧 | `/topics` | トピック検索、参加切替、新規作成 | 稼働中 | `TopicFormModal` で作成/編集。統計は `get_topic_stats` を使用 |
| トピック詳細 | `/topics/$topicId` | 投稿一覧、P2P メッシュ表示、参加/離脱 | 改善中 | 最終更新表示は修正済み。トピック削除・編集はモーダル導線あり |
| P2P Mesh | `/topics/$topicId` 内 | `TopicMeshVisualization` で Gossip/Mainline 状態を表示 | 改善中 | ステータス更新のリトライは今後の改善項目 |

### 1.4 検索
| タブ | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| 投稿 | `/search` (posts) | フロント側フィルタで投稿検索 | 稼働中 | 初回ロードで `get_posts` 呼び出し |
| トピック | `/search` (topics) | トピック名/説明で検索 | 稼働中 | `get_topics` 再利用 |
| ユーザー | `/search` (users) | `search_users` で実ユーザー検索、フォロー/解除ボタン | 改善中 | フォロー結果は即時反映。エラーUI・ページネーション・入力バリデーションは未整備。 |

### 1.5 設定 & デバッグ
| セクション | パス | 主な機能 | 導線状態 | 備考 |
| --- | --- | --- | --- | --- |
| 外観 | `/settings` | テーマ切替（ライト/ダーク） | 稼働中 | `useUIStore` 経由で永続化 |
| アカウント | `/settings` | プロフィール編集モーダル、鍵管理プレースホルダー | 改善中 | プロフィール編集は稼働中。鍵管理ボタンは未配線 |
| プライバシー | `/settings` | 公開設定／オンライン表示トグル | 改善中 | `usePrivacySettingsStore` でローカル永続。バックエンド連携待ち |
| P2P 接続 | `/settings` | `PeerConnectionPanel` で手動接続/履歴管理 | 稼働中 | `connect_to_peer` コマンドに紐づく |
| Bootstrap 設定 | `/settings` | ブートストラップノード一覧の取得/登録/リセット | 稼働中 | `set_bootstrap_nodes` などと連携 |
| 開発者ツール (DEV) | `/settings`（開発モード） | `NostrTestPanel`, `P2PDebugPanel` | 改善中 | UI は Dev 限定。計測ログとテスト誘導の整理が backlog |

## 2. グローバル要素
- **ステータスカード**: `RelayStatus` / `P2PStatus` が 30 秒間隔でステータス取得。フェイルオーバー時のバックオフと手動再試行を実装。
- **同期系 UI**: `SyncStatusIndicator`／`OfflineIndicator` が `offlineStore` と `syncEngine` の状態を表示し、未同期アクションの再送を支援。
- **リアルタイム更新**: `RealtimeIndicator` と `useP2PEventListener` で投稿受信を通知し、`topicStore` の未読管理を更新。
- **グローバルコンポーザー**: `useComposerStore` で Home/Sidebar/Topic から共通モーダルを制御し、投稿完了後にストアをリセット。
- **プロフィール導線**: `UserSearchResults` と `/profile/$userId` が連携し、フォロー操作後に React Query キャッシュを即時更新。`DirectMessageDialog` は UI/楽観送信が整備済みで、Inventory 5.6.1 に Tauri 実装計画（コマンド・永続化・テスト）が確定。フォロワー一覧は無限スクロール運用中で、5.6.2 にソート/ページネーションの詳細仕様とテスト計画を追記済み。

## 3. 導線ギャップ Quick View
1. `/trending`・`/following` ルートは未実装（Inventory 5.7 で UI/バックエンド/テスト計画を定義済み）。
2. `/profile/$userId` はフォロー導線とフォロワーリスト（無限スクロール）を備えたが、DirectMessageDialog は Tauri 側の `send_direct_message` / `list_direct_messages` が未実装で送受信不可。Inventory 5.6.1/5.6.2 に実装計画を追記済みで、次ステップは Tauri コマンド実装 + React Query ソート/ページネーション接続とテスト整備。
3. 投稿削除フローは 2025年11月03日に `delete_post` を UI に配線済み。今後は React Query キャッシュ無効化とバックエンド統合テストのフォローアップが必要。
4. 設定 > 鍵管理ボタンがバックエンドと未接続。
5. プライバシー設定のローカル値をバックエンドへ同期する API が未提供。
6. ユーザー検索タブは `search_users` で動作するが、ページネーション・エラー UI・バリデーションの整備が未実装（改善計画は Inventory 5.8 に整理済み）。

## 4. テストカバレッジ概要
- フロントエンド: `pnpm test:unit`（Home/Sidebar/RelayStatus/P2PStatus/Composer/Settings のユニットテストを含む）、`pnpm vitest run src/tests/integration/profileAvatarSync.test.ts`。
- Rust: `cargo test`（`kukuri-tauri/src-tauri` と `kukuri-cli`）で P2P ステータスおよびプロフィール同期を検証。
- Docker: `./scripts/test-docker.sh p2p`・`./scripts/test-docker.ps1 rust` で Gossip/Mainline スモークを再現。

## 5. 関連資料
- `phase5_user_flow_inventory.md` — 詳細な導線/コマンド対応表・設計メモ。
- `tauri_app_implementation_plan.md` Phase 5 — 導線改善タスクとスケジュール。
- `phase5_ci_path_audit.md` — 関連テストと CI パスの依存関係。
- `refactoring_plan_2025-08-08_v3.md` 2.5 節 — 導線指標と未対応項目チェックリスト。

## 6. 未実装項目の優先度見直し（2025年11月04日）

| 優先度 | 項目 | 現状/課題 | ユーザー影響 | 次アクション |
| --- | --- | --- | --- | --- |
| A | 投稿削除 (`delete_post`) | 2025年11月03日: PostCard 削除メニューと `postStore.deletePostRemote` のオフライン対応を実装し、ユニットテストで検証済み。 | 楽観削除は機能するが、React Query キャッシュと Rust 統合テストが未整備。 | React Query 側のキャッシュ無効化と `delete_post` コマンドの統合テスト追加、CI での回帰監視をフォローアップ。 |
| B | `/profile/$userId` ルート | `DirectMessageDialog` は UI/楽観送信を備えるが、Tauri の `send_direct_message` / `list_direct_messages` が未実装。Inventory 5.6.1/5.6.2 にコマンド・永続化・ソート/ページネーションの実装計画を追記済み。 | DM が送れず、フォロワー一覧もソート切替・ページングができない。 | `direct_message_service` / `messaging_gateway` / `direct_message_repository` 実装とマイグレーション、コマンド配線後に React Query から履歴ロード。続けて `get_followers` 拡張（sort/cursor）と `FollowerList` のソート UI + 無限スクロールテスト、Vitest / Rust / Docker シナリオを追加。 |
| B | 鍵管理ダイアログ | 設定>鍵管理ボタンがダミー。バックアップ・復旧手段が提供できていない。 | 端末故障時に復旧不能。運用リスク高。 | `KeyManagementDialog` 実装（エクスポート/インポート）、`export_private_key`/`SecureStorageApi.addAccount` 連携、注意喚起 UI とテスト追加。 |
| B | プライバシー設定のバックエンド連携 | トグルはローカル永続のみで、他クライアントへ反映されない。 | 公開範囲が端末ごとに不一致。誤公開や表示不整合の恐れ。 | `usePrivacySettingsStore` から Tauri コマンドを呼ぶ設計策定、Nostr/P2P への伝播API定義、同期テスト計画を追記。 |
| B | ユーザー検索導線改善 | `/search` (users) は `search_users` で実ユーザーを表示できるが、ページネーション・エラー UI・入力バリデーションが未整備。 | 検索結果が多い場合に追跡・再試行が困難で UX が限定的。 | Inventory 5.8 の設計に沿って `search_users` コマンド拡張（cursor/sort/limit/レートリミット）と React Query リファクタ、`SearchErrorState` コンポーネント、Vitest/Rust/Docker テストを追加。 |
| B | `/trending` / `/following` フィード | サイドバーからの発見導線がプレースホルダーのまま。トレンド指標/フォロー中タイムラインの UI・API が未実装。 | カテゴリークリックが無反応で混乱。トピック発見・フォロー体験の向上機会を逃す。 | Inventory 5.7 と Phase 5 計画に沿って `list_trending_topics`/`list_following_feed` コマンド実装、React Query フック・新規ルートの追加、メトリクス集計ジョブとユニット/統合テストを整備。 |

> 優先度A: 現行体験に致命的影響があるもの。<br>
> 優先度B: 早期に手当てしたいが依存タスクがあるもの。<br>
> 優先度C: 情報提供や暫定UIでの回避が可能なもの。
