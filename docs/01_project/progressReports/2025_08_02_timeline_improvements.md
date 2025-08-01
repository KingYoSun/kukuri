# 進捗レポート: タイムライン機能の改善
日付: 2025年8月2日

## 実装内容

### 1. デフォルトトピック設定
- **実装箇所**: `src/stores/authStore.ts`
- **内容**: 
  - アカウント追加時（新規作成・初回ログイン）に自動的に#publicトピックに参加
  - #publicトピックをデフォルトの表示トピックとして設定
  - セキュアストレージ保存時のみ実行（アカウント追加時のみ）

### 2. モック投稿データの削除
- **実装箇所**: `src-tauri/src/modules/post/commands.rs`
- **内容**:
  - `get_posts`関数のモックデータを削除し、空配列を返すように変更
  - ローカルファーストなDB実装は今後のTODOとして設定

### 3. トピック別タイムライン表示
- **実装箇所**: 
  - `src/pages/Home.tsx`
  - `src/hooks/usePosts.ts`
- **内容**:
  - `currentTopic`の有無に応じて表示内容を切り替え
  - トピック選択時はそのトピックの投稿のみ表示
  - トピック未選択時は全体のタイムラインを表示

### 4. 未同期投稿の表記
- **実装箇所**: 
  - `src/stores/types.ts` - Post型に`isSynced`フィールドを追加
  - `src/components/posts/PostCard.tsx` - 未同期表記の表示
  - `src-tauri/src/modules/post/commands.rs` - バックエンドの型定義
- **内容**:
  - 自分の投稿は作成時は未同期（`isSynced: false`）
  - P2Pネットワークへの送信完了後に同期済みとなる設計
  - 未同期の投稿には「未同期」バッジを表示

### 5. 前回表示トピックの復元
- **実装箇所**: `src/stores/topicStore.ts`
- **内容**:
  - `currentTopic`をlocalStorageに永続化
  - アプリ起動時に前回表示していたトピックを自動復元

### 6. タイムラインへの遷移導線
- **実装箇所**: 
  - `src/components/layout/Sidebar.tsx` - サイドバーのトピッククリック時
  - `src/components/topics/TopicCard.tsx` - トピック一覧ページでのクリック時
  - `src/components/layout/Header.tsx` - ロゴクリック時
- **内容**:
  - サイドバーの参加中トピックをクリックでそのトピックのタイムラインへ
  - トピック一覧ページのトピック名クリックで同様の動作
  - ヘッダーのkukuriロゴクリックで全体タイムラインへ

## 技術的詳細

### トピック管理の仕組み
- Zustandストアで`currentTopic`を管理
- localStorageに永続化して前回の選択を記憶
- トピック選択時は`setCurrentTopic()`を呼び出し

### 投稿の同期状態管理
- データベースでの`is_synced`フィールドによる管理を想定
- フロントエンドでは`Post.isSynced`で状態を保持
- 将来的にP2P送信成功時に更新する仕組みを実装予定

## 今後の課題

### データベース実装
- 現在は`get_posts`が空配列を返す暫定実装
- ローカルファーストなデータベース実装が必要
- SQLiteを使用した投稿の永続化機能

### P2P同期機能
- 投稿のP2Pネットワークへの送信処理
- 同期状態の更新機能
- 同期失敗時のリトライ機能

### UI/UX改善
- 未同期投稿の再送信ボタン
- 同期中のプログレス表示
- オフライン時の動作改善

## 関連ファイル
- `src/stores/authStore.ts`
- `src/stores/topicStore.ts`
- `src/stores/types.ts`
- `src/pages/Home.tsx`
- `src/components/posts/PostCard.tsx`
- `src/components/layout/Sidebar.tsx`
- `src/components/topics/TopicCard.tsx`
- `src/components/layout/Header.tsx`
- `src-tauri/src/modules/post/commands.rs`
- `src-tauri/src/modules/topic/commands.rs`