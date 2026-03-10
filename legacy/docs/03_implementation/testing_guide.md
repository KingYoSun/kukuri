# Kukuri テストガイド

## 概要

このドキュメントでは、Kukuriプロジェクトのテスト戦略、実装方法、実行手順について説明します。

> 補足: `kukuri-community-node` のテスト実行は Linux/macOS/Windows すべてでコンテナ経路を既定とします（`docker compose -f docker-compose.test.yml up -d community-node-postgres` + `docker compose -f docker-compose.test.yml build test-runner` + `docker run ... kukuri-test-runner ... cargo test --workspace --all-features`）。

## テスト構成

### 1. フロントエンドテスト

#### 単体テスト（Vitest + React Testing Library）

**場所**: `src/**/*.test.tsx`, `src/**/*.test.ts`

**実行方法**:
```bash
pnpm test
```

**主なテスト対象**:
- Reactコンポーネント
- カスタムフック
- Zustandストア
- ユーティリティ関数

#### 統合テスト

**場所**: `src/test/integration/*.integration.test.tsx`

**テスト内容**:
- 認証フロー全体の動作
- トピック管理機能の統合
- 投稿管理機能の統合
- Nostr機能との統合

### 2. バックエンドテスト（Rust）

**場所**: 各モジュール内の`#[cfg(test)]`ブロック

**実行方法**:
```bash
cargo test
```

**主なテスト対象**:
- 鍵管理モジュール（KeyManager）
- 暗号化モジュール（EncryptionManager）
- データベース接続
- Nostrイベント処理
- Tauriコマンド

### 3. E2Eテスト（WebDriverIO）

**場所**: `tests/e2e/specs/*.e2e.ts`

**実行方法**:
```bash
# Tauriアプリをビルド
pnpm tauri build

# E2Eテスト実行
pnpm test:e2e
```

**前提条件**:
- tauri-driverのインストール: `cargo install tauri-driver`

## テスト実装のベストプラクティス

### 1. コンポーネントテスト

```typescript
import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'

describe('MyComponent', () => {
  it('should handle user interaction', async () => {
    // Arrange
    const mockHandler = vi.fn()
    render(<MyComponent onClick={mockHandler} />)
    
    // Act
    const button = screen.getByRole('button')
    fireEvent.click(button)
    
    // Assert
    expect(mockHandler).toHaveBeenCalledOnce()
  })
})
```

### 2. Zustandストアテスト

```typescript
import { renderHook, act } from '@testing-library/react'
import { useMyStore } from '@/stores/myStore'

describe('myStore', () => {
  it('should update state correctly', () => {
    const { result } = renderHook(() => useMyStore())
    
    act(() => {
      result.current.updateValue('new value')
    })
    
    expect(result.current.value).toBe('new value')
  })
})
```

### 3. Tauriコマンドのモック

```typescript
// src/test/setup.ts
vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: vi.fn((cmd, args) => {
    switch (cmd) {
      case 'get_topics':
        return Promise.resolve([
          { id: 1, name: 'rust', description: 'Rust programming' }
        ])
      default:
        return Promise.resolve(null)
    }
  })
}))
```

### 4. E2Eテストパターン

```typescript
describe('Feature E2E', () => {
  it('should complete user flow', async () => {
    // ページ遷移
    await browser.url('/topics')
    
    // 要素の待機
    const element = await $('#my-element')
    await element.waitForDisplayed({ timeout: 5000 })
    
    // ユーザー操作
    await element.click()
    
    // 結果の検証
    const result = await $('#result')
    expect(await result.getText()).toBe('Expected Result')
  })
})
```

## CI/CD統合

### GitHub Actions設定

`.github/workflows/integration-tests.yml`でマルチプラットフォームテストを実行：

1. **並列実行**: Ubuntu、Windows、macOSで同時実行
2. **キャッシュ**: 依存関係のキャッシュで高速化
3. **アーティファクト**: テスト結果の保存

## テストコマンド一覧

```bash
# フロントエンドテスト
pnpm test              # 単体テスト実行
pnpm test:watch        # ウォッチモード
pnpm test:coverage     # カバレッジレポート生成

# 型チェック
pnpm type-check        # TypeScript型チェック

# リント
pnpm lint              # ESLintチェック

# Rustテスト
cargo test             # 全テスト実行
cargo test -- --nocapture  # 出力を表示
cargo clippy           # Rustリントチェック

# Community node（全OS既定: コンテナ）
docker compose -f docker-compose.test.yml up -d community-node-postgres
docker compose -f docker-compose.test.yml build test-runner
docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"

# E2Eテスト
pnpm test:e2e          # E2Eテスト実行
```

## Windows環境でのテスト実行（WSLなし）

### 重要な注意事項

Windows環境（WSLを使用しない場合）では、`pnpm`コマンド実行時にBashエラーが発生する場合があります。この場合は`npm run`を使用してください。

```bash
# pnpmでエラーが出る場合
npm run test           # pnpm test の代わりに
npm run lint           # pnpm lint の代わりに
npm run type-check     # pnpm type-check の代わりに
```

### Windows環境での推奨コマンド

```bash
# テスト実行
npm run test

# 型チェック
npx tsc --noEmit

# リント実行
npm run lint

# ビルド（Windows向け）
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc
```

### Windows環境特有の問題と対処法

1. **Bashパスエラー**
   - エラー例: `/usr/bin/bash: Files\Git\bin\bash.exe: No such file or directory`
   - 対処法: `pnpm`の代わりに`npm run`を使用

2. **パス区切り文字**
   - Windows環境では`\`が使用されるが、テストでは`/`を期待する場合がある
   - 必要に応じてパスを正規化

## トラブルシューティング

### よくある問題

1. **ResizeObserverエラー**
   - `src/test/setup.ts`でグローバルモックを設定

2. **Zustand persist middleware**
   - テスト環境では自動的に無効化される設定

3. **Tauri API呼び出し**
   - モックを使用してテスト環境で動作

4. **E2Eテストのタイムアウト**
   - `waitForDisplayed`のタイムアウトを調整

5. **Windows環境でのpnpmエラー**
   - `npm run`を使用するか、WSL環境を使用

## テストカバレッジ目標

- 単体テスト: 80%以上
- 統合テスト: 主要フローを100%カバー
- E2Eテスト: クリティカルパスを100%カバー

## 継続的な改善

1. **新機能追加時**
   - 必ず対応するテストを作成
   - 既存テストへの影響を確認

2. **バグ修正時**
   - バグを再現するテストを先に作成
   - 修正後にテストが通ることを確認

3. **リファクタリング時**
   - テストが変わらず通ることを確認
   - 必要に応じてテストも改善
