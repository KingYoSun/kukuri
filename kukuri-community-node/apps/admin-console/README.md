# cn-admin-console

Community Node の運用管理向け Admin Console（React + TypeScript + Vite）。

## 開発

```bash
pnpm dev
```

## ビルド

```bash
pnpm build
```

## 型チェック

```bash
pnpm typecheck
```

## UI テスト（Vitest + Testing Library）

```bash
pnpm test
```

テスト基盤:
- `vite.config.ts` の `test` 設定（`jsdom` / `setupFiles`）
- `src/test/setup.ts`（`@testing-library/jest-dom` と cleanup）
- `src/test/renderWithQueryClient.tsx`（React Query ラッパー）
