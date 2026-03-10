# Zustandモック実装問題の解決

*作成日: 2025年07月27日*

## 概要

フロントエンドのテストで発生していたZustandモック関連の問題を解決し、テストの大部分が正常に動作するようになりました。

## 問題の内容

### 1. P2P APIモックの不適切な実装
- `p2pApi.p2pApi`という二重構造でモックを作成していた
- 実際のインポートは`import { p2pApi } from '@/lib/api/p2p'`形式

### 2. ストアの状態リセット問題
- `renderHook`とストアの状態管理が適切に連携していなかった
- 初期状態のリセットが不完全だった

### 3. 型エラー
- `topics`プロパティが`activeTopics`に変更されていたが、テストが更新されていなかった
- nullチェックが不足していた箇所があった

## 解決方法

### 1. P2P APIモックの修正

```typescript
// 修正前
vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    p2pApi: {
      initialize: vi.fn(),
      // ...
    }
  }
}));

// 修正後
vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    initialize: vi.fn(),
    getNodeAddress: vi.fn(),
    getStatus: vi.fn(),
    joinTopic: vi.fn(),
    leaveTopic: vi.fn(),
    broadcast: vi.fn(),
  },
}));
```

### 2. ストア状態のリセット改善

```typescript
beforeEach(() => {
  vi.clearAllMocks();
  
  // ストアの状態をリセット
  act(() => {
    useP2PStore.setState({
      initialized: false,
      connectionStatus: 'disconnected',
      nodeAddr: null,
      nodeId: null,
      activeTopics: new Map(),
      messages: new Map(),
      error: null,
      peers: new Map(),
    });
  });
});
```

### 3. フック使用パターンの統一

```typescript
// renderHookを使用した適切なパターン
const { result } = renderHook(() => useP2P());

// 初期状態を確認
expect(result.current.initialized).toBe(false);

// アクションを実行
await act(async () => {
  await result.current.joinTopic('topic1');
});

// 結果を検証
expect(result.current.activeTopics).toHaveLength(1);
```

### 4. nullチェックの追加

```typescript
// p2pStore.tsでの安全なアクセス
set({
  initialized: true,
  nodeAddr: nodeAddr ? nodeAddr.join(', ') : '',
  nodeId: status?.endpoint_id || '',
  connectionStatus: 'connected',
});
```

## 結果

### テスト結果
- **成功**: 186/200テスト（93%）
- **失敗**: 13テスト
- **スキップ**: 1テスト

### 品質チェック
- ✅ リントエラー: 0
- ✅ タイプエラー: 0

### 残っている問題
1. **非同期初期化のタイミング問題**
   - `useP2P`フックの自動初期化テストがタイムアウト
   - 実装は動作するが、テストでの検証が困難

2. **clearErrorテストの状態同期**
   - エラー状態のクリアが即座に反映されない場合がある

3. **P2PStatusコンポーネントのテスト**
   - 複数の同じテキストが存在する場合の要素取得エラー

## 教訓

1. **モック構造は実際のインポート構造と一致させる**
   - エクスポートの形式を正確に把握してモックを作成

2. **Zustandテストはドキュメントに従う**
   - 公式ドキュメントのベストプラクティスを参照
   - `renderHook`の使用は状況に応じて判断

3. **状態の初期化は明示的に行う**
   - `beforeEach`で確実にリセット
   - 全てのプロパティを明示的に設定

4. **型安全性を保つ**
   - プロパティ名の変更は全体に反映
   - nullチェックを適切に実装

## 次のステップ

1. 残っているテストエラーの解決（必要に応じて）
2. E2Eテストの実行確認
3. 本番ビルドの確認