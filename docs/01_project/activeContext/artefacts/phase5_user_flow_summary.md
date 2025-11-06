# Phase 5 ユーザー導線サマリー
作成日: 2025年11月03日  
最終更新: 2025年11月06日

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
| サイドバー | 共通 | 参加トピック一覧、未読バッジ、「新規投稿」ボタン | 改善中 | カテゴリーは `useUIStore.activeSidebarCategory` で同期。`prefetchTrendingCategory`/`prefetchFollowingCategory` によりトレンド/フォロー導線のレスポンスを改善。追加要素（サマリーパネル）を継続検討 |
| ヘッダー | 共通 | `SyncStatusIndicator`、`RealtimeIndicator`、`AccountSwitcher` | 稼働中 | アカウント切替/追加/削除、同期状態表示、オフライン通知を提供 |
| Global Composer | 共通（モーダル） | どの画面からでも投稿／トピック選択 | 改善中 | 基本導線は実装済み。トピック初期選択とショートカット改善が backlog |
| トレンドフィード | `/trending` | トレンドスコア上位トピックのランキングカード、最新投稿プレビュー | 改善中 | `list_trending_topics`/`list_trending_posts`（limit=10/per_topic=3, staleTime=60s）を利用。`generated_at` はミリ秒エポック要件（Inventory 5.7）。`routes/trending.test.tsx` と Sidebar/Hook テストで Loading/Error/Empty/Prefetch を検証済み。参加ボタンの体験向上と Docker シナリオは backlog |
| フォロー中フィード | `/following` | フォロー中ユーザーの専用タイムライン、無限スクロール | 改善中 | `list_following_feed`（limit=20, cursor=`{created_at}:{event_id}`）を `useInfiniteQuery` で表示。Prefetch + Retry 導線は整備済み。`routes/following.test.tsx` でローディング/追加ロード/エラー/終端を確認。Summary Panel と DM 未読表示は backlog |
| プロフィール詳細 | `/profile/$userId` | プロフィール表示、フォロー/フォロー解除、投稿一覧、DM モーダル起動 | 改善中 | `DirectMessageDialog` は React Query で履歴読み込み・未読リセット・無限スクロールを実装済み。Kind4 IPC 同期とフォロワー無限スクロールのソート/ページネーションは backlog。 |

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
| ユーザー | `/search` (users) | `search_users` で実ユーザー検索、フォロー/解除ボタン | 改善中 | フォロー結果は即時反映。ページネーション仕様・SearchErrorState・入力バリデーション指針を Inventory 5.8（2025年11月06日更新）/エラーハンドリングガイドラインへ反映済み。実装待ち。 |

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
- **プロフィール導線**: `UserSearchResults` と `/profile/$userId` が連携し、フォロー操作後に React Query キャッシュを即時更新。`DirectMessageDialog` は React Query ベースの履歴ロード・未読リセット・無限スクロールまで接続済みで、Inventory 5.6.1 に IPC 連携と再送タスクを追加。フォロワー一覧は無限スクロール運用中で、5.6.2 にソート/ページネーションの詳細仕様とテスト計画を追記済み。
- **ユーザー検索**: `UserSearchResults` の状態遷移（idle/typing/ready/loading/success/empty/rateLimited/error）と `SearchErrorState` ハンドリング、`query` バリデーション（2〜64文字、制御文字除去、連続スペース正規化）を Inventory 5.8 と `error_handling_guidelines.md` に記録。React Query のデバウンス・AbortController 方針もドキュメント化。

## 3. 導線ギャップ Quick View
1. `/trending`・`/following` ルートは実装済み（Inventory 5.7 に残タスクとテスト計画を記載）。2025年11月06日時点でデータ要件とテスト計画を整理済み。`generated_at` をミリ秒エポックへ揃えること、Summary Panel や Docker シナリオの整備を継続。
2. `/profile/$userId` はフォロー導線とフォロワーリスト（無限スクロール）を備え、DirectMessageDialog も React Query で履歴読み込み・未読リセット・無限スクロールを実装済み。引き続き Kind4 IPC 連携と会話リスト未読バッジ、フォロワー一覧のソート/ページネーションを Inventory 5.6.1/5.6.2 に沿って進める。
3. 投稿削除フローは 2025年11月03日に `delete_post` を UI に配線済み。今後は React Query キャッシュ無効化とバックエンド統合テストのフォローアップが必要。
4. 設定 > 鍵管理ボタンがバックエンドと未接続。
5. プライバシー設定のローカル値をバックエンドへ同期する API が未提供。
6. ユーザー検索タブは `search_users` で動作するが、無限スクロール/状態遷移/エラーUIは未実装（Inventory 5.8 に状態機械・入力バリデーション・SearchErrorState 設計を追記済み、`error_handling_guidelines.md` にメッセージ鍵を登録済み）。

## 4. テストカバレッジ概要
- フロントエンド: `pnpm test:unit`（Home/Sidebar/RelayStatus/P2PStatus/Composer/Settings のユニットテストを含む）、`pnpm vitest run src/tests/integration/profileAvatarSync.test.ts`、`npx vitest run src/tests/unit/routes/trending.test.tsx src/tests/unit/routes/following.test.tsx`。
- Rust: `cargo test`（`kukuri-tauri/src-tauri` と `kukuri-cli`）で P2P ステータスおよびプロフィール同期を検証。
- Docker: `./scripts/test-docker.sh p2p`・`./scripts/test-docker.ps1 rust` で Gossip/Mainline スモークを再現。

## 5. 関連資料
- `phase5_user_flow_inventory.md` — 詳細な導線/コマンド対応表・設計メモ。
- `tauri_app_implementation_plan.md` Phase 5 — 導線改善タスクとスケジュール。
- `phase5_ci_path_audit.md` — 関連テストと CI パスの依存関係。
- `refactoring_plan_2025-08-08_v3.md` 2.5 節 — 導線指標と未対応項目チェックリスト。

## 6. 未実装項目の優先度見直し（2025年11月05日）

| 優先度 | 項目 | 現状/課題 | ユーザー影響 | 次アクション |
| --- | --- | --- | --- | --- |
| A | 投稿削除 (`delete_post`) | 2025年11月03日: PostCard 削除メニューと `postStore.deletePostRemote` のオフライン対応を実装し、ユニットテストで検証済み。 | 楽観削除は機能するが、React Query キャッシュと Rust 統合テストが未整備。 | React Query 側のキャッシュ無効化と `delete_post` コマンドの統合テスト追加、CI での回帰監視をフォローアップ。 |
| B | `/profile/$userId` ルート | `DirectMessageDialog` は React Query で履歴読み込み・未読リセットまで完了。Kind4 IPC 同期とフォロワー一覧のソート/ページネーションは未実装。 | DM 履歴はモーダル表示で確認できるが、会話リストの未読バッジとフォロワー一覧の絞り込みが不足し、継続的な会話/フォロー管理が難しい。 | Inventory 5.6.1 に沿って IPC イベント連携・会話リスト未読同期・再送 UI を実装し、`get_followers` sort/cursor 拡張と `FollowerList` のソート UI、Vitest/Rust/Docker のシナリオを整備。 |
| B | 鍵管理ダイアログ | 設定>鍵管理ボタンがダミー。バックアップ・復旧手段が提供できていない。 | 端末故障時に復旧不能。運用リスク高。 | `KeyManagementDialog` 実装（エクスポート/インポート）、`export_private_key`/`SecureStorageApi.addAccount` 連携、注意喚起 UI とテスト追加。 |
| B | プライバシー設定のバックエンド連携 | トグルはローカル永続のみで、他クライアントへ反映されない。 | 公開範囲が端末ごとに不一致。誤公開や表示不整合の恐れ。 | `usePrivacySettingsStore` から Tauri コマンドを呼ぶ設計策定、Nostr/P2P への伝播API定義、同期テスト計画を追記。 |
| B | ユーザー検索導線改善 | `/search` (users) は `search_users` で実ユーザーを表示できるが、ページネーション・エラー UI・入力バリデーションが未整備。 | 検索結果が多い場合に追跡・再試行が困難で UX が限定的。 | Inventory 5.8 の状態遷移図・入力ガード・`SearchErrorState` 設計に沿って `search_users` コマンド拡張（cursor/sort/limit/レートリミット）と React Query リファクタ、Vitest/Rust/Docker テストを追加。 |
| B | `/trending` / `/following` フィード | 2025年11月05日: ルート・コマンド実装済み。`list_trending_topics`/`list_following_feed` で稼働中だが、`generated_at` のミリ秒化・Summary Panel・Docker シナリオ・DM 未読バッジが未対応。 | フィード自体は閲覧できるものの、更新時刻表示のずれや周辺メトリクス不足で発見体験が限定的。 | Inventory 5.7 と Phase 5 計画に沿って `generated_at` をミリ秒へ修正、Summary Panel / DM 未読バッジ追加、`trending_metrics_job` 実装と Nightly テスト追加、Docker シナリオを整備。 |

> 優先度A: 現行体験に致命的影響があるもの。<br>
> 優先度B: 早期に手当てしたいが依存タスクがあるもの。<br>
> 優先度C: 情報提供や暫定UIでの回避が可能なもの。
