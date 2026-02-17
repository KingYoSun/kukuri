# Issue #45 PR-2 locale drift 是正

最終更新日: 2026年02月17日

## 概要

Issue #45 の PR-2 スコープとして、`ja/en/zh-CN` の locale drift（キー欠落）を最小差分で解消し、
再発防止の軽量キー集合チェックを追加した。
本PRでは PR-3（時刻ロケール統一）には着手していない。

## 実施内容

1. `en.posts.submit` を追加
- ファイル: `kukuri-tauri/src/locales/en.json`
- 変更: `posts.submit: "Post"` を追加。
- 理由: `PostComposer` で参照される投稿送信ラベルのキー欠落を解消。

2. `zh-CN.bootstrapConfig.add` / `zh-CN.bootstrapConfig.noNodes` を追加
- ファイル: `kukuri-tauri/src/locales/zh-CN.json`
- 変更: `bootstrapConfig` セクションを追加し、`add` / `noNodes` を定義。
- 理由: `BootstrapConfigPanel` で参照されるキー欠落を解消。

3. locale key drift 再発防止の軽量チェックを追加
- ファイル: `scripts/check-locale-keys.mjs`
- ファイル: `kukuri-tauri/package.json`
- ファイル: `kukuri-tauri/src/tests/unit/i18n/localeKeyDrift.test.ts`
- 変更:
  - `ja/en/zh-CN` を再帰フラット化し、キー集合一致を検証するスクリプトを追加。
  - `pnpm check:locale-keys` コマンドを追加。
  - 同一検証をユニットテストでも実行するガードテストを追加。

## スコープ外（明示）

- PR-3: `toLocaleString` / `Intl.DateTimeFormat(undefined, ...)` の `i18n.language` 基準への統一。

## 検証コマンド

- `cd kukuri-tauri && pnpm check:locale-keys`
- `cd kukuri-tauri && pnpm vitest run src/tests/unit/i18n/localeKeyDrift.test.ts`
- `cd kukuri-tauri && pnpm exec prettier --check src/locales/en.json src/locales/zh-CN.json src/tests/unit/i18n/localeKeyDrift.test.ts package.json ../scripts/check-locale-keys.mjs`
- `cd /tmp/kukuri-issue45-pr2 && XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue45-pr2.log`
- `cd /tmp/kukuri-issue45-pr2 && bash -lc 'set -o pipefail; XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue45-pr2.log'`
- `cd /tmp/kukuri-issue45-pr2 && bash -lc 'set -o pipefail; XDG_CACHE_HOME=/tmp/xdg-cache NPM_CONFIG_PREFIX=/tmp/npm-global DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue45-pr2.log'`

## 検証結果

- `pnpm check:locale-keys`: pass（`Locale keys are aligned across ja, en, zh-CN (1050 keys).`）
- `localeKeyDrift.test.ts`: pass（1 file / 1 test）
- `prettier --check`（変更ファイル）: pass
- `gh act --job format-check`: pass（ログ: `tmp/logs/gh-act-format-check-issue45-pr2.log`）
- `gh act --job native-test-linux`: fail（既知要因、ログ: `tmp/logs/gh-act-native-test-linux-issue45-pr2.log`）
  - `failed to read plugin permissions ... app_hide.toml: No such file or directory`
- `gh act --job community-node-tests`: pass（ログ: `tmp/logs/gh-act-community-node-tests-issue45-pr2.log`）

## 影響範囲

- 影響は i18n リソース（`en` / `zh-CN`）とロケール検証ガード（script + test）のみに限定。
- アプリ挙動の機能追加や API 契約変更はなし。
