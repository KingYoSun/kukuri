# Issue #22 / PR #23 レビュー修正: advisory lock 衝突リスク低減

作成日: 2026年02月15日

## 背景

- 対象レビュー: `https://github.com/KingYoSun/kukuri/pull/23#discussion_r2809304474`
- 指摘内容: `pg_advisory_xact_lock(hashtext($1))` は 32-bit ハッシュのため、異なる pubkey 間の衝突で不要な直列化が起こり得る。

## 対応内容

- `cn-user-api` の pending 上限判定ロックを以下へ変更:
  - 変更前: `pg_advisory_xact_lock(hashtext($1))`
  - 変更後: `blake3` で pubkey から 64-bit 相当（2 x 32-bit）鍵を導出し、`pg_advisory_xact_lock($1, $2)` を使用
- 鍵導出ヘルパーを追加:
  - `advisory_lock_keys_for_pubkey(pubkey: &str) -> (i32, i32)`
  - 固定コンテキスト `cn-user-api.topic-subscription-request.pending-limit` を混ぜて導出
- テスト追加:
  - 同一 pubkey で鍵ペアが安定すること
  - 異なる pubkey で鍵ペアが異なること

## 変更ファイル

- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-15.md`
- `docs/01_project/progressReports/2026-02-15_issue22_pr23_advisory_lock_collision_fix.md`

## 検証

- Community Node（Docker 経路）
  - `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
  - `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`
  - `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- 実行結果
  - 1回目: `cn-admin-api` 契約テストで `40P01 deadlock detected`（既知 flaky）
  - 2回目: 全クレートテスト + `cn-cli` release build 成功
- `gh act`（`.github/workflows/test.yml`）
  - `XDG_CACHE_HOME=/tmp/xdg-cache ACT_CACHE_DIR=/tmp/xdg-cache/act NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
    - 成功
  - `XDG_CACHE_HOME=/tmp/xdg-cache ACT_CACHE_DIR=/tmp/xdg-cache/act NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
    - 成功
  - `XDG_CACHE_HOME=/tmp/xdg-cache ACT_CACHE_DIR=/tmp/xdg-cache/act NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`
    - 失敗（再実行でも失敗）
    - 失敗理由: `cn-admin-api` 契約テスト `admin_mutations_fail_when_audit_log_write_fails` が `tuple concurrently updated` / `trigger already exists` で不安定化（既知 flaky）

## 影響範囲

- pending 上限判定時の pubkey ロックキー生成のみ。
- API 契約（レスポンス構造・ステータス）や DB スキーマには変更なし。
