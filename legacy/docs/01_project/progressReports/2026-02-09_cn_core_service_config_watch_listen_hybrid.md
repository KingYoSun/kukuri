# 2026年02月09日 `cn_core::service_config::watch_service_config` LISTEN + poll ハイブリッド反映

## 概要

- `cn_core::service_config::watch_service_config` に `LISTEN cn_admin_config` を追加し、`cn-admin-api` の `pg_notify` を設定反映経路として利用可能にした。
- 通知経路が切断・不達でも反映が止まらないよう、既存 poll をフォールバックとして維持するハイブリッド構成に拡張した。

## 実装内容

- `kukuri-community-node/crates/cn-core/src/service_config.rs`
  - `PgListener` で `cn_admin_config` を購読する処理を追加。
  - 通知 payload の service 判定（`service:version` 形式）を追加し、対象サービスの通知時のみ再読込するように変更。
  - listener 切断時の再接続（5秒リトライ）を追加。
  - poll ループを `tokio::time::interval` ベースに変更し、listener 経路と `tokio::select!` で並行待機する構成へ変更。
  - watcher 起動直後の通知取りこぼしを抑えるため、spawn 前に初回 `LISTEN` 接続を試行するよう調整。
  - テストを追加:
    - 通知 payload 判定のユニットテスト
    - `pg_notify` 経由で即時反映される統合テスト
    - 通知なしでも poll フォールバックで反映される統合テスト

## 検証

- `docker build -t kukuri-postgres-age ./kukuri-community-node/docker/postgres-age`（成功）
- `docker run -d --name cn-core-test-postgres-age ... -p 15432:5432 kukuri-postgres-age -c shared_preload_libraries=age`（成功）
- `docker run --rm -e DATABASE_URL=postgres://cn:cn_password@host.docker.internal:15432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -c "cargo test -p cn-core service_config::tests -- --nocapture"`（成功: 3 passed）
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功。ログ: `tmp/logs/gh-act-format-check-cn-core-config-final-20260210-025214.log`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global`（成功。ログ: `tmp/logs/gh-act-native-test-linux-cn-core-config-final-20260210-025334.log`）

## 結果

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の該当未実装項目を完了（[x]）へ更新。
