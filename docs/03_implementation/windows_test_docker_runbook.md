# Windows向け Docker テスト運用ガイド

作成日: 2025年10月20日
最終更新日: 2025年10月22日

## 位置づけ
Windows では DLL 依存の問題によりネイティブ実行が不安定なため、`.\scripts\test-docker.ps1` を **標準テスト経路** とする。  
`docs/03_implementation/docker_test_environment.md` が共通事項（背景・コンテナ構成・環境変数）を扱い、本ドキュメントでは Windows 固有の運用手順と CI（GitHub Actions）との対応付けを記録する。

## 前提条件
- Docker Desktop 4.33 以降が起動済みであること。
- PowerShell 7 以上を推奨（5.1 でも実行可だが配色と例外処理が限定的）。
- `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser` を設定済み。
- リポジトリ直下（`kukuri/`）でコマンドを実行する。

## 基本フロー
1. **初回 or 依存更新時**: `.\scripts\test-docker.ps1 build` でテスト用イメージを作成（10〜15分目安）。
2. **日常実行**: `.\scripts\test-docker.ps1` で `/app/run-tests.sh` を呼び出し、Rust/TypeScript/統合テストを一括実行。
3. **結果確認**: 成功・失敗に関わらず `test-results/`（JUnit XML / ログ）と標準出力を確認。
4. **終了処理**: 必要に応じて `.\scripts\test-docker.ps1 clean` または `cache-clean` でコンテナ/ボリュームを整理。

## コマンドカタログ
| コマンド | 概要 | 代表的な用途 |
| --- | --- | --- |
| `.\scripts\test-docker.ps1` | `/app/run-tests.sh` を実行し全テストを走査 | PR 提出前の一括検証 |
| `.\scripts\test-docker.ps1 rust` | Rust テスト専用コンテナを実行 | Rust 変更のスポット確認 |
| `.\scripts\test-docker.ps1 integration` / `rust -Integration` | P2P 統合テストのみ実行（ブートストラップ自動起動） | DHT/iroh 改修時の回帰確認 |
| `.\scripts\run-rust-tests.ps1` | Rust 向けラッパー。`-Integration` / `-NoBuild` オプションをサポート | PowerShell スクリプトから Rust テストを呼び出す場合 |
| `.\scripts\test-docker.ps1 ts` | TypeScript テスト | UI 改修の単体テスト確認 |
| `.\scripts\test-docker.ps1 lint` | ESLint / rustfmt / pnpm format:check を一括実行 | Lint 修復後の再確認 |
| `.\scripts\test-docker.ps1 metrics` | メトリクス抽出向けショートテスト | `scripts/metrics/collect-metrics.ps1` 実行前のスモーク |
| `.\scripts\test-docker.ps1 build` | イメージのみビルド | 依存更新時のキャッシュ再生成 |
| `.\scripts\test-docker.ps1 clean` | コンテナとネットワークを削除 | テスト失敗後の後片付け |
| `.\scripts\test-docker.ps1 cache-clean` | 上記 + キャッシュボリューム削除 | キャッシュ破損や容量逼迫時 |

主なオプション:
- `-IntegrationLog <value>`: 統合テスト時の `RUST_LOG` を上書き（例: `-IntegrationLog "debug,iroh_tests=trace"`）。
- `-BootstrapPeers <node@host:port,...>`: 任意のブートストラップピアを指定。
- `-IrohBin <path>`: ホスト上の iroh バイナリをマウント。
- `-NoBuild`: 直前にビルド済みの場合の高速化。

## CI とローカルの対応表
| シナリオ | ローカル（PowerShell） | GitHub Actions | 差分吸収ポイント |
| --- | --- | --- | --- |
| 全テストスイート | `.\scripts\test-docker.ps1` | `docker-test` ジョブの `run-tests.sh` ステップ | ローカルは 1 コマンドで Rust/TS/統合を順番実行。CI は同じスクリプトを呼び出しつつジョブ内で Rust テストを追加起動。 |
| Rust テスト（ユニット） | `.\scripts\test-docker.ps1 rust` | `docker-test` ジョブの `rust-test` ステップ | コンテナ・環境変数は共通。ローカルは `-NoBuild` を併用して反復を高速化。 |
| Rust P2P 統合 | `.\scripts\test-docker.ps1 integration` | `docker-test` ジョブ内で `ENABLE_P2P_INTEGRATION=1` を付与 | ブートストラップ起動タイミングのみ差異あり。ローカルは PowerShell 側でヘルスチェックを行う。 |
| TypeScript テスト | `.\scripts\test-docker.ps1 ts` | `native-test-linux` ジョブ `pnpm test` | コンテナ実行と pnpm 直接実行でログ出力形式が異なる。`test-results/ts/` を確認し整合を取る。 |
| Lint / Format | `.\scripts\test-docker.ps1 lint` | `format-check` ジョブ | ローカルは Docker 内で Rust/TS の整形確認を一括実行。CI は Rust/TS を個別ステップで実行。 |
| キャッシュ再構築 | `.\scripts\test-docker.ps1 build` / `cache-clean` | `docker system prune` + Buildx キャッシュ | ローカルは Docker ボリューム（`kukuri-cargo-*`）を利用。CI は Actions Cache を使用。 |
| Windows ビルド互換確認 | `.\scripts\test-docker.ps1 ts` + `run-rust-tests.ps1` | `build-test-windows` ジョブ | CI はネイティブビルドのみ。ローカルでは Docker を基本とし、ネイティブビルドが必要な場合は別途 `cargo check` を手動実行する。 |

### CI 等価性チェックリスト
- [ ] `docker version` / `docker info` でデーモンが稼働している。
- [ ] 初回実行は `.\scripts\test-docker.ps1 build` でキャッシュをウォームアップ。
- [ ] PR 前は `.\scripts\test-docker.ps1` と `.\scripts\test-docker.ps1 lint` を順に実行し、CI の `docker-test`・`format-check` と同じ出口条件を満たす。
- [ ] Rust のみ変更時は `.\scripts\run-rust-tests.ps1 -NoBuild` を使用し、`docker-test` の Rust ステップと同じ結果を得る。
- [ ] 統合テストが失敗した際は `docker logs kukuri-p2p-bootstrap` を確認し、CI と同じログ情報を取得する。

## ログと成果物
- `test-results/`: JUnit XML と要約ログ。CI の `upload-artifact` と同一構成。
- PowerShell 出力: 成功時は `✓`, 警告は `⚠`, 失敗は `Error:` で表示。
- `docker logs kukuri-p2p-bootstrap`: 統合テスト専用ブートストラップのログ取得。
- 追加メトリクス: `.\scripts\test-docker.ps1 metrics` 実行時は `artefacts/metrics/` 配下に結果が保存される。

## 運用フロー詳細
### 実行前チェック
- Docker Desktop の CPU: 4 core / メモリ: 8 GB 以上を推奨。
- Windows Defender のリアルタイム監視が遅延要因となる場合は `kukuri\` ディレクトリを除外設定。
- `docker compose ls` で既存セッションが残っていないか確認（残っている場合は `clean` を実行）。

### 実行中の Tips
- 連続実行時は `-NoBuild` を付与して BuildKit キャッシュを再利用。
- 統合テストでログ詳細が欲しい場合は `-IntegrationLog "debug,iroh_tests=trace"` を指定。
- `-BootstrapPeers` で開発中の別ノードに向けたデバッグが可能。

### 失敗時の対応
- Rust テスト失敗: `test-results/rust/` の `cargo-test-*.log` を参照し、CI の Artefact と比較。
- TypeScript 失敗: `test-results/ts/` の `vitest.report.xml` を参照。
- Docker エラー発生時は `.\scripts\test-docker.ps1 clean` で環境をリセットし再実行。

## メンテナンス
- `.\scripts\test-docker.ps1 cache-clean` で Docker ボリューム（`kukuri-cargo-*` / `kukuri-pnpm-store`）を削除。次回実行は時間がかかるため、週次メンテナンスでの利用を推奨。
- `docker system prune -af` はプロジェクト以外にも影響するため、本スクリプト経由でのクリーンアップを優先する。
- `scripts/run-rust-tests.ps1` を CI 連携スクリプトから呼び出す場合は `-NoBuild` と `-Integration` の引数を明示して運用フローを統一する。

## トラブルシューティング
- `docker: command not found` → Docker Desktop が PATH に登録されていない。再起動または `RefreshEnv` を実行。
- `Cannot connect to the Docker daemon` → Docker Desktop 停止中。起動後に再実行。
- `execution of scripts is disabled on this system` → 実行ポリシーを再設定。
- `STATUS_ENTRYPOINT_NOT_FOUND` → ネイティブ実行を避け、Docker スクリプトを使用。
- 統合テストだけ再実行したい → `.\scripts\test-docker.ps1 rust -Integration -NoBuild`。

## 参照
- `docs/03_implementation/docker_test_environment.md`: Docker 共通設定と Linux/macOS 手順。
- `.github/workflows/test.yml`: CI ジョブ構成と Artefact 収集の詳細。
- `scripts/test-docker.ps1` / `scripts/run-rust-tests.ps1`: 本ドキュメントで参照するスクリプト定義。
