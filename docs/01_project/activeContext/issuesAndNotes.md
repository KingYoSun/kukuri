# 既知の問題と注意事項

**最終更新**: 2025年7月26日

## 解決済みの問題

### nostr-sdk v0.42 API変更（2025年7月26日）
**問題**: nostr-sdk v0.42でEventBuilderのAPIが変更され、多くのメソッドが破壊的変更を受けた

**症状**:
- EventBuilder::text_note()の第2引数（空配列）が不要になった
- Eventのフィールドがメソッドからフィールドアクセスに変更
- 52件のRustテストが全てコンパイルエラー

**解決策**:
1. EventBuilder APIの更新
```rust
// 変更前
EventBuilder::text_note("Test message", [])
EventBuilder::metadata(&metadata)

// 変更後
EventBuilder::text_note("Test message")
EventBuilder::metadata(metadata)
```

2. フィールドアクセスへの変更
```rust
// 変更前
event.kind()
event.author()
event.content()

// 変更後
event.kind
event.author
event.content
```

**影響範囲**: event/handler.rs、event/publisher.rs、event/manager.rsの全テスト

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

### テスト関連
- **テストカバレッジ**: 合計158件（Rust 52件、TypeScript 106件）のテストを実装
- **act警告**: 一部のReactコンポーネントテストでact警告が発生する場合がある
  - 主に非同期state更新時に発生
  - 実害はないが、将来的に対応が必要
- **any型の使用**: テストファイル内で35箇所のany型警告
  - モックの柔軟性のため意図的に使用
  - production codeでは使用していない

### フロントエンド
- **ESLint設定**: src/test/setup.tsで`@typescript-eslint/no-explicit-any`を無効化
  - テストモック実装では型の厳密性よりも柔軟性を優先
- **zustandテスト**: v5対応のモック実装が必要
  - persistミドルウェアも別途モックが必要

### バックエンド
- **未使用コード**: 多くのモジュールに`#[allow(dead_code)]`が付与されている
  - 実装時に随時削除する必要がある
- **データベース接続**: 現在は初期化コードのみで、実際の接続処理は未実装
- **Rustリント警告**: clippy実行時に15件の警告
  - 未使用メソッド: add_callback、verify_event、create_repost等
  - 未使用フィールド: db_pool、encryption_manager
  - format!マクロでの変数展開（uninlined_format_args）
  - 複雑な型定義（type_complexity）
  - いずれも実装進行に伴い解消予定

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