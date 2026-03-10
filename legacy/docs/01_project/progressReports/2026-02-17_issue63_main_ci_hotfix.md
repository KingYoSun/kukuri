# Issue #63 main CI failure hotfix

最終更新日: 2026年02月17日

## 概要

main の CI Run `22101614830` で `OpenAPI Artifacts Check` と `Desktop E2E (Community Node, Docker)` が同時に失敗したため、job ログをトリアージして最小修正を実施した。

根本原因はどちらもネットワーク一時障害に対する耐性不足で、前者は Rust 1.88.0 toolchain 取得時の timeout、後者は `Dockerfile.test` の `apt-get update`（`deb.debian.org:80`）timeout に起因していた。

## 実施内容

1. `gh api repos/KingYoSun/kukuri/actions/jobs/63872622145/logs` / `63872622185/logs` で失敗箇所を特定。
2. `.github/workflows/test.yml` の `openapi-artifacts-check` に Rust setup 3回目を追加し、`RUSTUP_MAX_RETRIES=10` を設定。
3. `Dockerfile.test` で APT ミラーを `https://deb.debian.org` へ統一し、システム依存導入と Node.js 導入を retry 付きへ変更。
4. tracking issue #63 へ進捗コメントを投稿。

## 変更ファイル

- `.github/workflows/test.yml`
- `Dockerfile.test`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/progressReports/2026-02-17_issue63_main_ci_hotfix.md`

## 検証

- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `RUSTUP_HOME=/tmp/rustup CARGO_HOME=/tmp/cargo-home RUSTUP_MAX_RETRIES=10 rustup toolchain install 1.88.0 --profile minimal --no-self-update`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache-server`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache-server`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests --action-cache-path /tmp/act-cache --cache-server-path /tmp/actcache-server`（pass）

## 参照

- PR #64: `https://github.com/KingYoSun/kukuri/pull/64`
- tracking issue #63: `https://github.com/KingYoSun/kukuri/issues/63`
- CI Run: `https://github.com/KingYoSun/kukuri/actions/runs/22101614830`
- OpenAPI Artifacts Check (failed job): `https://github.com/KingYoSun/kukuri/actions/runs/22101614830/job/63872622145`
- Desktop E2E (Community Node, Docker) (failed job): `https://github.com/KingYoSun/kukuri/actions/runs/22101614830/job/63872622185`
