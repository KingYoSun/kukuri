# Issue #174 自動品質ゲート改善 #1 実装レポート

作成日: 2026年02月25日

## 概要

Issue #174 の要求（Close Conditions の現実化、CI 成功条件の明示、Issue 自動再オープン時 Discord 通知）を GitHub テンプレート・ワークフロー・運用ドキュメントに反映した。

## 実装内容

1. Bug report テンプレートの Close Conditions 修正

- ファイル: `.github/ISSUE_TEMPLATE/bug_report.md`
- 人力 UI 操作前提の項目を整理し、以下の必須条件へ更新。
  - `bug` ラベル維持
  - 修正 PR / コミットとの関連付け
  - CI（`Test` workflow required checks）成功
  - 検証証跡添付
  - UI 変更時の Before/After 証跡
  - `verified-fixed` ラベル付与

2. close 条件ワークフローへ reopen 通知を追加

- ファイル: `.github/workflows/verify-close-condition.yml`
- `verified-fixed` 未達で Issue を reopen する既存処理に加え、以下を追加。
  - `DISCORD_WEBHOOK_URL` を job-level `env` で参照
  - reopen 発生時のみ Discord 通知 step を実行
  - payload は `jq` で生成し `flags: 4` を付与
  - webhook 未設定時は skip ログを明示
- close 条件未達コメント文面に `CI（Test workflow）成功` 要件を追記。

3. 運用ドキュメント更新

- ファイル: `docs/03_implementation/ci_required_checks_policy.md`
- `Issue close 品質ゲート` セクションを追加し、close 条件と reopen 通知運用を明文化。

## 検証結果

- `gh act`（verify-close-condition, verified-fixed あり）: pass
  - コマンド:
    - `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act issues -W .github/workflows/verify-close-condition.yml -j enforce-verified-fixed -e tmp/act-events/issues-closed-verified.json`
  - ログ: `tmp/logs/gh-act-verify-close-condition-verified.log`
- `gh act`（verify-close-condition, verified-fixed なし）: known fail
  - コマンド:
    - `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act issues -W .github/workflows/verify-close-condition.yml -j enforce-verified-fixed -e tmp/act-events/issues-closed-missing-verified.json`
  - 結果: ダミー Issue `#999998` が実在しないため `PATCH /issues/999998` が 404
  - ログ: `tmp/logs/gh-act-verify-close-condition-missing-verified.log`
- セッション完了要件の `gh act` 3ジョブ:
  - `format-check`: pass（`tmp/logs/gh-act-test-format-check-issue174.log`）
  - `native-test-linux`: pass（`tmp/logs/gh-act-test-native-test-linux-issue174.log`）
  - `community-node-tests`: pass（`tmp/logs/gh-act-test-community-node-tests-issue174.log`）
