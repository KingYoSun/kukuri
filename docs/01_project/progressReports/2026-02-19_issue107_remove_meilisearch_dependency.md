# Issue #107 Meilisearch dependency 完全撤去（PG-only）

作成日: 2026年02月19日
Issue: https://github.com/KingYoSun/kukuri/issues/107
PR: https://github.com/KingYoSun/kukuri/pull/108
対象ブランチ: `chore/issue-107-remove-meilisearch-dependency`
handoff_id: `LHO-meilisearch-removal-20260219-1427Z`

## 背景

- Community Node 検索基盤を PostgreSQL 側に一本化し、Meilisearch 依存をランタイム/運用経路から外す必要があった。
- 目的は「既存機能を壊さず、最小安全変更で PG-only へ移行する」こと。

## 実施内容

- Community Node コードから Meilisearch runtime 依存を削除。
  - `cn-core`: `meili` モジュール廃止、runtime flag 既定値を PG-only 化。
  - `cn-user-api`: `MEILI_*` 読み込み/healthz/read-path を削除し PG 検索へ一本化。
  - `cn-index`: dual-write / meili health / meili reindex を廃止し `cn_search.post_search_documents` 更新に集約。
  - `cn-cli e2e_seed`: Meili投入/削除を廃止し PG search documents へ直接反映。
- DB migration を追加し、運用時の runtime flag を PG-only 既定へ補正。
  - `kukuri-community-node/migrations/20260219000000_m13_remove_meili_runtime_dependency.sql`
- compose / scripts / CI から Meilisearch コンテナ依存を削除。
  - `docker-compose.test.yml`
  - `kukuri-community-node/docker-compose.yml`
  - `.github/workflows/test.yml`
  - `scripts/test-docker.sh`
  - `scripts/test-docker.ps1`
- 主要ドキュメントと README 群を postgres-only 手順へ更新。

## 検証

- Docker 経路で Community Node の全テストと `cn-cli` release build を実行。
  - `cargo test --workspace --all-features`
  - `cargo build --release -p cn-cli`
- required CI 相当ジョブを `gh act` で実行。
  - `format-check`（初回 format 差分で fail、`cargo fmt` 後に pass）
  - `native-test-linux`（pass）
  - `community-node-tests`（pass）

## 結果

- Meilisearch は Community Node の実行・テスト経路から除去され、PG-only 構成で動作確認を完了。
- CI 必須3ジョブは最終的にすべて成功。
