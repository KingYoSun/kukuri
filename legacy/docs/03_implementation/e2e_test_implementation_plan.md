# WebdriverIO + tauri-driver を使用した E2E テスト導入プラン

## 概要
このドキュメントは、GitHub リポジトリ [KingYoSun/kukuri](https://github.com/KingYoSun/kukuri) の Tauri ベースのデスクトップアプリケーション (kukuri-tauri サブディレクトリ) に、WebdriverIO と tauri-driver を使用した E2E (End-to-End) テスト環境を導入するためのプランです。  
WebdriverIO は WebDriver プロトコルベースのテストフレームワークで、tauri-driver と組み合わせることで Tauri アプリの WebView を自動操作できます。これにより、React UI のインタラクション、Rust バックエンドとの連携 (Nostr/P2P 機能など) をエンドツーエンドで検証可能になります。  
導入のメリット:  
- Tauri 公式推奨のツールセットで、クロスプラットフォーム対応 (Windows/macOS/Linux)。  
- 既存のユニットテスト (pnpm test) と統合しやすく、CI/CD で自動化可能。  
- テスト範囲: UI ナビゲーション、投稿機能、P2P 同期、オフライン動作など。  

前提: リポジトリのルート (kukuri-tauri) で作業。package.json は [こちら](https://github.com/KingYoSun/kukuri/blob/main/kukuri-tauri/package.json) にあり、既存の devDependencies (例: vite, typescript) を基に拡張。E2E 関連の依存が未導入の場合、新規インストールします。

## 前提条件
- Node.js v18+ と pnpm がインストール済み (リポジトリの scripts から推測)。  
- Tauri CLI がグローバルインストール済み (`cargo install tauri-cli --version "^2.0.0-beta"` 推奨)。  
- Rust 環境がセットアップ済み (src-tauri で cargo test が実行可能)。  
- 開発環境で Tauri アプリがビルド/実行可能 (`pnpm tauri dev`)。  
- macOS の場合、WebDriver 制限に注意 (デスクトップクライアント未サポートのため、代替ブラウザテストを検討)。  

## ステップ 1: 依存関係のインストール
kukuri-tauri ディレクトリで実行。package.json に WebdriverIO 関連がなければ追加。  
```bash
pnpm add -D webdriverio @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter ts-node typescript
pnpm add -D tauri-driver
```
- **説明**:  
  - `webdriverio`, `@wdio/cli`: WebdriverIO コアと CLI。  
  - `@wdio/local-runner`: ローカル実行ランナー。  
  - `@wdio/mocha-framework`: Mocha をテストフレームワークとして使用 (TypeScript 対応)。Jasmine や Cucumber も代替可能。  
  - `@wdio/spec-reporter`: テストレポート出力。  
  - `ts-node`, `typescript`: TypeScript サポート (リポジトリが TS ベースのため)。  
  - `tauri-driver`: Tauri 専用 WebDriver (Tauri v2 対応)。  
インストール後、package.json の devDependencies に追加確認。scripts セクションに E2E コマンドを追加:  
```json
"scripts": {
  ... // 既存の scripts
  "e2e": "wdio run ./wdio.conf.ts",
  "e2e:build": "tauri build --debug && wdio run ./wdio.conf.ts" // ビルド後テスト用
}
```

## ステップ 2: 設定ファイルの作成
プロジェクトルート (kukuri-tauri) に以下のファイルを追加。

### wdio.conf.ts (TypeScript ベースの設定ファイル)
```typescript
import { join } from 'path';

export const config = {
  runner: 'local',
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tsconfig.json', // 既存の tsconfig.json を参照
      transpileOnly: true
    }
  },
  specs: ['./tests/e2e/**/*.ts'], // テストファイルの場所
  exclude: [],
  maxInstances: 1, // Tauri アプリは並行実行しにくいため 1 に制限
  capabilities: [{
    browserName: 'tauri',
    'tauri:options': {
      application: join(process.cwd(), 'src-tauri/target/debug/kukuri') // アプリのバイナリパス (アプリ名に合わせる)
    }
  }],
  logLevel: 'info',
  bail: 0,
  baseUrl: 'tauri://localhost',
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  services: ['tauri-driver'],
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000
  }
};
```
- **カスタマイズポイント**:  
  - `specs`: テストファイルを置くディレクトリ (後述)。  
  - `capabilities`: tauri-driver を指定。アプリのバイナリパスはビルド後の場所に合わせる (Windows: .exe, macOS: .app など)。  
  - Tauri v2 では `automation: true` を有効化 (次ステップ)。

### tsconfig.json の更新 (既存の場合)
E2E テスト用に拡張。types に `@wdio/types` を追加:
```json
{
  "compilerOptions": {
    ... // 既存
    "types": ["@wdio/types", "node"]
  }
}
```

## ステップ 3: tauri.conf.json の修正
src-tauri/tauri.conf.json を編集し、E2E テスト用の automation を有効化:
```json
{
  "productName": "kukuri",
  ... // 既存
  "tauri": {
    "allowlist": {
      ... // 既存
      "automation": true // これを追加 (Tauri v2 で WebDriver を有効化)
    }
  }
}
```
- これにより、tauri-driver がアプリの WebView にアクセス可能になります。  
- 変更後、`pnpm tauri dev` で確認。

## ステップ 4: テストスクリプトのサンプル作成
`tests/e2e/` ディレクトリを作成し、basic.spec.ts を追加:
```typescript
import { expect, browser } from '@wdio/globals';

describe('Kukuri E2E Tests', () => {
  it('should load the main page and check title', async () => {
    await browser.url('/'); // Tauri のベース URL
    const title = await browser.getTitle();
    expect(title).toEqual('Kukuri'); // アプリタイトルに合わせる
  });

  it('should test Nostr post functionality', async () => {
    // UI 要素を操作 (React コンポーネントのセレクタを使用)
    const input = await $('[data-testid="post-input"]'); // 例: 投稿入力フィールド
    await input.setValue('Test post via E2E');
    const button = await $('[data-testid="submit-button"]');
    await button.click();
    
    // バックエンド連携検証 (例: 投稿が表示されるか)
    const postElement = await $('[data-testid="posted-content"]');
    const text = await postElement.getText();
    expect(text).toContain('Test post via E2E');
  });

  // 追加テスト: P2P 同期、オフライン動作など
});
```
- **Tips**: セレクタは React コンポーネントに data-testid を追加して安定化。Rust コマンド (invoke) の結果を UI で検証。

## ステップ 5: テストの実行
1. Tauri アプリをデバッグビルド: `pnpm tauri dev` (または `tauri build --debug`)。  
2. 別のターミナルで tauri-driver 起動: `tauri-driver` (グローバルインストール推奨)。  
3. E2E テスト実行: `pnpm e2e`。  
- 成功すれば、コンソールにテスト結果出力。  
- デバッグ: logLevel を 'debug' に変更。

## ステップ 6: CI/CD 統合の提案
GitHub Actions を使用。`.github/workflows/e2e.yml` を追加:
```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    steps:
      - uses: actions/checkout@v4
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with: { node-version: 20 }
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Install pnpm
        run: npm install -g pnpm
      - name: Install dependencies
        run: pnpm install
      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        with:
          tagName: v__VERSION__
          releaseName: 'v__VERSION__'
      - name: Run E2E tests
        run: pnpm e2e
```
- tauri-action でクロスプラットフォームビルド。tauri-driver を CI で起動。

## 注意点
- **クロスプラットフォーム**: macOS で tauri-driver の制限がある場合、Playwright にフォールバック検討。  
- **セキュリティ**: automation: true は開発時のみ有効に。プロダクションでは false。  
- **拡張**: スクリーンショット追加 (`browser.saveScreenshot()`) や動画録画でデバッグ強化。  
- **トラブルシューティング**: Tauri ドキュメント (https://v2.tauri.app/plugin/webdriver/) を参照。エラー時は capabilities のパス確認。  
- **メンテナンス**: テストを定期更新し、React コンポーネント変更時にセレクタ調整。  

このプランを実施後、必要に応じてカスタマイズしてください。導入に関する質問があればお知らせください。
