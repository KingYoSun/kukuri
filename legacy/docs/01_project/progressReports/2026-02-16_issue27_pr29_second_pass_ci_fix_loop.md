# Issue #27 / PR #29 fix loop second pass（CI run 22050188054）

最終更新日: 2026年02月16日

## 背景

- 対象 Workflow: https://github.com/KingYoSun/kukuri/actions/runs/22050188054
- 失敗 Job:
  - Native Test (Linux): https://github.com/KingYoSun/kukuri/actions/runs/22050188054/job/63706569762
  - Community Node Tests: https://github.com/KingYoSun/kukuri/actions/runs/22050188054/job/63706569780

## 原因

1. Native Test (Linux)
- 失敗テスト: `application::services::access_control_service::tests::request_join_with_invite_broadcasts_to_issuer_topic`
- 直接原因: `Invite signature is invalid`
- 根本原因: `DefaultSignatureService::sign_event` で署名した `signed_event.created_at` と、ドメインイベントの `event.created_at` が秒境界で不一致になる経路があり、`to_nostr_event` 時にシリアライズされる時刻と署名対象時刻がズレて `verify()` が不安定化していた。

2. Community Node Tests
- 失敗テスト: `subscriptions::api_contract_tests::search_contract_success_shape_compatible`
- 直接原因: `total` 期待値 `Some(2)` に対し実値 `Some(0)`
- 根本原因: `cn_search.runtime_flags` は DB グローバル状態で、PG backend 切替系テストと Meili 前提テストが並列実行されると backend が干渉する。`search_contract_success_shape_compatible` 側で Meili を前提にしているが、並列実行時に PG backend を読んで 0 件判定になるケースがあった。

## 修正内容

1. 署名時刻の同期（Tauri）
- 変更: `kukuri-tauri/src-tauri/src/infrastructure/crypto/default_signature_service.rs`
- 対応:
  - 署名後に `signed_event.created_at` を取得し、`event.created_at` へ同期。
  - 不正なタイムスタンプは `InvalidData` として明示的にエラー化。
- 効果: 署名対象と検証対象の `created_at` を一致させ、秒境界起因の `Invite signature is invalid` を防止。

2. 検索契約テストの backend 競合防止（Community Node）
- 変更: `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs`
- 対応:
  - `SEARCH_BACKEND_TEST_LOCK`（`tokio::sync::Mutex`）を導入し、search backend 切替系契約テストを直列化。
  - `search_contract_success_shape_compatible` 冒頭で runtime flag を `meili_only` / `meili` へ明示初期化。
  - 以下 3 テストで lock を取得:
    - `search_contract_success_shape_compatible`
    - `search_contract_pg_backend_switch_normalization_and_version_filter`
    - `search_contract_pg_backend_preserves_multi_topic_rows_for_same_post_id`
- 効果: `cn_search.runtime_flags` の並列干渉を抑制し、Meili/PG backend 切替のテストを決定的に実行。

## 検証

- `cd kukuri-tauri/src-tauri && cargo test request_join_with_invite_broadcasts_to_issuer_topic -- --nocapture`（pass）
- `cd kukuri-tauri/src-tauri && cargo test`（pass）
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test -p cn-user-api search_contract_success_shape_compatible -- --nocapture; cargo test -p cn-user-api search_contract_pg_backend_switch_normalization_and_version_filter -- --nocapture; cargo test -p cn-user-api search_contract_pg_backend_preserves_multi_topic_rows_for_same_post_id -- --nocapture"`（pass）
- `DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`（pass）
- `XDG_CACHE_HOME=/tmp/act-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check | tee tmp/logs/gh-act-format-check-issue27-pr29-second-pass.log`（pass）
- `XDG_CACHE_HOME=/tmp/act-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux | tee tmp/logs/gh-act-native-test-linux-issue27-pr29-second-pass.log`（pass）
- `XDG_CACHE_HOME=/tmp/act-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests | tee tmp/logs/gh-act-community-node-tests-issue27-pr29-second-pass.log`（fail）
  - 失敗テスト: `subscriptions::api_contract_tests::auth_consent_quota_metrics_regression_counters_increment`
  - 失敗内容: `left: 428`, `right: 402`
  - 備考: 今回修正した検索 backend 切替テスト群は pass。上記は別系統（auth/quota metrics）で、ログを保存して切り分け可能な状態。

## 再発防止ポイント

- 署名時刻は「署名後の実値」を常にドメインイベントへ反映する。
- `cn_search.runtime_flags` のような共有状態を変更する契約テストは mutex で直列化し、前提フラグを各テスト内で明示初期化する。
