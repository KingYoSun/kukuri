# E2Eテスト環境セットアップガイド

## 更新日
2025年01月14日（実際のセットアップ経験を反映）

## 概要
このドキュメントは、Kukuri TauriアプリケーションのE2Eテスト環境のセットアップと実行方法を、実際の構築経験に基づいて説明します。

## 環境構築手順

### 1. 必要なツールのインストール

#### tauri-driverのインストール（Rustツール）
```bash
cargo install tauri-driver --locked
```
- インストール完了後、`C:\Users\{username}\.cargo\bin\tauri-driver.exe`に配置されます

#### Microsoft Edge Driverのセットアップ（Windows）
```bash
# msedgedriver-toolをインストール
cargo install --git https://github.com/chippers/msedgedriver-tool

# プロジェクトルートでEdge Driverをダウンロード
cd C:\Users\{username}\kukuri
msedgedriver-tool
```
- 実行すると、現在のWebView2バージョンに対応するmsedgedriver.exeが自動的にダウンロードされます
- プロジェクトルート（`C:\Users\{username}\kukuri\msedgedriver.exe`）に配置されます

### 2. Node.js依存関係のインストール

プロジェクトディレクトリ（kukuri-tauri）で実行：
```bash
cd kukuri-tauri
pnpm add -D webdriverio @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter ts-node @wdio/types
```

package.jsonのscriptsに以下を追加：
```json
{
  "scripts": {
    "e2e": "wdio run ./wdio.conf.ts",
    "e2e:build": "pnpm tauri build --debug && wdio run ./wdio.conf.ts"
  }
}
```

### 3. ビルド前の準備

#### TypeScriptエラーの修正
デバッグビルド前に、TypeScriptの型エラーを確認・修正する必要があります：
```bash
# 型チェック
npx tsc --noEmit

# エラーがある場合は修正
```

#### デバッグビルドの作成
```bash
pnpm tauri build --debug
```
- ビルドには数分かかります
- 成功すると`src-tauri/target/debug/kukuri-tauri.exe`が作成されます

## 設定ファイル

### wdio.conf.ts
- **場所**: `kukuri-tauri/wdio.conf.ts`
- **概要**: WebdriverIOの設定ファイル
- **重要な設定**:
  - host: `127.0.0.1`
  - port: `4445` (実際のtauri-driver起動ポート)
  - capabilities: `tauri:options`でアプリケーションパスを指定
  
```typescript
export const config: Options.Testrunner = {
  host: '127.0.0.1',
  port: 4445,  // 重要: 4444ではなく4445
  capabilities: [{
    'tauri:options': {
      application: join(process.cwd(), 'src-tauri/target/debug/kukuri-tauri.exe')
    }
  }],
  // ...
}
```

### tauri.conf.json
- **場所**: `kukuri-tauri/src-tauri/tauri.conf.json`
- **必要な変更**: Tauri v2では特別な設定は不要（`webdriver: true`は不要）

### Cargo.toml
- **場所**: `kukuri-tauri/src-tauri/Cargo.toml`
- **必要な変更**: `tauri = { version = "2", features = ["test"] }`を追加

## テストの実行

### 推奨方法: 手動実行（最も安定）
1. **tauri-driverを起動** (ターミナル1):
   ```bash
   # プロジェクトルートから実行
   tauri-driver --native-driver "C:\\Users\\{username}\\kukuri\\msedgedriver.exe"
   ```
   - 起動成功時のメッセージ: `msedgedriver was started successfully on port 4445`

2. **別のターミナルでテストを実行** (ターミナル2):
   ```bash
   cd kukuri-tauri
   npm run e2e
   ```

### 方法2: PowerShellスクリプト（Windows）
```bash
cd kukuri-tauri
powershell -ExecutionPolicy Bypass -File run-e2e.ps1
```

### 方法3: npmスクリプト（自動起動、不安定な場合あり）
```bash
npm run e2e
```

## テストファイルの構造

```
kukuri-tauri/
├── tests/
│   └── e2e/
│       ├── basic.spec.ts      # 基本的な起動テスト
│       └── nostr.spec.ts      # Nostr機能のテスト
├── wdio.conf.ts              # WebdriverIO設定
└── run-e2e.ps1               # Windows用実行スクリプト
```

## トラブルシューティング

### エラー: "msedgedriver.exe not found"
**原因**: tauri-driverがmsedgedriver.exeを見つけられない
**解決方法**:
1. `msedgedriver-tool`を実行してドライバーをダウンロード
2. tauri-driver起動時にパスを明示的に指定:
   ```bash
   tauri-driver --native-driver "C:\\Users\\{username}\\kukuri\\msedgedriver.exe"
   ```
   - 注意: Windowsではパスをダブルクォートで囲み、バックスラッシュをエスケープ

### エラー: "Connection refused on port 4444/4445"
**原因**: tauri-driverが起動していない、または別のポートで起動している
**解決方法**:
1. tauri-driverが起動していることを確認
2. ポート番号を確認（デフォルトは4445）
3. wdio.conf.tsのポート設定を一致させる

### エラー: "Invalid URL: /"
**原因**: Tauriアプリケーション内でのURL指定方法の問題
**解決方法**:
- テスト内で`browser.url('/')`を使用しない
- Tauriアプリケーションは起動時に自動的に開くため、初期URLの指定は不要

### エラー: TypeScriptビルドエラー
**原因**: 型の不一致やインポートエラー
**解決方法**:
1. `npx tsc --noEmit`で型エラーを確認
2. エラーを修正してから再度ビルド

### エラー: "DevToolsActivePort file doesn't exist"
**原因**: ChromeDriverとアプリケーションの接続問題
**解決方法**:
1. デバッグビルドが最新であることを確認
2. `pnpm tauri build --debug`で再ビルド
3. tauri-driverを再起動

## 実際のテスト実行結果例

```
Spec Files:  1 passed, 1 failed, 2 total (100% completed) in 00:00:34

basic.spec.ts:
✓ should load the application
✗ should display the main application container (#rootが見つからない)
✓ should check page title (タイトル: "Kukuri")
✗ should handle app elements

nostr.spec.ts:
✓ should handle post creation flow
✓ should display timeline content
✓ should handle topic navigation
✓ should handle P2P sync indicator
```

## 既知の制限事項

1. **プラットフォーム**: Windows/Linuxのみサポート（macOSは未対応）
2. **WebDriver**: Microsoft Edge WebView2が必要（Windows）
3. **並行実行**: maxInstancesは1に制限（Tauriアプリの特性上）
4. **ポート番号**: tauri-driverは4445ポートで起動（4444ではない）

## テスト作成時の注意点

### URLナビゲーション
- `browser.url('/')`は使用しない（Invalid URLエラーになる）
- Tauriアプリケーションは起動時に自動的にページが開く
- ページ遷移はアプリ内のボタンクリックなどで実現

### 要素の待機
- アプリケーションの起動に時間がかかる場合がある
- 要素が存在しない場合は`isExisting()`でチェック
- `data-testid`属性を追加して要素を特定しやすくする

## 今後の改善点

1. **自動起動の安定化**: tauri-driverの自動起動をより安定させる
2. **CI/CD統合**: GitHub Actionsでの自動テスト実行
3. **テストケースの充実**: 
   - ユーザー認証フロー
   - 投稿作成・編集・削除
   - P2P同期の検証
4. **待機処理の最適化**: 要素の出現を適切に待つロジック
5. **data-testid属性の追加**: アプリケーション側に識別用属性を追加

## 参考リンク

- [Tauri v2 WebDriver Documentation](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO Documentation](https://webdriver.io/)
- [Tauri WebDriver Example](https://github.com/tauri-apps/webdriver-example)
- [WebdriverIO + Tauri v2 Example](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)