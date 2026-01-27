# Desktop E2E (Community Node) CI 失敗修正
日付: 2026年01月27日

## 概要
- Desktop E2E の community node seed/cleanup が `docker compose run` 経由で CLI 引数が欠落し CI 失敗していたため、entrypoint 指定を修正。

## 対応内容
- `scripts/test-docker.ps1` の `Invoke-CommunityNodeE2ESeed/Cleanup` を `--entrypoint cn` で実行。
- `scripts/test-docker.sh` の seed/cleanup も `--entrypoint cn` に統一。

## 検証
- `gh run view 21398562889 --log-failed`（Desktop E2E の失敗原因を確認）
- `gh act --workflows .github/workflows/test.yml --job desktop-e2e`（失敗: act-latest で `pwsh` が見つからず）
- `gh act --workflows .github/workflows/test.yml --job desktop-e2e -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest`（中断: image pull が長時間）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功。警告: `git clone` の `some refs were not updated`、`pnpm approve-builds`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功。警告: `git clone` の `some refs were not updated`）

## 補足
- `docker-compose.test.yml` の `community-node-user-api` は `/bin/sh -c` entrypoint のため、`docker compose run` では `--entrypoint cn` を使う必要がある。
