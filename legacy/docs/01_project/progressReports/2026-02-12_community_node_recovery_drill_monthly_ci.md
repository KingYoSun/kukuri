# 2026-02-12 Community Node `recovery-drill` 月次CI統合

## 概要

- 対象: `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目
  - `ops_runbook.md` の「月1リストア演習」要件に合わせ、`recovery-drill` を定期 CI ジョブへ組み込み、`test-results/community-node-recovery/latest-summary.json` を artefact 収集・検証できるようにする
- 結果: `nightly.yml` に月次実行ガード付きジョブを追加し、サマリ検証・artefact 収集を実装。roadmap の該当項目を完了（`[x]`）へ更新。

## 実装内容

### 1. Nightly ワークフローへ `community-node-recovery-drill` ジョブを追加

- 変更ファイル: `.github/workflows/nightly.yml`
- 追加内容:
  - `community-node-recovery-drill` ジョブを新設
  - 実行ガード:
    - `workflow_dispatch` は常時実行
    - `schedule` は UTC 日付が毎月1日のみ実行（それ以外はスキップ）
  - 実行コマンド:
    - `./scripts/test-docker.ps1 recovery-drill`
  - サマリ検証:
    - `test-results/community-node-recovery/latest-summary.json` の存在確認
    - `jq` で以下を検証
      - `status == "passed"`
      - `baseline_event_count > 0`
      - `after_corruption_event_count == 0`
      - `after_restore_event_count == baseline_event_count`
      - `backup_file` / `log_path` が文字列であること
  - artefact 収集:
    - `nightly.community-node-recovery-logs` <- `tmp/logs/community-node-recovery`
    - `nightly.community-node-recovery-reports` <- `test-results/community-node-recovery`

### 2. Runbook / タスク管理の更新

- `docs/03_implementation/community_nodes/ops_runbook.md`
  - 2.4 に定期CI連携（`nightly.yml` の `community-node-recovery-drill`）を追記
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 該当未実装項目を `[x]` へ更新し、実行ガード/検証条件/artefact 名を明記
- `docs/01_project/activeContext/tasks/completed/2026-02-12.md`
  - 本対応と検証ログを追記

## 検証結果

- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - `tmp/logs/gh-act-format-check-recovery-ci-20260212-055400.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - `tmp/logs/gh-act-native-test-linux-recovery-ci-20260212-055522.log`
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（成功）
  - `tmp/logs/gh-act-community-node-tests-recovery-ci-20260212-060145.log`
- `gh act workflow_dispatch --workflows .github/workflows/nightly.yml --job community-node-recovery-drill`
  - `act` ジョブは `pwsh` 非同梱コンテナのため失敗（`pwsh: executable file not found`）
  - ログ: `tmp/logs/gh-act-nightly-community-node-recovery-drill-20260212-060600.log`
  - 補足: GitHub Hosted Runner（`ubuntu-latest`）では `pwsh` が標準提供されるため、実環境 Nightly 実行には影響しない想定
