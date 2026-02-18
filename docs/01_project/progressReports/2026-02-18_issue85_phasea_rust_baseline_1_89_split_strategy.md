# Issue #85 Phase A: Rust baseline 1.89 uplift（split strategy）

作成日: 2026年02月18日  
Issue: https://github.com/KingYoSun/kukuri/issues/85

## 実施概要

Issue #85 は manager 承認の 2 段分割（Phase A / Phase B）で再開し、本レポートでは Phase A（Rust 基盤更新のみ）を完了した。

- Phase A: Rust baseline 1.89 uplift（CI / Docker test-runner / 関連ドキュメント）
- Phase B: `iroh` / `iroh-gossip` 0.96 移行 + `cn-relay` gossip 統合テスト修正（別 PR）

## Phase A 変更内容

1. CI Rust baseline 更新
   - `.github/workflows/test.yml`
   - `env.RUST_VERSION` を `1.86` から `1.89` へ更新
   - `openapi-artifacts-check` の Rust toolchain 指定（3 箇所）を `1.88.0` 固定から `${{ env.RUST_VERSION }}` へ統一

2. Docker test-runner baseline 更新
   - `Dockerfile.test`
   - ベースイメージを `rust:1.86-bookworm` から `rust:1.89-bookworm` へ更新

3. 関連ドキュメント更新
   - `docs/03_implementation/docker_test_environment.md`
   - ベースイメージ記述と Rust 必要バージョン記述を 1.89 系へ同期

## 非対象（Phase B へ持ち越し）

- `Cargo.toml` の `iroh` / `iroh-gossip` バージョン更新は未実施（Phase A では intentionally no-op）
- `cn-relay` gossip 統合テスト安定化は未実施（Phase B で対応）

## 検証ログ

実行環境変数:
- `XDG_CACHE_HOME=/tmp/.cache`
- `NPM_CONFIG_PREFIX=/tmp/npm-global`

実行コマンド:
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - ログ: `tmp/logs/gh_act_format_check_20260218-055707.log`
  - 結果: pass
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - ログ: `tmp/logs/gh_act_native_test_linux_20260218-055743.log`
  - 結果: fail（既存 `clippy::collapsible_if` 多数。`Main Run Rust tests` は pass、`Main Run Rust clippy` で fail）
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - ログ: `tmp/logs/gh_act_community_node_tests_20260218-060042.log`
  - 結果: pass（`cargo test --workspace --all-features` + `cargo build --release -p cn-cli` 成功）

## 補足

- `native-test-linux` fail は今回の baseline 変更差分ではなく、既存 clippy 指摘（`-D warnings`）によるもの。
- `community-node-tests` 実行ログ内で `iroh 0.95.1` / `iroh-gossip 0.95.0` を確認し、Phase A の「iroh 非変更」方針を維持できている。
