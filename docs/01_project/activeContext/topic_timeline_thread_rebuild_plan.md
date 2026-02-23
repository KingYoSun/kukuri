# トピック別タイムライン/スレッド再構築 設計・実装計画

作成日: 2026年02月23日  
最終更新日: 2026年02月23日

## 1. 目的

- 現在の「トピックごとのタイムライン表示」を維持しつつ、同一トピックに「スレッド表示（フォーラム風）」を追加する。
- タイムラインは「コンテキスト薄めの閲覧」、スレッドビューは「コンテキスト濃い会話」に役割分離する。
- タイムラインで親投稿クリック時に右ペインでスレッドを並列表示し、さらに左へ引っ張る操作でルーター遷移してフルスレッド画面に入れる。
- タイムラインに P2P の強みを活かした「リアルタイム更新モード」を追加する。
- スレッド識別は「トピック名前空間の子」として UUID で定義する。
- 本番未運用前提のため、既存データ互換は持たず breaking change を許容して実装を簡素化する。

## 2. 現状ギャップ（実装確認ベース）

- 投稿取得は `get_posts(topic_id)` が中心で、スレッド情報（thread id / parent / root）を返していない。
- フロントは返信時に `reply_to` を送ろうとしているが、`create_post` DTO が `reply_to` を受理していないため、返信構造が保存されない。
- 投稿一覧は単純な時系列リストで、タイムライン向け要約（親+先頭返信）とスレッド向け展開表示を使い分ける層がない。
- ルーティングは `/topics/$topicId` までで、スレッド一覧/スレッド詳細のURLがない。

## 3. 情報設計・UX設計

### 3.1 画面区分

- `Timeline View`（軽い閲覧）
  - 1カード = 親投稿 + 先頭返信プレビュー1件 + 返信件数 + 最終アクティビティ。
  - 投稿本文は短縮表示優先。
  - 親投稿クリックで右ペインに `Thread Preview` を開く。
- `Thread View`（深い会話）
  - フォーラム風（Root + 階層返信）。
  - 返信作成、引用、展開/折りたたみ、未読位置ジャンプを提供。

### 3.2 右ペイン導線

- タイムライン上の親投稿クリック:
  - 右ペインを `preview` 状態で開く（同一画面並列）。
- 右ペインを左方向へドラッグ:
  - 閾値超過で `navigate('/topics/$topicId/threads/$threadUuid')` を実行。
- キーボード/アクセシビリティ代替:
  - 「スレッドを全画面で開く」ボタンを右ペイン上部に常設。

### 3.3 ルーティング案

- `/topics/$topicId` : タイムライン（既存の主導線）
- `/topics/$topicId/threads` : スレッド一覧
- `/topics/$topicId/threads/$threadUuid` : スレッド詳細

## 4. スレッドIDと名前空間設計

### 4.1 定義

- `topic_namespace`: 既存トピックID（例: `kukuri:topic:rust`）
- `thread_uuid`: UUIDv7（新規スレッド作成時に採番）
- `thread_namespace`: `<topic_namespace>/threads/<thread_uuid>`
  - 例: `kukuri:topic:rust/threads/018f...`

### 4.2 投稿タグ設計（NIP-10併用）

- 親投稿（スレッドRoot）:
  - `["t", "<topic_namespace>"]`
  - `["thread", "<thread_namespace>"]`
  - `["thread_uuid", "<thread_uuid>"]`
- 返信投稿:
  - 上記 `t/thread/thread_uuid` を継承
  - NIP-10 `e` タグ（`root`/`reply` marker）を付与

### 4.3 非互換方針（breaking）

- 旧投稿の互換変換ロジックは実装しない。
- スレッド関連データ（`thread_uuid` / 親子関係）は新フォーマットを必須とする。
- 開発環境DBは migration 適用時に再作成を前提とし、backfill は実施しない。

## 5. データ・API設計

### 5.1 Backend（Rust/Tauri）

- `CreatePostRequest` を拡張:
  - `thread_uuid: String`（必須）
  - `reply_to: Option<String>`（返信時のみ）
  - `thread_namespace` は受け取らず、`topic_id + thread_uuid` からサーバー側で決定
- `PostResponse` を拡張:
  - `thread_namespace`, `thread_uuid`, `thread_root_event_id`, `thread_parent_event_id`（すべて常時返却）
- 新規クエリ/コマンド:
  - `get_topic_timeline(request)`:
    - 返却: 親投稿 + 先頭返信 + 件数/最終更新
  - `list_topic_threads(request)`:
    - 返却: スレッド一覧（Root要約）
  - `get_thread_posts(request)`:
    - 返却: 指定スレッドの投稿群（ページング対応）

### 5.2 永続化

- `event_threads` テーブルを正式な参照元として追加する。
  - `event_id`, `topic_id`, `thread_namespace`, `thread_uuid`, `root_event_id`, `parent_event_id`, `created_at`
- `thread_uuid` / `root_event_id` を NOT NULL 制約にして整合性を強制する。
- 旧データ互換のための backfill migration は追加しない。

## 6. Frontend設計

### 6.1 状態管理

- `uiStore` 拡張:
  - `topicViewMode: 'timeline' | 'threads'`
  - `timelineUpdateMode: 'standard' | 'realtime'`
  - `previewThread: { topicId; threadUuid } | null`
  - `threadPaneState: 'closed' | 'preview' | 'pinned'`

### 6.2 Hook/Query

- `useTopicTimeline(topicId, mode)`
- `useTopicThreads(topicId)`
- `useThreadPosts(topicId, threadUuid)`
- `useRealtimeTimeline(mode)`:
  - `realtime` のときのみ push event を差分適用
  - 通常モードは既存 refetch（30秒）維持

### 6.3 コンポーネント

- `TopicTimelinePage`
- `TimelineThreadCard`（親+先頭返信）
- `ThreadPreviewPane`（右ペイン）
- `ThreadListPage`
- `ThreadDetailPage`（フォーラム風）
- `TimelineModeToggle`（標準/リアルタイム）

## 7. リアルタイム更新モード設計

- `standard`:
  - 現行どおり polling/refetch 主体。
- `realtime`:
  - `nostr://event` / `p2p://message` を timeline query に差分反映。
  - 高頻度イベントは 0.5-1.0秒バッチで反映（再描画抑制）。
  - UIに `LIVE` バッジ、切断時は自動で `standard` へフォールバック可能にする。

## 8. 実装フェーズ

### Phase 0: 仕様確定

- DTO/タグ仕様とURL仕様を ADR 化。

### Phase 1: Backend基盤

- `CreatePostRequest/PostResponse` 拡張。
- thread関連タグ保存と `get_thread_posts` 実装。
- `event_threads` migration + index追加（backfill なし、DB再作成前提）。

### Phase 2: Timeline再構築

- `get_topic_timeline` 実装。
- フロントで親+先頭返信カードへ置換。

### Phase 3: Thread UI

- `/topics/$topicId/threads` と `/topics/$topicId/threads/$threadUuid` 実装。
- フォーラム風スレッド詳細（階層描画/返信）。

### Phase 4: 右ペイン導線

- タイムライン親投稿クリックで右ペイン表示。
- ドラッグ閾値でフルルート遷移。

### Phase 5: リアルタイムモード

- `TimelineModeToggle` と `useRealtimeTimeline` 実装。
- 差分反映/フォールバック/重複排除。

### Phase 6: 検証

- 回帰テスト、性能確認、UI導線の最終確認。

## 9. テスト計画

- Frontend Unit:
  - `TimelineThreadCard`, `ThreadPreviewPane`, `TimelineModeToggle`, 新規 hooks。
- Frontend Route Integration:
  - 右ペイン表示、ドラッグ遷移、URL直打ち復元。
- Rust Unit/Integration:
  - thread tag parse、timeline集約、thread取得。
- E2E/Scenario:
  - 「親投稿→右ペイン→ドラッグで詳細遷移→返信→タイムライン反映」を通し確認。

## 10. 受け入れ基準（Definition of Done）

- タイムラインで「親投稿 + 先頭返信」の表示が動作する。
- スレッド一覧/詳細が topic ごとに参照できる。
- 親投稿クリックで右ペインが開き、ドラッグで詳細ルートへ遷移できる。
- `realtime` モードで新着が即時反映され、切断時のフォールバックが機能する。
- スレッド識別が `topic namespace + thread UUID` で一貫している。
- 旧フォーマット投稿への互換処理が存在しないこと（breaking 方針どおり）。
