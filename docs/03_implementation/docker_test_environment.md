# Docker環境でのテスト実行ガイド

**作成日**: 2025年8月5日

## 概要

Windows環境でのDLL不足によるテスト実行エラーを解決するため、Docker環境でのテスト実行環境を構築しました。この環境により、すべてのプラットフォームで一貫したテスト実行が可能になります。

## 背景

### 問題
Windows環境でRustのテストを実行すると以下のエラーが発生：
```
error: test failed, to rerun pass `--lib`
Caused by:
  process didn't exit successfully: exit code: 0xc0000139, STATUS_ENTRYPOINT_NOT_FOUND
```

このエラーは、Windows環境でのDLL依存関係の問題により発生しており、コード自体には問題がありません。

### 解決策
Dockerコンテナ内でLinux環境を使用してテストを実行することで、この問題を回避します。

## ファイル構成

```
kukuri/
├── Dockerfile.test              # テスト環境用Dockerfile
├── docker-compose.test.yml      # Docker Compose設定
├── scripts/
│   ├── test-docker.sh          # Linux/macOS用テスト実行スクリプト
│   └── test-docker.ps1         # Windows PowerShell用テスト実行スクリプト
└── .github/workflows/test.yml   # GitHub Actions CI設定
```

## 使用方法

### 前提条件
- Docker Desktop がインストールされていること
- Docker が起動していること

### 重要：Windows環境での推奨事項
Windows環境でDLLエラーによりネイティブでのテストが実行できない場合は、**必ずDockerを使用してテストを実行してください**。

### Windows環境での実行

PowerShellで以下のコマンドを実行：

```powershell
# すべてのテストを実行
.\scripts\test-docker.ps1

# Rustテストのみ実行
.\scripts\test-docker.ps1 rust

# TypeScriptテストのみ実行
.\scripts\test-docker.ps1 ts

# リントとフォーマットチェック
.\scripts\test-docker.ps1 lint

# ヘルプを表示
.\scripts\test-docker.ps1 -Help
```

### Linux/macOS環境での実行

```bash
# 実行権限を付与（初回のみ）
chmod +x scripts/test-docker.sh

# すべてのテストを実行
./scripts/test-docker.sh

# Rustテストのみ実行
./scripts/test-docker.sh rust

# TypeScriptテストのみ実行
./scripts/test-docker.sh ts

# リントとフォーマットチェック
./scripts/test-docker.sh lint

# ヘルプを表示
./scripts/test-docker.sh -h
```

### docker-composeコマンドでの直接実行

```bash
# すべてのテストを実行
docker-compose -f docker-compose.test.yml run --rm test-runner

# 個別のサービスを実行
docker-compose -f docker-compose.test.yml run --rm rust-test
docker-compose -f docker-compose.test.yml run --rm ts-test
docker-compose -f docker-compose.test.yml run --rm lint-check

# クリーンアップ
docker-compose -f docker-compose.test.yml down --rmi local --volumes
```

## Docker環境の詳細

### Dockerfile.test
- ベースイメージ: `rust:1.85-bookworm` （edition2024のサポートのため）
- Node.js 20.x と pnpm 9 をインストール
- Tauri開発に必要なシステムパッケージをすべて含む
- 依存関係のキャッシュを最適化
- pnpmワークスペースは`--ignore-workspace`オプションで無視

### docker-compose.test.yml
以下のサービスを定義：
- `test-runner`: すべてのテストを実行
- `rust-test`: Rustテストのみ実行
- `ts-test`: TypeScriptテストのみ実行
- `lint-check`: リントとフォーマットチェック

### 環境変数
- `RUST_BACKTRACE=1`: Rustのスタックトレースを有効化
- `RUST_LOG=debug`: デバッグログを出力
- `NODE_ENV=test`: Node.jsテスト環境
- `CI=true`: CI環境として実行

## CI/CDでの活用

GitHub Actionsでの自動テスト実行が設定されています：

1. **docker-test**: Dockerを使用したメインのテストスイート
2. **native-test-linux**: Linux環境でのネイティブテスト
3. **format-check**: フォーマットチェック
4. **build-test-windows**: Windows環境でのビルドチェック（テストは実行しない）

### ワークフローの実行タイミング
- mainまたはdevelopブランチへのpush時
- Pull Request作成時
- 手動実行（workflow_dispatch）

## トラブルシューティング

### Docker Desktopが起動していない
```
Error: Docker daemon is not running
```
→ Docker Desktopを起動してください

### ポート競合
```
Error: bind: address already in use
```
→ 他のDockerコンテナが実行中の可能性があります。`docker ps`で確認してください

### ビルドが遅い
初回ビルドには時間がかかりますが、2回目以降はキャッシュが効くため高速化されます。

### Windows環境でのスクリプト実行エラー
```
スクリプトの実行がシステムで無効になっています
```
→ PowerShellの実行ポリシーを変更：
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### PowerShellスクリプトの構文エラー
```
式またはステートメントのトークン '}' を使用できません。
```
→ ファイルがBOM付きUTF-8で保存されていることを確認してください

### Dockerビルド時のシンボリックリンクエラー
```
failed to solve: invalid file request kukuri-tauri/node_modules/.pnpm/@ampproject+remapping@2.3.0/node_modules/@jridgewell/trace-mapping
```
→ `.dockerignore`ファイルで`**/node_modules`を除外

### pnpmワークスペースエラー
```
ERROR  packages field missing or empty
```
→ `pnpm install`に`--ignore-workspace`オプションを追加

### Rust edition2024エラー
```
error: failed to parse manifest
feature `edition2024` is required
```
→ Rust 1.85以上を使用（Dockerfileで`FROM rust:1.85-bookworm`を指定）

## メリット

1. **環境依存の解消**: すべての開発者が同じ環境でテストを実行
2. **CI/CDとの一貫性**: ローカルとCIで同じ環境を使用
3. **セットアップの簡素化**: Dockerのみインストールすれば実行可能
4. **並列実行**: 複数のテストスイートを並列で実行可能
5. **クリーンな環境**: テストごとに新しいコンテナで実行

## 今後の改善案

1. **テスト結果のレポート生成**: JUnitフォーマットでの出力
2. **カバレッジレポート**: テストカバレッジの計測と可視化
3. **パフォーマンステスト**: ベンチマークテストの追加
4. **マルチプラットフォームビルド**: ARM64対応など

## 関連ドキュメント

- [現在のタスク状況](../01_project/activeContext/current_tasks.md)
- [既知の問題と注意事項](../01_project/activeContext/issuesAndNotes.md)