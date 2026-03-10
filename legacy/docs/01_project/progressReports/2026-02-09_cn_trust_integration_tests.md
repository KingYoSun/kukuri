# Community Nodes `cn-trust` 統合テスト追加（report/interactions -> score -> attestation -> jobs/schedules）

作成日: 2026年02月09日

## 概要

`docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目

- `cn-trust` の統合テストを追加し、`report/interactions` 取込 -> score 算出 -> `attestation(kind=39010)` 発行 -> `jobs/job_schedules` 更新までの一連フローを検証する

を実装し、完了状態に更新した。

## 実装内容

- `cn-trust` に統合テストモジュール `integration_tests.rs` を追加
  - `cn_relay.events` / `events_outbox` へ report(kind=39005) と interaction(kind=1 + `p`) を投入し、`handle_outbox_row` 経由で取込を実行
  - `cn_trust.report_scores` / `cn_trust.communication_scores` の算出結果と件数（report_count / interaction_count など）を検証
  - `cn_trust.attestations` の `event_json.kind == 39010`、claim タグ（`moderation.risk` / `reputation`）を検証
  - `ensure_job_schedules` -> `load_due_schedules` -> `enqueue_scheduled_job` -> `claim_job` -> `process_job` -> `finalize_job` を通し、`cn_trust.jobs` と `job_schedules.next_run_at` 更新を検証
- `cn-trust/src/lib.rs` を修正
  - AGE `cypher` 呼び出しを `$cypher$...$cypher$` 形式に変更し、PostgreSQL/AGE の引数制約に合わせた
  - `MERGE ... ON CREATE SET` 依存を避けるため、関係エッジ作成クエリを AGE 互換の `MERGE` 形へ調整
  - `ag_catalog.ag_graph` 存在確認の `query_scalar` 型を `i32` に修正（INT4 互換）
- `cn-trust/src/lib.rs` に `#[cfg(test)] mod integration_tests;` を追加

## 変更ファイル

- `kukuri-community-node/crates/cn-trust/src/lib.rs`
- `kukuri-community-node/crates/cn-trust/src/integration_tests.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-09.md`

## 検証

- `docker compose -f docker-compose.test.yml up -d community-node-postgres`（成功）
- `docker run --rm --network kukuri_community-node-network -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && rustup component add rustfmt && cargo fmt --all"`（成功）
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -lc "source /usr/local/cargo/env && cargo test -p cn-trust -- --nocapture"`（成功: 4 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
  - ログ: `tmp/logs/gh-act-format-check-20260209-171633.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功）
  - ログ: `tmp/logs/gh-act-native-test-linux-20260209-171750.log`

## 補足

- 統合テスト追加の過程で、`cn-trust` の AGE 実行クエリに環境依存の互換問題が露出したため、テスト追加と同時に本体実装を修正して CI で再検証した。
