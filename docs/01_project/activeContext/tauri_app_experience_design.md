# Tauriアプリケーション体験設計

**作成日**: 2025年7月28日  
**最終更新**: 2025年8月2日  
**目的**: kukuriアプリケーションのユーザー体験を実際の操作に即した形に改善（オフラインファースト対応）

## 現状の課題

### 1. 初期起動時の問題
- **問題**: アプリケーション起動時に既にログイン状態になっている
- **原因**: authStoreのpersist設定により、認証状態が保存されているが、実際の鍵情報が存在しない
- **影響**: ログアウトできず、新規ユーザーが利用開始できない

### 2. テスト表示の問題
- **問題**: ホームページにハードコードされたテスト投稿が表示される
- **原因**: 実際のAPIからデータを取得していない
- **影響**: 実際の投稿やトピックの機能が体験できない

### 3. 機能の未接続
- **問題**: トピック購読、新規投稿などの主要機能がUIと連携していない
- **原因**: ストアとAPIの接続は実装されているが、UIコンポーネントから適切に呼び出されていない
- **影響**: アプリケーションの主要機能が動作しない

## ユーザー体験フロー設計

### 1. 初回起動フロー
```
1. アプリケーション起動
   ↓
2. ウェルカム画面表示
   - アプリの説明
   - "新規アカウント作成" or "既存アカウントでログイン" 選択
   ↓
3a. 新規アカウント作成選択時
   - 鍵ペアを自動生成
   - nsecを安全に保存するよう案内
   - プロフィール設定画面へ
   ↓
3b. 既存アカウント選択時
   - nsec入力画面表示
   - ログイン処理
   ↓
4. ホーム画面へ遷移
```

### 2. ログイン済みユーザーフロー
```
1. アプリケーション起動
   ↓
2. 認証状態確認
   - localStorageから認証情報取得
   - 鍵の有効性を確認
   ↓
3. ホーム画面表示
   - デフォルトトピックのタイムライン表示
   - P2P接続状態表示
```

### 3. 主要機能フロー

#### トピック参加
```
1. サイドバーから "トピックを探す" 選択
   ↓
2. トピック一覧表示
   - 人気のトピック
   - 最新のトピック
   - 検索機能
   ↓
3. トピック選択
   - トピック詳細表示
   - "参加する" ボタン
   ↓
4. 参加処理
   - P2P接続確立
   - Nostrサブスクリプション開始
   - サイドバーの参加中リストに追加
```

#### 新規投稿
```
1. サイドバーの "新規投稿" ボタン
   ↓
2. 投稿作成ダイアログ
   - トピック選択（複数可）
   - 本文入力
   - 画像添付（オプション）
   ↓
3. 投稿送信
   - Nostrイベント作成・署名
   - P2P配信
   - タイムラインに即座反映
```

## 実装優先順位

### Phase 1: 認証フローの修正（最優先）
1. ウェルカム画面の実装
2. 認証状態の適切な管理
3. ログアウト機能の修正
4. 鍵管理の安全性向上

### Phase 2: データ連携の確立
1. ホームページの実データ表示
2. トピック一覧の実装
3. 投稿の取得・表示
4. リアルタイム更新の実装

### Phase 3: 主要機能の実装
1. トピック参加・離脱機能
2. 新規投稿機能
3. リアクション機能（いいね、リポスト）
4. 検索機能

### Phase 4: オフラインファースト機能の実装
1. ローカルファーストのデータ管理
2. オフライン時のシームレスな動作
3. 同期と競合解決の仕組み
4. 接続状態のインテリジェントな管理

## UI/UXの改善点

### 1. 状態表示の強化
- 接続状態（Nostrリレー、P2P）を常時表示
- 同期状態のインジケーター
- エラーメッセージの改善

### 2. 操作フィードバック
- ローディング状態の明確化
- 成功/失敗の通知（toast）
- プログレス表示（大量データ同期時）

### 3. オンボーディング
- 初回利用時のツアー
- ツールチップによる機能説明
- ヘルプドキュメントへのリンク

## 技術的実装詳細

### 1. 認証状態管理
```typescript
// authStoreの改善
- 初期状態をfalseに固定
- 起動時に鍵の有効性を確認
- 無効な場合は自動ログアウト
```

### 2. データフェッチング
```typescript
// useQueryを活用したデータ取得
- 初期ロード時のスケルトン表示
- エラーハンドリング
- リトライロジック
```

### 3. リアルタイム更新
```typescript
// Tauriイベントリスナーの活用
- nostr://event
- p2p://message
- 状態の自動更新
```

## オフラインファースト設計原則

### 1. データ管理の原則
- **ローカルデータベース優先**: すべての操作はまずローカルDBに保存
- **楽観的UI更新**: ユーザー操作は即座にUIに反映
- **背景同期**: ネットワーク接続時に自動的に同期
- **競合解決**: Last-Write-Wins戦略とユーザー通知

### 2. オフライン時の体験
- **完全な読み取り機能**: 過去のデータはすべて閲覧可能
- **作成・編集機能**: 投稿、いいね、トピック参加などすべて可能
- **同期待ちキュー**: オフライン中の操作を記録し、接続時に実行
- **状態の可視化**: オフライン状態と同期待ちアイテムの明示

### 3. 同期戦略
- **差分同期**: 最後の同期以降の変更のみを送受信
- **優先度付き同期**: ユーザーの現在のコンテキストを優先
- **帯域幅管理**: モバイル環境での通信量最適化
- **再試行メカニズム**: 失敗した同期の自動リトライ

### 4. キャッシュ戦略
- **プログレッシブ・エンハンスメント**: 基本機能から段階的に機能追加
- **適応的キャッシュ**: 使用頻度に基づくキャッシュ管理
- **メディアの遅延読み込み**: 画像・動画の効率的な管理
- **ストレージ制限対応**: デバイスの容量に応じた自動調整

## 次のステップ

1. 実装計画の詳細化
2. UIモックアップの作成（必要に応じて）
3. Phase 1の実装開始
4. ユーザーテストの実施