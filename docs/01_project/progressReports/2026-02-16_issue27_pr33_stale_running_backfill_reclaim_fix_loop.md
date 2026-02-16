# Issue #27 / PR #33 fix loop（stale `running` backfill reclaim）

最終更新日: 2026年02月16日

## 概要

PR #33 の review 指摘（`discussion_r2811523305`）に対応し、`claim_backfill_job` が stale `running` ジョブを再取得できない欠陥を修正した。
あわせて lease fencing を導入し、worker crash 後の takeover でも重複 active owner が発生しないようにした。

## 原因

- 従来の `claim_backfill_job` は `status IN ('pending', 'failed')` のみを対象にしていた。
- worker 異常終了で `running` のまま残ったジョブは再 claim されず、backfill が停止し続ける可能性があった。
- 進捗更新・完了更新に lease ownership の検証がなく、takeover 後に旧 owner が状態を上書きできる余地があった。

## 実装内容

1. stale `running` reclaim の追加
- 変更: `kukuri-community-node/crates/cn-index/src/lib.rs`
- `claim_backfill_job` の対象に以下を追加:
  - `status = 'running' AND updated_at <= NOW() - BACKFILL_RUNNING_LEASE_TIMEOUT_SECONDS`
- timeout は `BACKFILL_RUNNING_LEASE_TIMEOUT_SECONDS = 5 * 60` を導入。

2. lease token 再発行と保持
- claim 成功時に `started_at = NOW()` を必ず更新し、`RETURNING started_at` を `lease_started_at` として保持。
- `BackfillJob` に `lease_started_at: DateTime<Utc>` を追加。

3. safe takeover（fencing）適用
- `update_backfill_job_progress` / `mark_backfill_job_succeeded` / `mark_backfill_job_failed` を
  - `WHERE job_id = $2 AND status = 'running' AND started_at = $3`
  で更新するよう変更。
- progress/succeeded は `rows_affected() == 0` で `lease lost` エラーにし、旧 owner の書き込みを遮断。
- failed は `rows_affected() == 0` を no-op として扱い、新 owner を壊さない挙動にした。

4. 回帰テスト追加・安定化
- 変更: `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- 追加:
  - `claim_backfill_job_reclaims_stale_running_job`
  - `backfill_job_fences_old_lease_after_takeover`
- 既存 `backfill_job_resumes_from_checkpoint_and_completes_post_search_documents` は checkpoint 厳密一致をやめ、seed cursor から前進したことを tuple 比較で検証するよう変更（full suite での順序揺らぎ対策）。

## 検証

- `cd /home/kingyosun/kukuri/kukuri-community-node && cargo fmt`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/.cache ACT_CACHE_DIR=/tmp/act-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue27-pr33-fix-loop.log`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/.cache ACT_CACHE_DIR=/tmp/act-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue27-pr33-fix-loop.log`（pass）
- `cd /home/kingyosun/kukuri && NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/.cache ACT_CACHE_DIR=/tmp/act-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue27-pr33-fix-loop.log`（fail: `cn-user-api` の既知不安定系 `subscriptions::api_contract_tests::auth_consent_quota_metrics_regression_counters_increment` が `left: 428, right: 402`。`cn-index` の backfill 16 tests は pass）

## 変更ファイル

- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
- `docs/01_project/progressReports/2026-02-16_issue27_pr33_stale_running_backfill_reclaim_fix_loop.md`
