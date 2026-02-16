# Issue #27 PR-06 dual-write + backfill + shadow-read

最終更新日: 2026年02月16日

## 概要

Issue #27 の PR-06 スコープとして、検索 PG 移行の安全切替に必要な dual-write 片系失敗再送、backfill 基盤、shadow-read 検証を `kukuri-community-node` に実装した。
Meili を維持しつつ PG 経路の整合性確認を段階導入できるようにし、移行時の観測点と検証証跡を追加した。

## 実施内容

1. migration 追加（backfill/shadow 基盤）
- 追加: `kukuri-community-node/migrations/20260216080000_m12_search_backfill_shadow.sql`
- 追加テーブル:
  - `cn_search.backfill_jobs`
  - `cn_search.backfill_checkpoints`
  - `cn_search.shadow_read_logs`
- 追加 index:
  - `backfill_jobs_status_requested_at_idx`
  - `shadow_read_logs_recorded_at_idx`
  - `shadow_read_logs_path_idx`

2. dual-write 片系失敗再送（cn-index）
- 変更: `kukuri-community-node/crates/cn-index/src/lib.rs`
- `DualWriteFailureMarker` を導入し、`search_write_mode=dual` で片系失敗時に outbox を未commitで再処理可能に維持。
- `capture_write_side_failure` で backend/operation/seq/event_id/topic_id を警告ログへ出力。
- 再処理成功時に `search_dual_write_retries_total` を増分し、失敗側再送が成立したことを可視化。

3. backfill worker 実装（cn-index）
- 変更: `kukuri-community-node/crates/cn-index/src/lib.rs`
- `spawn_backfill_worker` を追加し、`backfill_jobs` の `pending -> running -> succeeded/failed` 遷移を実装。
- `high_watermark_seq`・`backfill_checkpoints` で再開可能な chunk 処理を実装。
- `processed_rows` と ETA を定期更新し、完了時に ETA を 0 にリセット。

4. shadow-read 比較保存（cn-user-api）
- 変更: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `/v1/search` で primary backend を返却しつつ、sampled 時に secondary backend を同時実行。
- `overlap@10` と `latency delta` を計算し、`cn_search.shadow_read_logs` へ保存。
- migration 未適用環境では `42P01/3F000` を検出して warn にフォールバックし、後方互換を維持。

5. observability 拡張（cn-core）
- 変更: `kukuri-community-node/crates/cn-core/src/metrics.rs`
- 追加メトリクス:
  - `backfill_processed_rows`
  - `backfill_eta_seconds`
  - `shadow_overlap_at_10`
  - `shadow_latency_delta_ms`
  - `search_dual_write_errors_total`
  - `search_dual_write_retries_total`

6. テスト拡張
- 変更: `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
  - `outbox_dual_write_retries_after_pg_side_failure_and_recovers`
  - `backfill_job_resumes_from_checkpoint_and_completes_post_search_documents`
- 変更: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
  - `search_shadow_read_logs_overlap_and_latency_when_sampled`
- fix loop:
  - `backfill_job_resumes_from_checkpoint_and_completes_post_search_documents` が full suite で他 fixture 混入するため、テスト `topic_id` を高ソートキーへ変更して再現性を固定。

## 後方互換性

- `search_read_backend` / `search_write_mode` の既定値（Meili 優先）を維持。
- shadow-read は `search_shadow_sample_rate` で段階導入可能。
- `shadow_read_logs` テーブルが未作成でも API 応答は継続し、ログのみ degrade する。

## 検証

- `cd /home/kingyosun/kukuri/kukuri-community-node && cargo fmt`（pass）
- `cd /home/kingyosun/kukuri && docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-index outbox_dual_write_updates_meili_and_post_search_documents -- --nocapture; cargo test -p cn-index outbox_dual_write_retries_after_pg_side_failure_and_recovers -- --nocapture; cargo test -p cn-index backfill_job_resumes_from_checkpoint_and_completes_post_search_documents -- --nocapture; cargo test -p cn-user-api search_contract_success_shape_compatible -- --nocapture; cargo test -p cn-user-api search_contract_pg_backend_switch_normalization_and_version_filter -- --nocapture; cargo test -p cn-user-api search_contract_pg_backend_preserves_multi_topic_rows_for_same_post_id -- --nocapture; cargo test -p cn-user-api search_shadow_read_logs_overlap_and_latency_when_sampled -- --nocapture"`（pass）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `cd /home/kingyosun/kukuri/kukuri-tauri/src-tauri && cargo test`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr06.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr06.log`（pass）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr06.log`（fail: `backfill_job_resumes_from_checkpoint_and_completes_post_search_documents` の `processed_rows` 期待値不一致）
- `cd /home/kingyosun/kukuri && DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-index backfill_job_resumes_from_checkpoint_and_completes_post_search_documents -- --nocapture"`（pass: fix loop 単体確認）
- `cd /home/kingyosun/kukuri && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr06-rerun.log`（pass）

## 変更ファイル（主要）

- `kukuri-community-node/migrations/20260216080000_m12_search_backfill_shadow.sql`
- `kukuri-community-node/crates/cn-core/src/metrics.rs`
- `kukuri-community-node/crates/cn-index/src/lib.rs`
- `kukuri-community-node/crates/cn-index/src/integration_tests.rs`
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/priority/search_pg_migration_roadmap.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-16.md`
