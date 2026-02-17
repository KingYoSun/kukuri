# Issue #52 Desktop E2E failure fix

最終更新日: 2026年02月17日

## 概要

Issue #52（Run `22075452530` / Job `63789452806`）で失敗していた `Desktop E2E (Community Node, Docker)` を修正した。
初回再現では `Spec Files: 16 passed, 1 failed` で、`sync.offline-status.spec.ts` の再送キュー更新ボタン探索が失敗した。
セレクタを `aria-label` 対応に拡張した上で再実行し、`Spec Files: 17 passed, 17 total` を確認した。

## 実施内容

1. 失敗ジョブログを取得し、失敗 spec とエラーメッセージを抽出。
2. E2E spec のセレクタを i18n と UI 実装（テキスト/aria-label）に合わせて最小修正。
3. トピック遷移の待機処理を補強し、`create-post-button` test id を UI に追加。
4. `scripts/test-docker.sh e2e-community-node` を再実行し、17/17 pass を確認。

## 変更ファイル

- `kukuri-tauri/src/routes/topics.$topicId.tsx`
- `kukuri-tauri/tests/e2e/specs/community-node.friend-plus.spec.ts`
- `kukuri-tauri/tests/e2e/specs/community-node.invite.spec.ts`
- `kukuri-tauri/tests/e2e/specs/direct-messages.inbox.spec.ts`
- `kukuri-tauri/tests/e2e/specs/sync.offline-status.spec.ts`

## 検証

- `DOCKER_CONFIG=/tmp/.docker bash scripts/test-docker.sh e2e-community-node`
  - 初回: `16 passed, 1 failed`（`sync.offline-status`）
  - 修正後: `17 passed, 17 total`
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job format-check`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）

ログ:

- `/tmp/issue52_e2e.log`
- `/tmp/issue52_e2e_rerun.log`
- `tmp/logs/gh-act-format-check-issue52.log`
- `tmp/logs/gh-act-native-test-linux-issue52.log`
- `tmp/logs/gh-act-community-node-tests-issue52.log`
