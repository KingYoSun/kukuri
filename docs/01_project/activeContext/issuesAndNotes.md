# 既知の問題と注意事項

**最終更新**: 2025年7月26日

## 解決済みの問題

### zustand v5テストモックの問題（2025年7月26日）
**問題**: zustand v5では`create`関数がフック関数を返すが、テストのモック実装が古いバージョンを想定していた

**症状**: 
- `store.getState is not a function` エラー
- 10件のテストスイートが失敗

**解決策**: src/test/setup.tsでv5対応のモック実装を作成
```typescript
// zustandをモック - v5対応
vi.mock('zustand', async () => {
  const { create: _actualCreate } = await vi.importActual<typeof import('zustand')>('zustand')
  
  const createMockStore = (createState: any) => {
    // 状態管理APIの実装
    let state: any
    const setState = (partial: any, replace?: any) => {
      const nextState = typeof partial === 'function' ? partial(state) : partial
      if (replace ?? typeof partial !== 'object') {
        state = nextState
      } else {
        state = Object.assign({}, state, nextState)
      }
    }
    const getState = () => state
    const subscribe = () => () => {}
    const destroy = () => {}
    
    const api = { setState, getState, subscribe, destroy }
    state = createState(setState, getState, api)
    
    // フック関数として返す
    const useStore = Object.assign(
      (selector = (state: any) => state) => selector(state),
      api
    )
    
    // 初期状態を保存してリセット可能にする
    const initialState = { ...state }
    storeResetFns.add(() => {
      setState(initialState, true)
    })
    
    return useStore
  }
  
  // カリー化されたcreate関数をサポート
  const create = ((createState?: any) => {
    if (!createState) {
      return (createState: any) => createMockStore(createState)
    }
    return createMockStore(createState)
  }) as typeof _actualCreate
  
  return { create }
})
```

**参考**: https://zustand.docs.pmnd.rs/guides/testing

### Rust dead code警告（2025年7月26日）
**問題**: 開発初期段階で多くのモジュールが未使用のため、21件のdead code警告が発生

**解決策**: 
1. 未使用importは`cargo clippy --fix`で自動削除
2. 実装済みだが未使用のコードには`#[allow(dead_code)]`を追加
   - KeyManager構造体と関連impl
   - Database構造体と関連impl  
   - EventHandler、EventPublisher構造体
   - 暗号化関数（encrypt、decrypt、derive_key_from_password）

**今後の対応**: 実装が進むにつれて、これらのアノテーションは削除していく

## 現在の注意事項

### フロントエンド
- **ESLint設定**: src/test/setup.tsで`@typescript-eslint/no-explicit-any`を無効化
  - テストモック実装では型の厳密性よりも柔軟性を優先
- **zustandテスト**: v5対応のモック実装が必要
  - persistミドルウェアも別途モックが必要

### バックエンド
- **未使用コード**: 多くのモジュールに`#[allow(dead_code)]`が付与されている
  - 実装時に随時削除する必要がある
- **データベース接続**: 現在は初期化コードのみで、実際の接続処理は未実装

### 開発環境
- **formatコマンド**: package.jsonにformatスクリプトが定義されていない
  - 必要に応じて追加する

## 技術的な決定事項

### テスト戦略
1. **フロントエンドテスト**
   - Vitest + React Testing Library
   - 全コンポーネント、フック、ストアに対してテストを作成
   - カバレッジ目標は設定せず、重要な機能に集中

2. **バックエンドテスト**
   - Rust標準のテスト機能を使用
   - 各モジュールに対して単体テストを作成
   - 統合テストは今後追加予定

### コード品質
1. **リント設定**
   - フロントエンド: ESLint（TypeScript、React対応）
   - バックエンド: cargo clippy
   - 両方とも警告ゼロを維持

2. **型安全性**
   - TypeScript: strictモード有効
   - Rust: 全ての警告を解消（一時的な抑制を除く）