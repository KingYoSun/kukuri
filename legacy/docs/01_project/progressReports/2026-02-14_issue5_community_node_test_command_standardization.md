# Issue #5: Community Node テストコマンド標準化レポート

作成日: 2026年02月14日

## 概要

- 目的: community-node テスト実行コマンドを Linux/macOS/Windows で共通の container-first 方針に統一する。
- 対応: README / Runbook / CI方針 / タスク文書 / AGENTS の記述差分を解消し、ホスト直実行を既定手順から外した。

## 主な更新

- `README.md` / `README.ja.md`
  - community-node テスト/ビルドの既定コマンドを Docker ベースに変更。
- `docs/03_implementation/docker_test_environment.md`
  - 「Community Node テスト（全OS共通）」セクションを追加。
- `docs/03_implementation/community_nodes/ops_runbook.md`
  - 運用原則に community-node の container-first 実行規約を追加。
- `docs/03_implementation/ci_required_checks_policy.md`
  - `Community Node Tests` のローカル再現方針を全OSコンテナ既定に明記。
- `AGENTS.md`
  - community-node の Rust テスト/ビルド手順をコンテナ経路へ更新。
- `docs/01_project/activeContext/tasks/*`
  - `in_progress.md` / `community_nodes_roadmap.md` / `completed/2026-02-14.md` に反映。

## 検証

- `python3 scripts/check_date_format.py` を実行し成功。
- `gh act --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-issue5-20260214.log`
- `gh act --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-issue5-20260214.log`
- `gh act --job community-node-tests`
  - 初回失敗: `tuple concurrently updated`（`cn-admin-api` 契約テスト）
  - 再実行成功
  - ログ: `tmp/logs/gh-act-community-node-tests-issue5-20260214.log`, `tmp/logs/gh-act-community-node-tests-issue5-20260214-rerun.log`

## 補足

- 実行ポリシーを文書で先行統一した。CI ワークフローの実行実装差分（必要に応じて container-first へ寄せる対応）は別PRで扱う。
