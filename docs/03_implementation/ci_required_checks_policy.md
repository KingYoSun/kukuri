# CI Required Checks ポリシー

最終更新日: 2026年02月25日

## 目的
- GitHub Actions の待ち時間を短縮し、PR のレビュー待ち時間を最小化する。
- 必須チェックを「PR向け高速セット」と「push/nightly向け重厚セット」に分離し、品質と速度を両立する。

## 運用方針
- PR (`pull_request`) では高速セットのみ実行し、これを required checks とする。
- 重厚セットは `push`（`main` / `develop`）と `nightly` を中心に実行する。
- docs-only 変更（`docs/**` のみ）では重い Docker ジョブをスキップする。
- 同一ブランチの古い実行は `concurrency.cancel-in-progress: true` で自動キャンセルする。
- `Community Node Tests` のローカル再現は OS を問わずコンテナ経路を既定とする（`docker compose -f docker-compose.test.yml up -d community-node-postgres` + `docker compose -f docker-compose.test.yml build test-runner` + `docker run ... kukuri-test-runner ... cargo test --workspace --all-features`）。

## Issue close 品質ゲート
- bug Issue の close 条件は `verified-fixed` ラベル付与・CI（`Test` workflow）成功・検証証跡添付の3点を必須とする。
- `verify-close-condition.yml` は上記条件未達で close された bug Issue を自動で reopen し、再 close の前提条件をコメントする。
- 自動 reopen が発生した場合は同ワークフローから Discord webhook 通知を送信する（payload は `flags: 4` を付与）。
- `DISCORD_WEBHOOK_URL` が未設定の環境では通知 step はスキップし、ログに理由を残す。

## PR向け高速セット（Required Checks）
- `Format Check`
- `Native Test (Linux)`
- `Community Node Tests`
- `Build Test (Windows)`
- 集約ジョブ: `PR Required Checks`

## 重厚セット（Push / Nightly）
- `Docker Test Suite`
- `Desktop E2E (Community Node, Docker)`
- `Smoke Tests (Docker)`
- Nightly の Docker シナリオ群（`desktop-e2e`, `community-node-e2e`, `trending-feed` など）
- 集約ジョブ: `Push Heavy Checks`

## Docker イメージ最適化
- `Dockerfile.test` は GHCR へプリビルド配布する（`build-test-runner-image.yml`）。
- テスト実行スクリプト（`scripts/test-docker.sh` / `scripts/test-docker.ps1`）は
  `KUKURI_TEST_RUNNER_IMAGE` を優先利用し、pull/tag 成功時はローカルビルドを省略する。
- プリビルド取得失敗時は自動で従来ビルドにフォールバックする。

## 関連ファイル
- `.github/workflows/test.yml`
- `.github/workflows/smoke-tests.yml`
- `.github/workflows/nightly.yml`
- `.github/workflows/build-test-runner-image.yml`
- `.github/workflows/verify-close-condition.yml`
- `.github/ISSUE_TEMPLATE/bug_report.md`
- `scripts/test-docker.sh`
- `scripts/test-docker.ps1`
