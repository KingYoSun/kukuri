# Windows向け Docker テスト運用ガイド（ドラフト）

作成日: 2025年10月20日

## 目的と範囲
`./scripts/test-docker.ps1` を Windows 開発環境で日常的に運用するための手順と注意事項をまとめたドラフトです。DLL 依存によるネイティブテスト失敗を回避しつつ、CI（GitHub Actions）のテスト構成との差分を明確化します。

## 前提条件
- Docker Desktop 4.33 以降がインストールされ、バックグラウンドで起動していること。
- PowerShell 7 以上（Windows PowerShell 5.1 でも動作しますが、エラーハンドリングとカラー出力が限定されます）。
- スクリプト実行ポリシー: `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser` を設定済み。
- リポジトリ直下（`kukuri/`）でコマンドを実行すること。

## 基本フロー
1. **初回のみ**: `.\scripts\test-docker.ps1 build` を実行し、テスト用イメージをビルド（約10〜15分）。
2. **通常運用**: `.\scripts\test-docker.ps1` を実行して全テスト（Rust/TS/統合）を Docker 上で実行。
3. **結果確認**: 標準出力と `test-results/` ディレクトリ（JUnit 互換ログ等）を確認。
4. **終了時**: 必要に応じて `.\scripts\test-docker.ps1 clean`（コンテナのみ削除）または `cache-clean`（キャッシュごと削除）で片付け。

## コマンドマトリクス
| コマンド | 内容 | 主な利用シナリオ |
| --- | --- | --- |
| `.\scripts\test-docker.ps1` | すべてのテスト（Rust/TypeScript/統合）を実行 | リリース前・PR 前の総合確認 |
| `.\scripts\test-docker.ps1 rust` | Rust テストのみ実行 | Rust 変更時のスポット確認 |
| `.\scripts\run-rust-tests.ps1` | Windows専用の Rust テスト自動化ラッパー（Docker 実行） | Lint/メトリクス更新で Rust テストを安定実行したい場合 |
| `.\scripts\test-docker.ps1 rust -Integration` | Rust の P2P 統合テストのみ実行 | iroh / DHT 関連の結合テスト |
| `.\scripts\test-docker.ps1 integration` | `-Integration` と同等（ショートカット） | 統合テスト専用フロー |
| `.\scripts\test-docker.ps1 ts` | TypeScript テストのみ | フロント変更時 |
| `.\scripts\test-docker.ps1 lint` | フロント/バックのリントと整形チェック | ESLint / rustfmt / pnpm format:check を一括確認 |
| `.\scripts\test-docker.ps1 metrics` | メトリクス関連ショートテスト | 運用メトリクス更新前の検証 |
| `.\scripts\test-docker.ps1 build` | テスト用 Docker イメージの再ビルド | 依存更新・Dockerfile 変更時 |
| `.\scripts\test-docker.ps1 clean` | コンテナとネットワークを削除 | テスト失敗後の後片付け |
| `.\scripts\test-docker.ps1 cache-clean` | イメージ・ボリュームを含む完全削除 | キャッシュ破損時や容量節約 |

### 主なオプション
- `-IntegrationLog`: 統合テスト時の `RUST_LOG` を上書き（例: `-IntegrationLog "debug,iroh_tests=trace"`）。
- `-BootstrapPeers`: 任意のブートストラップピア（カンマ区切り）を指定。
- `-IrohBin`: ホスト上の iroh バイナリをマウントして利用。
- `-NoBuild`: イメージ再ビルドをスキップ（繰り返し実行時に推奨）。

## ログと成果物
- `test-results/`: Docker 実行時に生成されるテスト結果。CI のアップロード対象と同一形式。
- PowerShell 標準出力: 成功時は緑色（✓）、警告は黄色（⚠）で表示。エラーは `Error:` の赤文字。
- `docker logs kukuri-p2p-bootstrap`: 統合テスト向けブートストラップコンテナのログ確認に使用。

## CI（GitHub Actions）との主な差分
| 項目 | ローカル（test-docker.ps1） | CI（`.github/workflows/test.yml`） |
| --- | --- | --- |
| 実行ホスト | 開発者の Windows + Docker Desktop | `ubuntu-latest` ランナー（Docker in Linux） |
| トリガー | 手動実行 | push / PR / workflow_dispatch |
| 既定コマンド | `all` → `/app/run-tests.sh` を1回 | `docker-test` ジョブで `run-tests.sh` と `rust-test` を別々に実行 |
| ブートストラップ制御 | `-Integration` で PowerShell から `p2p-bootstrap` を起動 | Docker Compose サービス定義で自動起動 |
| キャッシュ | Docker ボリューム（`kukuri-cargo-*` 等）を再利用 | Actions Cache に BuildKit 層・Cargo/pnpm キャッシュを保存 |
| 失敗時の処理 | 即時停止（PowerShell 例外） | `needs` 連鎖で `test-summary` が失敗可視化 |
| ログ収集 | 手動で `test-results/` を参照 | `upload-artifact` で自動収集 |
| 追加ジョブ | 任意（`lint`, `metrics` 等を手動実行） | `native-test-linux`, `format-check`, `build-test-windows` を並列実行 |
| 環境変数 | オプション指定で上書き（`-IntegrationLog` 等） | ワークフロー内で固定値 (`ENABLE_P2P_INTEGRATION`, `RUST_VERSION` など) |

## 運用上の注意
- **Docker Desktop のリソース割り当て**: CPU 4core / メモリ 8GB 程度を推奨。統合テスト時にタイムアウトが発生したら増量を検討。
- **初回実行の所要時間**: イメージビルドに 10〜15 分程度。以降はキャッシュ利用で 5 分前後が目安。
- **統合テストのヘルスチェック**: `p2p-bootstrap` が `healthy` になる前にコンテナ終了した場合、`-Integration` 実行時にリトライするか `docker ps -a` で状態確認。
- **Windows Defender との干渉**: 大量ファイルをコピーするため、リアルタイム保護がビルドを遅延させるケースあり。必要に応じて除外設定を検討。 

## トラブルシューティング（抜粋）
- **`docker: command not found`**: PowerShell セッションが Docker Desktop を認識していない場合。再起動または `RefreshEnv` を実行。
- **`Cannot connect to the Docker daemon`**: Docker Desktop 停止中。起動後に再実行。
- **`execution of scripts is disabled on this system`**: 実行ポリシーの設定不足。前提条件を参照。
- **`STATUS_ENTRYPOINT_NOT_FOUND`**: ネイティブでテストされた場合の既知エラー。Docker 経由に切り替える。
- **統合テストだけ再実行したい**: `.\scripts\test-docker.ps1 rust -Integration -NoBuild`。

## 今後の ToDo
- `metrics` コマンドとの連携でテスト結果サマリーを自動抽出する PowerShell 関数を追加。
- `-ReportPath` オプションを実装し、JUnit XML を任意ディレクトリへ保存できるようにする。
- `scripts/test-docker.sh` との手順差異を `docs/03_implementation/docker_test_environment.md` に統合してクロスリンク。

## Linux/macOS ガイドとの共通化ポイント（検討メモ）
- 基本構造（背景・前提条件・コマンド一覧・トラブルシューティング）が双方で重複しているため、`docker_test_environment.md` に共通セクションを集約し、本ドキュメントでは Windows 固有差分のみを記述する構成が望ましい。
- コマンドテーブルは `docker_test_environment.md` の「Windows環境での実行」節と列構成が近い。テーブル項目を揃え、PowerShell 版・Bash 版を同一表で比較できる形式に再編成する。
- 環境変数の説明は Linux 版に網羅されているため、Windows 版では値が異なる項目（例: `ENABLE_P2P_INTEGRATION` の既定値、PowerShell からの上書き方法）に限定して追記する。
- トラブルシューティングは Docker 共通の内容が多い。OS 固有の事象（`execution policy` や Defender との干渉など）は本ドキュメント、共通エラーは `docker_test_environment.md` へ移動する。
- 今後 `metrics` コマンドや成果物の扱いを標準化する際は、CI/CD との整合性を `docker_test_environment.md` に記載し、Windows 版からは参照リンクを配置する。
