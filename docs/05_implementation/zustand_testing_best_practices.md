# Zustandテストのベストプラクティス

*最終更新日: 2025年07月27日*

## 概要

本ドキュメントは、kukuriプロジェクトにおけるZustandストアのテスト実装に関するベストプラクティスをまとめたものです。

## 重要な原則

### 1. テスト実装方法

#### ❌ 避けるべきパターン
```typescript
import { renderHook } from '@testing-library/react'

// renderHookを使用したパターンは避ける
const { result } = renderHook(() => useStore())
result.current.action()
```

#### ✅ 推奨パターン
```typescript
// getState()を直接使用する
useStore.getState().action()
```

**理由**: プロジェクトの既存のセットアップ（`src/test/setup.ts`）が適切なモックを提供しているため、`renderHook`は不要です。

### 2. セットアップと初期化

#### beforeEachでの初期化
```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { act } from '@testing-library/react'
import { useStore } from '../store'

describe('Store Tests', () => {
  beforeEach(() => {
    // モックをリセット
    vi.clearAllMocks()
    
    // ストアの状態をリセット
    act(() => {
      useStore.setState({
        // 初期状態を明示的に設定
        initialized: false,
        data: null,
        error: null,
        // ...その他の初期状態
      })
    })
  })
})
```

### 3. APIモックの実装

```typescript
// APIモジュールのモック
vi.mock('@/lib/api/module', () => ({
  apiModule: {
    method1: vi.fn(),
    method2: vi.fn(),
    // ...
  }
}))

// モック関数の取得
import { apiModule } from '@/lib/api/module'
```

### 4. 非同期処理のテスト

```typescript
it('非同期アクションをテストする', async () => {
  // モックの設定
  vi.mocked(apiModule.fetchData).mockResolvedValueOnce(mockData)
  
  // アクションの実行（必ずactでラップ）
  await act(async () => {
    await useStore.getState().fetchData()
  })
  
  // 状態の検証
  expect(useStore.getState().data).toEqual(mockData)
})
```

### 5. エラーハンドリングのテスト

```typescript
it('エラーを適切に処理する', async () => {
  const mockError = new Error('API Error')
  vi.mocked(apiModule.fetchData).mockRejectedValueOnce(mockError)
  
  await act(async () => {
    await useStore.getState().fetchData()
  })
  
  // エラーメッセージは実装と完全に一致させる
  expect(useStore.getState().error).toBe('API Error')
})
```

### 6. 状態更新のテスト

```typescript
it('状態を更新できる', () => {
  const newData = { id: 1, name: 'Test' }
  
  act(() => {
    useStore.getState().updateData(newData)
  })
  
  expect(useStore.getState().data).toEqual(newData)
})
```

### 7. Mapやセットを含む状態のテスト

```typescript
it('Mapの状態を更新できる', () => {
  act(() => {
    useStore.getState().addItem('key1', { value: 'test' })
  })
  
  const items = useStore.getState().items // Map
  expect(items.get('key1')).toEqual({ value: 'test' })
  expect(items.size).toBe(1)
})
```

## 注意事項

### 1. Zustandのモックについて
- `vi.mock('zustand')`は不要です
- `src/test/setup.ts`が既に適切なモックを提供しています
- カスタムモックを作成する場合は、既存のセットアップと競合しないよう注意してください

### 2. actの使用
- 状態を更新する操作は必ず`act()`でラップしてください
- 非同期操作の場合は`await act(async () => { ... })`を使用してください

### 3. エラーメッセージの一致
- テストで期待するエラーメッセージは、実装のエラーメッセージと完全に一致させてください
- エラーメッセージが変更された場合は、テストも更新する必要があります

### 4. テストの独立性
- 各テストは独立して実行できるようにしてください
- `beforeEach`で状態を適切にリセットしてください
- テスト間で状態が共有されないようにしてください

## 実装例

完全な実装例については、以下のファイルを参照してください：
- `src/stores/__tests__/p2pStore.test.ts`
- `src/stores/__tests__/authStore.test.ts`

## トラブルシューティング

### 問題: renderHookが期待通りに動作しない
**解決策**: `renderHook`の使用を避け、`useStore.getState()`を直接使用してください。

### 問題: 状態がテスト間で共有される
**解決策**: `beforeEach`で確実に状態をリセットしてください。

### 問題: 非同期テストがタイムアウトする
**解決策**: 
1. `act`で適切にラップされているか確認
2. モックが正しく設定されているか確認
3. Promiseが適切に解決/拒否されているか確認

## 関連ドキュメント
- [Zustand公式テストガイド](https://zustand.docs.pmnd.rs/guides/testing)
- [Vitest公式ドキュメント](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/docs/react-testing-library/intro/)