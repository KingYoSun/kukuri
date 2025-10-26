# Phase 5 Workstream B - Rustカバレッジ定着レポート
最終更新日: 2025年10月26日

## 概要
- `docker-compose.test.yml` に `rust-coverage` サービスを追加し、`cargo tarpaulin --locked --all-features --skip-clean --out Json --out Lcov --output-dir /app/test-results/tarpaulin --timeout 1800` を ptrace 許可付きで実行できるようにした。
- `scripts/test-docker.{sh,ps1}` に `coverage` コマンドを追加し、Docker 上で tarpaulin を走らせた後に `docs/01_project/activeContext/artefacts/metrics/<timestamp>-tarpaulin.{json,lcov}` へ成果物を自動保存する運用フローを整備した。
- 初回計測（2025年10月26日）で Rust カバレッジ 25.23%（1630/6460 行）を取得し、`tasks/metrics/test_results.md` / `phase5_test_inventory.md` / `p2p_mainline_runbook.md` / `phase5_ci_path_audit.md` に閾値と実行手順を反映した。

## 変更詳細
- Shell/PowerShell 双方の `test-docker` スクリプトに `coverage` 用ヘルパーを実装し、Docker ビルド → tarpaulin 実行 → JSON/LCOV コピー → 進行状況ログ表示を自動化。lcov 生成パターン（`tarpaulin-report.lcov` または `lcov.info`）の双方を検出して保存する。
- `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` に coverage コマンドのパス依存を追記し、CI との整合性を明示。`phase5_test_inventory.md` と `p2p_mainline_runbook.md` に tarpaulin 実行手順と達成目標（Phase 5:50%、Phase 6:70%）を追加した。
- Rust テスト結果と tarpaulin レポートを `docs/01_project/activeContext/artefacts/metrics/2025-10-26-153751-tarpaulin.{json,lcov}` として保存し、`tasks/metrics/test_results.md` に最新の測定値と次のアクションを記載した。

## 実施テスト
- ./scripts/test-docker.sh rust
- ./scripts/test-docker.sh coverage
