# Kukuri Integration Tests

このディレクトリには、Kukuriアプリケーションのインテグレーションテストとエンドツーエンド（E2E）テストが含まれています。

## テスト構成

### ユニットテスト
- **場所**: `src/**/__tests__/`, `src-tauri/src/**/*.rs`
- **実行**: `pnpm test` (フロントエンド), `cargo test` (Rust)
- **目的**: 個々のコンポーネントや関数の動作を検証

### インテグレーションテスト
- **場所**: `src/test/integration/`, `src-tauri/tests/integration/`
- **実行**: `pnpm test:integration`
- **目的**: 複数のコンポーネント間の連携を検証

### E2Eテスト
- **場所**: `tests/e2e/`
- **実行**: `pnpm test:e2e`
- **目的**: アプリケーション全体のユーザーフローを検証

## テストの実行

### すべてのテストを実行
```bash
# Linux/macOS
./scripts/run-integration-tests.sh --all

# Windows
.\scripts\run-integration-tests.ps1 --all
```

### 特定のテストのみ実行
```bash
# ユニットテストのみ
./scripts/run-integration-tests.sh --unit-only

# インテグレーションテストのみ
./scripts/run-integration-tests.sh --integration-only

# E2Eテストのみ
./scripts/run-integration-tests.sh --e2e-only

# リントをスキップ
./scripts/run-integration-tests.sh --no-lint
```

### 個別実行
```bash
# フロントエンドのユニットテスト
pnpm test

# フロントエンドのインテグレーションテスト
pnpm test:integration

# Rustのテスト
cd src-tauri && cargo test

# E2Eテスト（要ビルド）
pnpm tauri build --debug
pnpm test:e2e
```

## E2Eテストの準備

### 必要な依存関係
1. **tauri-driver**: WebDriverプロトコルを使用してTauriアプリをテスト
   ```bash
   cargo install tauri-driver
   ```

2. **WebDriver依存関係**: package.jsonに含まれています
   ```bash
   pnpm install
   ```

### Linux環境での実行
Linux環境では、ヘッドレスモードで実行するために`xvfb`が必要です：
```bash
sudo apt-get install xvfb
xvfb-run -a pnpm test:e2e
```

## CI/CD統合

GitHub Actionsでの自動テスト実行が設定されています：
- **ファイル**: `.github/workflows/integration-tests.yml`
- **トリガー**: プッシュ、プルリクエスト
- **環境**: Ubuntu, Windows, macOS

## テストの書き方

### インテグレーションテスト例
```typescript
// src/test/integration/auth.integration.test.ts
import { setupIntegrationTest, setMockResponse } from './setup'

describe('Auth Integration', () => {
  beforeEach(() => {
    setupIntegrationTest()
  })
  
  it('should authenticate user', async () => {
    setMockResponse('generate_keypair', {
      publicKey: 'npub1...',
      secretKey: 'nsec1...'
    })
    
    // テストロジック
  })
})
```

### E2Eテスト例
```typescript
// tests/e2e/specs/app.e2e.ts
import { AppHelper } from '../helpers/app'

describe('App E2E', () => {
  it('should launch successfully', async () => {
    await AppHelper.waitForAppReady()
    
    const title = await $('h1')
    expect(await title.getText()).toContain('Kukuri')
  })
})
```

## トラブルシューティング

### テストが失敗する場合

1. **依存関係の確認**
   ```bash
   pnpm install
   cd src-tauri && cargo build
   ```

2. **データベースのリセット**
   - テスト用の一時データベースが使用されますが、問題がある場合は手動でクリア

3. **ポートの競合**
   - tauri-driverのデフォルトポート(4444)が使用されていないか確認

4. **スクリーンショット**
   - E2Eテスト失敗時は`tests/e2e/screenshots/`にスクリーンショットが保存されます

### デバッグモード

環境変数を設定してデバッグ情報を表示：
```bash
RUST_LOG=debug pnpm test:e2e
E2E_SCREENSHOT=true pnpm test:e2e
```

## ベストプラクティス

1. **独立性**: 各テストは他のテストに依存しない
2. **再現性**: テストは何度実行しても同じ結果になる
3. **速度**: モックを活用して実行時間を短縮
4. **可読性**: テストケースは何をテストしているか明確に

## 貢献

新しいテストを追加する際は：
1. 適切なディレクトリに配置
2. 既存の命名規則に従う
3. setupヘルパーを活用
4. CIで実行されることを確認