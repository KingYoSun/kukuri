# Admin Console Privacy/Data・Index・Access Control 実装（2026年02月07日）

最終更新日: 2026年02月07日

## 概要

Community Nodes ロードマップの未実装項目だった Admin Console 拡張を完了した。`Privacy / Data`・`Index`・`Access Control` の 3 ページを追加し、ルーティングとナビゲーションを接続。あわせて Vitest + Testing Library の UI テスト基盤を導入し、新規ページの基本操作をテストで担保した。

## 実装内容

- 画面追加:
  - `Privacy / Data`: 現行ポリシー表示、relay/user-api 設定編集、関連監査ログ表示
  - `Index`: index 設定編集、`/v1/reindex` 起動フォーム、reindex 監査ログ表示
  - `Access Control`: epoch rotate と member revoke の実行フォーム、関連監査ログ表示
- 導線追加:
  - `src/router.tsx` に新規ルート（`/privacy-data`・`/index`・`/access-control`）を追加
  - `src/App.tsx` のサイドバーへ新規ナビゲーションを追加
- API クライアント拡張:
  - `rotateAccessControl` / `revokeAccessControl` / `reindex` を `src/lib/api.ts` に追加
  - 追加 API のレスポンス型を `src/lib/types.ts` に追加
- テスト基盤整備:
  - `package.json` に `vitest`・`@testing-library/*`・`jsdom` と test scripts を追加
  - `vite.config.ts` に `test` 設定を追加
  - `src/test/setup.ts` と `src/test/renderWithQueryClient.tsx` を追加
  - `PrivacyDataPage` / `IndexPage` / `AccessControlPage` の UI テストを追加

## 検証

- `docker run --rm -e CI=true -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node/apps/admin-console node:20-bookworm bash -lc "corepack enable && pnpm typecheck && pnpm test"` 成功
- `./scripts/test-docker.ps1 ts` 成功
- `gh act --workflows .github/workflows/test.yml --job format-check` 成功
  - ログ: `tmp/logs/gh-act-format-check-20260207-003615.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` 成功
  - ログ: `tmp/logs/gh-act-native-test-linux-20260207-003730.log`

## 備考

- `gh act` 実行時に `some refs were not updated` / `pnpm approve-builds` 警告が出るが、既知事象でジョブ成功を確認済み。
