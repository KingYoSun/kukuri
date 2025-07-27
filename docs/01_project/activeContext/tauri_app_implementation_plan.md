# Tauriアプリケーション実装計画

**作成日**: 2025年7月28日
**目的**: 体験設計に基づいた具体的な実装タスクとスケジュール

## Phase 1: 認証フローの修正（最優先）

### 1.1 ウェルカム画面の実装

#### タスク
1. `src/routes/welcome.tsx` の作成
2. `src/components/auth/WelcomeScreen.tsx` の実装
   - アプリケーションの説明
   - 新規アカウント作成ボタン
   - 既存アカウントでログインボタン
3. `src/components/auth/LoginForm.tsx` の作成
   - nsec入力フォーム
   - バリデーション
   - エラーハンドリング
4. `src/components/auth/ProfileSetup.tsx` の作成
   - 名前、自己紹介の入力
   - アバター設定

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

### 1.2 認証状態の適切な管理

#### タスク
1. `authStore.ts` の修正
   - 初期状態を `isAuthenticated: false` に固定
   - 起動時に鍵の有効性を確認するロジック追加
2. `src/hooks/useAuth.ts` の改善
   - 初期化ロジックの実装
   - 認証ガードの実装
3. `src/routes/__root.tsx` の修正
   - 認証状態によるリダイレクト

#### 実装詳細
```typescript
// authStore.ts の修正
const initialState: AuthState = {
  isAuthenticated: false,
  currentUser: null,
  privateKey: null,
};

// 起動時の初期化メソッド追加
initialize: async () => {
  const stored = localStorage.getItem('auth-storage');
  if (stored) {
    const parsed = JSON.parse(stored);
    if (parsed.state?.privateKey) {
      try {
        // 鍵の有効性を確認
        await TauriApi.login({ nsec: parsed.state.privateKey });
        // 成功したらそのまま
      } catch {
        // 失敗したらログアウト
        set(initialState);
      }
    }
  }
}
```

### 1.3 ログアウト機能の修正

#### タスク
1. Headerコンポーネントにユーザーメニュー追加
   - プロフィール表示
   - 設定メニュー
   - ログアウトボタン
2. ログアウト処理の実装
   - 確認ダイアログ
   - 状態のクリア
   - ウェルカム画面へのリダイレクト

## Phase 2: データ連携の確立

### 2.1 ホームページの実データ表示

#### タスク
1. `src/pages/Home.tsx` の修正
   - ハードコードされた投稿を削除
   - useQueryを使用した実データ取得
2. `src/hooks/usePosts.ts` の改善
   - タイムライン用の投稿取得ロジック
   - ページネーション対応
3. `src/components/posts/PostCard.tsx` の作成
   - 投稿表示コンポーネント
   - リアクションボタンの動作実装

#### 実装詳細
```typescript
// usePosts.ts
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

### 2.2 トピック一覧の実装

#### タスク
1. `src/routes/topics.tsx` の作成
   - トピック探索ページ
2. `src/components/topics/TopicList.tsx` の実装
   - トピック一覧表示
   - 検索機能
   - ソート機能
3. `src/components/topics/TopicCard.tsx` の作成
   - トピック情報表示
   - 参加ボタン

### 2.3 リアルタイム更新の実装

#### タスク
1. `src/hooks/useNostrEvents.ts` の作成
   - Tauriイベントリスナーの設定
   - イベント受信時のストア更新
2. `src/hooks/useP2PEvents.ts` の改善
   - P2Pメッセージのリアルタイム反映

### 2.4 手動P2P接続機能

#### タスク
1. `src/components/p2p/PeerConnectionPanel.tsx` の作成
   - 自分のピアアドレス表示
   - コピーボタン
   - 手動ピアアドレス入力フォーム
   - 接続ボタン
2. P2P接続処理の実装
   - アドレスのバリデーション
   - 接続試行とエラーハンドリング
   - 接続成功時の通知
3. 接続履歴管理
   - 最近接続したピアの保存
   - ピアリストの表示

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

### 3.1 トピック参加・離脱機能

#### タスク
1. P2P接続の自動化
   - トピック参加時のP2Pトピック参加
   - Nostrサブスクリプション開始
2. UIの状態管理
   - 参加中トピックの表示
   - ボタンの状態変更

### 3.2 新規投稿機能

#### タスク
1. `src/components/posts/PostComposer.tsx` の作成
   - 投稿作成フォーム
   - トピック選択
   - プレビュー
2. 投稿送信処理
   - Nostrイベント作成
   - P2P配信
   - UIの即座更新

## Phase 4: P2P機能の拡張

### 4.1 P2P接続状態の可視化改善
- 接続中のピア一覧
- ネットワークトポロジー表示
- 接続品質インジケーター

### 4.2 トピックメッシュの活用
- トピックごとのピア管理
- メッセージ伝播経路の可視化

### 4.3 オフライン機能
- ローカルキャッシュの実装
- オフライン時のメッセージ保存
- 再接続時の同期処理

## 開発スケジュール

### 工数見積もり
- Phase 1: 2-3日
- Phase 2: 3-4日（手動P2P接続機能を含む）
- Phase 3: 3-4日
- Phase 4: 2-3日

### 発見層実装との連携
- Phase 1-2完了後、並行して発見層実装を開始
- 手動接続機能により、発見層完成前でもP2P機能をテスト可能

### 優先順位による調整
- 最初にMVPとしてPhase 1-2を完成させる
- ユーザーテストの結果に基づいてPhase 3-4を調整

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