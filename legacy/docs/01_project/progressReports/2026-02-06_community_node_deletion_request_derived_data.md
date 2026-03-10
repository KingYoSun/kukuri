# Community Nodes 削除要求 派生データ削除・再計算

日付: 2026年02月06日

## 概要
- `cn-user-api` の個人データ削除フローに、Meilisearch/AGE/モデレーション/Trust の派生データ削除と再計算キュー投入を追加した。
- 削除要求完了時に `cn_relay`・`cn_moderation`・`cn_trust` の派生データが仕様どおり反映されることを統合テストで確認した。

## 実施内容
- `kukuri-community-node/crates/cn-user-api/src/personal_data.rs` の `perform_deletion` を DB トランザクション化。
- 削除対象 pubkey のイベント ID / topic ID / 影響 subject を事前抽出するヘルパーを追加。
- 派生データの削除フローを追加:
  - `cn_relay.events` の soft delete、`events_outbox(op=delete, reason=dsar)` 追加、`replaceable_current` / `addressable_current` 削除
  - `cn_moderation.labels` / `cn_moderation.jobs` 削除
  - `cn_trust.report_events` / `interactions` / `report_scores` / `communication_scores` / `attestations` 削除
  - Apache AGE グラフ (`kukuri_cn`) から対象 `User` 頂点を `DETACH DELETE`
- 再計算フローを追加:
  - `cn_index.reindex_jobs` へ topic 単位で `pending` 追加（`pending/running` 重複は抑止）
  - `cn_trust.jobs` へ `report_based` / `communication_density` を subject 単位で `pending` 追加（`pending/running` 重複は抑止）
- 回帰防止として `perform_deletion_removes_derived_data_and_enqueues_jobs` を追加し、削除・再計算・状態遷移を検証。

## 検証
- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -v ${PWD}:/app -w /app/kukuri-community-node kukuri-test-runner bash -lc "/usr/local/cargo/bin/rustup toolchain install 1.90.0 --profile minimal && /usr/local/cargo/bin/rustup component add --toolchain 1.90.0-x86_64-unknown-linux-gnu rustfmt && /usr/local/cargo/bin/cargo +1.90.0 fmt --all"`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v ${PWD}:/app -w /app/kukuri-community-node kukuri-test-runner bash -lc "/usr/local/cargo/bin/rustup toolchain install 1.90.0 --profile minimal && /usr/local/cargo/bin/cargo +1.90.0 run -p cn-cli -- migrate && /usr/local/cargo/bin/cargo +1.90.0 test -p cn-user-api --tests -- --nocapture"`
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功。ログ: `tmp/logs/gh-act-format-check-20260206-161920.log`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功。ログ: `tmp/logs/gh-act-native-test-linux-20260206-162037.log`）

## 補足
- `gh act` 実行時に `some refs were not updated` と `pnpm approve-builds` 警告、および一部テストの `useRouter` 警告は出るが、ジョブ自体は成功した。
