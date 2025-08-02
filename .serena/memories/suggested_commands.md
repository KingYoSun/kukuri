# 推奨コマンド一覧

## 開発環境 (Windows)

### 基本的な開発コマンド
```bash
# 開発サーバー起動
pnpm tauri dev

# ビルド
pnpm tauri build

# ビルド(Windows)
pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc

# ビルド(Android)
pnpm tauri android build
```

### テスト実行
```bash
# フロントエンドテスト
pnpm test
pnpm test:ui          # UI付きテスト
pnpm test:coverage    # カバレッジ付き

# バックエンドテスト
cargo test

# E2Eテスト
pnpm test:e2e

# 統合テスト
pnpm test:integration
```

### コード品質チェック
```bash
# リント
pnpm lint
cargo clippy

# フォーマット
pnpm format
cargo fmt

# フォーマットチェック（CIで使用）
pnpm format:check

# 型チェック
pnpm type-check
```

### Windows用ユーティリティコマンド
```bash
# ディレクトリ一覧
dir

# ファイル内容表示
type filename.txt

# 現在の日付確認
date /t

# 環境変数確認
echo %PATH%

# PowerShellでの開発ツールインストール
powershell -ExecutionPolicy Bypass -File scripts\install-dev-tools.ps1
```

### Git操作
```bash
# ステータス確認
git status

# 差分確認
git diff

# コミット履歴
git log --oneline -10

# ブランチ一覧
git branch -a
```

### 依存関係管理
```bash
# 依存関係インストール
pnpm install

# 依存関係の更新確認
pnpm outdated

# Rustの依存関係更新
cargo update
```

### データベース操作
```bash
# マイグレーション実行
sqlx migrate run

# マイグレーション作成
sqlx migrate add <migration_name>
```

## 重要な注意事項
- **Windows環境**: PowerShellまたはコマンドプロンプトを使用
- **管理者権限**: 一部のコマンド（特にインストール時）は管理者権限が必要
- **パス区切り**: Windowsではバックスラッシュ（\）を使用