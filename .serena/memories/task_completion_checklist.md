# タスク完了時のチェックリスト

## 必須実行コマンド

### 1. コード品質チェック
```bash
# TypeScript型チェック
pnpm type-check

# フロントエンドリント
pnpm lint

# Rustリント
cargo clippy

# フォーマット
pnpm format
cargo fmt
```

### 2. テスト実行
```bash
# フロントエンドテスト
pnpm test

# バックエンドテスト
cargo test
```

## ドキュメント更新

### 1. 進捗レポート
- 重要な機能実装後は`docs/01_project/progressReports/`に進捗レポートを作成
- ファイル名形式: `YYYY-MM-DD_<feature_name>.md`

### 2. current_tasks.md更新
- `docs/01_project/activeContext/current_tasks.md`を更新
- 完了したタスクを「完了済みタスク」セクションに移動
- 新しいタスクがあれば「現在進行中のタスク」に追加

### 3. TodoWriteツール更新
- TodoWriteツールを使用してタスクリストを更新
- 完了したタスクをcompletedにマーク

## 重要な確認事項

### 1. エラーハンドリング
- `console.error`を使用していないか確認
- `errorHandler`を適切に使用しているか

### 2. テストカバレッジ
- 新機能には必ずテストを追加
- 既存テストが壊れていないか確認

### 3. 型安全性
- TypeScriptの型エラーがないか
- any型を避けているか

### 4. セキュリティ
- 秘密鍵や機密情報をログに出力していないか
- セキュアストレージを適切に使用しているか

## Windows環境特有の注意事項

### パス区切り文字
- ファイルパスではバックスラッシュ（\）を使用
- import文では通常のスラッシュ（/）を使用

### 改行コード
- Gitの設定で自動変換されるが、.editorconfigで統一

### ビルド時の注意
- Windows Defenderが誤検知することがあるので、除外設定を推奨

## 日付の確認
ドキュメント作成・更新前に必ず今日の日付を確認：
```bash
# Windows
date /t

# PowerShell
Get-Date -Format "yyyy年MM月dd日"
```