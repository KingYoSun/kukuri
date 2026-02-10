# Community Nodes Moderation LLM 実行時上限制御（2026年02月07日）

最終更新日: 2026年02月07日

## 概要

`cn-moderation` の LLM ラベリング処理に、日次予算と同時実行の実行時ガードを追加した。`max_requests_per_day` / `max_cost_per_day` / `max_concurrency` の各上限をトランザクション内で強制し、上限到達時は LLM 分類を実行せず `cn_admin.audit_logs` にスキップ理由を記録するようにした。

## 実装内容

- migration 追加:
  - `kukuri-community-node/migrations/20260207000000_m4_llm_runtime_limits.sql`
  - `cn_moderation.llm_daily_usage`（日次リクエスト数・推定コスト）
  - `cn_moderation.llm_inflight`（実行中リクエスト）
- `cn-moderation` 実装:
  - `acquire_llm_execution_gate` を追加し、LLM 実行前に上限判定とカウンタ更新を実施
  - `release_llm_execution_gate` で実行中スロットを解放
  - `log_llm_skip_audit` で `moderation.llm.skip` の監査ログを記録（`skip_reason`/usage/limits を保持）
  - `process_job` の LLM ブロックを上記ゲート経由に変更
  - `LlmRuntimeConfig.max_concurrency` を最小 1 に正規化
- テスト追加（`cn-moderation/src/lib.rs`）:
  - `max_requests_per_day` 到達時にスキップされること
  - `max_cost_per_day` 到達時にスキップされること
  - `max_concurrency` 到達時にスキップされること

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "${PWD}/kukuri-community-node:/app" -w /app rust:1.88-bookworm bash -c "cargo test -p cn-moderation -- --nocapture"` 成功
- `./scripts/test-docker.ps1 rust -NoBuild` 成功
- `docker run --rm -v "${PWD}/kukuri-community-node:/app" -w /app rust:1.88-bookworm cargo test -- --nocapture` 成功
- `gh act --workflows .github/workflows/test.yml --job format-check` 成功
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` 成功

## 補足

- `gh act` 実行時に `some refs were not updated` が表示されるが、ジョブ自体は成功した。
