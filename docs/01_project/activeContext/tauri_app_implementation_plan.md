# Tauriアプリケーション実装計画

**作成日**: 2025年7月28日  
**最終更新**: 2025年8月2日  
**目的**: 体験設計に基づいた具体的な実装タスクとスケジュール（オフラインファースト対応）

## Phase 1: 認証フローの修正 ✓ 完了

### 1.1 ウェルカム画面の実装 ✓ 完了

#### 完了したタスク
1. `src/routes/welcome.tsx` の作成 ✓
2. `src/components/auth/WelcomeScreen.tsx` の実装 ✓
   - アプリケーションの説明
   - 新規アカウント作成ボタン
   - 既存アカウントでログインボタン
   - テスト作成（5件）
3. `src/components/auth/LoginForm.tsx` の作成 ✓
   - nsec入力フォーム
   - バリデーション（nsec形式の秘密鍵検証）
   - エラーハンドリング
   - セキュア保存オプション
   - テスト作成（8件）
4. `src/components/auth/ProfileSetup.tsx` の作成 ✓
   - 名前、自己紹介の入力
   - アバター設定（イニシャル生成）
   - スキップ機能
   - テスト作成（9件）

#### 実装詳細
```typescript
// WelcomeScreen.tsx
export function WelcomeScreen() {
  const navigate = useNavigate();
  const { generateNewKeypair } = useAuthStore();
  
  const handleCreateAccount = async () => {
    try {
      await generateNewKeypair();
      navigate({ to: '/profile-setup' });
    } catch (error) {
      toast.error('アカウントの作成に失敗しました');
    }
  };
  
  return (
    <div className="flex flex-col items-center justify-center min-h-screen">
      <h1>kukuriへようこそ</h1>
      <p>分散型トピック中心ソーシャルアプリ</p>
      <Button onClick={handleCreateAccount}>新規アカウント作成</Button>
      <Button variant="outline" onClick={() => navigate({ to: '/login' })}>
        既存アカウントでログイン
      </Button>
    </div>
  );
}
```

### 1.2 認証状態の適切な管理 ✓ 完了

#### 完了したタスク
1. `authStore.ts` の修正 ✓
   - 初期状態を `isAuthenticated: false` に固定
   - 起動時に鍵の有効性を確認するロジック追加
   - initializeメソッドの実装（自動ログイン）
   - 複数アカウント管理機能の追加
   - テスト作成（initializeメソッド4件、統合テスト5件）
2. `src/hooks/useAuth.ts` の改善 ✓
   - 初期化ロジックの実装
   - 認証ガードの実装
3. `src/routes/__root.tsx` の修正 ✓
   - 認証状態によるリダイレクト
   - 認証ガードテストの作成

#### 実装の特徴
- **セキュアな鍵管理**: プラットフォーム固有のセキュアストレージを使用し、秘密鍵をメモリに保持しない
- **複数アカウント対応**: 複数のアカウントを安全に管理し、簡単に切り替え可能
- **自動ログイン**: 起動時に前回使用したアカウントで自動的にログイン
- **包括的なテスト**: 全37件のテストによる品質保証

### 1.3 ログアウト機能の修正 ✓ 完了

#### 完了したタスク
1. Headerコンポーネントにユーザーメニュー追加 ✓
   - プロフィール表示
   - 設定メニュー
   - ログアウトボタン
2. ログアウト処理の実装 ✓
   - 確認ダイアログ
   - 状態のクリア
   - ウェルカム画面へのリダイレクト
3. AccountSwitcherコンポーネントの実装 ✓
   - 複数アカウント切り替えUI
   - 現在のアカウント表示
   - アカウント追加・削除機能

### 1.4 セキュアストレージ実装 ✓ 完了

#### 完了したタスク
1. Rustバックエンドのセキュアストレージ実装 ✓
   - keyring crateによるプラットフォーム固有セキュアストレージアクセス
   - macOS Keychain、Windows Credential Manager、Linux Secret Service対応
   - 秘密鍵の個別暗号化保存（npubごと）
   - アカウントメタデータ管理（公開情報のみ）
2. Tauriコマンドの実装 ✓
   - add_account - アカウント追加とセキュア保存
   - list_accounts - 保存済みアカウント一覧
   - switch_account - アカウント切り替え
   - remove_account - アカウント削除
   - get_current_account - 現在のアカウント取得（自動ログイン用）
   - secure_login - セキュアストレージからのログイン
3. フロントエンドの複数アカウント対応 ✓
   - SecureStorageApi TypeScriptラッパー実装
   - authStoreの拡張（複数アカウント管理機能）
   - 自動ログイン機能（起動時の自動認証）
4. 包括的なテストの作成 ✓
   - Rustバックエンドテスト（8件）
   - フロントエンドAPIテスト（6テストスイート）
   - 統合テスト（3テストスイート）

## Phase 2: データ連携の確立 ✓ 完了

### 2.1 ホームページの実データ表示 ✓ 完了

#### 完了したタスク
1. 投稿の実データ表示 ✓
   - `src/pages/Home.tsx` の修正
   - `src/hooks/usePosts.ts` の改善（タイムライン用投稿取得、30秒ごとの自動更新）
   - `src/components/posts/PostCard.tsx` の作成（いいね機能、日本語相対時刻表示）
   - PostCardコンポーネントのテスト作成（9件）
2. トピック一覧の実データ表示 ✓
   - `src/routes/topics.tsx` の作成（トピック探索ページ、リアルタイム検索機能）
   - `src/components/topics/TopicCard.tsx` の作成（参加/退出ボタン、統計情報表示）
   - `src/hooks/useTopics.ts` の実装（TauriAPI連携、CRUD操作ミューテーション）
   - TopicCardコンポーネントのテスト作成（9件）
   - Topics.tsxページのテスト作成（12件）
   - useTopicsフックのテスト作成（7件）
3. 既存テストの修正（QueryClientProvider対応）

#### 実装詳細
```typescript
// usePosts.ts（実装済み）
export function useTimelinePosts() {
  return useQuery({
    queryKey: ['timeline'],
    queryFn: async () => {
      const posts = await TauriApi.getPosts({ limit: 50 });
      return posts;
    },
    refetchInterval: 30000, // 30秒ごとに更新
  });
}
```

### 2.2 トピック機能の実装 ✓ 完了

#### 完了したタスク
1. 投稿作成機能 ✓
   - `src/components/PostComposer.tsx` の実装（投稿作成フォーム）
   - `src/components/TopicSelector.tsx` の実装（トピック選択コンポーネント）
   - Home画面とトピック詳細画面への統合
   - PostComposerのテスト作成（11件）
   - TopicSelectorのテスト作成（12件）
2. トピック管理機能 ✓
   - `src/components/topics/TopicFormModal.tsx` の実装（作成/編集フォーム）
   - `src/components/topics/TopicDeleteDialog.tsx` の実装（削除確認ダイアログ）
   - トピック一覧・詳細ページへの統合
   - react-hook-formを使用したフォームバリデーション
3. P2P連携実装 ✓
   - トピック参加時のP2Pトピック自動参加
   - トピック離脱時のP2Pトピック自動離脱
   - TauriAPIとP2P APIの完全統合

### 2.3 リアルタイム更新の実装 ✓ 完了

#### 完了したタスク
1. Nostrイベントのリアルタイム処理 ✓
   - `src/hooks/useNostrEvents.ts` の作成
   - Tauriイベントリスナーの設定（nostr://event）
   - イベント受信時の自動ストア更新
   - 新規投稿、トピック更新、いいねの即座反映
2. P2Pイベントのリアルタイム処理 ✓
   - `src/hooks/useP2PEventListener.ts` の改善
   - P2Pメッセージの即座反映
   - トピック参加/離脱の自動更新
3. データ同期機能 ✓
   - `src/hooks/useDataSync.ts` の実装
   - 定期的なデータ更新（30秒間隔）
   - イベント駆動とポーリングのハイブリッド方式
4. UI表示機能 ✓
   - `src/components/RealtimeIndicator.tsx` の実装
   - Nostr/P2P接続状態の可視化
   - 最新データ受信時刻の表示
5. 包括的なテスト ✓
   - useNostrEventsのテスト（10件）
   - useDataSyncのテスト（8件）
   - RealtimeIndicatorのテスト（6件）
   - 合計24件の新規テスト追加

### 2.4 追加機能 ✓ 完了

#### 完了したタスク
1. 手動P2P接続機能 ✓
   - `src/components/p2p/PeerConnectionPanel.tsx` の作成 ✓
     - 自分のピアアドレス表示とコピー機能
     - 手動ピアアドレス入力フォーム
     - 接続処理（バリデーション、エラーハンドリング）
     - 接続履歴管理（LocalStorage使用）
   - 設定ページへの統合 ✓
   - 包括的なテストの作成 ✓
2. リアクション機能の実装 ✓
   - 返信機能（ReplyForm） ✓
     - NIP-10準拠の返信タグ実装
     - リアルタイム更新対応
   - 引用機能（QuoteForm） ✓
     - NIP-10準拠の引用タグ実装
     - nostr:プロトコルリンク生成
   - PostCardへの統合 ✓
3. 検索機能の基本実装 ✓
   - SearchBarコンポーネント（デバウンス付き） ✓
   - PostSearchResults（投稿検索） ✓
   - TopicSearchResults（トピック検索） ✓
   - UserSearchResults（ユーザー検索） ✓
   - 検索ページ（/search）の実装 ✓
   - ヘッダーへの検索バー統合 ✓

#### 実装詳細
```typescript
// PeerConnectionPanel.tsx
export function PeerConnectionPanel() {
  const { nodeAddress, connectToPeer } = useP2PStore();
  const [peerAddress, setPeerAddress] = useState('');
  const [isConnecting, setIsConnecting] = useState(false);
  
  const handleConnect = async () => {
    if (!peerAddress.trim()) return;
    
    setIsConnecting(true);
    try {
      await p2pApi.connectToPeer(peerAddress);
      toast.success('ピアに接続しました');
      setPeerAddress('');
    } catch (error) {
      toast.error('接続に失敗しました');
    } finally {
      setIsConnecting(false);
    }
  };
  
  return (
    <Card>
      <CardHeader>
        <CardTitle>P2P接続設定</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div>
            <Label>あなたのピアアドレス</Label>
            <div className="flex gap-2">
              <Input value={nodeAddress} readOnly />
              <Button onClick={() => copyToClipboard(nodeAddress)}>
                コピー
              </Button>
            </div>
          </div>
          
          <div>
            <Label>ピアに接続</Label>
            <div className="flex gap-2">
              <Input
                value={peerAddress}
                onChange={(e) => setPeerAddress(e.target.value)}
                placeholder="/ip4/192.168.1.100/tcp/4001/p2p/QmXXX..."
              />
              <Button 
                onClick={handleConnect}
                disabled={isConnecting || !peerAddress.trim()}
              >
                {接続}
              </Button>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
```

## Phase 3: 主要機能の実装

### 3.1 トピック参加・離脱機能の改善 ✓ 完了

#### 完了したタスク
1. P2P接続の自動化 ✓
   - トピック参加時のP2Pトピック自動参加の最適化
   - topicStoreのjoinTopic/leaveTopicメソッドを非同期化
   - P2P接続とNostrサブスクリプションの統合
   - Nostrサブスクリプション開始タイミングの調整（P2P接続後500ms遅延）
2. UIの状態管理改善 ✓
   - 参加中トピックの一覧表示強化（サイドバー）
   - 最終活動時刻でソートされた一覧表示
   - P2Pメッセージの最終活動時刻を考慮
   - ボタンの状態変更のリアルタイム反映
3. 包括的なテストの追加 ✓
   - topicStore.test.ts（8テストケース）
   - Sidebar.test.tsx（7テストケース）
   - TopicCard.test.tsxの更新（5テストケース追加）

### 3.2 新規投稿機能の拡張 ✓ 部分完了

#### 完了したタスク
1. リッチテキストエディタの実装 ✓
   - マークダウンサポート（@uiw/react-md-editor@4.0.8）
   - MarkdownEditorコンポーネントの作成
   - 画像アップロード機能（ドラッグ&ドロップ対応）
   - メディア埋め込み（画像、動画、YouTube、Vimeo、Twitter/X）
   - プレビュー機能（MarkdownPreviewコンポーネント）
2. 投稿オプションの追加 ✓
   - 予約投稿機能のUI実装（PostScheduler、react-day-picker）
   - 下書き保存機能の実装（draftStore、DraftManager）
   - 自動保存機能（2秒デバウンス）
3. PostComposerコンポーネントの更新 ✓
   - シンプル/Markdownモードのタブ切り替え
   - 全新機能の統合
4. 包括的なテストの追加 ✓
   - 各コンポーネントのテスト作成
   - 17個のテストエラーを全て修正
   - テスト総数: 517個全て成功

#### MVP完成後の改善として保留
- 予約投稿のバックエンド実装
  - 予約投稿の保存機能（SQLite）
  - 予約投稿の実行スケジューラー
  - Tauriコマンドの実装

### 3.3 その他のリアクション機能

#### タスク
1. ブースト機能（リポスト）の実装
   - NostrのNIP-18準拠のリポストイベント
   - UI: ブーストボタンとカウント表示
   - ブースト済み状態の管理
2. ブックマーク機能の実装
   - ローカルストレージでのブックマーク管理
   - ブックマーク一覧ページ
   - UI: ブックマークボタンと状態表示
3. カスタムリアクション絵文字の対応
   - NIP-25準拠のリアクションイベント
   - 絵文字ピッカーの実装
   - リアクション一覧の表示

## Phase 4: オフラインファースト機能の実装

### 4.1 ローカルファーストデータ管理

#### タスク
1. ローカルDBスキーマの拡張
   - 同期ステータステーブル（sync_queue）
   - オフラインアクションログ（offline_actions）
   - キャッシュメタデータテーブル（cache_metadata）
2. オフラインストレージAPIの実装
   - save_offline_action - オフラインアクションの保存
   - get_offline_actions - 保存済みアクションの取得
   - sync_offline_actions - オフラインアクションの同期
   - get_cache_status - キャッシュ状態の取得
3. オフラインストアの実装
   - offlineStore.ts - オフライン状態管理
   - 接続状態監視（navigator.onLine）
   - オフラインキュー管理

### 4.2 楽観的UI更新の実装

#### タスク
1. 操作のローカル実行
   - 投稿作成 - 即座にUIへ反映、背景で同期
   - いいね/リアクション - ローカルステート即座更新
   - トピック参加/離脱 - UI即座反映、同期待ちキュー追加
2. ロールバック機能
   - 同期失敗時のローカル変更の巻き戻し
   - エラー通知と再試行オプション
3. Tanstack Queryの最適化
   - optimistic updatesの設定
   - キャッシュ無効化戦略
   - 背景再フェッチの制御

### 4.3 同期と競合解決

#### タスク
1. 同期エンジンの実装
   - 差分同期アルゴリズム
   - タイムスタンプベースの競合検出
   - 並列同期処理（トピック別）
2. 競合解決戦略
   - Last-Write-Wins (LWW) ベースライン
   - カスタムマージルール（トピック参加状態など）
   - ユーザーへの競合通知UI
3. 同期ステータスの管理
   - 同期進捗の表示
   - 同期エラーのハンドリング
   - 手動同期トリガー

### 4.4 オフラインUI/UX

#### タスク
1. オフラインインジケーター
   - ヘッダーにオフライン状態表示
   - 同期待ちアイテム数の表示
   - 最後の同期時刻表示
2. オフライン用UI調整
   - オフライン中の投稿に「同期待ち」バッジ
   - オンライン時の同期アニメーション
   - オフラインモード専用のトースト通知
3. Service Workerの活用
   - バックグラウンド同期の実装
   - キャッシュ管理
   - オフラインリソースの事前ロード

## MVP完成後の改善

### 予約投稿のバックエンド実装
- 予約投稿の保存機能（SQLite）
- 予約投稿の実行スケジューラー
- Tauriコマンドの実装
- 注：UIは既に実装済み（Phase 3.2）

### 検索機能の拡張
- バックエンドAPI統合
  - 全文検索エンジンの実装
  - 検索結果のキャッシング
- 高度な検索オプション
  - フィルター機能（日付範囲、ユーザー、トピック）
  - ソート機能（関連度、新着順、人気順）
- 注：基本的な検索機能は実装済み（Phase 2.4）

## 開発スケジュール

### 工数見積もり
- Phase 1: ✓ 完了（2日）
- Phase 2: ✓ 完了（3日）
- Phase 3: 一部完了（3.1-3.2完了、3.3残り1-2日）
- Phase 4: オフラインファースト機能 3-4日
  - 4.1 ローカルファーストデータ管理: 1日
  - 4.2 楽観的UI更新: 1日
  - 4.3 同期と競合解決: 1日
  - 4.4 オフラインUI/UX: 1日
- MVP完成後の改善: 2-3日

### 実績
- Phase 1: 2025年7月28日完了（認証フロー実装とテスト）
- Phase 2: 2025年7月30日完了（データ連携基盤と追加機能）
  - 2.1: ホームページの実データ表示（投稿＋トピック一覧）
  - 2.2: トピック機能の実装（投稿作成、トピック管理、P2P連携）
  - 2.3: リアルタイム更新の実装（Nostr/P2Pイベント、データ同期）
  - 2.4: 追加機能の実装（返信/引用機能、検索機能、P2P接続管理）
- Phase 3: 進行中
  - 3.1: 2025年7月31日完了（トピック参加・離脱機能の改善）
  - 3.2: 2025年8月1日完了（新規投稿機能の拡張、予約投稿のバックエンドは保留）
  - 3.3: 次の実装対象（その他のリアクション機能）

### 発見層実装との連携
- Phase 1-2完了後、並行して発見層実装を開始
- 手動接続機能により、発見層完成前でもP2P機能をテスト可能

### 優先順位による調整
- Phase 1-2は完了 ✓
- Phase 3.1-3.2は完了 ✓（予約投稿のバックエンドは保留）
- Phase 3.3（その他のリアクション機能）を次に実装
- Phase 4（オフラインファースト機能）はPhase 3.3完了後に実施
- オフラインファースト機能は、現在の実装基盤（SQLite、Tanstack Query、P2P同期）を活用
- MVP完成後の改善は、ユーザーフィードバックに基づいて実装
- 発見層実装と並行して進行可能

## テスト計画

### 単体テスト
- 各コンポーネントのテスト作成
- ストアのテスト更新
- カバレッジ目標: 80%以上

### 統合テスト
- 認証フローのテスト
- 投稿作成から表示までのフロー
- トピック参加から投稿までのフロー

### E2Eテスト
- 新規ユーザーのオンボーディング
- 既存ユーザーの主要操作

## リスクと対策

### 技術的リスク
1. **Tauriイベントの信頼性**
   - 対策: イベントの再送・リトライロジック

2. **パフォーマンス問題**
   - 対策: 仮想スクロール、ページネーション

3. **データ整合性**
   - 対策: 楽観的UI更新、背景同期

4. **オフライン同期の複雑性**
   - 対策: 段階的実装、十分なテスト

5. **ストレージ容量問題**
   - 対策: 適応的キャッシュ、古いデータの自動削除

### スケジュールリスク
1. **予想外の技術的問題**
   - 対策: バッファ期間の確保、段階的リリース

## 成果物

1. 完全に動作する認証フロー
2. 実データを表示するホーム画面
3. トピック参加・投稿機能
4. リアルタイム更新機能
5. テストカバレッジ80%以上
6. ユーザードキュメント