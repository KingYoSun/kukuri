# テストガイド

## テストの実行

```bash
# すべてのテストを実行
pnpm test

# テストをウォッチモードで実行
pnpm test --watch

# テストカバレッジを表示
pnpm test:coverage

# UIモードでテストを実行
pnpm test:ui
```

## テスト構成

### セットアップ

- **フレームワーク**: Vitest
- **テストユーティリティ**: React Testing Library
- **アサーション**: @testing-library/jest-dom
- **環境**: jsdom

### ディレクトリ構造

```
src/tests/
├── README.md
├── global.d.ts
├── integration/
│   ├── setup.ts
│   └── ui/                  # React + DI 統合テスト
│       └── *.integration.test.tsx
├── setup.ts                 # 共通セットアップ（vitest.config.ts から参照）
├── test-utils.tsx           # Testing Library 補助関数
└── unit/
    ├── components/
    ├── hooks/
    ├── lib/
    ├── pages/
    ├── routes/
    ├── services/
    └── stores/
```

### テストファイルの命名規則

- コンポーネントテスト: `ComponentName.test.tsx`
- ユーティリティテスト: `utilityName.test.ts`
- 統合テスト: `feature.integration.test.tsx`

## テストの書き方

### 基本的なコンポーネントテスト

```typescript
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MyComponent } from '../MyComponent'

describe('MyComponent', () => {
  it('正しくレンダリングされること', () => {
    render(<MyComponent />)
    expect(screen.getByText('期待するテキスト')).toBeInTheDocument()
  })
})
```

### ユーザーインタラクションのテスト

```typescript
import userEvent from '@testing-library/user-event'

it('ボタンクリックが動作すること', async () => {
  const user = userEvent.setup()
  render(<MyComponent />)

  await user.click(screen.getByRole('button'))
  expect(screen.getByText('クリック後のテキスト')).toBeInTheDocument()
})
```

## モック

### Tauri APIのモック

`src/tests/setup.ts`でTauri APIは自動的にモックされています。

### カスタムモックの追加

```typescript
vi.mock('@/services/api', () => ({
  fetchData: vi.fn().mockResolvedValue({ data: 'mocked' }),
}));
```

## デバッグ

### DOM構造の確認

```typescript
import { screen } from '@testing-library/react';

// DOM全体を表示
screen.debug();

// 特定の要素を表示
screen.debug(screen.getByRole('button'));
```

### テストの失敗時のデバッグ

1. `screen.debug()`を使用してDOM構造を確認
2. `console.log()`でデータを確認
3. Vitest UIモード（`pnpm test:ui`）で視覚的にデバッグ
