# Issue #45 / PR #49 Format Check fix loop

最終更新日: 2026年02月16日

## 概要

PR #49 の CI `Format Check` が失敗したため、原因を特定し、PR-1（i18nキー整合）の意図を崩さない最小修正で解消した。

- 対象Run: `https://github.com/KingYoSun/kukuri/actions/runs/22070578309`
- 対象Job: `Format Check`（Job ID `63773790779`）

## 失敗原因（確定）

`Check TypeScript formatting` ステップの `pnpm format:check` が失敗し、`kukuri-tauri/src` 配下の40ファイルを未整形として検出した。

- エラー要点: `Code style issues found in 40 files. Run Prettier with --write to fix.`
- 失敗種別: 整形差分のみ（型・実装ロジックのエラーではない）。

## 方針（最小修正）

PR-1の本来目的は i18n キー整合であり、今回の失敗は整形ドリフトに起因する。
そのため、以下を採用した。

1. CIで指摘された40ファイルのみに `prettier --write` を適用。
2. ロジック変更・キー追加変更は実施しない。
3. `format-check` 再実行で通過を確認。

## 実施内容

1. `gh api` で Job ログを直接取得して失敗ステップを確定。
2. `pnpm format:check` をローカル再現し、同一40ファイルで失敗を確認。
3. 指摘40ファイル限定で `prettier --write` を実施。
4. `pnpm format:check` 再実行で pass を確認。
5. AGENTS要件に従い `gh act` 3ジョブを実施しログを収集。

## 検証結果

- `gh act --job format-check`: pass  
  ログ: `tmp/logs/gh-act-format-check-issue45-pr49-fix-loop.log`
- `gh act --job native-test-linux`: fail（既知）  
  i18n文言の期待値ズレ、および `OfflineIndicator.test.tsx` の `date-fns/locale` mock 不整合により多数失敗。  
  ログ: `tmp/logs/gh-act-native-test-linux-issue45-pr49-fix-loop.log`
- `gh act --job community-node-tests`: pass  
  ログ: `tmp/logs/gh-act-community-node-tests-issue45-pr49-fix-loop.log`

## 影響範囲

- 変更は Prettier 整形のみ。
- Issue #45 PR-1 の機能意図（i18nキー不整合修正）を保持。
- API仕様・ドメインロジック・Rust実装への影響なし。
