# Issue #175 E2Eスクリーンショット Cloudflare R2 永続化

作成日: 2026年02月25日

## 概要

Issue #175 の要件に合わせて、`Test` workflow の `playwright-tauri-smoke` ジョブへ Cloudflare R2 連携を追加した。
Playwright が生成したスクリーンショットを R2 へアップロードし、URL を PR コメントへ自動投稿できる状態にした。
あわせて、完了通知の webhook callback（`flags: 4`）を追加した。

## 実装内容

1. Playwright スクリーンショットの R2 自動アップロード

- 対象ファイル: `.github/workflows/test.yml`
- 追加ステップ: `Upload Playwright screenshots to Cloudflare R2`
- 動作:
  - `kukuri-tauri/test-results` 配下の `png/jpg/jpeg/webp` を探索
  - `s3://kukuri-screenshots/playwright/<repo>/<run_id>/<run_attempt>/<sha>/...` へアップロード
  - 認証は GitHub Secrets（`CLOUDFLARE_R2_ENDPOINT`, `CLOUDFLARE_R2_ACCESS_KEY`, `CLOUDFLARE_R2_SECRET`）を利用

2. URL 生成とフォールバック

- 公開URLは Cloudflare API (`domains/managed`, `domains/custom`) から解決。
- 公開ドメインが解決できない場合は `aws s3 presign` で 7日署名付き URL を生成するフォールバックを実装。
- 生成結果を `tmp/playwright-r2-comment.md` にまとめ、PRコメント/ジョブサマリーで共有する。

3. PR コメント自動投稿

- 追加ステップ: `Comment PR with Playwright screenshot URLs`
- 実装:
  - `actions/github-script@v7` で PR コメントを作成/更新
  - 固定マーカー `<!-- playwright-r2-screenshots -->` で同一コメントを更新
  - fork PR では `Skip PR comment for fork pull requests` でスキップ

4. completion webhook callback (`flags: 4`)

- 追加ステップ: `Send completion webhook callback (Playwright screenshots)`
- `DISCORD_WEBHOOK_URL` がある場合のみ送信し、payload は `{"content": ..., "flags": 4}` を利用。
- webhook 未設定時は `Skip completion webhook callback (webhook not configured)` で明示スキップ。

## 変更ファイル

- `.github/workflows/test.yml`
- `docs/01_project/activeContext/tasks/completed/2026-02-25.md`
- `docs/01_project/progressReports/2026-02-25_issue175_playwright_screenshots_r2.md`

## 検証

- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-test-format-check-issue175-rerun.log`
  - 結果: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-test-native-test-linux-issue175-rerun.log`
  - 結果: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-test-community-node-tests-issue175-rerun.log`
  - 結果: pass
