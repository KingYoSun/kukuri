# Docker環境でのテスト実行ガイド

**作成日**: 2025年08月05日
**最終更新**: 2025年10月22日

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
Windows では DLL 依存の問題を避けるため `scripts/test-docker.ps1` を既定のテスト経路とする。詳細な運用手順と CI との対応関係は `docs/03_implementation/windows_test_docker_runbook.md` を参照。以下は頻出コマンドの抜粋。

```powershell
.\scripts\test-docker.ps1             # すべてのテスト
.\scripts\test-docker.ps1 rust        # Rust テストのみ
.\scripts\test-docker.ps1 integration # P2P 統合テスト
.\scripts\test-docker.ps1 ts          # TypeScript テスト
.\scripts\test-docker.ps1 lint        # Lint / format チェック
.\scripts\test-docker.ps1 build       # テスト用イメージの再ビルド
.\scripts\test-docker.ps1 clean       # コンテナとネットワークの削除
.\scripts\test-docker.ps1 cache-clean # キャッシュ削除を含む完全クリーンアップ
```
`integration` は `ENABLE_P2P_INTEGRATION=1` を付与し `p2p-bootstrap` コンテナを自動起動する。P2P 経路の再実行時は `-NoBuild` を組み合わせると高速化できる。

#### Rustテスト自動化
Rust のみを Docker で実行したい場合は `scripts/run-rust-tests.ps1` を利用するとリポジトリルートの切り替えと戻り値の伝播を自動化できる。詳細オプションは上記 runbook を参照。

```powershell
.\scripts\run-rust-tests.ps1             # Rust ユニットテスト
.\scripts\run-rust-tests.ps1 -Integration   # P2P 統合テスト
.\scripts\run-rust-tests.ps1 -NoBuild       # ビルド済みキャッシュを利用
```

### Linux/macOS環境での実行

```bash
# スモークテスト（Rust P2P + TypeScript 統合のみ、Tauri 起動なし）
docker compose -f docker-compose.test.yml up --build --exit-code-from test-runner p2p-bootstrap test-runner

# フルスイート（従来どおりの全テスト）
docker compose -f docker-compose.test.yml run --rm test-runner /app/run-tests.sh

# 個別のサービスを実行
docker compose -f docker-compose.test.yml run --rm rust-test
docker compose -f docker-compose.test.yml run --rm ts-test
docker compose -f docker-compose.test.yml run --rm lint-check

# クリーンアップ
docker compose -f docker-compose.test.yml down --rmi local --volumes
```


P2P統合テスト用に追加された `p2p` サブコマンドでは次のオプションを組み合わせて利用できます。
- `--tests <name>`: `iroh_integration_tests`（既定）を含む Cargo テストターゲットを指定
- `--bootstrap <node_id@host:port>`: デフォルトの `p2p-bootstrap` 設定を上書きしたい場合に使用（複数ノードはカンマ区切り）
- `--no-build`: 事前ビルドをスキップ（イメージ変更がない反復実行向け）
- `--keep-env`: 生成された `kukuri-tauri/tests/.env.p2p` を削除せず残す
- `--rust-log <value>` / `--rust-backtrace <value>`: Rust 側のロギング設定を上書き
実行時に生成される `.env.p2p` は `kukuri-tauri/tests/` 配下に保存され、デフォルトで `KUKURI_BOOTSTRAP_PEERS=03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233` を含みます。`--keep-env` を指定しなければ完了後に自動削除されます。
`p2p` サブコマンドは PowerShell 版 `integration` と同様に `p2p-bootstrap` を自動で起動し、ヘルスチェックが `healthy` になるまで待機してからテストを実行します。既定では `cargo test --package kukuri-tauri --lib modules::p2p::tests::iroh_integration_tests:: -- --nocapture --test-threads=1` を実行し、P2P 結合テストのみに絞って検証します。

詳細な設計背景と検証手順は `docs/03_implementation/p2p_dht_test_strategy.md` を参照してください。

### docker-composeコマンドでの直接実行

```bash
# スモークテスト（Rust P2P + TypeScript 統合のみ、Tauri 起動なし）
docker compose -f docker-compose.test.yml up --build --exit-code-from test-runner p2p-bootstrap test-runner

# フルスイート（従来どおりの全テスト）
docker compose -f docker-compose.test.yml run --rm test-runner /app/run-tests.sh

# 個別のサービスを実行
docker compose -f docker-compose.test.yml run --rm rust-test
docker compose -f docker-compose.test.yml run --rm ts-test
docker compose -f docker-compose.test.yml run --rm lint-check

# クリーンアップ
docker compose -f docker-compose.test.yml down --rmi local --volumes
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
- `p2p-bootstrap`: iroh DHT ブートストラップノード（`kukuri-cli` を使用）
- `test-runner`: 既定では `run-smoke-tests.sh` で Rust P2P + TypeScript 統合スモークを実行。フルスイートは `/app/run-tests.sh` を明示的に指定して実行する。
- `rust-test`: Rustテストのみ実行
- `ts-test`: TypeScriptテストのみ実行
- `lint-check`: リントとフォーマットチェック

`p2p-bootstrap` は固定シークレットで決定論的な NodeId を生成し、11233/TCP と 6881/UDP を公開してテスト用 DHT ブートストラップとして常駐します。テストスクリプト側がヘルスチェックの完了を待ってから `rust-test` / `test-runner` のコンテナを起動し、`.env.p2p` や PowerShell の一時環境変数を通じて `KUKURI_BOOTSTRAP_PEERS` を参照します。

### 環境変数
- `RUST_BACKTRACE=1`: Rustのスタックトレースを有効化
- `RUST_LOG=debug`: Rust側のログレベルをデバッグに固定
- `NODE_ENV=test`: Node.jsのテスト環境を明示
- `CI=true`: CI環境であることをライブラリに通知
- `ENABLE_P2P_INTEGRATION` (既定値 `0`): P2P統合テスト向けのパスを有効化。通常の `./scripts/test-docker.ps1 rust` / `all` では無効化され、`./scripts/test-docker.ps1 integration` を実行した場合のみ `1` に上書きされる
- `KUKURI_FORCE_LOCALHOST_ADDRS` (推奨値 `0`): DHT 経由で得たピアアドレスをそのまま利用するためのフラグ。p2p-bootstrap を利用するテストではスクリプト側で自動的に 0 に上書きされる
- `KUKURI_BOOTSTRAP_PEERS`: `node_id@host:port` 形式。p2p-bootstrap 起動時に `03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233` が自動設定される
- `KUKURI_BOOTSTRAP_HOST`: ブートストラップに接続するホスト名（既定値 `127.0.0.1`）。
- `KUKURI_BOOTSTRAP_PORT`: ブートストラップの待ち受けポート（既定値 `11233`）。
- `BOOTSTRAP_WAIT_SECONDS`: スモークテスト開始前に待機する秒数（既定値 `10`）。
- `KUKURI_SECRET_KEY`: p2p-bootstrap コンテナが使用する Base64 エンコード済み 32バイト秘密鍵。既定値は `AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=`

## CI/CDでの活用

GitHub Actionsでの自動テスト実行が設定されています。Windows から同等の実行手順を踏む際は `docs/03_implementation/windows_test_docker_runbook.md` の CI 対応表を参照してください：

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
