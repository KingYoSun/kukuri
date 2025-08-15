# E2Eテスト実行ガイド

## 前提条件

1. **tauri-driverのインストール**
```bash
cargo install tauri-driver
```

2. **アプリケーションのビルド**

リリースビルドを使用する場合：
```bash
cd kukuri-tauri
pnpm tauri build
```

デバッグビルドを使用する場合：
```bash
cd kukuri-tauri
pnpm tauri build --debug
```

## E2Eテストの実行

### 基本的な実行方法
```bash
# kukuri-tauriディレクトリで実行
pnpm e2e
```

### デバッグモードで実行
```bash
# より詳細なログを出力（Windows）
set E2E_DEBUG=true && pnpm e2e

# より詳細なログを出力（Unix/Mac/WSL）
E2E_DEBUG=true pnpm e2e
```

### 通常モード（ログ最小限）
```bash
# ログスパムを抑制した静かな実行
pnpm e2e
```

### 特定のテストファイルのみ実行
```bash
# basic.spec.tsのみ実行
pnpm wdio run ./tests/e2e/wdio.conf.ts --spec ./tests/e2e/basic.spec.ts
```

## トラブルシューティング

### エラー: "#root element not found"

原因と対策：
1. **アプリケーションが起動していない**
   - ビルドが正しく完成しているか確認
   - `kukuri-tauri/src-tauri/target/release/kukuri-tauri.exe`が存在するか確認

2. **tauri-driverが起動していない**
   - コマンドラインで`tauri-driver --version`を実行して確認
   - ポート4445が他のプロセスで使用されていないか確認

3. **アプリケーションの起動に時間がかかる**
   - `tests/e2e/helpers/debug.ts`のタイムアウトを増やす

### エラー: "No window handles found"

原因と対策：
1. **WebDriverがアプリケーションに接続できていない**
   - tauri.conf.jsonの設定を確認
   - Cargo.tomlに`test`フィーチャーが有効になっているか確認

### Windows環境での注意事項

1. **ファイアウォール**
   - Windows Defenderファイアウォールがtauri-driverやアプリケーションをブロックしていないか確認

2. **管理者権限**
   - 必要に応じて管理者権限でコマンドプロンプトを実行

## ログレベル設定

### WebDriverIOログレベル
`wdio.conf.ts`の`logLevel`設定：
- `error`: エラーのみ表示（デフォルト）
- `warn`: 警告とエラー
- `info`: 情報、警告、エラー
- `debug`: すべてのログ

### カスタムデバッグログ
環境変数で制御：
- `E2E_DEBUG=true`: デバッグヘルパーのログを有効化
- `VERBOSE=true`: 詳細ログを有効化

### ログスパム対策
以下の対策により、不要なログ出力を抑制しています：

1. **WebDriverIOログレベル**: `error`に設定
2. **tauri-driverログ**: 重要なメッセージのみ出力
3. **テストログ**: console.logをコメントアウト
4. **デバッグヘルパー**: 環境変数による制御

## デバッグ方法

### ブラウザ状態の確認
`tests/e2e/helpers/debug.ts`の`logBrowserState()`関数を使用して、現在のブラウザ状態を確認できます。

### スクリーンショットの取得
テスト失敗時に自動的にスクリーンショットが保存されます：
- 保存先: `tests/e2e/screenshots/`

### 手動でtauri-driverを起動
```bash
# 別のターミナルで実行
tauri-driver --port 4445

# アプリケーションを手動で起動してテスト
cd kukuri-tauri/src-tauri/target/release
./kukuri-tauri.exe
```

## CI/CD環境での実行

GitHub Actionsなどで実行する場合：
1. ヘッドレスモードの設定が必要
2. Xvfb（Linux）やWindows Server環境の考慮が必要