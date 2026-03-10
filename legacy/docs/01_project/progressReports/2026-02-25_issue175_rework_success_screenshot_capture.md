# Issue #175 再実装: 成功時スクリーンショット取得の担保

作成日: 2026年02月25日

## 背景

Issue #175 は reopen 後の再実装対象となり、前回実装では Playwright 成功時にスクリーンショットが生成されず、
R2 アップロード対象が 0 件になっていた。

## 実装内容

1. 成功時スクリーンショット生成の有効化

- 対象ファイル: `kukuri-tauri/playwright.config.ts`
- 変更: `use.screenshot` を `only-on-failure` から `on` へ変更
- 意図: テスト成功時でも `kukuri-tauri/test-results` に画像を確実に生成し、R2 アップロード対象を作る

2. CI 出力の URL 可視性改善

- 対象ファイル: `.github/workflows/test.yml`
- 変更:
  - R2 アップロード step 名を `Upload Playwright screenshots (success + failure) to Cloudflare R2` へ更新
  - `tmp/playwright-r2-urls.tsv` の path を step output (`urls_path`) として公開
  - `tmp/playwright-r2-comment.md` / `tmp/playwright-r2-urls.tsv` を Playwright artifact に同梱
- 意図: URL 一覧を PR コメントだけに依存せず、CI artifact からも追跡・再確認できるようにする

3. 失敗診断の維持

- 既存の `kukuri-tauri/playwright-report` と `kukuri-tauri/test-results` artifact 出力を保持
- no-screenshots 時のメッセージを「設定・ログ確認」導線へ更新し、原因調査をしやすくした

## 変更ファイル

- `.github/workflows/test.yml`
- `kukuri-tauri/playwright.config.ts`
- `docs/01_project/activeContext/tasks/completed/2026-02-25.md`
- `docs/01_project/progressReports/2026-02-25_issue175_rework_success_screenshot_capture.md`

## 検証

- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
  - 結果: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - 結果: pass
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - 結果: pass

## 補足

- R2 実アップロード URL の確認は GitHub Actions の `Playwright Tauri Smoke` 実行結果で実施し、Issue #175 の監査コメントへ証跡リンクを記録する。
