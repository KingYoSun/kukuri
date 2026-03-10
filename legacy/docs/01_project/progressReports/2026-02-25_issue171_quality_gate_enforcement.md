# Issue #171 品質ゲート運用不備修正レポート

作成日: 2026年02月25日

## 概要

- 対象:
  - PRテンプレート未使用でもマージできる
  - バグIssueが `verified-fixed` なしで close できる
- 事象:
  - `verify-close-condition` は実行されていたが、`bug` ラベルのみで判定していたため、`[Bug]` タイトルIssueを見逃していた。

## 根本原因

- close条件ワークフローが `hasBug = labels.includes('bug')` の単一条件だった。
- そのため `bug` ラベルが外れたバグIssue（例: `[Bug] ...`）では検証がスキップされた。
- PR本文に対する必須テンプレート検証ワークフローが存在せず、テンプレート未準拠をCIで防げなかった。

## 実装内容

- 変更ファイル:
  - `.github/workflows/verify-close-condition.yml`
  - `.github/workflows/validate-pr-template.yml`（新規）
  - `.github/pull_request_template.md`
  - `.github/ISSUE_TEMPLATE/bug_report.md`

1. close条件ワークフロー改善

- バグIssue判定を以下に拡張:
  - `bug` ラベル
  - タイトルが `^[Bug]` で始まる
  - 本文にバグテンプレート主要見出し（`再現手順` / `期待される動作`）を含む
- `verified-fixed` 未付与時は従来どおり再オープン + コメント。
- `bug` ラベル欠落時は自動で補完。

2. PRテンプレート品質ゲート追加

- `validate-pr-template.yml` を追加し、PRイベントで本文を検証。
- 必須チェック:
  - テンプレート必須セクション存在
  - `Closes #<issue番号>` 記載
  - `@codex` 記載
  - `## テスト手順` の未入力テンプレ状態禁止

3. テンプレート更新

- PRテンプレートに `Codexレビュー依頼（必須）` セクションと `@codex` 記載例を追加。
- bug_report に `bug` ラベル維持と `verified-fixed` 必須を明記。

## 検証結果

- `cargo fmt`: pass
- `pnpm format:check` / `pnpm lint`: pass（Docker node:22-bullseye経由）
- `gh act`:
  - `format-check`: pass
  - `native-test-linux`: pass
  - `community-node-tests`: pass
