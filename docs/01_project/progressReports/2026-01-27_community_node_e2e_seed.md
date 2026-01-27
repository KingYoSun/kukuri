# Community Node E2E 実ノードシード整備
日付: 2026年01月27日

## 概要
- 実ノード E2E/統合テスト向けに、DB/Meilisearch へシードを投入/掃除できる仕組みを追加。

## 対応内容
- `cn e2e seed/cleanup` を追加し、ユーザー/トピック/投稿/label/trust の固定シードを投入。
- `scripts/test-docker.ps1`/`scripts/test-docker.sh` の `e2e-community-node` で seed/cleanup を自動実行。

## 検証
- `gh act --workflows .github/workflows/test.yml --job format-check`（失敗: Docker pull 認証エラー `authentication required - incorrect username or password`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（失敗: Docker pull 認証エラー `authentication required - incorrect username or password`）

## 補足
- シードイベントは `seed=community-node-e2e` タグで識別し、cleanup で DB/Meilisearch から削除する。
