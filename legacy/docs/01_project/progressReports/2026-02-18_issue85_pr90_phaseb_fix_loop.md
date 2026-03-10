# Issue #85 Phase B fix loop（PR #90）

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/85
PR: https://github.com/KingYoSun/kukuri/pull/90
対象ブランチ: `chore/issue-85-phase-b-iroh-0.96-gossip-tests`

## 背景

- CI Run: `22131295998`
- 失敗ジョブ: `Docker Test Suite`（job: `63971958084`）
- 失敗ステップ: `Run all tests in Docker`

ログ診断の結果、`p2p-bootstrap` / `community-node` ビルドで `kukuri-community-node/Dockerfile` が `rust:1.88-bookworm` を参照しており、Phase B で導入済みの `iroh 0.96.x`（`rustc >= 1.89` 必須）と不整合を起こしていた。

## 実施内容（最小修正）

- `kukuri-community-node/Dockerfile`
  - `FROM rust:1.88-bookworm AS builder` を `FROM rust:1.89-bookworm AS builder` に更新。

Phase B スコープ外の refactor や追加変更は行っていない。

## 検証

- `gh run view 22131295998 --job 63971958084 --log > /tmp/gh_job_63971958084.log`（原因確認）
- `DOCKER_CONFIG=/tmp/docker-config docker compose --project-name kukuri_tests -f docker-compose.test.yml build p2p-bootstrap community-node-user-api community-node-bootstrap`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose --project-name kukuri_tests -f docker-compose.test.yml up -d p2p-bootstrap` + health check（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh_act_format_check_issue85_pr90_phaseb_fix_loop_20260218.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh_act_native_test_linux_issue85_pr90_phaseb_fix_loop_20260218.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh_act_community_node_tests_issue85_pr90_phaseb_fix_loop_20260218.log`（pass）

## 結果

- `Docker Test Suite` の失敗原因（Rust toolchain mismatch）を解消。
- Phase B の最小差分で修正し、関連 Docker 経路と必須 `gh act` 3ジョブの成功を確認。
